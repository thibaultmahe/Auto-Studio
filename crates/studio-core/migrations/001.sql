PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS photos (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, sha256 TEXT NOT NULL UNIQUE, extension TEXT NOT NULL,
 width INTEGER NOT NULL, height INTEGER NOT NULL, category TEXT NOT NULL DEFAULT 'UNKNOWN',
 technical TEXT NOT NULL DEFAULT 'PENDING', review TEXT NOT NULL DEFAULT 'NEEDS_REVIEW',
 mask_version INTEGER NOT NULL DEFAULT 0, mask_cache_key TEXT, mask_provider TEXT, render_key TEXT, settings TEXT,
 warnings TEXT NOT NULL DEFAULT '[]', error TEXT,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE IF NOT EXISTS submissions (
 id TEXT PRIMARY KEY, photo_id TEXT NOT NULL REFERENCES photos(id),
 cache_key TEXT NOT NULL, app_id TEXT NOT NULL, version_id TEXT NOT NULL,
 remote_uri TEXT, remote_id TEXT, state TEXT NOT NULL,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS submissions_photo ON submissions(photo_id);
CREATE INDEX IF NOT EXISTS submissions_cache ON submissions(cache_key);
CREATE TABLE IF NOT EXISTS backgrounds (
 id TEXT PRIMARY KEY, version INTEGER NOT NULL, sha256 TEXT NOT NULL,
 width INTEGER NOT NULL, height INTEGER NOT NULL, metadata TEXT NOT NULL
);
PRAGMA user_version = 1;
