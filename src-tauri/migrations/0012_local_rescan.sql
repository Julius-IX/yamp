-- A scan reads the tags of a file again only when its size or modification time differs from
-- the last scan. Covers are now also taken from every tag of a file and from a picture in its
-- folder, so the songs that were scanned before would never get the cover they can have now.
-- A stamp that no file has makes the next scan read each file once more, and from then on only
-- the files that changed are read again.
UPDATE local_files SET modified_ms = -1;
