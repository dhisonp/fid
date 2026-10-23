PRAGMA foreign_keys = ON;

CREATE TABLE observations (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  observed_at INTEGER NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('ok', 'fetch_failed', 'parse_failed')),
  discovered INTEGER NOT NULL DEFAULT 0,
  detail TEXT
);
CREATE INDEX observations_window ON observations(observed_at, source_id);

CREATE TABLE documents (
  id TEXT PRIMARY KEY,
  url TEXT NOT NULL,
  source_id TEXT NOT NULL,
  first_seen INTEGER NOT NULL,
  retrieved_at INTEGER NOT NULL,
  published_at TEXT,
  updated_at TEXT,
  content_hash TEXT NOT NULL,
  object_key TEXT NOT NULL UNIQUE,
  metadata TEXT NOT NULL CHECK (json_valid(metadata)),
  UNIQUE(url, content_hash)
);
CREATE INDEX documents_window ON documents(first_seen, retrieved_at);
CREATE INDEX documents_url ON documents(url, retrieved_at);

CREATE TABLE runs (
  id TEXT PRIMARY KEY,
  edition_date TEXT NOT NULL,
  cutoff INTEGER NOT NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  status TEXT NOT NULL CHECK (status IN ('running', 'incomplete', 'published')),
  detail TEXT
);
CREATE INDEX runs_date ON runs(edition_date, started_at);

CREATE TABLE editions (
  edition_date TEXT PRIMARY KEY,
  cutoff INTEGER NOT NULL,
  generation_started_at INTEGER NOT NULL,
  published_at INTEGER NOT NULL,
  current_revision INTEGER NOT NULL DEFAULT 1 CHECK(current_revision > 0)
);
CREATE TABLE revisions (
  edition_date TEXT NOT NULL REFERENCES editions(edition_date),
  revision INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  reason TEXT,
  payload TEXT NOT NULL CHECK (json_valid(payload)),
  PRIMARY KEY(edition_date, revision)
);
CREATE TABLE events (
  event_id TEXT PRIMARY KEY,
  last_edition TEXT NOT NULL,
  last_fingerprint TEXT NOT NULL,
  story TEXT NOT NULL CHECK (json_valid(story))
);
CREATE TABLE published_evidence (
  id TEXT NOT NULL REFERENCES documents(id),
  edition_date TEXT NOT NULL REFERENCES editions(edition_date),
  PRIMARY KEY(id, edition_date)
);
CREATE TABLE event_history (
  event_id TEXT NOT NULL,
  edition_date TEXT NOT NULL REFERENCES editions(edition_date),
  fingerprint TEXT NOT NULL,
  story TEXT NOT NULL CHECK(json_valid(story)),
  PRIMARY KEY(event_id,edition_date)
);
CREATE TABLE usage (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  measured_at INTEGER NOT NULL,
  kind TEXT NOT NULL,
  model TEXT,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cost_usd REAL,
  detail TEXT
);
CREATE TABLE leases (
  name TEXT PRIMARY KEY,
  owner TEXT NOT NULL,
  expires_at INTEGER NOT NULL
);
