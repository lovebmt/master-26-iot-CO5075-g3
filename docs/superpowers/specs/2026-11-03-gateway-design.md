# Gateway Design

## Goal and scope

Build the PC IoT Gateway for rubric items T08 and T10. The Gateway receives device telemetry from a local NanoMQ broker, validates and normalizes it, keeps the latest known device state, forwards telemetry to ThingsBoard/CoreIOT, receives CoreIOT RPC commands, and exposes a small local monitoring/control UI.

The Gateway application is one Rust monolith. NanoMQ remains a separate broker process. The first version stores latest state only; it does not calculate time-window aggregates. The UI covers current device state and basic controls only. Add other features only after the user asks.

## Architecture and stack

- Rust stable with Tokio for asynchronous work.
- Axum serves routes and HTML. MiniJinja renders templates; HTMX requests refresh the status fragment; plain CSS styles the small dashboard.
- `rumqttc` connects to NanoMQ for device telemetry and command delivery, and to ThingsBoard's MQTT Gateway API for telemetry and RPC.
- SQLx with SQLite stores latest device state and a durable outbound telemetry outbox. SQLite transactions update state and enqueue cloud telemetry together. The outbox retries delivery after a ThingsBoard outage.
- Configuration comes from environment variables (or a local ignored `.env` file). Never commit broker passwords, ThingsBoard tokens, or private URLs.

Keep the implementation in a single Cargo package with focused modules for configuration, local MQTT, ThingsBoard MQTT, telemetry normalization, persistence, command routing, and web UI. Avoid separate services and a frontend build pipeline.

## Data and command flow

1. NanoMQ routes ESP telemetry to the Gateway over MQTT.
2. The Gateway validates device identity, JSON shape, numeric measurements, and units; normalizes the message with a device ID and timestamp.
3. One SQLite transaction updates that device's latest state and inserts the normalized telemetry into the outbox.
4. The cloud publisher sends queued messages through ThingsBoard's MQTT Gateway API, then marks successful rows delivered. A failed/disconnected cloud connection leaves rows queued for retry.
5. ThingsBoard RPC and local dashboard controls become validated device commands, then publish to that device's local MQTT command topic.

The initial device set follows the current architecture: ESP1 publishes CO₂; ESP2 publishes its own state and TDS received from ESP3 over BLE. The exact local topic names, payload schema, measurement keys, units, timestamps, QoS, NanoMQ credentials, and command acknowledgements must be agreed with firmware before integration. Proposed local topics are `iot/{device_id}/telemetry` and `iot/{device_id}/command`; retain them only if the firmware can use them.

## ThingsBoard/CoreIOT boundary

Use ThingsBoard's MQTT Gateway API. Register one Gateway device in the CoreIOT UI and enable its `Is gateway` setting. The Rust app connects to the ThingsBoard MQTT endpoint with that device's access token as the MQTT username; the token stays in runtime configuration. Local ESPs authenticate only to NanoMQ and do not need ThingsBoard credentials.

For each configured downstream ESP, publish `{"device":"<ThingsBoard device name>","type":"<device profile name>"}` to `v1/gateway/connect` and wait for its QoS 1 PUBACK before forwarding telemetry. The Gateway API creates the downstream device/profile if absent; pre-create devices when applying a specific template/profile or assignment is required. Publish telemetry to `v1/gateway/telemetry` with entries keyed by downstream device name. Subscribe to `v1/gateway/rpc`; route each message's target `device`, `data.id`, `data.method`, and `data.params` to the local device, then publish the result to the same topic with the request ID.

The configured development server's OpenAPI document reports ThingsBoard 3.9.1. The deployed service must still be checked during integration, since public Gateway API docs are rolling docs and not version-pinned. Confirm MQTT TLS/port, gateway-mode support, device profiles, QoS/PUBACK handling, and RPC behavior on that instance before coding against it. Do not use HTTP login/JWT or maintain per-device ThingsBoard credentials in this first version.

The initial CoreIOT work is telemetry forwarding and server-to-device RPC. CoreIOT device templates, dashboards, RuleChain, and threshold notifications remain under `thingsboard/` and are outside the Gateway application scope.

## Local UI

Provide one server-rendered dashboard with a latest-state table (device, measurements, last-seen/connection status) and the basic control required by the system, initially light on/off through ESP2. HTMX refreshes the table fragment. No SPA, charting library, user accounts, event history, or custom analytics in the first version.

## Failure handling and security

- Reject malformed or out-of-contract local telemetry and record a concise error without stopping the consumer.
- Keep cloud-bound telemetry in SQLite while ThingsBoard is unavailable; retry with bounded backoff.
- Validate RPC and dashboard commands against known device IDs and supported command payloads before publishing.
- Bind the dashboard to localhost by default. Keep credentials out of source control and avoid logging tokens or full credential-bearing URLs.

## Verification

Provide automated tests for telemetry validation/normalization, command validation/routing decisions, SQLite latest-state and outbox transactions/retry behavior, and dashboard HTML/HTMX routes. Add a local integration test using NanoMQ to verify MQTT ingestion, outbox delivery behavior, and command publishing without requiring the external CoreIOT instance. Document a separate optional end-to-end check against the configured CoreIOT server.

## Open implementation contract

Before integrating device firmware, confirm the local MQTT topic and payload contract with ESP1/ESP2 owners. Before implementing cloud MQTT, inspect the deployed CoreIOT/ThingsBoard version and verify the Gateway API topic and RPC format. These are implementation checks; they do not expand the feature scope.
