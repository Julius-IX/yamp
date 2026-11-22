-- Covers that the user set for single tracks. The image is a small JPEG, see `store::track_cover`.
-- There is no foreign key and no trigger of `link` on purpose. A track can be a stream that
-- the library has no row for, and the covers stay on this device like the files of local songs.
CREATE TABLE track_covers (
  track_id   TEXT PRIMARY KEY NOT NULL,
  image      BLOB NOT NULL,
  updated_ms INTEGER NOT NULL
) STRICT;
