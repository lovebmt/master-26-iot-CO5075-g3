# ESP3 — TDS Sensor Node (C/C++ PlatformIO)

Firmware C/C++ (Arduino framework) cho ESP32.

## 1. Tính năng
* **BLE NUS Peripheral:** Quảng bá tên `ESP3-TDS`, service UUID `6e400001-b5a3-f393-e0a9-e50e24dcca9e`.
* **TDS Simulator:** Bước nhảy 500–570 ppm mỗi 5s, gửi gói tin `{"tds_ppm": val}` qua Notify.
* **Màn hình TFT ILI9341:** Hiển thị trực tiếp TDS và trạng thái BLE.

## 2. Biên dịch & Nạp
```sh
pio run
pio run -t upload --upload-port /dev/cu.usbserial-130
pio device monitor -b 115200
```
