CREATE TABLE IF NOT EXISTS telemetry_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    metric TEXT NOT NULL,
    value REAL NOT NULL,
    ts_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_history_device_ts ON telemetry_history (device_id, metric, ts_ms);
