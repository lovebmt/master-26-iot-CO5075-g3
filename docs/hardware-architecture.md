# IoT CO5075: hardware and system notes

## Hardware architecture

```text
ESP1 — CO₂ node
  CO₂ sensor over I²C (T01; use Tasmota-compatible setup)
  Wi-Fi + MQTT → local MQTT broker / PC Gateway
  Custom webserver (T07; shared rubric item with ESP2)

ESP2 — BLE central / display / light node
  BLE central: receives TDS data over NUS from ESP3
  LVGL display
  Light control
  Wi-Fi + MQTT → local MQTT broker / PC Gateway
  Custom webserver (T07; shared rubric item with ESP1)
  OTA firmware update (T14)

ESP3 — TDS node
  TDS sensor over Modbus RS485 (T02)
  BLE NUS peripheral: sends TDS data to ESP2 (T03)

PC Gateway
  MQTT broker + Gateway application
  Receives ESP2/ESP3 telemetry and aggregates it
  Local monitoring/control dashboard (T10)
  Two-way connection to CoreIOT (T08)

CoreIOT — development instance: fuvi-server
  Template and dashboard (T11)
  RuleChain (T12)
  Threshold notifications (T13)
```

The Gateway application and MQTT broker are separate responsibilities, even if both run on the same PC: the broker routes messages; the Gateway consumes local telemetry, aggregates it, communicates with CoreIOT, and routes control commands back to devices.

## Selected rubric items

| Code | Planned implementation | Evidence to capture |
|---|---|---|
| T01 | CO₂ sensing over I²C on ESP1 using Tasmota | Wiring, Tasmota configuration, live readings |
| T02 | TDS sensor over Modbus RS485 on ESP3 | Bus wiring, register reads, live readings |
| T03 | BLE NUS from ESP3 peripheral to ESP2 central | Service/characteristic discovery and received data |
| T07 | Custom webserver on ESP1 and/or ESP2 for local display/control (one rubric item) | Page served by device and a control action |
| T08 | PC Gateway sends telemetry to CoreIOT and routes CoreIOT commands back to devices | End-to-end telemetry and command logs |
| T10 | Local Gateway dashboard | Dashboard showing live values and control |
| T11 | CoreIOT template/dashboard | Device template and dashboard views |
| T12 | CoreIOT RuleChain | RuleChain configuration and triggered result |
| T13 | Threshold notification | Triggering telemetry and received notification |
| T14 | OTA update for ESP2 | Firmware version before/after and update log |
| N03 | LVGL display on ESP2 (bonus) | Working display showing system readings/status |

Rubric scores are earned only for completed features with report/demo evidence. Technology points are capped at 8; bonus points are capped at 1.

## Data and control flow

1. ESP1 reads CO₂ over I²C and publishes measurements to local MQTT over Wi-Fi.
2. ESP3 reads TDS over Modbus RS485 and sends measurements to ESP2 over BLE NUS.
3. ESP2 displays received TDS and local/system status on LVGL, controls the light, and publishes received TDS data and its own telemetry to local MQTT over Wi-Fi.
4. The PC Gateway aggregates both streams, updates its local dashboard, and forwards telemetry to CoreIOT.
5. Commands from CoreIOT return through the Gateway and MQTT to the relevant ESP node.
6. The Gateway performs OTA for ESP2 (T14), if supported by the selected firmware/update mechanism.

## Decisions and checks still required

- **T01 compatibility:** the rubric specifically says I²C on Tasmota. Verify the selected YoloUNO/ESP32-S3 Tasmota build supports the chosen CO₂ sensor and required MQTT behavior. A custom firmware implementation may not qualify for T01 without instructor approval.
- **T07 implementation:** the custom webserver can run on ESP1, ESP2, or both; this remains one rubric item. Confirm which values/controls it exposes and test coexistence with each board's other functions.
- **BLE format:** use Nordic UART Service (NUS), with ESP3 as peripheral and ESP2 as central. Confirm payload format, framing, and update rate before implementation.
- **MQTT topics/payloads:** define stable topic names, JSON fields, units, timestamps, and command acknowledgements before Gateway integration.
- **OTA scope:** current plan targets ESP2 only. Confirm the CoreIOT/Gateway update path is compatible with its firmware and bootloader.
- **Hardware details:** select sensor models, RS485 transceiver, light driver, power supply, pin assignments, and common-ground/wiring plan. Check YoloUNO pin availability and electrical limits before connecting modules.
- **Network:** keep MQTT on the trusted local network; use credentials from environment/config ignored by Git, never commit secrets.

## Development server

- Swagger UI: https://fuvi-server.tailc75d93.ts.net/swagger-ui/index.html
- API base URL: https://fuvi-server.tailc75d93.ts.net
- Authentication endpoint: `POST /api/auth/login`
- Development login: `master.iot@gmail.com` / `master.iot`
- These credentials are for the development instance only; do not reuse them for other services.

The development instance is useful for integration work. Confirm the instructor accepts it for grading; otherwise repeat the final CoreIOT evidence against the required course server.
