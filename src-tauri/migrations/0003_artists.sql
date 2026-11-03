-- Artists the user follows. Only followed artists are stored, there is no catalog of all artists.
CREATE TABLE followed_artists (
  id          TEXT PRIMARY KEY NOT NULL,
  name        TEXT NOT NULL,
  thumbnail   TEXT,
  followed_ms INTEGER NOT NULL
) STRICT;

CREATE INDEX followed_artists_name ON followed_artists (name COLLATE NOCASE);
