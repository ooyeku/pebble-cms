-- Rollback migration 003: Remove media optimization columns
-- Note: Loses webp_filename, width, height, and thumbnail_filename data

-- SQLite does not support DROP COLUMN before 3.35.0, so we recreate the table
CREATE TABLE media_rollback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL UNIQUE,
    original_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    alt_text TEXT DEFAULT '',
    uploaded_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO media_rollback (id, filename, original_name, mime_type, size_bytes, alt_text, uploaded_by, created_at)
SELECT id, filename, original_name, mime_type, size_bytes, alt_text, uploaded_by, created_at FROM media;

DROP TABLE media;
ALTER TABLE media_rollback RENAME TO media;
