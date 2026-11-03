use std::time::Duration;

use iot_gateway::{
    config::DeviceConfig,
    local_mqtt::LocalMqtt,
    store::Store,
};
use rumqttc::{Client, MqttOptions, QoS};
use tokio::{sync::oneshot, time::{sleep, timeout}};

#[tokio::test]
async fn ingests_valid_telemetry_and_continues_after_malformed_message() {
    let Ok(url) = std::env::var("NANOMQ_TEST_URL") else {
        eprintln!("skipping NanoMQ integration test: set NANOMQ_TEST_URL (for example mqtt://127.0.0.1:1883)");
        return;
    };
    let url = url::Url::parse(&url).expect("NANOMQ_TEST_URL must be a valid mqtt URL");
    let host = url.host_str().unwrap();
    let port = url.port().unwrap_or(1883);
    let devices = [
        DeviceConfig { local_id: "esp1".into(), name: "ESP1 CO2".into(), profile: "CO2".into() },
        DeviceConfig { local_id: "esp2".into(), name: "ESP2 TDS".into(), profile: "TDS".into() },
    ];
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let consumer = LocalMqtt::start(host, port, "", "", devices.to_vec(), store.clone()).await.unwrap();
    let (publisher, mut eventloop) = Client::new(MqttOptions::new("nanomq-flow-test", host, port), 10);
    let (connected_tx, connected_rx) = oneshot::channel();
    tokio::spawn(async move {
        let mut connected_tx = Some(connected_tx);
        while let Ok(event) = eventloop.eventloop.poll().await {
            if matches!(event, rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) {
                if let Some(connected_tx) = connected_tx.take() { let _ = connected_tx.send(()); }
            }
        }
    });
    timeout(Duration::from_secs(10), connected_rx).await.unwrap().unwrap();

    timeout(Duration::from_secs(5), async {
        loop {
            if publisher.publish("iot/esp1/telemetry", QoS::AtLeastOnce, false, br#"{"ts":1730000000000,"values":{"co2_ppm":620}}"#).is_ok() {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    }).await.unwrap();

    timeout(Duration::from_secs(10), async {
        loop {
            if store.latest_state("esp1").await.unwrap().is_some() { break; }
            sleep(Duration::from_millis(50)).await;
        }
    }).await.unwrap();
    let state = store.latest_state("esp1").await.unwrap().unwrap();
    assert_eq!(state.thingsboard_name, "ESP1 CO2");
    assert_eq!(state.payload_json, r#"{"ts":1730000000000,"values":{"co2_ppm":620}}"#);
    assert_eq!(store.pending(i64::MAX, 10).await.unwrap().len(), 1);

    publisher.publish("iot/esp1/telemetry", QoS::AtLeastOnce, false, b"{").unwrap();
    sleep(Duration::from_millis(500)).await;
    assert!(consumer.is_running(), "consumer stopped after malformed JSON");
    assert_eq!(store.pending(i64::MAX, 10).await.unwrap().len(), 1);
    consumer.stop();
}
