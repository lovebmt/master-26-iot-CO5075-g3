use std::time::Duration;

use rumqttc::{AsyncClient, QoS};
use tokio::time::sleep;

use crate::store::{OutboxEntry, Store};

const MAX_BACKOFF_SECS: u64 = 60;
const BATCH: i64 = 50;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

pub struct OutboxWorker;

impl OutboxWorker {
    /// Spawn the outbox delivery loop. Returns a JoinHandle.
    pub fn spawn(store: Store, client: AsyncClient) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let now_ms = now_ms();
                match store.pending(now_ms, BATCH).await {
                    Ok(rows) if rows.is_empty() => {
                        sleep(POLL_INTERVAL).await;
                    }
                    Ok(rows) => {
                        for row in rows {
                            deliver(&store, &client, &row).await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(reason = %e, "outbox poll failed");
                        sleep(POLL_INTERVAL).await;
                    }
                }
            }
        })
    }
}

async fn deliver(store: &Store, client: &AsyncClient, row: &OutboxEntry) {
    // Build ThingsBoard Gateway telemetry envelope
    // Payload stored is {"ts":...,"values":{...}}; we need {"<tb_name>":[<payload>]}
    // tb_name is looked up via device_id -> store has no name lookup, so we store tb_name
    // in outbox payload_json as {"device":"<name>","ts":...,"values":{...}}
    // BUT store.rs stores payload_json as {"ts":...,"values":{...}} without name embedded.
    // We need device thingsboard_name. Use latest_state to look it up.
    let tb_name = match store.latest_state(&row.device_id).await {
        Ok(Some(state)) => state.thingsboard_name,
        _ => {
            tracing::warn!(device_id = %row.device_id, "no state for outbox row, skipping");
            // mark failed with short retry
            let _ = store.mark_failed(row.id, now_ms() + 5_000, "no device state").await;
            return;
        }
    };

    let tb_payload = format!("{{\"{}\":[{}]}}", tb_name, row.payload_json);

    match client
        .publish("v1/gateway/telemetry", QoS::AtLeastOnce, false, tb_payload.as_bytes())
        .await
    {
        Ok(()) => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&row.payload_json) {
                if let Some(light_on) = v.get("values").and_then(|val| val.get("light_on")).and_then(|l| l.as_bool()) {
                    let attr_payload = serde_json::json!({
                        &tb_name: {
                            "light_on": light_on,
                            "state": light_on,
                        }
                    }).to_string();
                    let _ = client.publish("v1/gateway/attributes", QoS::AtLeastOnce, false, attr_payload.as_bytes()).await;
                }
            }
            if let Err(e) = store.mark_delivered(row.id).await {
                tracing::warn!(id = row.id, reason = %e, "failed to mark outbox row delivered");
            }
        }
        Err(e) => {
            let backoff_secs = backoff_secs(row.attempts);
            let next_ms = now_ms().saturating_add((backoff_secs * 1_000) as i64);
            let _ = store.mark_failed(row.id, next_ms as i64, &e.to_string()).await;
            tracing::warn!(id = row.id, reason = %e, backoff_secs, "outbox delivery failed");
        }
    }
}

fn backoff_secs(attempts: i64) -> u64 {
    // 1, 2, 4, 8, ... capped at MAX_BACKOFF_SECS
    let exp = u64::try_from(attempts).unwrap_or(u64::MAX);
    MAX_BACKOFF_SECS.min(1u64.checked_shl(exp as u32).unwrap_or(u64::MAX))
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::backoff_secs;

    #[test]
    fn backoff_doubles_and_caps() {
        assert_eq!(backoff_secs(0), 1);
        assert_eq!(backoff_secs(1), 2);
        assert_eq!(backoff_secs(5), 32);
        assert_eq!(backoff_secs(6), 60);
        assert_eq!(backoff_secs(100), 60);
    }
}

