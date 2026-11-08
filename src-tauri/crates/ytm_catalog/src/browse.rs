//! Reads the explore area of YouTube Music from its raw responses, since ytmapi-rs has no query
//! for any of it.
//!
//! A response is a deeply nested tree whose upper layers change between regions and over time, so
//! the renderers of interest are searched for instead of followed along a fixed path. A renderer
//! itself keeps its shape. The shelves of the explore page are told apart by the browse id their
//! title links to, because the title text follows the language. Nothing here does I/O. The
//! functions take parsed JSON, which keeps them testable without the network.
use crate::convert;
use serde_json::{json, Value};
use std::collections::HashSet;
use yamp_media::{Explore, Item, ItemKind, Mood, Track};

const ARTIST_PAGE: &str = "MUSIC_PAGE_TYPE_ARTIST";
const ALBUM_PAGE: &str = "MUSIC_PAGE_TYPE_ALBUM";
const PLAYLIST_PAGE: &str = "MUSIC_PAGE_TYPE_PLAYLIST";

const MAX_TRENDING: usize = 20;
const MAX_ARTISTS: usize = 20;

/// A page of the explore area, each one is read with a browse request.
pub enum Page<'a> {
  Explore,
  /// The chart of a country, see [`chart_country`].
  Charts(Option<&'a str>),
  /// The playlists of a mood or a genre. `params` comes from [`Mood`] and is checked by [`valid_params`].
  Mood(&'a str),
}

impl Page<'_> {
  pub fn body(&self, version: &str) -> Value {
    let mut body = json!({
      // The language is pinned, since the app is in English and so are the titles of a mood.
      "context": { "client": { "clientName": "WEB_REMIX", "clientVersion": version, "hl": "en" } },
    });
    match self {
      Page::Explore => body["browseId"] = json!("FEmusic_explore"),
      Page::Charts(country) => {
        body["browseId"] = json!("FEmusic_charts");
        body["formData"] = json!({ "selectedValues": [chart_country(*country)] });
      }
      Page::Mood(params) => {
        body["browseId"] = json!("FEmusic_moods_and_genres_category");
        body["params"] = json!(params);
      }
    }
    body
  }
}

/// A chart is always asked for a country. Without a usable one the answer would depend on where
/// the request seems to come from, and worldwide is the provider's code for a chart that doesn't.
pub fn chart_country(country: Option<&str>) -> &str {
  country.filter(|c| c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase())).unwrap_or("ZZ")
}

/// The params of a mood go into a request, so only the characters the provider itself uses are accepted.
pub fn valid_params(params: &str) -> bool {
  !params.is_empty()
    && params.len() <= 512
    && params.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'%' | b'=' | b'.'))
}

/// The web client of YouTube Music is named by the date of its build, and any recent one is
/// answered. Today's date is what the web app itself sends, so it is never out of reach.
pub fn client_version(unix_secs: u64) -> String {
  let (year, month, day) = civil_date((unix_secs / 86_400) as i64);
  format!("1.{year:04}{month:02}{day:02}.01.00")
}

/// The calendar date of a day count from 1970-01-01. This is the days to civil algorithm of
/// Howard Hinnant, which saves a date crate for one string.
fn civil_date(days: i64) -> (i64, u32, u32) {
  let z = days + 719_468;
  let era = z.div_euclid(146_097);
  let day_of_era = z.rem_euclid(146_097);
  let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
  let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
  let shifted_month = (5 * day_of_year + 2) / 153;
  let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
  let month = (if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 }) as u32;
  let year = year_of_era + era * 400 + i64::from(month <= 2);
  (year, month, day)
}

// SECTION: searching the tree

