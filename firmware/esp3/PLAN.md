# Kế hoạch phát triển ESP3 (C/C++ PlatformIO)

Chuyển đổi từ `firmware/esp3_r` (Rust ESP-IDF) sang C/C++ Arduino framework trên PlatformIO.

## 1. Mục tiêu & Trách nhiệm
* Đóng vai trò **BLE NUS Peripheral** quảng bá tên `ESP3-PT100` (hoặc `ESP3-TDS`).
* Đo chỉ số nhiệt độ đất PT100 qua RS485 Modbus RTU.
* Gửi dữ liệu định kỳ tới ESP2 qua BLE Notify định dạng JSON: `{"soil_temp": <val>}`.
* Hiển thị chỉ số đo và trạng thái kết nối BLE lên màn hình TFT LCD ILI9341 (`Soil: XX.X C (PT100)`).

## 2. Phần cứng & Sơ đồ nối chân (ESP32-2432S028R / ESP32 Dev)
* **Màn hình ILI9341 (SPI):**
  * TFT_SCK: GPIO 14
  * TFT_MOSI: GPIO 13
  * TFT_MISO: GPIO 12
  * TFT_CS: GPIO 15
  * TFT_DC: GPIO 2
  * TFT_BL (Đèn nền): GPIO 21 (Mức HIGH = Sáng)
* **RS485 Modbus RTU (Cảm biến nhiệt độ đất PT100):**
  * UART2 (RX2: GPIO 16, TX2: GPIO 17, DE/RE: GPIO 4)

## 3. Thư viện sử dụng (PlatformIO)
* `h2zero/NimBLE-Arduino @ ^1.4.2`: Bluetooth Low Energy NUS stack nhẹ, tiết kiệm RAM.
* `bodmer/TFT_eSPI @ ^2.5.43`: Driver màn hình LCD ILI9341 tối ưu SPI.
* `bblanchon/ArduinoJson @ ^7.0.0`: Đóng gói payload JSON.
* `4-20ma/ModbusMaster @ ^2.0.1`: Đọc thanh ghi Modbus RTU qua RS485.

## 4. Cấu hình PlatformIO (`platformio.ini`)
```ini
[env:esp32dev]
platform = espressif32
board = esp32dev
framework = arduino
monitor_speed = 115200
upload_speed = 460800
lib_deps =
    h2zero/NimBLE-Arduino @ ^1.4.2
    bodmer/TFT_eSPI @ ^2.5.43
    bblanchon/ArduinoJson @ ^7.0.0
    4-20ma/ModbusMaster @ ^2.0.1
build_flags =
    -D USER_SETUP_LOADED=1
    -D ILI9341_2_DRIVER=1
    -D TFT_WIDTH=240
    -D TFT_HEIGHT=320
    -D TFT_MISO=12
    -D TFT_MOSI=13
    -D TFT_SCLK=14
    -D TFT_CS=15
    -D TFT_DC=2
    -D TFT_RST=-1
    -D TFT_BL=21
    -D TFT_BACKLIGHT_ON=HIGH
    -D LOAD_GLCD=1
    -D LOAD_FONT2=1
    -D LOAD_FONT4=1
    -D SPI_FREQUENCY=40000000
```

## 5. Kiến trúc Module mã nguồn (`src/`)
```text
firmware/esp3/
├── platformio.ini
├── PLAN.md
├── include/
│   └── config.h            # Định nghĩa UUID BLE, chu kỳ đọc cảm biến
└── src/
    ├── ble_nus.h / .cpp    # Quản lý NimBLE Server, Advertising, Callbacks
    ├── display.h / .cpp    # Khởi tạo TFT_eSPI, render màn hình (TDS, BLE status)
    ├── sensor.h / .cpp     # Module đọc cảm biến Nhiệt độ đất qua RS485 Modbus
    └── main.cpp            # Setup & Vòng lặp điều phối chính (2s/lần)
```

## 6. Chi tiết kỹ thuật BLE NUS
* **Device Name:** `ESP3-PT100` (hoặc `ESP3-TDS`)
* **Service UUID:** `6e400001-b5a3-f393-e0a9-e50e24dcca9e`
* **TX Characteristic (Notify):** `6e400003-b5a3-f393-e0a9-e50e24dcca9e`
* **RX Characteristic (Write):** `6e400002-b5a3-f393-e0a9-e50e24dcca9e`
* **Payload Notify:** `{"soil_temp": 27.0}`

## 7. Các bước thực hiện
1. Tạo `platformio.ini` theo cấu hình trên.
2. Cài đặt module hiển thị `display` kiểm tra màn hình và đèn nền.
3. Cài đặt module BLE NUS với callback kết nối/ngắt kết nối.
4. Tích hợp module đọc cảm biến nhiệt độ đất qua RS485 Modbus RTU.
5. Build test bằng `pio run`.
