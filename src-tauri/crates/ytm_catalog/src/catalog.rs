use crate::convert::{self, TrackParts};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use tokio::sync::OnceCell;
use yamp_media::{
  AlbumPage, ArtistPage, Catalog, Item, ItemKind, MediaError, Playlist, Result, TopSong, Track,
};
use ytmapi_rs::{
  auth::noauth::NoAuthToken,
  common::{AlbumID, AlbumType, ArtistChannelID, PlaylistID, YoutubeID},
  parse::{AlbumResult, PlaylistItem, SearchResultPlaylist, SearchResultVideo},
  YtMusic, YtMusicBuilder,
};

fn failed(e: ytmapi_rs::Error) -> MediaError {
  MediaError::Engine(format!("youtube music: {e}"))
}

fn checked(id: &str) -> Result<&str> {
  convert::valid_id(id).then_some(id).ok_or(MediaError::InvalidId)
}

fn video_track(video: &SearchResultVideo) -> Option<Track> {
  // the other kind of result is a podcast episode
  match video {
    SearchResultVideo::Video { title, channel_name, video_id, length, thumbnails, .. } => {
      Some(convert::track(TrackParts {
        id: video_id.get_raw(),
        title: title.clone(),
        artist: channel_name.clone(),
        album: None,
        duration: length,
        thumbnails,
        release_year: None,
      }))
    }
    _ => None,
  }
}

fn release_item(album: &AlbumResult, artist: &str, label: &str) -> Item {
  Item {
    id: album.album_id.get_raw().to_owned(),
    kind: ItemKind::Album,
    title: album.title.clone(),
    subtitle: Some(artist.to_owned()),
    label: Some(label.to_owned()),
    thumbnail: convert::best_thumbnail(&album.thumbnails),
    release_year: convert::year(&album.year),
  }
}

fn release_label(kind: &AlbumType) -> Option<&'static str> {
  match kind {
    AlbumType::Album => Some("Album"),
    AlbumType::Single => Some("Single"),
    AlbumType::EP => Some("EP"),
    _ => None,
  }
}

/// The home page of YouTube Music answers with response headers well over the 16 KiB that hyper
/// accepts by default, and the whole exchange is then refused as a protocol error. Every call
/// starts with that page, so none of them works without a higher limit.
const MAX_HEADER_BYTES: u32 = 1 << 20;

/// Visitors from the EU are redirected to a consent page before they reach the real one, and
/// that page has none of the configuration the first call reads. The cookie is the one that
/// records a choice as made, which is how other clients of this API get past the page.
const CONSENT_COOKIE: &str = "SOCS=CAI";

async fn connect() -> Result<YtMusic<NoAuthToken>> {
  let mut headers = HeaderMap::new();
  headers.insert(COOKIE, HeaderValue::from_static(CONSENT_COOKIE));
  let http = reqwest::Client::builder()
    .use_rustls_tls()
    .default_headers(headers)
    .http2_max_header_list_size(MAX_HEADER_BYTES)
    .build()
    .map_err(|e| MediaError::Network(format!("couldn't build the http client: {e}")))?;
  YtMusicBuilder::new_with_client(ytmapi_rs::Client::new_from_reqwest_client(http)).build().await.map_err(failed)
}

/// Reads YouTube Music's own API, the one its web app uses, without an account.
///
/// The handle is made on first use and kept, since making it asks the service for a session.
/// A failure to make it is not kept, so the next call tries again.
#[derive(Default)]
pub struct YtMusicCatalog {
  client: OnceCell<YtMusic<NoAuthToken>>,
}

impl YtMusicCatalog {
  pub fn new() -> Self {
    Self::default()
  }

  async fn api(&self) -> Result<&YtMusic<NoAuthToken>> {
    self.client.get_or_try_init(connect).await
  }
}

#[async_trait]
impl Catalog for YtMusicCatalog {
  async fn search_songs(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
    if query.trim().is_empty() {
      return Ok(vec![]);
    }
    let songs = self.api().await?.search_songs(query.trim()).await.map_err(failed)?;
    Ok(
      songs
        .into_iter()
        .take(limit)
        .map(|s| {
          convert::track(TrackParts {
            id: s.video_id.get_raw(),
            album: s.album.map(|a| a.name),
            duration: &s.duration,
            thumbnails: &s.thumbnails,
            release_year: None,
            title: s.title.clone(),
            artist: s.artist.clone(),
          })
        })
        .collect(),
    )
  }

  async fn search_videos(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
    if query.trim().is_empty() {
      return Ok(vec![]);
    }
    let videos = self.api().await?.search_videos(query.trim()).await.map_err(failed)?;
    Ok(videos.iter().filter_map(video_track).take(limit).collect())
  }

