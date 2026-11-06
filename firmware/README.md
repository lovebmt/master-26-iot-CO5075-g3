# Firmware Vi Điều Khiển (ESP Nodes Firmware)

Thư mục này quản lý mã nguồn firmware cho ba node vi điều khiển trong hệ thống: **ESP1**, **ESP2**, và **ESP3**.

---

## 1. Phân chia nhiệm vụ các Node

| Node | Thư mục | Phần cứng | Framework / Nền tảng | Chức năng chính | Giao tiếp |
|:---|:---|:---|:---|:---|:---|
| **ESP1** | [`esp1/`](esp1/) | YoloUNO (ESP32-S3) | Tasmota v16.0.0 (Berry Script) | Đo nhiệt độ, độ ẩm môi trường qua cảm biến DHT20 I²C | Wi-Fi MQTT $\rightarrow$ Gateway |
| **ESP2** | [`esp2/`](esp2/) | ESP32 Cheap Yellow Display (**ESP32-2432S028R**) | C/C++ PlatformIO (Arduino) | Hub hiển thị LCD ILI9341, cảm ứng XPT2046, điều khiển LED RGB, Web Server `:8080`, Cập nhật OTA | BLE Central (nhận ESP3) + Wi-Fi MQTT $\leftrightarrow$ Gateway |
| **ESP3** | [`esp3/`](esp3/) | ESP32 Dev Module | C/C++ PlatformIO (Arduino) | Đọc cảm biến nhiệt độ đất PT100 qua RS485 Modbus RTU (`soil_temp`) | BLE NUS Peripheral $\rightarrow$ ESP2 |

---

## 2. Hướng dẫn Biên dịch và Nạp Firmware

### 2.1. Chuẩn bị môi trường
* Đã cài đặt **PlatformIO Core (CLI)** hoặc Extension PlatformIO trên VSCode/Antigravity.
* Cổng nạp Serial mặc định trên máy Mac phát triển:
  * `/dev/cu.usbserial-130` $\rightarrow$ **ESP3**
  * `/dev/cu.usbserial-140` $\rightarrow$ **ESP2**

### 2.2. ESP3 — PT100 Modbus Node
```bash
# Biên dịch
pio run -d firmware/esp3

# Nạp qua cổng serial
pio run -d firmware/esp3 -t upload --upload-port /dev/cu.usbserial-130

# Mở serial monitor (115200 baud)
pio device monitor -d firmware/esp3 -b 115200
```

### 2.3. ESP2 — Central Hub & OTA Node
```bash
# Biên dịch
pio run -d firmware/esp2

# Nạp qua cổng serial
pio run -d firmware/esp2 -t upload --upload-port /dev/cu.usbserial-140

# Mở serial monitor (115200 baud)
pio device monitor -d firmware/esp2 -b 115200
```

### 2.4. ESP1 — Tasmota Node
* Nạp firmware Tasmota chuẩn cho ESP32-S3.
* Cấu hình chân I²C trong Tasmota Console: GPIO 11 (SDA), GPIO 12 (SCL).
* Tải file [`firmware/esp1/autoexec.be`](esp1/autoexec.be) vào bộ nhớ flash của Tasmota.
* Cấu hình MQTT broker trỏ về IP của PC Gateway (cổng 1883).

---

## 3. Kiến trúc Cập nhật OTA trên ESP2
* **Phân vùng bộ nhớ (`firmware/esp2/partitions.csv`)**: Phân bổ 2 phân vùng ứng dụng `ota_0` và `ota_1` (mỗi phân vùng 1.5MB) trên bộ nhớ flash 4MB.
* **Đa kênh kích hoạt**:
  1. Nút cảm ứng **`[CHECK FW]`** trên màn hình LCD $\rightarrow$ Hiện popup hỏi `[YES] [NO]`.
  2. Nút bấm trên giao diện Web cục bộ của ESP2: `http://<IP_ESP2>:8080/`.
  3. Lệnh RPC từ xa qua CoreIOT Dashboard: `Notify New FW` (hỏi trước) hoặc `Force OTA Update` (nạp ngay).
* **Nguồn tải an toàn**: ESP2 tải trực tiếp file nhị phân từ Gateway trong mạng LAN (`http://<IP_GATEWAY>:8080/firmware/esp2.bin`), file gốc được Gateway đồng bộ từ kho **CoreIOT OTA Packages**.

