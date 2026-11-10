-- Small key value pairs that are shared between devices, for data that has no table of its own,
-- like the synced settings and the listening statistics. The value is JSON as text, and what it
-- holds is up to the caller. Like the other shared tables, each change is noted for the link by
-- triggers, see 0007.
CREATE TABLE synced_values (
  key        TEXT PRIMARY KEY NOT NULL,
  value      TEXT NOT NULL,
  updated_ms INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TRIGGER link_synced_values_ins AFTER INSERT ON synced_values
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('value', NEW.key, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_synced_values_upd AFTER UPDATE ON synced_values
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('value', NEW.key, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;

CREATE TRIGGER link_synced_values_del AFTER DELETE ON synced_values
WHEN (SELECT applying FROM link_flag) = 0
BEGIN
  INSERT INTO link_dirty (kind, key, ts_ms) VALUES ('value', OLD.key, CAST(unixepoch('subsec') * 1000 AS INTEGER))
  ON CONFLICT (kind, key) DO UPDATE SET ts_ms = MAX(ts_ms, excluded.ts_ms);
END;
