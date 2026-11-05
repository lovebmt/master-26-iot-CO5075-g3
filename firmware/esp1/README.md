# ESP1 đo nhiệt độ và độ ẩm

ESP1 dùng YoloUNO ESP32-S3, DHT20 và Tasmota 16.0.0. Trình đọc Berry lấy mẫu qua I²C mỗi 5 giây, kiểm tra CRC và thêm số đo vào bản tin cảm biến của Tasmota.

1. Cắm DHT20 vào I2C1. Trong Tasmota, đặt GPIO11 là I2C SDA1 và GPIO12 là I2C SCL1.
2. Tải [autoexec.be](autoexec.be) vào thư mục gốc của hệ thống tệp trên Tasmota rồi khởi động lại.
3. Trong Console, chạy `I2CScan` để kiểm tra địa chỉ `0x38`. Sau khi trình đọc lấy mẫu, chạy `Status 10` và kiểm tra hai số đo trong trường `DHT20`.
4. Điền địa chỉ, cổng và thông tin xác thực của broker nhóm. Đặt `Topic` là `g3-esp1` với `FullTopic` là `%prefix%/%topic%/`. Chốt chu kỳ gửi (`TelePeriod`) với người phụ trách Gateway.

Gateway đăng ký `tele/g3-esp1/SENSOR`. Dùng [bản tin MQTT thật](examples/mqtt-sensor-mau.json) để thử ánh xạ `DHT20.Temperature` sang `temperature_c` (°C) và `DHT20.Humidity` sang `humidity_pct` (%RH). Nếu Gateway gắn thời điểm nhận, dùng `ts_ms` theo Unix mili giây và `time_source: "gateway_received"`.

Đã kiểm chứng đọc cảm biến trên bo và nhận hai bản tin trên broker thử ở Mac với chu kỳ 10 giây. Nghiệm thu với broker nhóm và ThingsBoard thực hiện trong lượt tích hợp chung.
