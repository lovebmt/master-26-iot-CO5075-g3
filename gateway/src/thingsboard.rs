use std::time::Duration;

use anyhow::Result;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::{json, Value};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{config::DeviceConfig, outbox::OutboxWorker, store::Store};

pub struct ThingsBoard {
    pub rpc_rx: mpsc::Receiver<Value>,
    _event_task: JoinHandle<()>,
    _outbox_task: JoinHandle<()>,
    pub client: AsyncClient,
}

impl ThingsBoard {
    pub async fn start(
        host: &str,
        port: u16,
        gateway_token: &str,
        devices: &[DeviceConfig],
        store: Store,
    ) -> Result<Self> {
        let mut opts = MqttOptions::new("iot-gateway-tb", host, port);
        opts.set_credentials(gateway_token, "");
        opts.set_keep_alive(Duration::from_secs(30));
        if port != 1883 {
            opts.set_transport(rumqttc::Transport::Tls(rumqttc::TlsConfiguration::default()));
        }
        let (client, mut eventloop) = AsyncClient::new(opts, 64);

        client.subscribe("v1/gateway/rpc", QoS::AtLeastOnce).await?;

        let (rpc_tx, rpc_rx) = mpsc::channel::<Value>(64);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

        let event_client = client.clone();
        let event_task = tokio::spawn(async move {
            let mut ready_tx = Some(ready_tx);
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::SubAck(_))) => {
                        if let Some(tx) = ready_tx.take() {
                            let _ = tx.send(());
                        }
                    }
                    Ok(Event::Incoming(Packet::Publish(msg))) => {
                        if msg.topic == "v1/gateway/rpc" {
                            match serde_json::from_slice::<Value>(&msg.payload) {
                                Ok(v) => { let _ = rpc_tx.try_send(v); }
                                Err(e) => tracing::warn!(reason = %e, "invalid RPC JSON from ThingsBoard"),
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(reason = %e, "ThingsBoard MQTT connection error");
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
                let _ = &event_client;
            }
        });

        tokio::time::timeout(Duration::from_secs(10), ready_rx)
            .await
            .map_err(|_| anyhow::anyhow!("ThingsBoard subscribe timed out"))??;

        for dev in devices {
            let payload = json!({"device": dev.name, "type": dev.profile}).to_string();
            client
                .publish("v1/gateway/connect", QoS::AtLeastOnce, false, payload.as_bytes())
                .await?;
        }

        let outbox_task = OutboxWorker::spawn(store, client.clone());

        Ok(Self {
            rpc_rx,
            _event_task: event_task,
            _outbox_task: outbox_task,
            client,
        })
    }
}

/// Build the JSON payload for a v1/gateway/connect message (pure, testable).
pub fn connect_payload(device_name: &str, profile: &str) -> String {
    json!({"device": device_name, "type": profile}).to_string()
}

/// Build the JSON payload for v1/gateway/telemetry (pure, testable).
pub fn telemetry_payload(tb_name: &str, ts_ms: u64, values_json: &str) -> String {
    let values_val: Value = serde_json::from_str(values_json).unwrap_or(Value::Null);
    json!({
        tb_name: [{
            "ts": ts_ms,
            "values": values_val,
        }]
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{connect_payload, telemetry_payload};
    use serde_json::{from_str, Value};

    #[test]
    fn connect_payload_includes_device_and_type() {
        let p: Value = from_str(&connect_payload("ESP1", "CO2 Sensor")).unwrap();
        assert_eq!(p["device"], "ESP1");
        assert_eq!(p["type"], "CO2 Sensor");
    }

    #[test]
    fn telemetry_payload_wraps_values_under_device_name() {
        let p: Value = from_str(&telemetry_payload("ESP1", 1_730_000_000_000, r#"{"co2_ppm":620}"#)).unwrap();
        let entry = &p["ESP1"][0];
        assert_eq!(entry["ts"], 1_730_000_000_000_u64);
        assert_eq!(entry["values"]["co2_ppm"], 620);
    }

    #[test]
    fn telemetry_payload_filters_only_known_device() {
        let p: Value = from_str(&telemetry_payload("UNKNOWN", 1, r#"{"x":1}"#)).unwrap();
        assert!(p.get("UNKNOWN").is_some());
    }
}

