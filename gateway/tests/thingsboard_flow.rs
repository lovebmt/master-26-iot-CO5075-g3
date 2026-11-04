use std::time::Duration;

use iot_gateway::{
    config::DeviceConfig,
    store::Store,
    thingsboard::{connect_payload, telemetry_payload, ThingsBoard},
};

#[tokio::test]
async fn thingsboard_protocol_unit_tests_pass_without_broker() {
    // Pure payload shape tests - always run
    let cp = connect_payload("ESP1", "CO2 Sensor");
    let v: serde_json::Value = serde_json::from_str(&cp).unwrap();
    assert_eq!(v["device"], "ESP1");
    assert_eq!(v["type"], "CO2 Sensor");

    let tp = telemetry_payload("ESP1", 1_730_000_000_000, r#"{"co2_ppm":620}"#);
    let v: serde_json::Value = serde_json::from_str(&tp).unwrap();
    assert_eq!(v["ESP1"][0]["ts"], 1_730_000_000_000_u64);
    assert_eq!(v["ESP1"][0]["values"]["co2_ppm"], 620);
}

#[tokio::test]
async fn thingsboard_connects_and_enqueues_telemetry_via_outbox() {
    let Ok(url) = std::env::var("THINGSBOARD_TEST_URL") else {
        eprintln!(
            "skipping ThingsBoard integration test: set THINGSBOARD_TEST_URL \
             (e.g. mqtt://access-token@127.0.0.1:1883)"
        );
        return;
    };
    let url = url::Url::parse(&url).expect("THINGSBOARD_TEST_URL must be a valid mqtt URL");
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url.port().unwrap_or(1883);
    let token = url.username();

    let devices = vec![
        DeviceConfig { local_id: "esp1".into(), name: "ESP1".into(), profile: "CO2 Sensor".into() },
    ];
    let store = Store::connect("sqlite::memory:").await.unwrap();

    // Pre-populate one outbox row so the worker has something to publish
    use iot_gateway::telemetry::NormalizedTelemetry;
    use serde_json::{Map, json};
    let item = NormalizedTelemetry {
        device_id: "esp1".into(),
        thingsboard_name: "ESP1".into(),
        timestamp_ms: 1_730_000_000_000,
        values: Map::from_iter([("co2_ppm".into(), json!(620))]),
    };
    store.save_telemetry(&item, 1_000).await.unwrap();

    let _tb = ThingsBoard::start(host, port, token, &devices, store.clone())
        .await
        .expect("ThingsBoard start failed");

    // Give outbox worker time to deliver
    tokio::time::sleep(Duration::from_secs(3)).await;

    // After delivery the outbox should be empty
    let pending = store.pending(i64::MAX, 10).await.unwrap();
    assert!(pending.is_empty(), "outbox should be empty after delivery, got {} rows", pending.len());
}

