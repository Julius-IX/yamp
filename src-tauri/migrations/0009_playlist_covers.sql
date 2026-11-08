-- Covers that the user set for playlists. The image is a small JPEG, see `store::cover`, so it
-- lives in the row and travels with the library like everything else. There is no foreign key on
-- purpose, a cover can arrive from another device before its playlist does, the same way a like
-- can arrive before its track. A cover without a playlist is never listed.
CREATE TABLE playlist_covers (
  playlist_id TEXT PRIMARY KEY NOT NULL,
  image       BLOB NOT NULL,
  updated_ms  INTEGER NOT NULL
) STRICT;

-- A deleted playlist takes its cover along, whether it was deleted here or by a sync.
CREATE TRIGGER playlist_covers_cleanup AFTER DELETE ON playlists
BEGIN
  DELETE FROM playlist_covers WHERE playlist_id = OLD.id;
END;

CREATE TRIGGER link_playlist_covers_ins AFTER INSERT ON playlist_covers
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('cover', NEW.playlist_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlist_covers_upd AFTER UPDATE ON playlist_covers
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('cover', NEW.playlist_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlist_covers_del AFTER DELETE ON playlist_covers
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('cover', OLD.playlist_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;
