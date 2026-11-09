-- Audio that is read from folders on this device and not streamed. A folder is added by the user
-- and the files below it are found by a scan, see `store::local`.
--
-- A local file is a track like any other, so likes, playlists and the queue treat it the same.
-- Its metadata sits in `tracks` under an id that starts with `local-`, and the path of the file
-- lives here. These rows describe files of one device and are never shared with other devices,
-- see `link`.
CREATE TABLE local_dirs (
  path     TEXT PRIMARY KEY NOT NULL,
  added_ms INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE local_files (
  track_id    TEXT PRIMARY KEY NOT NULL,
  path        TEXT NOT NULL UNIQUE,
  -- the folder of `local_dirs` that the file was found under
  dir         TEXT NOT NULL,
  size_bytes  INTEGER NOT NULL,
  -- size and modification time tell a later scan that the tags need no new read
  modified_ms INTEGER NOT NULL,
  added_ms    INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE INDEX local_files_dir ON local_files (dir);
CREATE INDEX local_files_recent ON local_files (added_ms DESC);

-- The metadata of a local track must not travel to other devices, there is no file for it
-- there. The triggers of `0007_link` stay quiet for these ids. Likes and playlist entries of such a
-- track are still shared, and a device without the file skips them as it does for a track whose
-- metadata has not arrived yet.
DROP TRIGGER link_tracks_ins;
DROP TRIGGER link_tracks_upd;
DROP TRIGGER link_tracks_del;

CREATE TRIGGER link_tracks_ins AFTER INSERT ON tracks
WHEN (SELECT applying FROM link_flag) = 0 AND substr(NEW.id, 1, 6) <> 'local-'
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('track', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_tracks_upd AFTER UPDATE ON tracks
WHEN (SELECT applying FROM link_flag) = 0 AND substr(NEW.id, 1, 6) <> 'local-'
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('track', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_tracks_del AFTER DELETE ON tracks
WHEN (SELECT applying FROM link_flag) = 0 AND substr(OLD.id, 1, 6) <> 'local-'
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('track', OLD.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;
