-- Audio files saved on this device. The file is named after the primary key, so the row
-- is the only index of the folder. These rows describe files of one device and are
-- never shared with other devices, see `link`.
CREATE TABLE downloads (
  track_id      TEXT PRIMARY KEY NOT NULL,
  ext           TEXT NOT NULL,
  size_bytes    INTEGER NOT NULL,
  downloaded_ms INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX downloads_recent ON downloads (downloaded_ms DESC);
