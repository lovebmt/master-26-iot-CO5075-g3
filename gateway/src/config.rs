use std::{collections::HashMap, net::SocketAddr, str::FromStr};

use anyhow::{bail, Result};

#[derive(Clone)]
pub struct Config {
    pub nanomq_host: String,
    pub nanomq_port: u16,
    pub nanomq_username: String,
    pub nanomq_password: String,
    pub tb_mqtt_host: String,
    pub tb_mqtt_port: u16,
    pub tb_gateway_device_name: String,
    pub tb_gateway_token: String,
    pub devices: [DeviceConfig; 2],
    pub database_url: String,
    pub http_bind: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    pub local_id: String,
    pub name: String,
    pub profile: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_iter(std::env::vars())
    }

    pub fn from_iter<I, K, V>(env: I) -> Result<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let env: HashMap<String, String> = env
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        let required = |key: &'static str| -> Result<String> {
            env.get(key)
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing or empty configuration: {key}"))
        };
        let port = |key: &'static str| -> Result<u16> {
            required(key)?
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid configuration: {key}"))
        };

        let nanomq_username = env.get("NANOMQ_USERNAME").cloned().unwrap_or_default();
        let nanomq_password = env.get("NANOMQ_PASSWORD").cloned().unwrap_or_default();
        let anonymous_allowed = env
            .get("NANOMQ_ALLOW_ANONYMOUS")
            .map(|value| {
                bool::from_str(value)
                    .map_err(|_| anyhow::anyhow!("invalid configuration: NANOMQ_ALLOW_ANONYMOUS"))
            })
            .transpose()?
            .unwrap_or(false);
        if (nanomq_username.is_empty() || nanomq_password.is_empty()) && !anonymous_allowed {
            bail!("empty NanoMQ credentials require NANOMQ_ALLOW_ANONYMOUS=true");
        }

        let http_bind = env
            .get("HTTP_BIND")
            .map(|value| {
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid configuration: HTTP_BIND"))
            })
            .transpose()?
            .unwrap_or_else(|| "127.0.0.1:8080".parse().expect("valid default socket address"));

        Ok(Self {
            nanomq_host: required("NANOMQ_HOST")?,
            nanomq_port: port("NANOMQ_PORT")?,
            nanomq_username,
            nanomq_password,
            tb_mqtt_host: required("TB_MQTT_HOST")?,
            tb_mqtt_port: port("TB_MQTT_PORT")?,
            tb_gateway_device_name: required("TB_GATEWAY_DEVICE_NAME")?,
            tb_gateway_token: required("TB_GATEWAY_TOKEN")?,
            devices: [
                DeviceConfig {
                    local_id: "esp1".into(),
                    name: required("TB_DEVICE_ESP1")?,
                    profile: required("TB_PROFILE_ESP1")?,
                },
                DeviceConfig {
                    local_id: "esp2".into(),
                    name: required("TB_DEVICE_ESP2")?,
                    profile: required("TB_PROFILE_ESP2")?,
                },
            ],
            database_url: required("DATABASE_URL")?,
            http_bind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use std::collections::HashMap;

    fn valid_env() -> HashMap<String, String> {
        [
            ("NANOMQ_HOST", "localhost"),
            ("NANOMQ_PORT", "1883"),
            ("NANOMQ_USERNAME", "local-user"),
            ("NANOMQ_PASSWORD", "local-secret"),
            ("TB_MQTT_HOST", "thingsboard.example"),
            ("TB_MQTT_PORT", "1883"),
            ("TB_GATEWAY_DEVICE_NAME", "PC Gateway"),
            ("TB_GATEWAY_TOKEN", "secret-value"),
            ("TB_DEVICE_ESP1", "ESP1"),
            ("TB_PROFILE_ESP1", "CO2 Sensor"),
            ("TB_DEVICE_ESP2", "ESP2"),
            ("TB_PROFILE_ESP2", "TDS Sensor"),
            ("DATABASE_URL", "sqlite://gateway.db?mode=rwc"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
    }

    #[test]
    fn parses_required_settings_and_maps_devices() {
        let config = Config::from_iter(valid_env()).unwrap();
        assert_eq!(config.nanomq_host, "localhost");
        assert_eq!(config.nanomq_port, 1883);
        assert_eq!(config.tb_mqtt_host, "thingsboard.example");
        assert_eq!(config.tb_mqtt_port, 1883);
        assert_eq!(config.tb_gateway_token, "secret-value");
        assert_eq!(config.devices[0].local_id, "esp1");
        assert_eq!(config.devices[0].name, "ESP1");
        assert_eq!(config.devices[0].profile, "CO2 Sensor");
        assert_eq!(config.devices[1].local_id, "esp2");
        assert_eq!(config.devices[1].name, "ESP2");
        assert_eq!(config.devices[1].profile, "TDS Sensor");
    }

    #[test]
    fn defaults_http_bind_to_localhost() {
        let config = Config::from_iter(valid_env()).unwrap();
        assert_eq!(config.http_bind.to_string(), "127.0.0.1:8080");
    }

    #[test]
    fn missing_gateway_token_is_rejected_without_echoing_secrets() {
        let mut env = valid_env();
        env.remove("TB_GATEWAY_TOKEN");
        let error = Config::from_iter(env).err().unwrap().to_string();
        assert!(error.contains("TB_GATEWAY_TOKEN"));
        assert!(!error.contains("secret-value"));
    }

    #[test]
    fn missing_nanomq_host_is_rejected() {
        let mut env = valid_env();
        env.remove("NANOMQ_HOST");
        let error = Config::from_iter(env).err().unwrap().to_string();
        assert!(error.contains("NANOMQ_HOST"));
    }

    #[test]
    fn missing_thingsboard_host_is_rejected() {
        let mut env = valid_env();
        env.remove("TB_MQTT_HOST");
        let error = Config::from_iter(env).err().unwrap().to_string();
        assert!(error.contains("TB_MQTT_HOST"));
    }

    #[test]
    fn rejects_empty_local_credentials_unless_anonymous_is_enabled() {
        let mut env = valid_env();
        env.insert("NANOMQ_USERNAME".into(), String::new());
        env.insert("NANOMQ_PASSWORD".into(), String::new());
        assert!(Config::from_iter(env.clone()).is_err());
        env.insert("NANOMQ_ALLOW_ANONYMOUS".into(), "true".into());
        assert!(Config::from_iter(env).is_ok());
    }
}
