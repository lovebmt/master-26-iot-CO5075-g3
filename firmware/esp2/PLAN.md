# Kế hoạch phát triển ESP2 (C/C++ PlatformIO)

Chuyển đổi từ `firmware/esp2_r` (Rust ESP-IDF) sang C/C++ Arduino framework trên PlatformIO cho bo mạch **ESP32-2432S028R** (tích hợp màn hình LCD cảm ứng).

## 1. Mục tiêu & Trách nhiệm
* **BLE NUS Central:** Quét, kết nối và nhận dữ liệu nhiệt độ đất PT100 từ `ESP3-PT100` (`soil_temp`).
* **Màn hình hiển thị & Cảm ứng:**
  * Hiển thị trạng thái BLE, giá trị `Soil: XX.X C (PT100)`, phiên bản `ESP2 - Ver: xxx`, nút điều khiển đèn, nút **`[CHECK FW]`**, trạng thái Wi-Fi theo cơ chế delta rendering (không chớp nháy).
  * Đọc cảm ứng điện trở XPT2046 để bật/tắt đèn tại chỗ và kiểm tra cập nhật firmware.
  * Hộp thoại OTA Prompt: Popup hỏi xác nhận `[YES] [NO]` trực tiếp trên màn hình cảm ứng khi có bản build mới.
* **Điều khiển LED:** Điều khiển LED RGB onboard (Active-Low) đồng bộ giữa Touch, MQTT và HTTP.
* **Kết nối Wi-Fi & MQTT:**
  * Kết nối Wi-Fi (hoặc phát AP cấu hình `ESP2-Setup` 192.168.4.1 nếu chưa có Wi-Fi).
  * Publish telemetry định kỳ lên `iot/esp2/telemetry` (`{"values":{"soil_temp":..., "light_on":..., "fw_version":...}}`).
  * Subscribe `iot/esp2/command` nhận lệnh `set_light` và `ota_update`, phản hồi kết quả lên `iot/esp2/result`.
* **Local Webserver & OTA (Port 8080):**
  * `GET /`: Giao diện Web hiển thị Soil Temp, trạng thái đèn, trạng thái BLE, nút bấm điều khiển đèn và kiểm tra firmware.
  * `POST /light`: HTTP API bật/tắt đèn.
  * `POST /check_fw`: Kích hoạt kiểm tra phiên bản mới từ Gateway.
  * `POST /ota`: Nhận firmware binary stream, ghi flash OTA và reboot.

## 2. Phần cứng & Sơ đồ nối chân (ESP32-2432S028R)
* **Màn hình TFT ILI9341 (SPI2):**
  * SCK: GPIO 14, MOSI: GPIO 13, MISO: GPIO 12, CS: GPIO 15, DC: GPIO 2, BL: GPIO 21 (HIGH = Sáng)
* **Cảm ứng Touch XPT2046 (SPI3 riêng biệt):**
  * SCK: GPIO 25, MOSI: GPIO 32, MISO: GPIO 39, CS: GPIO 33, IRQ: GPIO 36
* **RGB LED Onboard:**
  * Red: GPIO 4, Green: GPIO 16, Blue: GPIO 17 (Active-Low: LOW = Bật, HIGH = Tắt)

## 3. Thư viện sử dụng (PlatformIO)
* `bodmer/TFT_eSPI @ ^2.5.43`: Driver màn hình LCD ILI9341.
* `paulstoffregen/XPT2046_Touchscreen`: Đọc dữ liệu tọa độ/áp lực cảm ứng SPI3.
* `h2zero/NimBLE-Arduino @ ^1.4.2`: BLE Central scan & notify client.
* `knolleary/PubSubClient @ ^2.8`: MQTT client kết nối PC Gateway.
* `bblanchon/ArduinoJson @ ^7.0.0`: Parse lệnh RPC và đóng gói telemetry.
* Thư viện core ESP32: `WiFi.h`, `WebServer.h`, `Update.h`.

