use std::{time::{SystemTime, UNIX_EPOCH}, time::Duration};

use anyhow::Result;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::{sync::oneshot, task::JoinHandle, time::timeout};

use crate::{config::DeviceConfig, store::Store, telemetry};

const TOPICS: [&str; 2] = ["iot/esp1/telemetry", "iot/esp2/telemetry"];

pub struct LocalMqtt {
    task: JoinHandle<()>,
}

impl LocalMqtt {
    pub async fn start(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        devices: Vec<DeviceConfig>,
        store: Store,
    ) -> Result<(AsyncClient, Self)> {
        let mut options = MqttOptions::new("iot-gateway", host, port);
        if !username.is_empty() {
            options.set_credentials(username, password);
        }
        options.set_keep_alive(Duration::from_secs(30));
        let (client, mut eventloop) = AsyncClient::new(options, 32);
        let (ready_tx, ready_rx) = oneshot::channel();
        for topic in TOPICS {
            client.subscribe(topic, QoS::AtLeastOnce).await?;
        }

        let task = tokio::spawn(async move {
            let mut ready_tx = Some(ready_tx);
            let mut subscribed = 0;
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::SubAck(_))) => {
                        subscribed += 1;
                        if subscribed == TOPICS.len() {
                            if let Some(ready_tx) = ready_tx.take() {
                                let _ = ready_tx.send(());
                            }
                        }
                    }
                    Ok(Event::Incoming(Packet::Publish(message))) => {
                        let Some(device_id) = (match message.topic.as_str() {
                            "iot/esp1/telemetry" => Some("esp1"),
                            "iot/esp2/telemetry" => Some("esp2"),
                            _ => None,
                        }) else { continue };
                        let received_at_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
                            Ok(duration) => duration.as_millis().try_into().unwrap_or(i64::MAX),
                            Err(_) => {
                                tracing::warn!(topic = %message.topic, reason = "system clock before Unix epoch", "discarding local MQTT telemetry");
                                continue;
                            }
                        };
                        match telemetry::parse(device_id, &message.payload, &devices) {
                            Ok(item) => {
                                if let Err(error) = store.save_telemetry(&item, received_at_ms).await {
                                    tracing::warn!(topic = %message.topic, reason = %error, "failed to store local MQTT telemetry");
                                }
                            }
                            Err(error) => tracing::warn!(topic = %message.topic, reason = %error, "discarding malformed local MQTT telemetry"),
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(reason = %error, "local MQTT connection error");
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                }
            }
        });
        timeout(Duration::from_secs(10), ready_rx)
            .await??;
        Ok((client, Self { task }))
    }

    pub fn is_running(&self) -> bool {
        !self.task.is_finished()
    }

    pub fn stop(self) {
        self.task.abort();
    }
}
