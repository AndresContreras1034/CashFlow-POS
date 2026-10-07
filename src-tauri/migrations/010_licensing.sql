CREATE TABLE license_state (
    id              SMALLINT PRIMARY KEY CHECK (id = 1),
    installation_id TEXT NOT NULL UNIQUE,
    license_file    TEXT,
    last_seen_at    TIMESTAMPTZ
);
