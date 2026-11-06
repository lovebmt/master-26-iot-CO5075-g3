# ESP2 — Display, BLE Central, Light Control & Web Gateway (C/C++ PlatformIO)

Firmware C/C++ (Arduino framework) cho bo mạch **ESP32-2432S028R** (ESP32 tích hợp màn hình LCD cảm ứng).

## 1. Tính năng
* **BLE NUS Central:** Quét và kết nối thiết bị `ESP3-PT100`, nhận dữ liệu nhiệt độ đất PT100 Modbus (`soil_temp`).
* **Màn hình LCD ILI9341:** Dòng đầu hiển thị `ESP2 - Ver: xxx`, hiển thị `Soil: XX.X C (PT100)` từ BLE ESP3, trạng thái kết nối Wi-Fi, 2 nút cảm ứng điều khiển đèn và kiểm tra cập nhật.
* **Cảm ứng điện trở XPT2046:**
  * Chạm ô bên trái: Bật/tắt đèn LED tại chỗ.
  * Chạm ô bên phải: **`CHECK FW`** để chủ động kiểm tra phiên bản mới từ Gateway.
  * Hộp thoại OTA Prompt: Nút cảm ứng **`YES`** (xanh) để nạp OTA, **`NO`** (đỏ) để hủy.
* **Đèn RGB onboard:** Active-Low (GPIO 4, 16, 17).
* **Wi-Fi & Fallback AP:** Tự động kết nối Wi-Fi hoặc mở AP `ESP2-Setup` (192.168.4.1).
* **MQTT:** Telemetry `iot/esp2/telemetry` (kèm `soil_temp`, `light_on`, `fw_version`), Command `iot/esp2/command`, Result `iot/esp2/result`.
* **Web Server & OTA (Port 8080):**
  * `GET /`: Giao diện giám sát, nút bật/tắt đèn, nút *Check for New FW*, form upload binary nạp OTA tại chỗ.
  * `POST /light`: Điều khiển đèn.
  * `POST /check_fw`: Kích hoạt kiểm tra firmware mới từ Gateway.
  * `POST /ota`: Nhận luồng nhị phân nạp trực tiếp qua HTTP.

## 2. OTA Firmware Delivery Architecture
1. **Kho lưu trữ tập trung CoreIOT**: File `.bin` (1.26MB) được lưu trữ tại CoreIOT OTA Packages repository (`ESP2 Hub Firmware 1.0.3`, Profile `esp_hub`).
2. **Download URL trên CoreIOT**: `https://fuvi-server.tailc75d93.ts.net/api/otaPackage/{id}/download`.
3. **Gateway Caching & Local Delivery**: Gateway kết nối CoreIOT kéo firmware về bộ đệm `gateway/firmware/esp2.bin`, sau đó cấp phát qua HTTP nội bộ `http://<IP_GATEWAY>:8080/firmware/esp2.bin` cho ESP2 tải trong mạng LAN (không dùng external URL).

## 3. Biên dịch & Nạp
```sh
pio run
pio run -t upload --upload-port /dev/cu.usbserial-140
pio device monitor -b 115200
```
