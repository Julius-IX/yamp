-- Liked tracks. The row holds only the track id, the metadata lives in `tracks`.
CREATE TABLE likes (
  track_id TEXT PRIMARY KEY NOT NULL,
  liked_ms INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX likes_recent ON likes (liked_ms DESC);
