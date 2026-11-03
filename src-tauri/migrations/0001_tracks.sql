-- Metadata of every track the library refers to. Likes, playlists, albums and downloads
-- store only a track id, so a track row is what lets a list show titles and covers.
-- There are no foreign keys to this table on purpose, see the sync notes in `link`. A
-- change from another device can reference a track that arrives a moment later.
CREATE TABLE tracks (
  id            TEXT PRIMARY KEY NOT NULL,
  title         TEXT NOT NULL,
  artist        TEXT NOT NULL,
  album         TEXT,
  duration_secs REAL,
  thumbnail     TEXT,
  release_year  INTEGER
) STRICT;

CREATE INDEX tracks_title ON tracks (title COLLATE NOCASE);
