-- Bookkeeping for linking devices, see the `link` module.
--
-- What is shared between devices is the state of whole entities (a track, a like, an
-- artist, an album, a playlist), never single statements. Triggers note which entities
-- changed in `link_dirty`, so the stores need no knowledge of syncing and no code path
-- can forget to report a change.

-- Entities changed on this device that have not been sent yet, with the time of the
-- latest change. A later change of the same entity only moves the time.
CREATE TABLE link_dirty (
  kind  TEXT NOT NULL,
  key   TEXT NOT NULL,
  ts_ms INTEGER NOT NULL,
  PRIMARY KEY (kind, key)
) STRICT, WITHOUT ROWID;

-- The newest known version of every entity that came from a sync, and who made it.
-- `deleted` rows are tombstones. They stay, otherwise an older copy from a device that
-- was offline would bring a deleted entity back.
CREATE TABLE link_versions (
  kind    TEXT NOT NULL,
  key     TEXT NOT NULL,
  ts_ms   INTEGER NOT NULL,
  device  TEXT NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (kind, key)
) STRICT, WITHOUT ROWID;

-- A single row that is 1 while changes from another device are written. The triggers
-- stay quiet then, otherwise every received change would be sent straight back.
CREATE TABLE link_flag (
  id       INTEGER PRIMARY KEY CHECK (id = 1),
  applying INTEGER NOT NULL
) STRICT;
INSERT INTO link_flag (id, applying) VALUES (1, 0);

-- Per other device, the name of the last log entry that was applied.
CREATE TABLE link_cursors (
  device TEXT PRIMARY KEY NOT NULL,
  cursor TEXT NOT NULL
) STRICT, WITHOUT ROWID;

-- Small values of this device, such as its id.
CREATE TABLE link_meta (
  key   TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
) STRICT, WITHOUT ROWID;

-- Every table below marks its entity dirty on insert, update and delete. A table that
-- only holds the list of another entity (album_tracks, playlist_tracks) marks that entity.

CREATE TRIGGER link_tracks_ins AFTER INSERT ON tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('track', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_tracks_upd AFTER UPDATE ON tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('track', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_tracks_del AFTER DELETE ON tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('track', OLD.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_likes_ins AFTER INSERT ON likes
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('like', NEW.track_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_likes_upd AFTER UPDATE ON likes
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('like', NEW.track_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_likes_del AFTER DELETE ON likes
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('like', OLD.track_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_followed_artists_ins AFTER INSERT ON followed_artists
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('artist', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_followed_artists_upd AFTER UPDATE ON followed_artists
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('artist', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_followed_artists_del AFTER DELETE ON followed_artists
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('artist', OLD.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_albums_ins AFTER INSERT ON albums
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('album', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_albums_upd AFTER UPDATE ON albums
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('album', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_albums_del AFTER DELETE ON albums
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('album', OLD.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_album_tracks_ins AFTER INSERT ON album_tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('album', NEW.album_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_album_tracks_upd AFTER UPDATE ON album_tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('album', NEW.album_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_album_tracks_del AFTER DELETE ON album_tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('album', OLD.album_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlists_ins AFTER INSERT ON playlists
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('playlist', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlists_upd AFTER UPDATE ON playlists
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('playlist', NEW.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlists_del AFTER DELETE ON playlists
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('playlist', OLD.id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlist_tracks_ins AFTER INSERT ON playlist_tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('playlist', NEW.playlist_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlist_tracks_upd AFTER UPDATE ON playlist_tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('playlist', NEW.playlist_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_playlist_tracks_del AFTER DELETE ON playlist_tracks
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('playlist', OLD.playlist_id, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

-- Data that exists before this migration is sent too.
INSERT INTO link_dirty (kind, key, ts_ms) SELECT 'track', id, CAST(unixepoch('subsec') * 1000 AS INTEGER) FROM tracks;
INSERT INTO link_dirty (kind, key, ts_ms) SELECT 'like', track_id, CAST(unixepoch('subsec') * 1000 AS INTEGER) FROM likes;
INSERT INTO link_dirty (kind, key, ts_ms) SELECT 'artist', id, CAST(unixepoch('subsec') * 1000 AS INTEGER) FROM followed_artists;
INSERT INTO link_dirty (kind, key, ts_ms) SELECT 'album', id, CAST(unixepoch('subsec') * 1000 AS INTEGER) FROM albums;
INSERT INTO link_dirty (kind, key, ts_ms) SELECT 'playlist', id, CAST(unixepoch('subsec') * 1000 AS INTEGER) FROM playlists;