/// Every value stored under one of `keys` below `node`, in page order. A match is not searched
/// any deeper, the renderers do not nest in each other.
fn all<'a>(node: &'a Value, keys: &[&str]) -> Vec<&'a Value> {
  fn collect<'a>(node: &'a Value, keys: &[&str], out: &mut Vec<&'a Value>) {
    match node {
      Value::Object(map) => {
        for (key, value) in map {
          if keys.contains(&key.as_str()) {
            out.push(value);
          } else {
            collect(value, keys, out);
          }
        }
      }
      Value::Array(list) => list.iter().for_each(|v| collect(v, keys, out)),
      _ => {}
    }
  }
  let mut out = Vec::new();
  collect(node, keys, &mut out);
  out
}

/// The first string stored under `key` anywhere below `node`.
fn find_str<'a>(node: &'a Value, key: &str) -> Option<&'a str> {
  match node {
    Value::Object(map) => {
      map.get(key).and_then(Value::as_str).or_else(|| map.values().find_map(|v| find_str(v, key)))
    }
    Value::Array(list) => list.iter().find_map(|v| find_str(v, key)),
    _ => None,
  }
}

fn runs_at<'a>(node: &'a Value, pointer: &str) -> &'a [Value] {
  node.pointer(pointer).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn flex_runs(row: &Value, column: usize) -> &[Value] {
  runs_at(row, &format!("/flexColumns/{column}/musicResponsiveListItemFlexColumnRenderer/text/runs"))
}

fn run_text(run: &Value) -> &str {
  run.get("text").and_then(Value::as_str).unwrap_or("")
}

fn joined(runs: &[Value]) -> String {
  runs.iter().map(run_text).collect()
}

/// The non empty pieces of a line, without the dots that separate them.
fn parts(runs: &[Value]) -> Vec<&str> {
  runs.iter().map(run_text).map(str::trim).filter(|t| !t.is_empty() && *t != "•").collect()
}

fn page_type(node: &Value) -> Option<&str> {
  node
    .pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType")
    .and_then(Value::as_str)
}

fn browse_id(node: &Value) -> Option<&str> {
  node.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(Value::as_str)
}

/// The largest picture below `node`.
fn thumbnail(node: &Value) -> Option<String> {
  let pictures = all(node, &["thumbnails"]).into_iter().next()?.as_array()?;
  let area = |p: &Value| p.get("width").and_then(Value::as_u64).unwrap_or(0) * p.get("height").and_then(Value::as_u64).unwrap_or(0);
  pictures.iter().filter(|p| p.get("url").is_some()).max_by_key(|p| area(p))?.get("url")?.as_str().map(str::to_owned)
}

// SECTION: listings

fn is_kind(text: &str) -> bool {
  matches!(text, "Album" | "Single" | "EP")
}

fn is_year(text: &str) -> bool {
  text.len() == 4 && text.bytes().all(|b| b.is_ascii_digit())
}

/// The artists of a listing, which are the linked runs of its subtitle. Without a link it is the
/// first piece that is not the kind of release or the year.
fn artist_line(runs: &[Value]) -> Option<String> {
  let linked: Vec<&str> = runs.iter().filter(|r| page_type(r) == Some(ARTIST_PAGE)).map(run_text).collect();
  if !linked.is_empty() {
    return Some(linked.join(", "));
  }
  parts(runs).into_iter().find(|p| !is_kind(p) && !is_year(p)).map(str::to_owned)
}

/// A release from a shelf of cards. Singles and EPs have album pages as well.
fn release(card: &Value) -> Option<Item> {
  let id = browse_id(card).filter(|id| convert::valid_id(id))?;
  if page_type(card) != Some(ALBUM_PAGE) {
    return None;
  }
  let subtitle = runs_at(card, "/subtitle/runs");
  let pieces = parts(subtitle);
  Some(Item {
    id: id.to_owned(),
    kind: ItemKind::Album,
    title: joined(runs_at(card, "/title/runs")),
    subtitle: artist_line(subtitle),
    label: pieces.iter().find(|p| is_kind(p)).map(|p| (*p).to_owned()),
    thumbnail: thumbnail(card),
    release_year: pieces.iter().find(|p| is_year(p)).and_then(|p| p.parse().ok()),
  })
}

