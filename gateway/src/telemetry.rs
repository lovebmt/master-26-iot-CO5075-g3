use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use serde_json::{Map, Value};

use crate::config::DeviceConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedTelemetry {
    pub device_id: String,
    pub thingsboard_name: String,
    pub timestamp_ms: u64,
    pub values: Map<String, Value>,
}

pub fn parse(
    device_id: &str,
    payload: &[u8],
    devices: &[DeviceConfig],
) -> Result<NormalizedTelemetry> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| anyhow!("system clock is before Unix epoch"))?
        .as_millis()
        .try_into()
        .map_err(|_| anyhow!("system timestamp is out of range"))?;
    parse_at(device_id, payload, devices, now_ms)
}

/// Normalize the supported local MQTT formats using the gateway receive time.
/// Tasmota's local `Time` string is deliberately not treated as a UTC timestamp.
pub fn parse_mqtt_at(
    topic: &str,
    payload: &[u8],
    devices: &[DeviceConfig],
    received_at_ms: u64,
) -> Result<NormalizedTelemetry> {
    if topic == "tele/g3-esp1/SENSOR" {
        let document: Value = serde_json::from_slice(payload)
            .map_err(|_| anyhow!("invalid Tasmota JSON"))?;
        if document.get("TempUnit").and_then(Value::as_str) != Some("C") {
            bail!("Tasmota TempUnit must be C");
        }
        let sensor = document.get("DHT20")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Tasmota DHT20 must be an object"))?;
        let normalized = serde_json::json!({
            "values": {
                "temperature": sensor.get("Temperature"),
                "humidity": sensor.get("Humidity"),
            }
        });
        return parse_at("esp1", &serde_json::to_vec(&normalized)?, devices, received_at_ms);
    }

    let device_id = match topic {
        "iot/esp1/telemetry" => "esp1",
        "iot/esp2/telemetry" => "esp2",
        "iot/esp3/telemetry" => "esp3",
        _ => bail!("unsupported telemetry topic: {topic}"),
    };
    parse_at(device_id, payload, devices, received_at_ms)
}

