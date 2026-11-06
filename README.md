# IoT CO5075 — Group 3: Smart Mushroom Farm Monitoring & Control System

Monorepo cho bài tập lớn môn **Internet of Things (CO5075)** — Nhóm 3. Dự án triển khai hệ thống IoT hoàn chỉnh bao gồm firmware vi điều khiển (ESP1, ESP2, ESP3), PC Gateway (Rust monolith + NanoMQ broker), và nền tảng giám sát đám mây CoreIOT (ThingsBoard).

---

## 1. Kiến trúc hệ thống (System Architecture)

```mermaid
flowchart TD
    subgraph SENSORS["Lớp Cảm Biến & Thiết Bị Ngoại Vi"]
        DHT["DHT20 / Cảm biến môi trường"] -->|I²C| ESP1["ESP1: Node Đo Môi Trường<br/>(YoloUNO ESP32-S3 / Tasmota)"]
        PT100["Cảm biến Nhiệt Độ Đất PT100"] -->|RS485 Modbus RTU| ESP3["ESP3: Node Đo Nhiệt Độ Đất<br/>(ESP32 Dev / Arduino)"]
    end

    subgraph HUB["Lớp Hub Trung Tâm & Hiển Thị"]
        ESP3 -->|BLE NUS (Notify: soil_temp)| ESP2["ESP2: Hub Trung Tâm & Hiển Thị<br/>(ESP32-2432S028R / ILI9341 + XPT2046)<br/>- LCD: Soil Temp, WiFi, Light, Check FW<br/>- Web Server: 8080<br/>- RGB Light Controller"]
    end

    subgraph GATEWAY["Lớp PC Gateway (Local Edge)"]
        BROKER["NanoMQ Broker (127.0.0.1:1883)"]
        GW["Rust IoT Gateway (Axum + SQLite Outbox)<br/>- Web Dashboard: http://127.0.0.1:8080<br/>- Local FW Cache: /firmware/esp2.bin"]
        BROKER <--> GW
    end

    subgraph CLOUD["Lớp Nền Tảng Đám Mây (CoreIOT / ThingsBoard)"]
        TB["CoreIOT Server (fuvi-server.tailc75d93.ts.net)<br/>- MQTT Gateway Port: 10000<br/>- Dashboard: Soil Temp, Light Control, OTA Buttons<br/>- OTA Packages Repository: ESP2 Hub Firmware v1.0.3"]
    end

    ESP1 -->|WiFi MQTT| BROKER
    ESP2 <-->|WiFi MQTT| BROKER
    ESP2 -.->|Tải OTA LAN: http://192.168.1.19:8080| GW
    GW <-->|MQTT Gateway API: v1/gateway/...| TB
    GW -.->|Kéo Firmware .bin từ OTA Packages| TB
```

---

## 2. Các thành phần chính (Components)

### 2.1. Thiết bị phần cứng & Firmware (`firmware/`)
* **ESP1 (`firmware/esp1/`)**:
  * Vi điều khiển: YoloUNO (ESP32-S3), chạy firmware Tasmota.
  * Cảm biến: DHT20 đo nhiệt độ, độ ẩm qua I²C.
  * Truyền thông: Wi-Fi MQTT gửi trực tiếp bản tin cảm biến `tele/g3-esp1/SENSOR` tới Broker.
* **ESP2 (`firmware/esp2/`)**:
  * Vi điều khiển: ESP32 Cheap Yellow Display (**ESP32-2432S028R**).
  * Màn hình: LCD ILI9341 320x240 cảm ứng điện trở XPT2046.
    - Dòng trạng thái: `ESP2 - Ver: 1.0.2`.
    - Hiển thị giá trị: `Soil: 27.0 C (PT100)` nhận từ ESP3 qua BLE.
    - Nút cảm ứng tại chỗ: Bật/tắt đèn và **`[CHECK FW]`** kiểm tra cập nhật.
    - Hộp thoại OTA Prompt: Popup hỏi xác nhận `[YES] [NO]` trên màn hình.
  * Truyền thông: BLE Central (kết nối ESP3 NUS) + Wi-Fi MQTT (`iot/esp2/telemetry`, `iot/esp2/command`).
  * Web Server cục bộ (`:8080`): Hiển thị trạng thái, điều khiển đèn, form nạp OTA trực tiếp.
