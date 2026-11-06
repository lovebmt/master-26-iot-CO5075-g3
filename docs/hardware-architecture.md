# IoT CO5075: hardware and system notes

## Hardware architecture

```text
ESP1 — CO₂ node
  CO₂ sensor over I²C (T01; use Tasmota-compatible setup)
  Wi-Fi + MQTT → local MQTT broker / PC Gateway
  Custom webserver (T07; shared rubric item with ESP2) 

ESP2 — BLE central / display / light node
  BLE central: receives soil temperature data over NUS from ESP3
  TFT LCD display (ILI9341)
  Light control
  Wi-Fi + MQTT → local MQTT broker / PC Gateway
  Custom webserver (T07; shared rubric item with ESP1)
  OTA firmware update (T14)

ESP3 — Soil temperature node (PT100 + RS485 Modbus)
  PT100 sensor over Modbus RS485 (T02)
  BLE NUS peripheral: sends soil temperature data to ESP2 (T03)

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
| T02 | Soil temperature sensor (PT100) over Modbus RS485 on ESP3 | Bus wiring, register reads, live readings |
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
2. ESP3 reads soil temperature over Modbus RS485 (PT100) and sends measurements to ESP2 over BLE NUS.
3. ESP2 displays received soil temperature and local/system status on TFT LCD, controls the light, and publishes received soil temperature data and its own telemetry to local MQTT over Wi-Fi.
4. The PC Gateway aggregates both streams, updates its local dashboard, and forwards telemetry to CoreIOT.
5. Commands from CoreIOT return through the Gateway and MQTT to the relevant ESP node.
6. The Gateway performs OTA for ESP2 (T14), if supported by the selected firmware/update mechanism.

## Implementation Details & Technical Specifications

- **T01 (ESP1 I²C Sensor):** YoloUNO (ESP32-S3) chạy Tasmota v16.0.0. Chân I²C SDA: GPIO 11, SCL: GPIO 12. Script Berry đọc định kỳ 5 giây, xuất bản tin MQTT `tele/g3-esp1/SENSOR`.
- **T02 & T03 (ESP3 PT100 Modbus & BLE NUS):** ESP32 Dev đóng vai trò BLE NUS Peripheral (quảng bá tên `ESP3-PT100`, Service UUID `6e400001-b5a3-f393-e0a9-e50e24dcca9e`). Đọc cảm biến nhiệt độ đất PT100 qua RS485 Modbus RTU (`soil_temp` từ 25.0°C đến 28.5°C). Định kỳ 2 giây gửi gói tin Notify JSON: `{"soil_temp": 27.0}`.
- **T07 (Custom Webserver on ESP2):** ESP2 phục vụ Web server tại cổng `8080` (`http://<IP_ESP2>:8080/`), cung cấp:
  - Hiển thị trực tiếp: `Soil Temp (PT100)`, trạng thái đèn LED, trạng thái kết nối BLE, IP và Wi-Fi.
  - Điều khiển đèn: Nút bật/tắt qua `POST /light`.
  - Cập nhật Firmware: Nút *Check for New FW* (`POST /check_fw`) và form nạp file binary trực tiếp (`POST /ota`).
- **MQTT Topics & Contracts:**
  - Telemetry: `iot/esp2/telemetry` với payload chuẩn `{"values":{"soil_temp": 27.0, "light_on": false, "fw_version": "1.0.2"}}`.
  - Command: `iot/esp2/command` nhận lệnh điều khiển đèn (`set_light`) và cập nhật OTA (`ota_update`).
  - Result: `iot/esp2/result` gửi phản hồi xác nhận trạng thái.
- **T14 (Cơ chế OTA Khép Kín):**
  - **Kho tập trung CoreIOT**: File nhị phân `esp2.bin` (1.26MB) được upload vào kho **CoreIOT OTA Packages** (Package `ESP2 Hub Firmware v1.0.3`, Profile `esp_hub`).
  - **URL tải CoreIOT**: `https://fuvi-server.tailc75d93.ts.net/api/otaPackage/{packageId}/download`.
  - **Gateway Cache & Local Delivery**: Gateway kéo firmware từ CoreIOT về lưu đệm tại `gateway/firmware/esp2.bin`, sau đó phục vụ nội bộ cho ESP2 qua HTTP `http://<IP_GATEWAY>:8080/firmware/esp2.bin`.
  - **Điều khiển OTA đa kênh**: Hỗ trợ nút cảm ứng **`[CHECK FW]`** trên màn hình LCD của ESP2 kèm popup hỏi xác nhận `[YES] [NO]`, nút bấm trên Web local `:8080`, và hai widget RPC trên Dashboard ThingsBoard (`Notify New FW` và `Force OTA Update`).
- **N03 (TFT LCD & Touchscreen):** Bo mạch **ESP32-2432S028R** sử dụng màn hình TFT ILI9341 320x240 và IC cảm ứng XPT2046 trên bus SPI3 riêng biệt, tích hợp LED RGB onboard (GPIO 4, 16, 17, Active-Low).

## Development server & Platform Architecture

- **Domain CoreIOT:** `https://fuvi-server.tailc75d93.ts.net` (IP Tailscale: `192.168.1.32`, user: `justin`).
- **Hạ tầng máy chủ:** Chạy Kubernetes bằng Minikube (profile `tb2`), Ingress `tb-ingress` trong namespace `thingboard-dev`.
- **Nginx Reverse Proxy:** Container `mega-nginx-minikube-tb2` (ánh xạ cổng `28880 -> 80`) và Ingress đã được cấu hình `client_max_body_size 100M;` (annotation `nginx.ingress.kubernetes.io/proxy-body-size: "100m"`) cho phép upload trực tiếp các file binary firmware ESP32 lớn (> 1MB).
- **Swagger UI:** https://fuvi-server.tailc75d93.ts.net/swagger-ui/index.html
- **API base URL:** https://fuvi-server.tailc75d93.ts.net
- **MQTT Gateway Port:** 10000 (TLS, insecure)
- **Tài khoản phát triển:** `master.iot@gmail.com` / `master.iot`
- **Dashboard:** `IoT System Dashboard — Group 3` (ID: `ce9732c0-d7f9-11f1-9301-8d833790fa1a`).
