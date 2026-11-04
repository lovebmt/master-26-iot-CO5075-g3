# PC Gateway (Rust Monolith)

Monolith PC IoT Gateway for rubric items **T08** (CoreIOT telemetry sync & RPC routing) and **T10** (local dashboard).

## Architecture

- **Broker**: NanoMQ runs as a separate process (local MQTT on `127.0.0.1:1883`).
- **Monolith**: Rust (Tokio + Axum) consumes local telemetry, normalizes data, maintains SQLite latest-state & persistent outbox, forwards to ThingsBoard via MQTT Gateway API, and serves a server-rendered MiniJinja + HTMX dashboard.

## MQTT Contract

### Local Telemetry (NanoMQ)
- `iot/esp1/telemetry` — ESP1 CO2 payload: `{"ts": <epoch_ms>, "values": {"co2_ppm": 620}}`
- `iot/esp2/telemetry` — ESP2 TDS/Light payload: `{"ts": <epoch_ms>, "values": {"tds_ppm": 510, "light_on": true}}`
- Missing `ts` defaults to gateway receive time. Measurements must be finite non-negative numbers; `light_on` must be boolean.

### Commands (NanoMQ)
- `iot/{device_id}/command` — gateway publishes validated commands:
  `{"id": "<correlation_id>", "method": "set_light", "params": {"state": true}}`
- `iot/{device_id}/result` — device publishes acknowledgement:
  `{"id": "<correlation_id>", "success": true}`

### ThingsBoard (CoreIOT) Gateway API
- Gateway device authenticates with its **Access Token** as the MQTT username (no password).
- Connect downstream devices (QoS 1, wait PUBACK): `v1/gateway/connect`
  `{"device": "ESP1", "type": "CO2 Sensor"}`
- Telemetry: `v1/gateway/telemetry`
  `{"ESP1": [{"ts": 1730000000000, "values": {"co2_ppm": 620}}]}`
- RPC: Subscribe to `v1/gateway/rpc` for commands directed to downstream devices.

## Running Locally

### 1. Start NanoMQ
```bash
cd gateway
docker compose up -d
```

### 2. Configure Environment
```bash
cp .env.example .env
# Edit .env: set TB_GATEWAY_TOKEN from ThingsBoard UI
```

### 3. Run Gateway
```bash
cargo run --manifest-path Cargo.toml
```
Dashboard will be available at http://127.0.0.1:8080.

## Running Tests

```bash
cargo test --manifest-path Cargo.toml
```

To run the broker-backed NanoMQ flow test:
```bash
NANOMQ_TEST_URL=mqtt://127.0.0.1:1883 cargo test --manifest-path Cargo.toml --test nanomq_flow
```