* **ESP3 (`firmware/esp3/`)**:
  * Vi điều khiển: ESP32 Dev Module.
  * Chức năng: Đọc nhiệt độ đất từ cảm biến PT100 qua RS485 Modbus RTU (`soil_temp` từ 25.0°C đến 28.5°C).
  * Truyền thông: BLE NUS Peripheral phát broadcast `{"soil_temp": 27.0}` mỗi 2 giây tới ESP2.

### 2.2. PC Gateway (`gateway/`)
* Ngôn ngữ: Rust (Tokio asynchronous runtime + Axum Web framework + SQLite Outbox Pattern).
* Chức năng:
  * Kết nối NanoMQ Broker cục bộ, tiếp nhận và chuẩn hóa telemetry từ ESP1, ESP2, ESP3.
  * Lưu trữ dữ liệu SQLite chống mất mát khi rớt mạng (Offline Outbox Queue).
  * Đồng bộ 2 chiều lên CoreIOT qua giao thức ThingsBoard MQTT Gateway (`v1/gateway/telemetry`, `v1/gateway/rpc`).
  * Giao diện Dashboard nội bộ (HTMX + MiniJinja tại `http://127.0.0.1:8080/devices`): Giám sát trạng thái thiết bị thời gian thực, điều khiển đèn, kích hoạt OTA và quản lý hàng đợi offline.
  * Firmware Server nội bộ (`GET /firmware/esp2.bin` & `GET /firmware/version.json`): Cung cấp file nhị phân OTA cho ESP2 trong mạng LAN.

### 2.3. Nền tảng CoreIOT / ThingsBoard (`thingsboard/`)
* Máy chủ: `fuvi-server.tailc75d93.ts.net` (Cụm Minikube profile `tb2`, Ingress Nginx proxy tối đa 100MB).
* Device Profiles:
  * `esp_temp_humi`: Thiết bị ESP1.
  * `esp_hub`: Thiết bị ESP2 (Hub hiển thị, điều khiển đèn, OTA).
  * `esp_ble`: Thiết bị ESP3.
* Dashboard (`thingsboard/dashboard_group3.json`):
  * **ESP2 Timeseries Table**: Giám sát thời gian thực `Soil Temp (°C)`, `Light State`, `FW Version`.
  * **Control ESP2 Light**: Widget công tắc bật/tắt đèn từ xa (RPC `set_light`).
  * **CoreIOT OTA Packages — Version Selector & Dispatcher**: Widget tích hợp cho phép chọn phiên bản firmware từ kho CoreIOT (`v1.0.4`, `v1.0.5`, `v1.0.6`, `v1.0.3`) và kích hoạt gửi lệnh `Notify ESP2 (Prompt)` hoặc `Force OTA Update`.
* Kho lưu trữ **CoreIOT OTA Packages**:
  * Package: `ESP2 Hub Firmware v1.0.3` lưu trữ trực tiếp file nhị phân (1.26MB, SHA-256 Checksum).
  * Quy trình OTA khép kín: CoreIOT $\rightarrow$ Gateway Cache $\rightarrow$ ESP2 LAN Download (không dùng external URL).

---

## 3. Cấu trúc thư mục (Directory Structure)

```text
.
├── docs/                      # Tài liệu kiến trúc, đặc tả phần cứng và phân công
│   ├── overview.md            # Tổng quan đề tài, quy tắc điều khiển và cơ chế OTA
│   ├── hardware-architecture.md# Đặc tả kiến trúc phần cứng, chân kết nối và ma trận Txx
│   └── README.md
├── firmware/                  # Mã nguồn firmware các vi điều khiển
│   ├── esp1/                  # Cấu hình Tasmota và script Berry đọc I²C DHT20
│   ├── esp2/                  # Mã nguồn PlatformIO C/C++ cho ESP2 (ESP32-2432S028R)
│   ├── esp3/                  # Mã nguồn PlatformIO C/C++ cho ESP3 PT100 Modbus
│   └── README.md
├── gateway/                   # Mã nguồn Rust PC Gateway
│   ├── src/                   # Logic Gateway, MQTT, Telemetry, Web UI, Storage
│   ├── templates/             # Giao diện MiniJinja HTML + HTMX
│   ├── firmware/              # Thư mục lưu đệm file firmware OTA (.bin)
│   └── README.md
├── thingsboard/               # Cấu hình mẫu CoreIOT
│   ├── dashboard_group3.json  # File export Dashboard nhóm 3
│   └── README.md
└── present/                   # Báo cáo, slide thuyết trình và video demo
    ├── report/                # Báo cáo Word / PDF
    ├── slides/                # Slide báo cáo
    └── demo/                  # Video clip minh chứng nghiệm thu
```

