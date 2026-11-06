# ESP3 — Soil Temperature Node (PT100 + RS485 Modbus)

Firmware C/C++ (Arduino framework trên PlatformIO) cho bo mạch **ESP32 Dev Module**.

---

## 1. Tính năng & Trách nhiệm
* **Cảm biến nhiệt độ đất PT100 qua RS485 Modbus RTU:**
  * Đọc dải nhiệt độ đất (`soil_temp`) từ đầu dò PT100 qua RS485 Modbus RTU.
  * Cấu hình chân phần cứng chuẩn RS485: UART2 (RX2: GPIO 16, TX2: GPIO 17, DE/RE: GPIO 4).
* **BLE NUS Peripheral:**
  * Quảng bá tên thiết bị: **`ESP3-PT100`** (tương thích backward với tên cũ `ESP3-TDS`).
  * Service UUID: `6e400001-b5a3-f393-e0a9-e50e24dcca9e`.
  * TX Characteristic (Notify): `6e400003-b5a3-f393-e0a9-e50e24dcca9e`.
  * Định kỳ 2 giây gửi gói tin Notify JSON: `{"soil_temp": 27.0}`.
* **Màn hình LCD TFT ILI9341:**
  * Tiêu đề: `ESP3 / PT100 Modbus Node`.
  * Hiển thị chỉ số nhiệt độ đất hiện tại: `Soil: 27.0 C (PT100)`.
  * Hiển thị trạng thái kết nối BLE: `BLE: Connected` hoặc `BLE: Advertising...`.

---

## 2. Biên dịch & Nạp Firmware
Sử dụng PlatformIO:
```bash
# Biên dịch mã nguồn
pio run

# Nạp vào board qua cổng serial
pio run -t upload --upload-port /dev/cu.usbserial-130

# Mở serial monitor theo dõi log
pio device monitor -b 115200
```
