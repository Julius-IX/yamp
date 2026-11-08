-- Albums become playlists. A saved album was a titled track list, which is what a playlist is.
--
-- The playlist keeps the album id. The app looks an album up by that id to tell whether it is
-- saved, and two devices that saved the same album then agree on one playlist.
-- The artist and the year move into the description, a playlist has no fields for them.
INSERT INTO playlists (id, name, description, created_ms, updated_ms)
SELECT
  id,
  title,
  CASE
    WHEN artist <> '' AND release_year IS NOT NULL THEN artist || ' · ' || release_year
    WHEN artist <> '' THEN artist
    ELSE CAST(release_year AS TEXT)
  END,
  added_ms,
  added_ms
FROM albums;

INSERT INTO playlist_tracks (playlist_id, position, track_id)
SELECT album_id, position, track_id FROM album_tracks;

-- A playlist shows the thumbnail of its first track and the album cover had no place to go.
-- Tracks without a thumbnail of their own take it, so the cover survives.
UPDATE tracks SET thumbnail = (
  SELECT a.cover FROM album_tracks x JOIN albums a ON a.id = x.album_id
  WHERE x.track_id = tracks.id AND a.cover IS NOT NULL
  ORDER BY a.added_ms, a.id LIMIT 1
)
WHERE thumbnail IS NULL AND EXISTS (
  SELECT 1 FROM album_tracks x JOIN albums a ON a.id = x.album_id
  WHERE x.track_id = tracks.id AND a.cover IS NOT NULL
);

-- Albums are no longer an entity that is linked between devices. Their triggers go with the
-- tables.
DELETE FROM link_dirty WHERE kind = 'album';
DELETE FROM link_versions WHERE kind = 'album';

DROP TABLE album_tracks;
DROP TABLE albums;