  async fn search_items(&self, query: &str, kind: ItemKind, limit: usize) -> Result<Vec<Item>> {
    let q = query.trim();
    if q.is_empty() {
      return Ok(vec![]);
    }
    let api = self.api().await?;
    let items: Vec<Item> = match kind {
      ItemKind::Artist => api
        .search_artists(q)
        .await
        .map_err(failed)?
        .into_iter()
        .map(|a| Item {
          id: a.browse_id.get_raw().to_owned(),
          kind,
          title: a.artist.clone(),
          subtitle: a.subscribers.clone(),
          label: None,
          thumbnail: convert::best_thumbnail(&a.thumbnails),
          release_year: None,
        })
        .collect(),
      ItemKind::Album => api
        .search_albums(q)
        .await
        .map_err(failed)?
        .into_iter()
        .map(|a| Item {
          id: a.album_id.get_raw().to_owned(),
          kind,
          title: a.title.clone(),
          subtitle: Some(a.artist.clone()),
          label: release_label(&a.album_type).map(str::to_owned),
          thumbnail: convert::best_thumbnail(&a.thumbnails),
          release_year: convert::year(&a.year),
        })
        .collect(),
      ItemKind::Playlist => api
        .search_playlists(q)
        .await
        .map_err(failed)?
        .into_iter()
        .filter_map(|p| match p {
          // a podcast comes back among the playlists, and it is not one
          SearchResultPlaylist::Featured(p) => Some(("Featured", p.playlist_id, p.title, p.author, p.thumbnails)),
          SearchResultPlaylist::Community(p) => Some(("Playlist", p.playlist_id, p.title, p.author, p.thumbnails)),
          _ => None,
        })
        .map(|(label, id, title, author, thumbnails)| Item {
          id: id.get_raw().to_owned(),
          kind,
          title,
          subtitle: Some(author),
          label: Some(label.to_owned()),
          thumbnail: convert::best_thumbnail(&thumbnails),
          release_year: None,
        })
        .collect(),
    };
    Ok(items.into_iter().take(limit).collect())
  }

  async fn artist(&self, id: &str) -> Result<ArtistPage> {
    let id = checked(id)?;
    let artist = self.api().await?.get_artist(ArtistChannelID::from_raw(id)).await.map_err(failed)?;
    let name = artist.name.clone();
    let releases = artist.top_releases;

    let top_songs = releases
      .songs
      .into_iter()
      .flat_map(|s| s.results)
      .map(|s| TopSong {
        plays: Some(s.plays.clone()).filter(|p| !p.is_empty()),
        track: convert::track(TrackParts {
          id: s.video_id.get_raw(),
          title: s.title.clone(),
          artist: convert::artist_names(&s.artists),
          album: Some(s.album.name.clone()),
          // a top song comes without a length or a cover of its own
          duration: "",
          thumbnails: &[],
          release_year: None,
        }),
      })
      .collect();

    let albums = releases.albums.iter().flat_map(|a| &a.results).map(|a| release_item(a, &name, "Album")).collect();
    let singles = releases.singles.iter().flat_map(|a| &a.results).map(|a| release_item(a, &name, "Single")).collect();
    let videos = releases.videos.iter().flat_map(|v| &v.results).filter_map(video_track).collect();
    let related = releases
      .related
      .into_iter()
      .flat_map(|r| r.results)
      .map(|r| Item {
        id: r.browse_id.get_raw().to_owned(),
        kind: ItemKind::Artist,
        title: r.title.clone(),
        subtitle: Some(r.subscribers.clone()).filter(|s| !s.is_empty()),
        label: None,
        thumbnail: None,
        release_year: None,
      })
      .collect();

    Ok(ArtistPage {
      id: id.to_owned(),
      description: artist.description.clone(),
      audience: artist.subscribers.clone().or_else(|| artist.views.clone()),
      thumbnail: convert::best_thumbnail(&artist.thumbnails),
      top_songs,
      albums,
      singles,
      videos,
      related,
      name,
    })
  }

  async fn album(&self, id: &str) -> Result<AlbumPage> {
    let id = checked(id)?;
    let album = self.api().await?.get_album(AlbumID::from_raw(id)).await.map_err(failed)?;
    let artist = convert::artist_names(&album.artists);
    let year = convert::year(&album.year);
    let thumbnail = convert::best_thumbnail(&album.thumbnails);

    // The tracks of an album are listed without artist, album or cover, those are the album's.
    let tracks = album
      .tracks
      .iter()
      .map(|t| {
        let mut track = convert::track(TrackParts {
          id: t.video_id.get_raw(),
          title: t.title.clone(),
          artist: artist.clone(),
          album: Some(album.title.clone()),
          duration: &t.duration,
          thumbnails: &album.thumbnails,
          release_year: year,
        });
        track.thumbnail = thumbnail.clone().or(track.thumbnail);
        track
      })
      .collect();

    Ok(AlbumPage {
      id: id.to_owned(),
      title: album.title.clone(),
      release_type: release_label(&album.category).map(str::to_owned),
      artist,
      year,
      thumbnail,
      tracks,
    })
  }

  async fn playlist(&self, id: &str) -> Result<Playlist> {
    let id = checked(id)?;
    let api = self.api().await?;
    // the title is a nicety, a playlist that lists its tracks is still worth showing
    let (items, details) =
      tokio::join!(api.get_playlist_tracks(PlaylistID::from_raw(id)), api.get_playlist_details(PlaylistID::from_raw(id)));
    let items = items.map_err(failed)?;
    let title = details.map(|d| d.title).unwrap_or_else(|e| {
      log::debug!("no details for playlist {id}: {e}");
      String::new()
    });

    let tracks = items
      .iter()
      .filter_map(|item| match item {
        PlaylistItem::Song(s) if s.is_available => Some(convert::track(TrackParts {
          id: s.video_id.get_raw(),
          title: s.title.clone(),
          artist: convert::artist_names(&s.artists),
          album: Some(s.album.name.clone()),
          duration: &s.duration,
          thumbnails: &s.thumbnails,
          release_year: None,
        })),
        PlaylistItem::Video(v) if v.is_available => Some(convert::track(TrackParts {
          id: v.video_id.get_raw(),
          title: v.title.clone(),
          artist: v.channel_name.clone(),
          album: None,
          duration: &v.duration,
          thumbnails: &v.thumbnails,
          release_year: None,
        })),
        // episodes and uploads are not something this app plays
        _ => None,
      })
      .collect();

    Ok(Playlist { id: id.to_owned(), title, tracks })
  }
}