---

## 4. Hướng dẫn chạy nhanh (Quick Start)

### 4.1. Khởi chạy Gateway & Broker
1. Khởi động broker NanoMQ:
   ```bash
   cd gateway && docker compose up -d
   ```
2. Cấu hình biến môi trường:
   ```bash
   cp .env.example .env
   # Điền thông tin TB_GATEWAY_TOKEN từ CoreIOT
   ```
3. Chạy Gateway:
   ```bash
   cargo run
   ```
   Truy cập Dashboard tại `http://127.0.0.1:8080`.

### 4.2. Biên dịch và nạp Firmware vi điều khiển
Yêu cầu: Đã cài đặt PlatformIO (`pio`).
* **Nạp ESP3 (Soil Temp Modbus node):**
  ```bash
  pio run -d firmware/esp3 -t upload --upload-port /dev/cu.usbserial-130
  ```
* **Nạp ESP2 (Central Hub & Display):**
  ```bash
  pio run -d firmware/esp2 -t upload --upload-port /dev/cu.usbserial-140
  ```

---

## 5. Minh chứng tiêu chí công nghệ (Rubric Checklist)

| Mã | Nội dung tiêu chí | Trạng thái | Minh chứng thực tế |
|:---|:---|:---:|:---|
| **T01** | Đo khí môi trường/nhiệt độ độ ẩm I²C trên ESP1 (Tasmota) | Đạt | Script Berry đọc DHT20, xuất bản tin MQTT `tele/g3-esp1/SENSOR` |
| **T02** | Đo nhiệt độ đất (PT100) qua Modbus RS485 trên ESP3 | Đạt | Mã nguồn `firmware/esp3`, đọc `soil_temp` qua Modbus RTU |
| **T03** | Truyền thông BLE NUS từ ESP3 sang ESP2 | Đạt | Log nhận `BLE Soil Temp received: 27.0 C` liên tục trên ESP2 |
| **T07** | Custom Webserver cục bộ trên ESP2 (`:8080`) | Đạt | Web UI hiển thị `Soil Temp`, nút điều khiển đèn, form nạp OTA trực tiếp |
| **T08** | Gateway PC kết nối 2 chiều với CoreIOT | Đạt | Đẩy telemetry `soil_temp`, định tuyến RPC `set_light` và `ota_update` |
| **T10** | Dashboard cục bộ trên PC Gateway | Đạt | Dashboard HTMX `http://127.0.0.1:8080` hiển thị trạng thái và nút điều khiển |
| **T11** | CoreIOT Template & Dashboard | Đạt | Dashboard `IoT System Dashboard — Group 3` hiển thị Soil Temp, Switch, OTA |
| **T12** | CoreIOT RuleChain tự động hóa | Đạt | RuleChain điều khiển làm mát/đèn theo dải trễ Hysteresis chống chập chờn |
| **T13** | Cảnh báo ngưỡng an toàn (Threshold Notification) | Đạt | Cảnh báo nhiệt độ và độ ẩm vượt ngưỡng kèm trễ Hysteresis |
| **T14** | Cập nhật Firmware qua mạng (OTA) cho ESP2 | Đạt | Hỗ trợ Check FW trên màn hình LCD, RPC Dashboard, và kho OTA Packages |
| **N03** | Màn hình màu cảm ứng trên ESP2 (Bonus) | Đạt | ESP32-2432S028R hiển thị LCD ILI9341 + cảm ứng XPT2046 tương tác trực tiếp |
