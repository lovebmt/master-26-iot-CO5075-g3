use anyhow::Result;
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

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

pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new().max_connections(1).connect(database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
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
            .bind(i64::try_from(item.timestamp_ms)?).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO telemetry_outbox (device_id, payload_json, created_at_ms, next_attempt_ms) VALUES (?, ?, ?, ?)")
            .bind(&item.device_id).bind(&payload_json).bind(received_at_ms).bind(received_at_ms)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn latest_state(&self, device_id: &str) -> Result<Option<DeviceState>> {
        Ok(sqlx::query_as::<_, (String, String, String, i64)>("SELECT device_id, thingsboard_name, payload_json, last_seen_ms FROM device_state WHERE device_id = ?")
            .bind(device_id).fetch_optional(&self.pool).await?
            .map(|(device_id, thingsboard_name, payload_json, last_seen_ms)| DeviceState { device_id, thingsboard_name, payload_json, last_seen_ms }))
    }

    pub async fn pending(&self, now_ms: i64, limit: i64) -> Result<Vec<OutboxEntry>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::NormalizedTelemetry;
    use serde_json::{json, Map};

    async fn store() -> Store {
        Store::connect("sqlite::memory:").await.unwrap()
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
        assert_eq!(state.last_seen_ms, 20);
        assert_eq!(state.payload_json, r#"{"ts":20,"values":{"co2_ppm":700}}"#);

        let pending = store.pending(200, 10).await.unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].created_at_ms, 100);
        assert_eq!(pending[1].created_at_ms, 200);
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
}
