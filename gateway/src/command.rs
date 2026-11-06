use anyhow::{bail, Result};

use serde_json::Value;

/// A validated, routable command ready to publish on the local broker.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalCommand {
    /// Local device id (e.g. "esp2")
    pub device_id: String,
    /// Local MQTT command topic
    pub topic: String,
    /// Correlation id echoed back in the result message
    pub correlation_id: String,
    /// JSON payload for the device
    pub payload: String,
    /// Original ThingsBoard RPC id (None when sent from dashboard)
    pub rpc_id: Option<u64>,
    /// ThingsBoard device name for the RPC reply
    pub tb_device_name: Option<String>,
    /// Desired state for set_light
    pub state: Option<bool>,
    /// Whether this is a get_light query
    pub is_get: bool,
    /// Whether this is an OTA update command
    pub is_ota: bool,
}

/// Parse and validate a ThingsBoard RPC message.
/// Input shape: {"device":"ESP2","data":{"id":42,"method":"set_light","params":true}}
/// or: {"device":"ESP2","data":{"id":42,"method":"ota_update","params":{"url":"...","version":"v1.1","force":false}}}
pub fn from_rpc(
    rpc_json: &Value,
    tb_name_to_device_id: &dyn Fn(&str) -> Option<String>,
    new_correlation_id: &mut dyn FnMut() -> String,
) -> Result<LocalCommand> {
    let tb_device_name = rpc_json["device"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("RPC missing device field"))?;
    let device_id = tb_name_to_device_id(tb_device_name)
        .ok_or_else(|| anyhow::anyhow!("RPC targets unknown device: {tb_device_name}"))?;
    let data = &rpc_json["data"];
    let rpc_id = data["id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("RPC missing data.id"))?;
    let method_str = data["method"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("RPC missing data.method"))?;
    let params = &data["params"];

    if method_str == "get_light" {
        return Ok(LocalCommand {
            device_id,
            topic: String::new(),
            correlation_id: String::new(),
            payload: String::new(),
            rpc_id: Some(rpc_id),
            tb_device_name: Some(tb_device_name.to_owned()),
            state: None,
            is_get: true,
            is_ota: false,
        });
    }

    if method_str == "ota_update" || method_str == "set_firmware" {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("http://192.168.1.19:8080/firmware/esp2.bin");
        let version = params
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("v1.0.1");
        let force = params
            .get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let correlation_id = new_correlation_id();
        let topic = format!("iot/{device_id}/command");
        let payload = serde_json::json!({
            "id": correlation_id,
            "method": "ota_update",
            "params": {
                "url": url,
                "version": version,
                "force": force
            }
        })
        .to_string();

        return Ok(LocalCommand {
            device_id,
            topic,
            correlation_id,
            payload,
            rpc_id: Some(rpc_id),
            tb_device_name: Some(tb_device_name.to_owned()),
            state: None,
            is_get: false,
            is_ota: true,
        });
    }

    let state = match params {
        Value::Bool(b) => *b,
        Value::Object(map) => map
            .get("state")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!("set_light params.state must be a boolean"))?,
        _ => bail!("unsupported RPC method: {method_str}"),
    };

    let correlation_id = new_correlation_id();
    let topic = format!("iot/{device_id}/command");
    let payload = serde_json::json!({
        "id": correlation_id,
        "method": "set_light",
        "params": {"state": state},
    })
    .to_string();

    Ok(LocalCommand {
        device_id,
        topic,
        correlation_id,
        payload,
        rpc_id: Some(rpc_id),
        tb_device_name: Some(tb_device_name.to_owned()),
        state: Some(state),
        is_get: false,
        is_ota: false,
    })
}

/// Parse and validate a dashboard control request.
/// device_id: local id (e.g. "esp2"), method: "set_light", value: "on" | "off"
pub fn from_dashboard(
    device_id: &str,
    method: &str,
    value: &str,
    new_correlation_id: &mut dyn FnMut() -> String,
) -> Result<LocalCommand> {
    if method != "set_light" {
        bail!("unsupported dashboard command: {method}");
    }
    let state = match value {
        "on" => true,
        "off" => false,
        _ => bail!("set_light value must be 'on' or 'off', got: {value}"),
    };

    let correlation_id = new_correlation_id();
    let topic = format!("iot/{device_id}/command");
    let payload = serde_json::json!({
        "id": correlation_id,
        "method": "set_light",
        "params": {"state": state},
    })
    .to_string();

    Ok(LocalCommand {
        device_id: device_id.to_owned(),
        topic,
        correlation_id,
        payload,
        rpc_id: None,
        tb_device_name: None,
        state: Some(state),
        is_get: false,
        is_ota: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn seq_id() -> impl FnMut() -> String {
        let mut n = 0u32;
        move || { n += 1; format!("test-id-{n}") }
    }

    fn lookup(name: &str) -> Option<String> {
        match name {
            "ESP2" => Some("esp2".into()),
            _ => None,
        }
    }

    #[test]
    fn rpc_valid_set_light_object() {
        let rpc = json!({"device":"ESP2","data":{"id":42,"method":"set_light","params":{"state":true}}});
        let mut id = seq_id();
        let cmd = from_rpc(&rpc, &lookup, &mut id).unwrap();
        assert_eq!(cmd.device_id, "esp2");
        assert_eq!(cmd.state, Some(true));
        assert!(!cmd.is_get);
        assert!(!cmd.is_ota);
    }

    #[test]
    fn rpc_valid_set_light_boolean_primitive() {
        let rpc = json!({"device":"ESP2","data":{"id":5,"method":"set_light","params":true}});
        let mut id = seq_id();
        let cmd = from_rpc(&rpc, &lookup, &mut id).unwrap();
        assert_eq!(cmd.device_id, "esp2");
        assert_eq!(cmd.state, Some(true));
        assert!(!cmd.is_get);
        assert!(!cmd.is_ota);
    }

    #[test]
    fn rpc_valid_get_light() {
        let rpc = json!({"device":"ESP2","data":{"id":4,"method":"get_light","params":null}});
        let mut id = seq_id();
        let cmd = from_rpc(&rpc, &lookup, &mut id).unwrap();
        assert_eq!(cmd.device_id, "esp2");
        assert!(cmd.is_get);
        assert!(!cmd.is_ota);
    }

    #[test]
    fn rpc_valid_ota_update() {
        let rpc = json!({
            "device": "ESP2",
            "data": {
                "id": 99,
                "method": "ota_update",
                "params": {
                    "url": "http://192.168.1.19:8080/firmware/esp2.bin",
                    "version": "v1.2.0",
                    "force": true
                }
            }
        });
        let mut id = seq_id();
        let cmd = from_rpc(&rpc, &lookup, &mut id).unwrap();
        assert_eq!(cmd.device_id, "esp2");
        assert!(cmd.is_ota);
        assert_eq!(cmd.rpc_id, Some(99));
        assert!(cmd.payload.contains("ota_update"));
        assert!(cmd.payload.contains("v1.2.0"));
        assert!(cmd.payload.contains("force"));
    }
}

