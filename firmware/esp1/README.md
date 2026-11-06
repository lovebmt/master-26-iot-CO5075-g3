# ESP1 đo nhiệt độ và độ ẩm

ESP1 dùng YoloUNO ESP32-S3, DHT20 và Tasmota 16.0.0. Trình đọc Berry lấy mẫu qua I²C mỗi 5 giây, kiểm tra CRC và thêm số đo vào bản tin cảm biến của Tasmota.

1. Cắm DHT20 vào I2C1. Trong Tasmota, đặt GPIO11 là I2C SDA1 và GPIO12 là I2C SCL1.
2. Tải [autoexec.be](autoexec.be) vào thư mục gốc của hệ thống tệp trên Tasmota rồi khởi động lại.
3. Trong Console, chạy `I2CScan` để kiểm tra địa chỉ `0x38`. Sau khi trình đọc lấy mẫu, chạy `Status 10` và kiểm tra hai số đo trong trường `DHT20`.
4. Điền địa chỉ, cổng và thông tin xác thực của broker nhóm. Đặt `Topic` là `g3-esp1` với `FullTopic` là `%prefix%/%topic%/`. Chốt chu kỳ gửi (`TelePeriod`) với người phụ trách Gateway.

Gateway đăng ký `tele/g3-esp1/SENSOR` và `iot/esp1/telemetry`. Dùng [bản tin MQTT thật](examples/mqtt-sensor-mau.json) để ánh xạ `temperature` (°C) và `humidity` (%RH) đẩy lên CoreIOT.

Đã kiểm chứng đọc cảm biến DHT20 trên bo và gửi bản tin đo đạc tới broker nhóm, kích hoạt đầy đủ chuỗi quy tắc RuleChain cảnh báo Hysteresis và điều khiển tự động máy lạnh Air.Con trên ESP2.
