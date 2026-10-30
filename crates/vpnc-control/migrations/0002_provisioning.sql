-- Provisioning jobs and regions (Schritt 8)

CREATE TABLE IF NOT EXISTS regions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS region_fallbacks (
    region_id TEXT NOT NULL REFERENCES regions(id) ON DELETE CASCADE,
    fallback_region_id TEXT NOT NULL REFERENCES regions(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (region_id, fallback_region_id)
);

CREATE TABLE IF NOT EXISTS provisioning_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'pending', 'awaiting_hostkey', 'running', 'succeeded', 'failed', 'cancelled'
    )),
    host TEXT NOT NULL,
    ssh_port INTEGER NOT NULL DEFAULT 22,
    ssh_user TEXT NOT NULL,
    role TEXT NOT NULL,
    region_id TEXT REFERENCES regions(id),
    cluster_id TEXT,
    host_key_fingerprint TEXT,
    error_message TEXT,
    log_text TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_provisioning_jobs_status ON provisioning_jobs(status);

ALTER TABLE nodes ADD COLUMN region_id TEXT;
ALTER TABLE nodes ADD COLUMN ssh_host_key TEXT;
