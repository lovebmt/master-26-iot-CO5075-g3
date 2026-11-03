CREATE TABLE device_state (
    device_id TEXT PRIMARY KEY,
    thingsboard_name TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    last_seen_ms INTEGER NOT NULL
);

CREATE TABLE telemetry_outbox (
    id INTEGER PRIMARY KEY,
    device_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_ms INTEGER NOT NULL,
    last_error TEXT
);