## 4. Phân vùng bộ nhớ OTA (`partitions.csv`)
Bộ nhớ Flash 4MB chia 2 phân vùng ứng dụng 1.5MB hỗ trợ nạp OTA:
```csv
# Name,   Type, SubType, Offset,  Size, Flags
nvs,      data, nvs,     0x9000,  0x6000,
otadata,  data, ota,     0xf000,  0x2000,
phy_init, data, phy,     0x11000, 0x1000,
ota_0,    app,  ota_0,   0x20000, 0x180000,
ota_1,    app,  ota_1,   0x1a0000,0x180000,
```

## 5. Cấu hình PlatformIO (`platformio.ini`)
```ini
[env:esp32dev]
platform = espressif32
board = esp32dev
framework = arduino
board_build.partitions = partitions.csv
monitor_speed = 115200
upload_speed = 460800
lib_deps =
    bodmer/TFT_eSPI @ ^2.5.43
    paulstoffregen/XPT2046_Touchscreen
    h2zero/NimBLE-Arduino @ ^1.4.2
    knolleary/PubSubClient @ ^2.8
    bblanchon/ArduinoJson @ ^7.0.0
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
    -D TOUCH_CS=33
```

## 6. Kiến trúc Module mã nguồn (`src/`)
```text
firmware/esp2/
├── platformio.ini
├── partitions.csv
├── PLAN.md
├── include/
│   ├── config.h            # Chân GPIO, Topic MQTT, UUID BLE, Thông số Wi-Fi mặc định
│   └── state.h             # Biến trạng thái toàn cục (TDS, light, wifi, ble)
└── src/
    ├── display_ui.h / .cpp # Xử lý TFT_eSPI, vẽ giao diện delta-rendering không giật
    ├── touch_mgr.h / .cpp  # Khởi tạo XPT2046 trên SPI riêng, debounce xử lý nút bấm
    ├── led_ctrl.h / .cpp   # Điều khiển 3 chân GPIO 4, 16, 17 theo mức Active-Low
    ├── ble_central.h / .cpp# Quét "ESP3-TDS", kết nối NUS và callback cập nhật TDS
    ├── net_mqtt.h / .cpp   # Quản lý Wi-Fi STA/AP fallback, PubSubClient MQTT pub/sub
    ├── web_ota.h / .cpp    # WebServer port 8080: root UI, POST /light, POST /ota
    └── main.cpp            # Setup phần cứng, task loop phối hợp các service
```

## 7. Giao thức & Định dạng dữ liệu
* **BLE NUS:**
  * Service UUID: `6e400001-b5a3-f393-e0a9-e50e24dcca9e`
  * TX Characteristic: `6e400003-b5a3-f393-e0a9-e50e24dcca9e`
  * Nhận: `{"soil_temp": <number>}`
* **MQTT Topics:**
  * Telemetry (1s/lần): `iot/esp2/telemetry` → `{"values":{"soil_temp":27.0,"light_on":false,"fw_version":"1.0.2"}}`
  * Command: `iot/esp2/command` → `{"id":"<correlation_id>","method":"set_light","params":{"state":true}}` hoặc `{"method":"ota_update","params":{"url":"...","version":"...","force":false}}`
  * Result: `iot/esp2/result` → `{"id":"<correlation_id>","success":true}`
* **HTTP Endpoints (Port 8080):**
  * `GET /`: Trang HTML quản lý (hiển thị Soil Temp, nút điều khiển đèn, kiểm tra firmware)
  * `POST /light`: Điều khiển đèn
  * `POST /check_fw`: Kích hoạt kiểm tra cập nhật từ Gateway
  * `POST /ota`: Nhận nhị phân firmware và nạp trực tiếp qua `Update` class

## 8. Các bước thực hiện
1. Tạo `platformio.ini` và `partitions.csv`.
2. Tạo module điều khiển LED onboard (GPIO 4, 16, 17) và Touch XPT2046 trên SPI3.
3. Cài đặt module hiển thị `display_ui` với TFT_eSPI, kiểm tra delta rendering.
4. Cài đặt NimBLE Central kết nối với ESP3.
5. Cài đặt Wi-Fi + MQTT giao tiếp với Gateway.
6. Cài đặt WebServer port 8080 và endpoint OTA nạp nhị phân.
7. Build test và verify toàn bộ tính năng bằng `pio run`.
