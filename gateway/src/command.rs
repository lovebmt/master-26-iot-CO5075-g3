use anyhow::{bail, Result};
use serde::Deserialize;
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
}

/// Methods that the gateway will route to local devices.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Method {
    SetLight,
}

/// Parse and validate a ThingsBoard RPC message.
/// Input shape: {"device":"ESP2","data":{"id":42,"method":"set_light","params":{"state":true}}}
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
    build_command(
        device_id,
        tb_device_name.to_owned(),
        method_str,
        params,
        Some(rpc_id),
        new_correlation_id,
    )
}

/// Parse and validate a dashboard control request.
/// device_id: local id (e.g. "esp2"), method: "set_light", value: "on" | "off"
pub fn from_dashboard(
    device_id: &str,
    method: &str,
    value: &str,
    new_correlation_id: &mut dyn FnMut() -> String,
) -> Result<LocalCommand> {
    let params = match method {
        "set_light" => {
            let state = match value {
                "on" => true,
                "off" => false,
                _ => bail!("set_light value must be 'on' or 'off', got: {value}"),
            };
            serde_json::json!({"state": state})
        }
        _ => bail!("unsupported dashboard command: {method}"),
    };
    build_command(
        device_id.to_owned(),
        String::new(),
        method,
        &params,
        None,
        new_correlation_id,
    )
}

fn build_command(
    device_id: String,
    tb_device_name: String,
    method_str: &str,
    params: &Value,
    rpc_id: Option<u64>,
    new_correlation_id: &mut dyn FnMut() -> String,
) -> Result<LocalCommand> {
    let _method: Method = serde_json::from_value(Value::String(method_str.to_owned()))
        .map_err(|_| anyhow::anyhow!("unsupported method: {method_str}"))?;
    // Validate set_light params
    let state = params["state"]
        .as_bool()
        .ok_or_else(|| anyhow::anyhow!("set_light params.state must be a boolean"))?;
    let correlation_id = new_correlation_id();
    let topic = format!("iot/{device_id}/command");
    let payload = serde_json::json!({
        "id": correlation_id,
        "method": method_str,
        "params": {"state": state},
    })
    .to_string();
    Ok(LocalCommand {
        device_id,
        topic,
        correlation_id,
        payload,
        rpc_id,
        tb_device_name: if tb_device_name.is_empty() { None } else { Some(tb_device_name) },
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
    fn rpc_valid_set_light_routes_to_esp2() {
        let rpc = json!({"device":"ESP2","data":{"id":42,"method":"set_light","params":{"state":true}}});
        let mut id = seq_id();
        let cmd = from_rpc(&rpc, &lookup, &mut id).unwrap();
        assert_eq!(cmd.device_id, "esp2");
        assert_eq!(cmd.topic, "iot/esp2/command");
        assert_eq!(cmd.rpc_id, Some(42));
        assert_eq!(cmd.tb_device_name, Some("ESP2".into()));
        let p: Value = serde_json::from_str(&cmd.payload).unwrap();
        assert_eq!(p["method"], "set_light");
        assert_eq!(p["params"]["state"], true);
        assert!(!cmd.correlation_id.is_empty());
    }

    #[test]
    fn rpc_unknown_device_is_rejected() {
        let rpc = json!({"device":"ESP99","data":{"id":1,"method":"set_light","params":{"state":true}}});
        let mut id = seq_id();
        assert!(from_rpc(&rpc, &lookup, &mut id).is_err());
    }

    #[test]
    fn rpc_unknown_method_is_rejected() {
        let rpc = json!({"device":"ESP2","data":{"id":1,"method":"reboot","params":{}}});
        let mut id = seq_id();
        assert!(from_rpc(&rpc, &lookup, &mut id).is_err());
    }

    #[test]
    fn rpc_missing_params_state_is_rejected() {
        let rpc = json!({"device":"ESP2","data":{"id":1,"method":"set_light","params":{}}});
        let mut id = seq_id();
        assert!(from_rpc(&rpc, &lookup, &mut id).is_err());
    }

    #[test]
    fn rpc_malformed_missing_data_id_is_rejected() {
        let rpc = json!({"device":"ESP2","data":{"method":"set_light","params":{"state":true}}});
        let mut id = seq_id();
        assert!(from_rpc(&rpc, &lookup, &mut id).is_err());
    }

    #[test]
    fn dashboard_set_light_on_accepted() {
        let mut id = seq_id();
        let cmd = from_dashboard("esp2", "set_light", "on", &mut id).unwrap();
        let p: Value = serde_json::from_str(&cmd.payload).unwrap();
        assert_eq!(p["params"]["state"], true);
        assert_eq!(cmd.topic, "iot/esp2/command");
        assert!(cmd.rpc_id.is_none());
    }

    #[test]
    fn dashboard_set_light_off_accepted() {
        let mut id = seq_id();
        let cmd = from_dashboard("esp2", "set_light", "off", &mut id).unwrap();
        let p: Value = serde_json::from_str(&cmd.payload).unwrap();
        assert_eq!(p["params"]["state"], false);
    }

    #[test]
    fn dashboard_invalid_value_rejected() {
        let mut id = seq_id();
        assert!(from_dashboard("esp2", "set_light", "maybe", &mut id).is_err());
    }

    #[test]
    fn dashboard_unknown_method_rejected() {
        let mut id = seq_id();
        assert!(from_dashboard("esp2", "reboot", "on", &mut id).is_err());
    }

    #[test]
    fn rpc_does_not_produce_publish_for_unknown_device() {
        // Confirm the command is an Err (no LocalCommand produced = no publish)
        let rpc = json!({"device":"ESP99","data":{"id":1,"method":"set_light","params":{"state":true}}});
        let mut id = seq_id();
        let result = from_rpc(&rpc, &lookup, &mut id);
        assert!(result.is_err(), "unknown device must not produce a command");
    }
}