/// An artist from a row or a card. The rank is the place in the list, a chart has no other.
fn artist(entry: &Value) -> Option<Item> {
  let id = browse_id(entry).filter(|id| convert::valid_id(id))?;
  if page_type(entry) != Some(ARTIST_PAGE) {
    return None;
  }
  let flex = flex_runs(entry, 0);
  let title = if flex.is_empty() { joined(runs_at(entry, "/title/runs")) } else { joined(flex) };
  (!title.is_empty()).then(|| Item {
    id: id.to_owned(),
    kind: ItemKind::Artist,
    title,
    subtitle: None,
    label: None,
    thumbnail: thumbnail(entry),
    release_year: None,
  })
}

/// A playlist from a shelf of cards. Its browse id has the prefix of the page that lists it,
/// which the rest of the app does not use for a playlist.
fn playlist(card: &Value) -> Option<Item> {
  let raw = browse_id(card)?;
  let id = raw.strip_prefix("VL").unwrap_or(raw);
  if page_type(card) != Some(PLAYLIST_PAGE) || !convert::valid_id(id) {
    return None;
  }
  let by = parts(runs_at(card, "/subtitle/runs")).into_iter().filter(|p| *p != "Playlist").collect::<Vec<_>>().join(" · ");
  Some(Item {
    id: id.to_owned(),
    kind: ItemKind::Playlist,
    title: joined(runs_at(card, "/title/runs")),
    subtitle: Some(by).filter(|s| !s.is_empty()),
    label: Some("Playlist".to_owned()),
    thumbnail: thumbnail(card),
    release_year: None,
  })
}

