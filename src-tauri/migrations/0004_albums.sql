-- Albums saved to the library, with their track list in order. The track ids have no
-- foreign key for the reason given in 0001, only the owning album does.
CREATE TABLE albums (
  id           TEXT PRIMARY KEY NOT NULL,
  title        TEXT NOT NULL,
  artist       TEXT NOT NULL,
  cover        TEXT,
  release_year INTEGER,
  added_ms     INTEGER NOT NULL
) STRICT;

CREATE TABLE album_tracks (
  album_id TEXT NOT NULL REFERENCES albums (id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  track_id TEXT NOT NULL,
  PRIMARY KEY (album_id, position)
) STRICT, WITHOUT ROWID;
