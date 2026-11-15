use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use anyhow::Result;
use serde::Serialize;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use crate::telemetry::NormalizedTelemetry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceState {
    pub device_id: String,
    pub thingsboard_name: String,
    pub payload_json: String,
    pub last_seen_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxEntry {
    pub id: i64,
    pub device_id: String,
    pub payload_json: String,
    pub created_at_ms: i64,
    pub attempts: i64,
    pub next_attempt_ms: i64,
    pub last_error: Option<String>,
}

struct DeviceWindow {
    start_ms: i64,
    samples: Vec<(u64, serde_json::Map<String, serde_json::Value>)>,
}

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
    agg_window_secs: Arc<AtomicU64>,
    agg_buffer: Arc<Mutex<HashMap<String, DeviceWindow>>>,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let agg_secs = std::env::var("AGGREGATION_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        Ok(Self {
            pool,
            agg_window_secs: Arc::new(AtomicU64::new(agg_secs)),
            agg_buffer: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn get_aggregation_window(&self) -> u64 {
        self.agg_window_secs.load(Ordering::Relaxed)
    }

    pub fn set_aggregation_window(&self, secs: u64) {
        self.agg_window_secs.store(secs, Ordering::Relaxed);
        if secs == 0 {
            if let Ok(mut buf) = self.agg_buffer.lock() {
                buf.clear();
            }
        }
    }

    // ponytail: in-memory aggregation flushes on incoming telemetry ticks; add background timer if sensor halts while buffer has pending samples.
    fn aggregate_sample(
        &self,
        device_id: &str,
        now_ms: i64,
        ts_ms: u64,
        values: &serde_json::Map<String, serde_json::Value>,
        window_secs: u64,
    ) -> Option<String> {
        let mut buf = self.agg_buffer.lock().ok()?;
        let window = buf.entry(device_id.to_string()).or_insert_with(|| DeviceWindow {
            start_ms: now_ms,
            samples: Vec::new(),
        });
        window.samples.push((ts_ms, values.clone()));

        let elapsed = now_ms.saturating_sub(window.start_ms);
        if elapsed >= (window_secs as i64) * 1000 {
            let count = window.samples.len();
            let last_ts = window.samples.last().map(|s| s.0).unwrap_or(ts_ms);
            let mut agg_map = serde_json::Map::new();
            let mut num_sums: HashMap<String, (f64, usize)> = HashMap::new();

            for (_s_ts, s_vals) in &window.samples {
                for (k, v) in s_vals {
                    if let Some(n) = v.as_f64() {
                        let entry = num_sums.entry(k.clone()).or_insert((0.0, 0));
                        entry.0 += n;
                        entry.1 += 1;
                    } else {
                        agg_map.insert(k.clone(), v.clone());
                    }
                }
            }
            for (k, (sum, c)) in num_sums {
                if c > 0 {
                    let avg = ((sum / c as f64) * 100.0).round() / 100.0;
                    agg_map.insert(k, serde_json::json!(avg));
                }
            }
            agg_map.insert("samples_count".into(), serde_json::json!(count));

            #[derive(Serialize)]
            struct AggPayload<'a> {
                ts: u64,
                values: &'a serde_json::Map<String, serde_json::Value>,
            }
            let out_json = serde_json::to_string(&AggPayload {
                ts: last_ts,
                values: &agg_map,
            })
            .ok();
            buf.remove(device_id);
            out_json
        } else {
            None
        }
    }

    pub async fn save_telemetry(&self, item: &NormalizedTelemetry, received_at_ms: i64) -> Result<()> {
        #[derive(Serialize)]
        struct Payload<'a> {
            ts: u64,
            values: &'a serde_json::Map<String, serde_json::Value>,
        }
        let payload_json = serde_json::to_string(&Payload { ts: item.timestamp_ms, values: &item.values })?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO device_state (device_id, thingsboard_name, payload_json, last_seen_ms) VALUES (?, ?, ?, ?) ON CONFLICT(device_id) DO UPDATE SET thingsboard_name = excluded.thingsboard_name, payload_json = excluded.payload_json, last_seen_ms = excluded.last_seen_ms")
            .bind(&item.device_id).bind(&item.thingsboard_name).bind(&payload_json)
            .bind(received_at_ms).execute(&mut *tx).await?;
        let window_secs = self.get_aggregation_window();
        let to_enqueue: Option<String> = if window_secs == 0 {
            Some(payload_json)
        } else {
            self.aggregate_sample(&item.device_id, received_at_ms, item.timestamp_ms, &item.values, window_secs)
        };
        if let Some(outbox_payload) = to_enqueue {
            sqlx::query("INSERT INTO telemetry_outbox (device_id, payload_json, created_at_ms, next_attempt_ms) VALUES (?, ?, ?, ?)")
                .bind(&item.device_id).bind(&outbox_payload).bind(received_at_ms).bind(received_at_ms)
                .execute(&mut *tx).await?;
        }
        for (key, val) in &item.values {
            let num_val = if let Some(n) = val.as_f64() {
                Some(n)
            } else if let Some(b) = val.as_bool() {
                Some(if b { 1.0 } else { 0.0 })
            } else {
                None
            };
            if let Some(v) = num_val {
                sqlx::query("INSERT INTO telemetry_history (device_id, metric, value, ts_ms) VALUES (?, ?, ?, ?)")
                    .bind(&item.device_id).bind(key).bind(v).bind(i64::try_from(item.timestamp_ms)?)
                    .execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn latest_state(&self, device_id: &str) -> Result<Option<DeviceState>> {
        Ok(sqlx::query_as::<_, (String, String, String, i64)>("SELECT device_id, thingsboard_name, payload_json, last_seen_ms FROM device_state WHERE device_id = ?")
            .bind(device_id).fetch_optional(&self.pool).await?
            .map(|(device_id, thingsboard_name, payload_json, last_seen_ms)| DeviceState { device_id, thingsboard_name, payload_json, last_seen_ms }))
    }

    pub async fn pending(&self, now_ms: i64, limit: i64) -> Result<Vec<OutboxEntry>> {
        anyhow::ensure!(limit > 0, "pending limit must be positive");
        Ok(sqlx::query_as::<_, (i64, String, String, i64, i64, i64, Option<String>)>("SELECT id, device_id, payload_json, created_at_ms, attempts, next_attempt_ms, last_error FROM telemetry_outbox WHERE next_attempt_ms <= ? ORDER BY id ASC LIMIT ?")
            .bind(now_ms).bind(limit).fetch_all(&self.pool).await?
            .into_iter().map(|(id, device_id, payload_json, created_at_ms, attempts, next_attempt_ms, last_error)| OutboxEntry { id, device_id, payload_json, created_at_ms, attempts, next_attempt_ms, last_error }).collect())
    }

    pub async fn mark_delivered(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM telemetry_outbox WHERE id = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: i64, next_attempt_ms: i64, error: &str) -> Result<()> {
        sqlx::query("UPDATE telemetry_outbox SET attempts = attempts + 1, next_attempt_ms = ?, last_error = ? WHERE id = ?")
            .bind(next_attempt_ms).bind(error).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn outbox_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM telemetry_outbox")
            .fetch_one(&self.pool).await?;
        Ok(count)
    }

    pub async fn clean_outbox_range(&self, from_ms: i64, to_ms: i64) -> Result<u64> {
        let res = sqlx::query("DELETE FROM telemetry_outbox WHERE created_at_ms >= ? AND created_at_ms <= ?")
            .bind(from_ms).bind(to_ms).execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    pub async fn history_metrics(&self, device_id: &str) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT DISTINCT metric FROM telemetry_history WHERE device_id = ? ORDER BY metric")
            .bind(device_id).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(m,)| m).collect())
    }

    pub async fn history_points(&self, device_id: &str, metric: &str, limit: i64) -> Result<Vec<(i64, f64)>> {
        let rows: Vec<(i64, f64)> = sqlx::query_as("SELECT ts_ms, value FROM telemetry_history WHERE device_id = ? AND metric = ? ORDER BY ts_ms ASC LIMIT ?")
            .bind(device_id).bind(metric).bind(limit).fetch_all(&self.pool).await?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::NormalizedTelemetry;
    use serde_json::{json, Map};

    async fn store() -> Store {
        let s = Store::connect("sqlite::memory:").await.unwrap();
        s.set_aggregation_window(0);
        s
    }

    fn sample(device_id: &str, timestamp_ms: u64, value: i64) -> NormalizedTelemetry {
        NormalizedTelemetry {
            device_id: device_id.into(),
            thingsboard_name: format!("{device_id} device"),
            timestamp_ms,
            values: Map::from_iter([("co2_ppm".into(), json!(value))]),
        }
    }

    #[tokio::test]
    async fn saves_latest_state_and_enqueues_atomically() {
        let store = store().await;
        store.save_telemetry(&sample("esp1", 10, 620), 100).await.unwrap();
        store.save_telemetry(&sample("esp1", 20, 700), 200).await.unwrap();

        let state = store.latest_state("esp1").await.unwrap().unwrap();
        assert_eq!(state.thingsboard_name, "esp1 device");
        assert_eq!(state.last_seen_ms, 200);
        assert_eq!(state.payload_json, r#"{"ts":20,"values":{"co2_ppm":700}}"#);

        let pending = store.pending(200, 10).await.unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].created_at_ms, 100);
        assert_eq!(pending[1].created_at_ms, 200);
    }

    #[tokio::test]
    async fn pending_rejects_nonpositive_limits() {
        let store = store().await;
        assert!(store.pending(0, 0).await.is_err());
        assert!(store.pending(0, -1).await.is_err());
    }

    #[tokio::test]
    async fn creates_and_migrates_fresh_database_file() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("gateway.db");
        let url = format!("sqlite://{}?mode=rwc", database_path.display());

        let store = Store::connect(&url).await.unwrap();
        assert!(store.latest_state("esp1").await.unwrap().is_none());
        assert!(database_path.exists());
    }

    #[tokio::test]
    async fn marks_delivery_and_tracks_retry_state() {
        let store = store().await;
        store.save_telemetry(&sample("esp1", 10, 620), 100).await.unwrap();
        store.save_telemetry(&sample("esp2", 11, 400), 101).await.unwrap();
        let pending = store.pending(200, 10).await.unwrap();

        store.mark_failed(pending[0].id, 500, "offline").await.unwrap();
        let retry = store.pending(499, 10).await.unwrap();
        assert_eq!(retry.len(), 1);
        let retry = store.pending(500, 10).await.unwrap();
        assert_eq!(retry.len(), 2);
        assert_eq!(retry[0].attempts, 1);
        assert_eq!(retry[0].last_error.as_deref(), Some("offline"));

        store.mark_delivered(pending[0].id).await.unwrap();
        let pending = store.pending(500, 10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].device_id, "esp2");
    }

    #[tokio::test]
    async fn enqueue_failure_rolls_back_latest_state() {
        let store = store().await;
        sqlx::query("CREATE TRIGGER reject_outbox BEFORE INSERT ON telemetry_outbox BEGIN SELECT RAISE(ABORT, 'rejected'); END")
            .execute(&store.pool).await.unwrap();

        assert!(store.save_telemetry(&sample("esp1", 42, 620), 100).await.is_err());
        assert!(store.latest_state("esp1").await.unwrap().is_none());
        assert!(store.pending(100, 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn counts_and_cleans_outbox_date_range() {
        let store = store().await;
        store.save_telemetry(&sample("esp1", 1, 100), 1_000).await.unwrap();
        store.save_telemetry(&sample("esp1", 2, 200), 5_000).await.unwrap();
        store.save_telemetry(&sample("esp1", 3, 300), 10_000).await.unwrap();
        assert_eq!(store.outbox_count().await.unwrap(), 3);

        let removed = store.clean_outbox_range(2_000, 8_000).await.unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.outbox_count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn stores_and_queries_history() {
        let store = store().await;
        store.save_telemetry(&sample("esp1", 100, 620), 100).await.unwrap();
        store.save_telemetry(&sample("esp1", 200, 750), 200).await.unwrap();
        let metrics = store.history_metrics("esp1").await.unwrap();
        assert_eq!(metrics, vec!["co2_ppm".to_string()]);
        let pts = store.history_points("esp1", "co2_ppm", 10).await.unwrap();
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], (100, 620.0));
        assert_eq!(pts[1], (200, 750.0));
    }

    #[tokio::test]
    async fn aggregates_telemetry_over_configured_window() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        store.set_aggregation_window(10);

        store.save_telemetry(&sample("esp1", 1_000, 100), 1_000).await.unwrap();
        assert_eq!(store.pending(i64::MAX, 10).await.unwrap().len(), 0);

        store.save_telemetry(&sample("esp1", 6_000, 200), 6_000).await.unwrap();
        assert_eq!(store.pending(i64::MAX, 10).await.unwrap().len(), 0);

        store.save_telemetry(&sample("esp1", 11_500, 300), 11_500).await.unwrap();
        let pending = store.pending(i64::MAX, 10).await.unwrap();
        assert_eq!(pending.len(), 1);

        let parsed: serde_json::Value = serde_json::from_str(&pending[0].payload_json).unwrap();
        assert_eq!(parsed["values"]["co2_ppm"], 200.0);
        assert_eq!(parsed["values"]["samples_count"], 3);
    }
}