/// A cover is not always listed, but the video of a track always has a frame.
fn picture(id: &str, node: &Value) -> Option<String> {
  thumbnail(node).or_else(|| Some(format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg")))
}

/// A song from a list row. The video id is looked for below the row because it is stored in
/// several places, the title, the play button and the item data.
fn song(row: &Value) -> Option<Track> {
  let id = find_str(row, "videoId").filter(|id| convert::valid_id(id))?;
  let title = joined(flex_runs(row, 0));
  if title.is_empty() {
    return None;
  }
  let credits = flex_runs(row, 1);
  Some(Track {
    id: id.to_owned(),
    title,
    artist: artist_line(credits).unwrap_or_default(),
    album: credits.iter().find(|r| page_type(r) == Some(ALBUM_PAGE)).map(|r| run_text(r).to_owned()),
    duration_secs: None,
    thumbnail: picture(id, row),
    release_year: None,
  })
}

/// A music video from a card.
fn video(card: &Value) -> Option<Track> {
  let id = card
    .pointer("/navigationEndpoint/watchEndpoint/videoId")
    .and_then(Value::as_str)
    .filter(|id| convert::valid_id(id))?;
  let title = joined(runs_at(card, "/title/runs"));
  (!title.is_empty()).then(|| Track {
    id: id.to_owned(),
    title,
    artist: artist_line(runs_at(card, "/subtitle/runs")).unwrap_or_default(),
    album: None,
    duration_secs: None,
    thumbnail: picture(id, card),
    release_year: None,
  })
}

// SECTION: pages

/// A shelf of tracks is laid out as rows of a list or as cards, depending on the shelf.
fn tracks_in(shelf: &Value) -> Vec<Track> {
  let rows: Vec<Track> = all(shelf, &["musicResponsiveListItemRenderer"]).into_iter().filter_map(song).collect();
  if !rows.is_empty() {
    return rows;
  }
  all(shelf, &["musicTwoRowItemRenderer"]).into_iter().filter_map(video).collect()
}

fn moods_in(shelf: &Value) -> Vec<Mood> {
  all(shelf, &["musicNavigationButtonRenderer"])
    .into_iter()
    .filter_map(|button| {
      let title = joined(runs_at(button, "/buttonText/runs"));
      let params = button.pointer("/clickCommand/browseEndpoint/params")?.as_str()?;
      if title.is_empty() || !valid_params(params) {
        return None;
      }
      Some(Mood {
        title,
        params: params.to_owned(),
        // an ARGB number, the low 24 bits are the color and it may arrive signed
        color: button.pointer("/solid/leftStripeColor").and_then(Value::as_i64).map(|argb| format!("#{:06x}", argb & 0xFF_FFFF)),
      })
    })
    .collect()
}

/// What the title of a shelf links to, the one thing that says which shelf it is.
fn shelf_id(shelf: &Value) -> Option<&str> {
  shelf
    .pointer("/header/musicCarouselShelfBasicHeaderRenderer/title/runs/0/navigationEndpoint/browseEndpoint/browseId")
    .and_then(Value::as_str)
}

pub fn explore(page: &Value) -> Explore {
  let mut out = Explore { new_releases: vec![], trending: vec![], new_videos: vec![], moods: vec![] };
  // the top songs only come with an account, and stand in for the trending ones when those are missing
  let mut top_songs = vec![];
  for shelf in all(page, &["musicCarouselShelfRenderer"]) {
    match shelf_id(shelf) {
      Some("FEmusic_new_releases_albums") => {
        out.new_releases = all(shelf, &["musicTwoRowItemRenderer"]).into_iter().filter_map(release).collect();
      }
      Some("FEmusic_new_releases_videos") => out.new_videos = tracks_in(shelf),
      Some("FEmusic_moods_and_genres") => out.moods = moods_in(shelf),
      // both of these are links to a playlist, trending ones start with OLA and chart ones with PL
      Some(id) if id.starts_with("VLOLA") => out.trending = tracks_in(shelf),
      Some(id) if id.starts_with("VLPL") => top_songs = tracks_in(shelf),
      _ => {}
    }
  }
  if out.trending.is_empty() {
    out.trending = top_songs;
  }
  out.trending.truncate(MAX_TRENDING);
  out
}

/// Every artist on the chart page, in the order of the chart.
pub fn top_artists(page: &Value) -> Vec<Item> {
  let mut seen = HashSet::new();
  all(page, &["musicResponsiveListItemRenderer", "musicTwoRowItemRenderer"])
    .into_iter()
    .filter_map(artist)
    .filter(|a| seen.insert(a.id.clone()))
    .take(MAX_ARTISTS)
    .collect()
}

pub fn mood_playlists(page: &Value) -> Vec<Item> {
  let mut seen = HashSet::new();
  all(page, &["musicTwoRowItemRenderer"])
    .into_iter()
    .filter_map(playlist)
    .filter(|p| seen.insert(p.id.clone()))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn endpoint(id: &str, page_type: &str) -> Value {
    json!({ "browseEndpoint": { "browseId": id, "browseEndpointContextSupportedConfigs": {
      "browseEndpointContextMusicConfig": { "pageType": page_type } } } })
  }

  fn pictures(url: &str) -> Value {
    json!({ "thumbnails": [
      { "url": format!("{url}-small"), "width": 60, "height": 60 },
      { "url": url, "width": 544, "height": 544 },
    ] })
  }

  fn run(text: &str) -> Value {
    json!({ "text": text })
  }

  fn linked(text: &str, id: &str, page_type: &str) -> Value {
    json!({ "text": text, "navigationEndpoint": endpoint(id, page_type) })
  }

  fn album_card(id: &str, title: &str, kind: &str, artist: &str, year: Option<&str>) -> Value {
    let mut subtitle = vec![run(kind), run(" • "), linked(artist, "UCartist", ARTIST_PAGE)];
    if let Some(year) = year {
      subtitle.extend([run(" • "), run(year)]);
    }
    json!({ "musicTwoRowItemRenderer": {
      "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": pictures("https://img/album") } },
      "title": { "runs": [run(title)] },
      "subtitle": { "runs": subtitle },
      "navigationEndpoint": endpoint(id, ALBUM_PAGE),
    } })
  }

  fn video_card(id: &str, title: &str, artist: &str) -> Value {
    json!({ "musicTwoRowItemRenderer": {
      "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": pictures("https://img/video") } },
      "title": { "runs": [run(title)] },
      "subtitle": { "runs": [linked(artist, "UCartist", ARTIST_PAGE), run(" • "), run("1.2M views")] },
      "navigationEndpoint": { "watchEndpoint": { "videoId": id } },
    } })
  }

  fn song_row(id: &str, title: &str, artist: &str) -> Value {
    json!({ "musicResponsiveListItemRenderer": {
      "thumbnail": { "musicThumbnailRenderer": { "thumbnail": pictures("https://img/song") } },
      "flexColumns": [
        { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [
          { "text": title, "navigationEndpoint": { "watchEndpoint": { "videoId": id } } } ] } } },
        { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [
          linked(artist, "UCartist", ARTIST_PAGE), run(" • "), linked("The Album", "MPREb_album", ALBUM_PAGE) ] } } },
      ],
    } })
  }

  fn artist_row(id: &str, name: &str) -> Value {
    json!({ "musicResponsiveListItemRenderer": {
      "thumbnail": { "musicThumbnailRenderer": { "thumbnail": pictures("https://img/artist") } },
      "flexColumns": [
        { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [run(name)] } } },
        { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [run("1.2M subscribers")] } } },
      ],
      "navigationEndpoint": endpoint(id, ARTIST_PAGE),
    } })
  }

  fn playlist_card(id: &str, title: &str) -> Value {
    json!({ "musicTwoRowItemRenderer": {
      "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": pictures("https://img/playlist") } },
      "title": { "runs": [run(title)] },
      "subtitle": { "runs": [run("Playlist"), run(" • "), run("YouTube Music")] },
      "navigationEndpoint": endpoint(id, PLAYLIST_PAGE),
    } })
  }

  fn chip(title: &str, params: &str, color: i64) -> Value {
    json!({ "musicNavigationButtonRenderer": {
      "buttonText": { "runs": [run(title)] },
      "solid": { "leftStripeColor": color },
      "clickCommand": { "browseEndpoint": { "browseId": "FEmusic_moods_and_genres_category", "params": params } },
    } })
  }

  fn shelf(browse_id: &str, contents: Vec<Value>) -> Value {
    json!({ "musicCarouselShelfRenderer": {
      "header": { "musicCarouselShelfBasicHeaderRenderer": { "title": { "runs": [
        { "text": "Whatever the language says", "navigationEndpoint": { "browseEndpoint": { "browseId": browse_id } } } ] } } },
      "contents": contents,
    } })
  }

  /// The layers above the shelves, which a search is not supposed to depend on.
  fn page(sections: Vec<Value>) -> Value {
    json!({ "contents": { "singleColumnBrowseResultsRenderer": { "tabs": [ { "tabRenderer": { "content": {
      "sectionListRenderer": { "contents": sections } } } } ] } } })
  }

  #[test]
  fn the_explore_page_is_sorted_into_its_shelves() {
    let page = page(vec![
      shelf("FEmusic_new_releases_albums", vec![album_card("MPREb_one", "Feel The Darkness", "Single", "The Dark Horror", Some("2026"))]),
      shelf("VLOLAK5uy_trending", vec![song_row("aaaaaaaaaaa", "Spijt Komt Later", "Isabèl Usher")]),
      shelf("FEmusic_new_releases_videos", vec![video_card("bbbbbbbbbbb", "Mona Lisa", "Sondae")]),
      shelf("FEmusic_moods_and_genres", vec![chip("Chill", "ggMPOg1uX1", 0xFF12_34AB)]),
      shelf("VLPLsomethingelse", vec![song_row("ccccccccccc", "Not Shown", "Nobody")]),
      shelf("FEmusic_something_new", vec![video_card("ddddddddddd", "Unknown shelf", "Nobody")]),
    ]);
    let explore = explore(&page);

    assert_eq!(explore.new_releases.len(), 1);
    let release = &explore.new_releases[0];
    assert_eq!(release.id, "MPREb_one");
    assert_eq!(release.kind, ItemKind::Album);
    assert_eq!(release.title, "Feel The Darkness");
    assert_eq!(release.subtitle.as_deref(), Some("The Dark Horror"));
    assert_eq!(release.label.as_deref(), Some("Single"));
    assert_eq!(release.release_year, Some(2026));
    assert_eq!(release.thumbnail.as_deref(), Some("https://img/album"));

    assert_eq!(explore.trending.len(), 1);
    let track = &explore.trending[0];
    assert_eq!(track.id, "aaaaaaaaaaa");
    assert_eq!(track.title, "Spijt Komt Later");
    assert_eq!(track.artist, "Isabèl Usher");
    assert_eq!(track.album.as_deref(), Some("The Album"));

    assert_eq!(explore.new_videos.len(), 1);
    assert_eq!(explore.new_videos[0].artist, "Sondae");
    assert_eq!(explore.new_videos[0].thumbnail.as_deref(), Some("https://img/video"));

    assert_eq!(explore.moods.len(), 1);
    assert_eq!(explore.moods[0].title, "Chill");
    assert_eq!(explore.moods[0].params, "ggMPOg1uX1");
    assert_eq!(explore.moods[0].color.as_deref(), Some("#1234ab"));
  }

  #[test]
  fn top_songs_stand_in_when_nothing_is_trending() {
    let page = page(vec![shelf("VLPLtopsongs", vec![song_row("ccccccccccc", "Chart Song", "Somebody")])]);
    assert_eq!(explore(&page).trending[0].id, "ccccccccccc");
  }

  #[test]
  fn trending_is_capped_and_a_shelf_of_cards_still_reads() {
    let videos = (0..30).map(|i| video_card(&format!("{i:011}"), "Video", "Band")).collect();
    let page = page(vec![shelf("VLOLAK5uy_trending", videos)]);
    assert_eq!(explore(&page).trending.len(), MAX_TRENDING);
  }

  #[test]
  fn a_page_without_shelves_is_empty() {
    let explore = explore(&json!({}));
    assert!(explore.new_releases.is_empty() && explore.trending.is_empty());
    assert!(explore.new_videos.is_empty() && explore.moods.is_empty());
  }

  #[test]
  fn a_release_without_a_link_on_its_artist_still_has_one() {
    let mut card = album_card("MPREb_two", "Album", "Album", "x", None);
    card["musicTwoRowItemRenderer"]["subtitle"] = json!({ "runs": [run("Album"), run(" • "), run("Various Artists")] });
    let item = release(&card["musicTwoRowItemRenderer"]).unwrap();
    assert_eq!(item.subtitle.as_deref(), Some("Various Artists"));
    assert_eq!(item.release_year, None);
  }

  #[test]
  fn a_listing_that_is_not_an_album_is_not_a_release() {
    let mut card = album_card("MPREb_two", "Album", "Album", "Band", None);
    card["musicTwoRowItemRenderer"]["navigationEndpoint"] = endpoint("UCband", ARTIST_PAGE);
    assert!(release(&card["musicTwoRowItemRenderer"]).is_none());
    assert!(release(&json!({})).is_none());
  }

  #[test]
  fn a_song_without_a_cover_gets_the_video_frame() {
    let mut row = song_row("aaaaaaaaaaa", "Song", "Band");
    row["musicResponsiveListItemRenderer"]["thumbnail"] = json!({});
    let track = song(&row["musicResponsiveListItemRenderer"]).unwrap();
    assert_eq!(track.thumbnail.as_deref(), Some("https://i.ytimg.com/vi/aaaaaaaaaaa/mqdefault.jpg"));
  }

  #[test]
  fn a_row_with_a_bad_video_id_is_dropped() {
    let row = song_row("not/valid?", "Song", "Band");
    assert!(song(&row["musicResponsiveListItemRenderer"]).is_none());
  }

  #[test]
  fn the_chart_lists_artists_in_order_and_once() {
    let page = page(vec![shelf("FEmusic_charts_x", vec![
      artist_row("UCone", "LISA"),
      song_row("aaaaaaaaaaa", "A song is not an artist", "Band"),
      artist_row("UCtwo", "KATSEYE"),
      artist_row("UCone", "LISA"),
    ])]);
    let artists = top_artists(&page);
    assert_eq!(artists.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(), ["LISA", "KATSEYE"]);
    assert_eq!(artists[0].kind, ItemKind::Artist);
    assert_eq!(artists[0].thumbnail.as_deref(), Some("https://img/artist"));
  }

  #[test]
  fn mood_playlists_lose_the_prefix_of_their_page() {
    let page = page(vec![shelf("x", vec![
      playlist_card("VLRDCLAK5uy_one", "Chill Vibes"),
      playlist_card("VLRDCLAK5uy_one", "Chill Vibes again"),
      album_card("MPREb_one", "An album", "Album", "Band", None),
    ])]);
    let playlists = mood_playlists(&page);
    assert_eq!(playlists.len(), 1);
    assert_eq!(playlists[0].id, "RDCLAK5uy_one");
    assert_eq!(playlists[0].kind, ItemKind::Playlist);
    assert_eq!(playlists[0].subtitle.as_deref(), Some("YouTube Music"));
  }

  #[test]
  fn a_mood_color_may_be_signed() {
    // 0xFF1234AB read as a signed 32 bit number
    let shelf = shelf("FEmusic_moods_and_genres", vec![chip("Focus", "ggMPOg1uX2", -15_584_085)]);
    assert_eq!(moods_in(&shelf["musicCarouselShelfRenderer"])[0].color.as_deref(), Some("#1234ab"));
  }

  #[test]
  fn a_mood_with_unusable_params_is_dropped() {
    let shelf = shelf("FEmusic_moods_and_genres", vec![chip("Bad", "a b\"c", 0), chip("Good", "ggMPOg1uX3", 0)]);
    let moods = moods_in(&shelf["musicCarouselShelfRenderer"]);
    assert_eq!(moods.len(), 1);
    assert_eq!(moods[0].title, "Good");
  }

  #[test]
  fn requests_carry_what_the_page_needs() {
    let explore = Page::Explore.body("1.20260101.01.00");
    assert_eq!(explore["browseId"], "FEmusic_explore");
    assert_eq!(explore["context"]["client"]["clientName"], "WEB_REMIX");
    assert_eq!(explore["context"]["client"]["clientVersion"], "1.20260101.01.00");

    let charts = Page::Charts(Some("NL")).body("v");
    assert_eq!(charts["browseId"], "FEmusic_charts");
    assert_eq!(charts["formData"]["selectedValues"][0], "NL");

    let mood = Page::Mood("ggMPOg1uX1").body("v");
    assert_eq!(mood["browseId"], "FEmusic_moods_and_genres_category");
    assert_eq!(mood["params"], "ggMPOg1uX1");
  }

  #[test]
  fn a_chart_falls_back_to_worldwide() {
    assert_eq!(chart_country(Some("NL")), "NL");
    assert_eq!(chart_country(Some("ZZ")), "ZZ");
    for bad in [None, Some(""), Some("nl"), Some("NLD"), Some("N1"), Some("N\"")] {
      assert_eq!(chart_country(bad), "ZZ");
    }
  }

  #[test]
  fn params_with_url_or_json_characters_are_rejected() {
    assert!(valid_params("ggMPOg1uX1JOQWZFeDByNzFB"));
    assert!(valid_params("abc%3D"));
    assert!(!valid_params(""));
    assert!(!valid_params("a b"));
    assert!(!valid_params("a\"b"));
    assert!(!valid_params(&"a".repeat(513)));
  }

  #[test]
  fn the_client_version_is_the_date() {
    assert_eq!(client_version(0), "1.19700101.01.00");
    // 2024-01-01, the first day after a year that is not a leap year
    assert_eq!(client_version(19_723 * 86_400 + 5), "1.20240101.01.00");
    assert_eq!(client_version(19_782 * 86_400 + 86_399), "1.20240229.01.00");
    assert_eq!(client_version(19_783 * 86_400), "1.20240301.01.00");
  }
}