fn parse_at(
    device_id: &str,
    payload: &[u8],
    devices: &[DeviceConfig],
    now_ms: u64,
) -> Result<NormalizedTelemetry> {
    let device = devices
        .iter()
        .find(|device| device.local_id == device_id)
        .ok_or_else(|| anyhow!("unknown device: {device_id}"))?;
    let document: Value =
        serde_json::from_slice(payload).map_err(|_| anyhow!("invalid telemetry JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| anyhow!("telemetry must be an object"))?;
    let timestamp_ms = match object.get("ts") {
        None => now_ms,
        Some(value) => value
            .as_u64()
            .filter(|timestamp| *timestamp <= i64::MAX as u64)
            .ok_or_else(|| anyhow!("ts must fit a non-negative SQLite integer"))?,
    };
    let values = object
        .get("values")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("values must be an object"))?;
    if values.is_empty() {
        bail!("values must not be empty");
    }

    let mut normalized = Map::new();
    for (key, value) in values {
        match (device_id, key.as_str()) {
            ("esp1", "co2_ppm")
            | ("esp2", "tds_ppm")
            | ("esp3", "tds_ppm") => {
                value
                    .as_f64()
                    .filter(|number| number.is_finite() && *number >= 0.0)
                    .ok_or_else(|| anyhow!("{key} must be a finite non-negative number"))?;
                normalized.insert(key.clone(), value.clone());
            }
            ("esp1", "temperature")
            | ("esp1", "temp")
            | ("esp2", "soil_temp")
            | ("esp2", "soil_temp_c")
            | ("esp3", "soil_temp")
            | ("esp3", "soil_temp_c") => {
                value
                    .as_f64()
                    .filter(|number| number.is_finite() && *number >= -50.0 && *number <= 150.0)
                    .ok_or_else(|| anyhow!("{key} must be a finite temperature"))?;
                normalized.insert(key.clone(), value.clone());
            }
            ("esp1", "humidity")
            | ("esp1", "humi") => {
                value
                    .as_f64()
                    .filter(|number| number.is_finite() && *number >= 0.0 && *number <= 100.0)
                    .ok_or_else(|| anyhow!("{key} must be a valid humidity"))?;
                normalized.insert(key.clone(), value.clone());
            }
            ("esp2", "light_on") if value.is_boolean() => {
                normalized.insert(key.clone(), value.clone());
            }
            ("esp1" | "esp2" | "esp3", "fw_version" | "version") if value.is_string() => {
                normalized.insert(key.clone(), value.clone());
            }
            _ => bail!("unknown measurement for {device_id}: {key}"),
        }
    }

    Ok(NormalizedTelemetry {
        device_id: device_id.to_owned(),
        thingsboard_name: device.name.clone(),
        timestamp_ms,
        values: normalized,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_at, parse_mqtt_at};
    use crate::config::DeviceConfig;

    fn devices() -> [DeviceConfig; 2] {
        [
            DeviceConfig {
                local_id: "esp1".into(),
                name: "CO2 Sensor".into(),
                profile: "CO2".into(),
            },
            DeviceConfig {
                local_id: "esp2".into(),
                name: "TDS Sensor".into(),
                profile: "TDS".into(),
            },
        ]
    }

    #[test]
    fn parses_valid_esp1_and_esp2_payloads() {
        let esp1 = parse_at(
            "esp1",
            br#"{"ts":1730000000000,"values":{"co2_ppm":620}}"#,
            &devices(),
            1,
        )
        .unwrap();
        assert_eq!(esp1.timestamp_ms, 1_730_000_000_000);
        assert_eq!(esp1.values["co2_ppm"], 620);
        assert_eq!(esp1.thingsboard_name, "CO2 Sensor");

        let esp2 = parse_at(
            "esp2",
            br#"{"values":{"tds_ppm":310,"light_on":true}}"#,
            &devices(),
            1,
        )
        .unwrap();
        assert_eq!(esp2.timestamp_ms, 1);
        assert_eq!(esp2.values["tds_ppm"], 310);
        assert_eq!(esp2.values["light_on"], true);
    }

    #[test]
    fn rejects_invalid_payloads() {
        let invalid: [(&str, &str, &[u8]); 9] = [
            ("unknown device", "esp3", br#"{"values":{"co2_ppm":1}}"#),
            ("malformed json", "esp1", b"{"),
            (
                "unknown measurement",
                "esp1",
                br#"{"values":{"pressure":1}}"#,
            ),
            ("missing values", "esp1", br#"{"ts":1}"#),
            ("empty values", "esp1", br#"{"values":{}}"#),
            (
                "string measurement",
                "esp1",
                br#"{"values":{"co2_ppm":"620"}}"#,
            ),
            (
                "negative measurement",
                "esp1",
                br#"{"values":{"co2_ppm":-1}}"#,
            ),
            (
                "non-finite measurement",
                "esp1",
                br#"{"values":{"co2_ppm":1e400}}"#,
            ),
            (
                "wrong light_on type",
                "esp2",
                br#"{"values":{"light_on":1}}"#,
            ),
        ];
        for (case, device, payload) in invalid {
            assert!(parse_at(device, payload, &devices(), 1).is_err(), "accepted {case}");
        }
    }

    #[test]
    fn falls_back_to_injected_receive_timestamp() {
        let item = parse_at("esp1", br#"{"values":{"co2_ppm":620}}"#, &devices(), 42).unwrap();
        assert_eq!(item.timestamp_ms, 42);
    }

    #[test]
    fn maps_tasmota_dht20_to_esp1_using_receive_time() {
        let payload = br#"{"Time":"2026-11-05T03:10:10","DHT20":{"Temperature":28.53,"Humidity":63.06},"TempUnit":"C"}"#;
        let item = parse_mqtt_at("tele/g3-esp1/SENSOR", payload, &devices(), 42).unwrap();
        assert_eq!(item.device_id, "esp1");
        assert_eq!(item.thingsboard_name, devices()[0].name);
        assert_eq!(item.timestamp_ms, 42);
        assert_eq!(item.values.len(), 2);
        assert_eq!(item.values["temperature"], 28.53);
        assert_eq!(item.values["humidity"], 63.06);
    }

    #[test]
    fn rejects_incomplete_invalid_and_non_celsius_tasmota_samples() {
        for payload in [
            "{",
            r#"{"TempUnit":"C"}"#,
            r#"{"DHT20":{},"TempUnit":"C"}"#,
            r#"{"DHT20":{"Temperature":28},"TempUnit":"C"}"#,
            r#"{"DHT20":{"Temperature":"28","Humidity":60},"TempUnit":"C"}"#,
            r#"{"DHT20":{"Temperature":null,"Humidity":60},"TempUnit":"C"}"#,
            r#"{"DHT20":{"Temperature":200,"Humidity":60},"TempUnit":"C"}"#,
            r#"{"DHT20":{"Temperature":28,"Humidity":101},"TempUnit":"C"}"#,
            r#"{"DHT20":{"Temperature":28,"Humidity":-1},"TempUnit":"C"}"#,
            r#"{"DHT20":{"Temperature":82,"Humidity":60},"TempUnit":"F"}"#,
            r#"{"DHT20":{"Temperature":28,"Humidity":60}}"#,
        ] {
            assert!(parse_mqtt_at("tele/g3-esp1/SENSOR", payload.as_bytes(), &devices(), 42).is_err(),
                "accepted invalid Tasmota sample: {payload}");
        }
    }

    #[test]
    fn preserves_normalized_topics_and_rejects_wrong_topic_formats() {
        let mut configured = devices().to_vec();
        configured.push(DeviceConfig {
            local_id: "esp3".into(), name: "ESP3".into(), profile: "esp_ble".into(),
        });
        for (topic, payload, id) in [
            ("iot/esp1/telemetry", r#"{"ts":7,"values":{"temperature":28,"humidity":60}}"#, "esp1"),
            ("iot/esp2/telemetry", r#"{"ts":7,"values":{"soil_temp":27,"light_on":false}}"#, "esp2"),
            ("iot/esp3/telemetry", r#"{"ts":7,"values":{"soil_temp":27}}"#, "esp3"),
        ] {
            let item = parse_mqtt_at(topic, payload.as_bytes(), &configured, 42).unwrap();
            assert_eq!(item.device_id, id);
            assert_eq!(item.timestamp_ms, 7);
        }
        let raw = br#"{"DHT20":{"Temperature":28,"Humidity":60},"TempUnit":"C"}"#;
        assert!(parse_mqtt_at("iot/esp1/telemetry", raw, &configured, 42).is_err());
        assert!(parse_mqtt_at("tele/another-device/SENSOR", raw, &configured, 42).is_err());
    }

    #[test]
    fn timestamp_must_fit_sqlite_signed_integer() {
        let max = parse_at(
            "esp1",
            br#"{"ts":9223372036854775807,"values":{"co2_ppm":620}}"#,
            &devices(),
            1,
        )
        .unwrap();
        assert_eq!(max.timestamp_ms, i64::MAX as u64);
        assert!(parse_at(
            "esp1",
            br#"{"ts":9223372036854775808,"values":{"co2_ppm":620}}"#,
            &devices(),
            1,
        )
        .is_err());
    }

    #[test]
    fn accepts_partial_updates_and_rejects_boolean_numeric_values() {
        let partial = parse_at("esp2", br#"{"values":{"light_on":false}}"#, &devices(), 1).unwrap();
        assert_eq!(partial.values.len(), 1);
        assert!(parse_at("esp1", br#"{"values":{"co2_ppm":true}}"#, &devices(), 1).is_err());
    }
}
