-- Playlists made by the user. A track may appear more than once, so the position is part
-- of the key and not the track id.
CREATE TABLE playlists (
  id          TEXT PRIMARY KEY NOT NULL,
  name        TEXT NOT NULL,
  description TEXT,
  created_ms  INTEGER NOT NULL,
  updated_ms  INTEGER NOT NULL
) STRICT;

CREATE TABLE playlist_tracks (
  playlist_id TEXT NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
  position    INTEGER NOT NULL,
  track_id    TEXT NOT NULL,
  PRIMARY KEY (playlist_id, position)
) STRICT, WITHOUT ROWID;
