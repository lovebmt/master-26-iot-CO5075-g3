# ESP2 — CYD Display, BLE Central, Light Control & Web Gateway (C/C++ PlatformIO)

Firmware C/C++ (Arduino framework) cho board **ESP32 Cheap Yellow Display (ESP32-2432S028R)**.

## 1. Tính năng
* **BLE NUS Central:** Quét và kết nối thiết bị `ESP3-TDS`, nhận dữ liệu TDS ppm.
* **Màn hình LCD ILI9341:** Delta-rendering không giật/nhấp nháy, hiển thị TDS, trạng thái BLE, nút điều khiển đèn, trạng thái Wi-Fi.
* **Cảm ứng điện trở XPT2046:** Chạm để bật/tắt đèn tại chỗ.
* **Đèn RGB onboard:** Active-Low (GPIO 4, 16, 17).
* **Wi-Fi & Fallback AP:** Tự động kết nối Wi-Fi hoặc mở AP `ESP2-Setup` (192.168.4.1).
* **MQTT:** Telemetry `iot/esp2/telemetry`, Command `iot/esp2/command`, Result `iot/esp2/result`.
* **Web Server & OTA (Port 8080):** `GET /`, `POST /light`, `POST /ota` nạp nhị phân.

## 2. Biên dịch & Nạp
```sh
pio run
pio run -t upload --upload-port /dev/cu.usbserial-120
pio device monitor -b 115200
```
