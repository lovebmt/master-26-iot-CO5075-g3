# PC Gateway (Rust Monolith)

Monolith PC IoT Gateway phục vụ tiêu chí bài tập lớn **T08** (CoreIOT telemetry sync & RPC routing) và **T10** (Local Dashboard).

---

## 1. Kiến trúc tổng thể (Architecture)

* **MQTT Broker:** NanoMQ chạy độc lập dưới dạng tiến trình/container trên `127.0.0.1:1883`.
* **Monolith Gateway:** Phát triển bằng Rust (Tokio asynchronous runtime + Axum Web framework + SQLite Outbox Pattern):
  * Lắng nghe và chuẩn hóa telemetry từ các thiết bị hạ tầng (ESP1, ESP2, ESP3).
  * Lưu trữ hàng đợi bền vững SQLite (Offline Outbox Queue) chống mất mát dữ liệu khi mất kết nối mạng.
  * Đồng bộ hai chiều với máy chủ ThingsBoard CoreIOT (`fuvi-server.tailc75d93.ts.net:10000`) qua ThingsBoard MQTT Gateway API.
  * Phục vụ giao diện giám sát & điều khiển trực quan bằng MiniJinja HTML + HTMX tại `http://127.0.0.1:8080`.
  * Đóng vai trò **Firmware Edge Server** phục vụ cập nhật OTA trong mạng nội bộ LAN.

---

## 2. Giao thức MQTT (MQTT Contract)

### 2.1. Telemetry nội bộ (Local NanoMQ)
* **ESP1:** `iot/esp1/telemetry` $\rightarrow$ `{"ts": <epoch_ms>, "values": {"co2_ppm": 620}}` (hoặc `tele/g3-esp1/SENSOR`).
* **ESP2:** `iot/esp2/telemetry` $\rightarrow$ `{"values": {"soil_temp": 27.0, "light_on": false, "fw_version": "1.0.2"}}`.
  * Trường `soil_temp` (Nhiệt độ đất PT100 Modbus): Số thực hợp lệ trong dải $-50^\circ\text{C} \le \text{temp} \le 150^\circ\text{C}$.
  * Trường `light_on`: Kiểu Boolean (`true`/`false`).
  * Trường `fw_version`: Chuỗi phiên bản (ví dụ `"1.0.2"`).

### 2.2. Lệnh điều khiển nội bộ (Local NanoMQ Commands)
* Topic lệnh: `iot/{device_id}/command`
  * Bật/tắt đèn: `{"id": "<correlation_id>", "method": "set_light", "params": {"state": true}}`
  * Cập nhật OTA: `{"id": "<correlation_id>", "method": "ota_update", "params": {"url": "http://192.168.1.19:8080/firmware/esp2.bin", "version": "1.0.3", "force": false}}`
* Topic phản hồi: `iot/{device_id}/result` $\rightarrow$ `{"id": "<correlation_id>", "success": true}`

### 2.3. ThingsBoard (CoreIOT) Gateway API
* **Xác thực:** Kết nối cổng `10000` (TLS), username là Gateway Access Token (`TB_GATEWAY_TOKEN`), không dùng mật khẩu.
* **Đăng ký thiết bị (Connect):** Topic `v1/gateway/connect`
  * `{"device": "ESP1", "type": "esp_temp_humi"}`
  * `{"device": "ESP2", "type": "esp_hub"}`
  * `{"device": "ESP3", "type": "esp_ble"}`
* **Gửi dữ liệu (Telemetry):** Topic `v1/gateway/telemetry`
  ```json
  {"ESP2": [{"ts": 1730000000000, "values": {"soil_temp": 27.0, "light_on": false, "fw_version": "1.0.2"}}]}
  ```
* **Nhận lệnh điều khiển (RPC):** Subscribe `v1/gateway/rpc` nhận lệnh từ Dashboard ThingsBoard (`set_light`, `ota_update`) và chuyển tiếp xuống thiết bị tương ứng.

---

## 3. Kiến trúc Phân phối OTA Nội bộ (Local Cache Delivery)

Gateway tích hợp sẵn HTTP Server phục vụ file nhị phân firmware cho ESP2:
* `GET /firmware/version.json`: Trả về thông tin phiên bản mới nhất:
  ```json
  {"version": "1.0.3", "url": "http://192.168.1.19:8080/firmware/esp2.bin"}
  ```
* `GET /firmware/esp2.bin`: Trả về trực tiếp luồng nhị phân file firmware `esp2.bin`.
* **Cơ chế hoạt động:**
  1. File `.bin` được upload và lưu trữ tập trung tại kho **CoreIOT OTA Packages** (`ESP2 Hub Firmware v1.0.3`).
  2. Gateway kết nối vào URL CoreIOT (`https://fuvi-server.tailc75d93.ts.net/api/otaPackage/{id}/download`) để kéo file về lưu đệm (cache) tại `gateway/firmware/esp2.bin`.
  3. ESP2 chỉ cần tải file qua mạng Wi-Fi LAN từ Gateway (`http://<IP_GATEWAY>:8080/firmware/esp2.bin`), nạp cực nhanh (~3–5s) mà không cần dùng external URL từ bên thứ ba.
* **Thao tác trên Web Dashboard Gateway (`http://127.0.0.1:8080/devices`):**
  * Nút **Notify FW**: Phát lệnh OTA yêu cầu ESP2 hỏi ý kiến người dùng `[YES] [NO]` trên màn hình cảm ứng LCD.
  * Nút **Force OTA**: Phát lệnh OTA ép buộc ESP2 cập nhật và reboot ngay lập tức.

---

## 4. Hướng dẫn Khởi chạy & Kiểm thử

### 4.1. Khởi động Broker NanoMQ
```bash
cd gateway
docker compose up -d
```

### 4.2. Cấu hình biến môi trường
```bash
cp .env.example .env
# Chỉnh sửa file .env: Cập nhật TB_GATEWAY_TOKEN từ ThingsBoard
```

### 4.3. Chạy Gateway Daemon
```bash
cargo run
```
Truy cập Dashboard tại `http://127.0.0.1:8080`.

### 4.4. Kiểm thử Unit Test & Integration Test
```bash
cargo test
```
Tất cả 36 unit tests và integration tests đều được kiểm chứng thành công.
