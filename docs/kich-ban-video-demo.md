# Kịch Bản Chi Tiết Video Demo Bài Tập Lớn IoT (Nhóm 3 - CO5075)

- **Đề tài:** Hệ thống giám sát và điều khiển tự động nhà nấm thông minh trên nền tảng CoreIOT và IoT Gateway
- **Thời lượng chuẩn:** 07 phút 30 giây (khung khuyến nghị 5–10 phút)
- **Độ phân giải khuyến nghị:** 1080p60 (Full HD), âm thanh rõ ràng hoặc lồng tiếng AI (TTS)
- **Quy tắc bắt buộc:** Mỗi phân cảnh phải có Text Overlay/Badge góc trên bên phải ghi rõ mã tiêu chí (ví dụ: [T02] Cảm biến Modbus RS485) để giảng viên đối chiếu chấm điểm theo Bảng tự đánh giá.

---

## 1. Bảng Khung Thời Gian & Ánh Xạ Tiêu Chí Rubric

| Timestamp | Phân cảnh | Tiêu chí Rubric | Điểm | Khung hình |
|---|---|---|---|---|
| **00:00 – 00:45** | 1. Giới thiệu tổng quan & kiến trúc | Toàn hệ thống | - | Cam toàn cảnh bàn demo + Slide kiến trúc |
| **00:45 – 01:45** | 2. Node ESP1: Cảm biến I²C DHT20 trên Tasmota | **T01** | 0.5 | Split: Web UI Tasmota + Console MQTT |
| **01:45 – 02:45** | 3. Node ESP3 & ESP2: Modbus RS485 PT100 & BLE NUS | **T02**, **T03**, **N03** | 3.5 | Cam cận cảnh đầu dò PT100 + Màn hình LCD CYD |
| **02:45 – 04:00** | 4. Webserver nhúng ESP2 & Rust PC Gateway | **T07**, **T08**, **T10** | 3.0 | Screen recording: Web :8080 ESP2 & Web Gateway |
| **04:00 – 05:45** | 5. CoreIOT Cloud & Tự động hóa Hysteresis | **T11**, **T12**, **T13**, **N07** | 3.5 | Split: Dashboard CoreIOT + Cam quay ESP2 |
| **05:45 – 07:00** | 6. Cập nhật Firmware OTA cho ESP2 | **T14** | 2.0 | Split: Widget OTA CoreIOT + LCD ESP2 |
| **07:00 – 07:30** | 7. Tổng kết & Đối chiếu bảng điểm | Báo cáo & Repo | - | Slide bảng tự đánh giá + Link GitHub |

> **Tổng điểm tự đánh giá:** Công nghệ: 8.0/8.0 (thực đạt 10.5) | Nâng cao: 1.0/1.0 (thực đạt 2.0).

---

## 2. Kịch Bản Chi Tiết Từng Phân Cảnh

### Phân cảnh 1: Giới thiệu tổng quan & Phần cứng (00:00 – 00:45)
- **Mục tiêu:** Giới thiệu bài toán nhà nấm, thành viên nhóm và layout thiết bị thực tế.
- **Khung hình:** Camera quay chậm toàn cảnh bàn thí nghiệm:
  1. Trạm ESP1 (YoloUNO ESP32-S3 + DHT20).
  2. Trạm ESP2 (Mạch CYD ESP32-2432S028R LCD 2.8" + Module Relay máy lạnh/LED).
  3. Trạm ESP3 (ESP32 DevKit V1 + Module XY-K485 + Đầu dò PT100).
  4. Máy tính PC Gateway đang chạy Terminal Rust Gateway.
- **Thuyết minh (Voiceover):**
  > "Xin chào thầy và các bạn, đây là video báo cáo và trình diễn thực nghiệm bài tập lớn môn Internet of Things của Nhóm 3. Đề tài của nhóm là: Hệ thống giám sát và điều khiển tự động nhà nấm thông minh trên nền tảng CoreIOT và IoT Gateway. Hệ thống được thiết kế theo kiến trúc phân tầng biên hoàn chỉnh gồm bốn thành phần chính: Nút cảm biến ESP1 thu thập vi khí hậu qua I2C chạy trên Tasmota; Nút ESP3 đo nhiệt độ giá thể nấm bằng cảm biến công nghiệp PT100 qua Modbus RS485 và truyền không dây qua Bluetooth Low Energy; Nút trung tâm ESP2 trang bị màn hình cảm ứng LCD CYD, phục vụ webserver điều khiển tại chỗ, kích hoạt rơ-le máy lạnh và hỗ trợ nạp firmware OTA; và Trạm PC IoT Gateway lập trình bằng Rust, tích hợp SQLite Outbox chống mất dữ liệu, đồng bộ hai chiều với máy chủ CoreIOT ThingsBoard tự triển khai qua mạng riêng ảo Tailscale Mesh VPN."

---

### Phân cảnh 2: Node ESP1 - Cảm biến I²C DHT20 trên Tasmota (00:45 – 01:45)
- **Tiêu chí:** [T01] (0.5 điểm)
- **Khung hình:** Split screen:
  - Bên trái: Giao diện Web của Tasmota trên ESP1 (http://<IP_ESP1>).
  - Bên phải: Cửa sổ Console Tasmota và Terminal theo dõi MQTT broker.
- **Thao tác thực tế:**
  1. Mở trang quản trị Web Tasmota, cho thấy thiết bị chạy Tasmota 16.0.0 trên ESP32-S3.
  2. Vào mục Console, gõ lệnh I2CScan -> Thấy bus I2C phát hiện thiết bị tại địa chỉ 0x38 (DHT20).
  3. Cho thấy tệp autoexec.be viết bằng ngôn ngữ Berry tự động nạp khi khởi động để đọc chu kỳ 5 giây.
  4. Gõ Status 10 hoặc quan sát log: hiển thị rõ trường DHT20: {Temperature: 28.5, Humidity: 88.2}.
  5. Mở Terminal nhận MQTT: cho thấy các bản tin JSON chuẩn liên tục được bắn lên topic tele/g3-esp1/SENSOR.
- **Thuyết minh:**
  > "Tại trạm ESP1, nhóm sử dụng bo YoloUNO ESP32-S3 nạp firmware Tasmota phiên bản 16.0.0. Cảm biến DHT20 được đấu nối trực tiếp vào bus I2C1 qua GPIO11 và 12. Khi gõ lệnh I2CScan trên Console, hệ thống quét bus và nhận diện chính xác cảm biến tại địa chỉ 0x38. Để tự động hóa đọc số đo, nhóm phát triển kịch bản Berry autoexec.be lưu trong Flash tự nạp khi boot: định kỳ 5 giây kích hoạt cảm biến, đọc thanh ghi, kiểm tra mã CRC8 và hợp nhất số đo vào đối tượng telemetry của Tasmota. Bản tin JSON định kỳ trên topic tele/g3-esp1/SENSOR liên tục được đẩy về broker nội bộ với đầy đủ thông số nhiệt ẩm thực tế và độ ổn định cao."

---

### Phân cảnh 3: ESP3 & ESP2 - Modbus RS485 PT100 & BLE NUS (01:45 – 02:45)
- **Tiêu chí:** [T02] (1.0đ), [T03] (1.5đ), [N03] (1.0đ)
- **Khung hình:**
  - Góc máy chính: Zoom cận cảnh màn hình màu LCD 2.8" của ESP2 (CYD) và đầu dò kim loại PT100 cắm vào cốc nước ấm/áp tay làm nóng.
  - PiP (góc dưới): Serial monitor của ESP3.
- **Thao tác thực tế:**
  1. **[T02]**: Áp tay hoặc nhúng đầu dò PT100 vào nước ấm. Serial log của ESP3 hiển thị frame Modbus RTU đọc qua UART2 tốc độ 9600 baud từ module XY-K485: giá trị nhiệt độ đất tăng dần từ 26.0°C lên 30.5°C.
  2. **[T03]**: ESP3 đóng gói JSON {"soil_temp": ...} và phát Notify qua kênh BLE NUS (Service UUID: 6e400001-b5a3-f393-e0a9-e50e24dcca9e).
  3. **[N03]**: Màn hình màu LCD CYD của ESP2 lập tức cập nhật giá trị nhiệt độ đất PT100 tương ứng theo thời gian thực mà không cần dây nối giữa ESP3 và ESP2.
  4. *(Tuỳ chọn điểm cộng)*: Tắt nguồn ESP3 -> sau 60 giây ESP2 báo trạng thái BLE Timeout, thể hiện cơ chế failover an toàn.
- **Thuyết minh:**
  > "Trạm ESP3 đảm nhận đo nhiệt độ giá thể bằng cảm biến công nghiệp PT100 nối module XY-K485, giao tiếp qua UART2 tốc độ 9600 baud. Thư viện ModbusMaster liên tục gửi frame đọc Input Register; khi tác động nhiệt lên đầu dò kim loại, số đo phản hồi tức thời từ 26 lên hơn 30 độ C. Để loại bỏ dây nối rườm rà trong nhà nấm ẩm ướt, ESP3 đóng vai trò BLE Server, đóng gói số đo JSON và phát Notify qua kênh Nordic UART Service với UUID 6e400001. Tại trạm ESP2, vi điều khiển hoạt động như BLE Central tự động ghép nối, bóc tách dữ liệu và vẽ trực tiếp số đo lên màn hình màu LCD CYD 2.8 inch với độ trễ dưới 100 mili giây. Khi mất kết nối BLE quá 60 giây, ESP3 tự động chuyển sang Wi-Fi MQTT dự phòng."

---

### Phân cảnh 4: Webserver nhúng trên ESP2 & Rust PC Gateway (02:45 – 04:00)
- **Tiêu chí:** [T07] (1.0đ), [T08] (1.0đ), [T10] (1.0đ)
- **Khung hình:** Quay màn hình PC trình duyệt web và cửa sổ terminal chạy Gateway.
- **Thao tác thực tế:**
  1. **[T07] Webserver nhúng trên ESP2:**
     - Truy cập trình duyệt vào IP của ESP2: http://<IP_ESP2>:8080/.
     - Cho thấy giao diện web hiển thị thông số FW, Soil Temp và trạng thái rơ-le.
     - Bấm nút 'Toggle Air.Con' trên Web -> Camera lồng góc quay thấy Relay đóng 'tạch', đèn LED trên ESP2 sáng lên và nút trên LCD chuyển sang Air.Con: ON.
  2. **[T08 & T10] Rust PC Gateway & Local Dashboard:**
     - Mở terminal PC: chạy cargo run --release của Rust Gateway. Log hiển thị tiến trình nhận telemetry đồng thời từ ESP1, ESP2, ESP3.
     - Mở trình duyệt tại http://127.0.0.1:8080/: Giao diện Web Dashboard cục bộ viết bằng Axum/Tera.
     - Thể hiện: bảng trạng thái MQTT, kết nối CoreIOT qua Tailscale, hàng đợi SQLite Outbox buffer và các thẻ đo thời gian thực của 3 node.
     - Rút dây mạng PC: tạo bản tin -> log Gateway cho thấy bản tin được lưu đệm an toàn vào SQLite; cắm mạng lại -> Gateway tự động flush đẩy sạch lên CoreIOT (chứng minh tính toàn vẹn T08).
- **Thuyết minh:**
  > "Trạm ESP2 tích hợp webserver nhúng tại cổng 8080 cho phép giám sát thông số phiên bản, nhiệt độ đất và điều khiển rơ-le tại chỗ trong mạng LAN. Thao tác nút Toggle Air.Con kích hoạt đóng rơ-le ngay lập tức, đèn LED GPIO 4 bật sáng và nút LCD đồng bộ sang ON. Tại biên, IoT Gateway được xây dựng bằng Rust với runtime Tokio bất đồng bộ, kết nối NanoMQ nội bộ với máy chủ CoreIOT. Dashboard cục bộ tại cổng 8080 sử dụng Axum và HTMX, hỗ trợ giám sát kết nối, biểu đồ lịch sử và quản lý hàng đợi. Nhờ kiến trúc SQLite Outbox Pattern, khi rút dây mạng ngắt kết nối Internet, mọi bản tin telemetry đều được lưu đệm an toàn và tự động đẩy bù toàn bộ lên ThingsBoard ngay khi có mạng trở lại."

---

### Phân cảnh 5: CoreIOT Cloud & Tự động hóa Hysteresis (04:00 – 05:45)
- **Tiêu chí:** [N07] (1.0đ), [T11] (1.0đ), [T12] (1.0đ), [T13] (0.5đ)
- **Khung hình:** Split screen:
  - Nửa màn hình: Trình duyệt truy cập CoreIOT Dashboard (https://fuvi-server.tailc75d93.ts.net).
  - Nửa màn hình: Camera quay cảm biến nhiệt độ và mạch ESP2.
- **Thao tác thực tế:**
  1. **[N07]**: Chỉ rõ thanh địa chỉ trình duyệt: tên miền máy chủ cá nhân fuvi-server.tailc75d93.ts.net kết nối qua mạng riêng ảo Tailscale Mesh VPN, chứng minh nền tảng CoreIOT ThingsBoard CE được tự triển khai 100%.
  2. **[T11]**: Mở trang Devices: thấy 3 profile esp_temp_humi, esp_hub, esp_ble. Mở Dashboard Group 3: đồng hồ Gauge, biểu đồ Timeseries và switch RPC.
  3. **[T12 & T13] Tự động hóa Hysteresis & Cảnh báo:**
     - Mở cấu hình Root Rule Chain (24 nodes): chỉ rõ node Hysteresis Alarm Switch chia nhánh logic 3°C và các node Create/Clear Alarm.
     - *Kích hoạt ngưỡng cao:* Thổi hơi nóng vào DHT20 cho nhiệt độ vượt 29.0°C:
       + Bảng Alarms trên CoreIOT lập tức xuất hiện cảnh báo đỏ CRITICAL: Air Temp High (T13).
       + RuleChain tự phát RPC set_light: true -> Gateway chuyển tiếp -> ESP2 kích hoạt rơ-le bật máy lạnh, màn hình LCD đổi sang Air.Con: ON.
     - *Dải trễ giữ nguyên:* Để nhiệt độ nguội dần từ 29°C xuống 27°C (nằm trong dải 27–29°C) -> Máy lạnh vẫn tiếp tục chạy, không bị ngắt chập chờn.
     - *Kích hoạt ngưỡng thấp:* Nhiệt độ hạ xuống < 27.0°C:
       + RuleChain tự phát RPC set_light: false ngắt máy lạnh.
       + Bảng Alarms tự động Clear giải phóng cảnh báo về trạng thái xanh an toàn.
- **Thuyết minh:**
  > "Nền tảng CoreIOT ThingsBoard Community Edition được nhóm tự triển khai trên máy chủ cá nhân, kết nối mã hóa qua mạng Tailscale Mesh VPN với tên miền fuvi-server.tailc75d93.ts.net qua cổng 10000 an toàn. Giao diện Dashboard Group 3 hiển thị trực quan thông số từ ba trạm thiết bị. Trọng tâm là Root Rule Chain 24 nodes tích hợp giải thuật ngưỡng trễ Hysteresis 3 độ C. Khi tác động nhiệt độ không khí vượt 29 độ, bảng Alarms lập tức phát sinh cảnh báo Critical Air Temp High, đồng thời phát lệnh RPC kích hoạt máy lạnh trên ESP2. Khi nhiệt độ hạ về dải an toàn từ 27 đến 29 độ, máy lạnh duy trì hoạt động ổn định, tránh rung rơ-le. Chỉ khi nhiệt độ hạ xuống dưới 27 độ C, RuleChain mới phát lệnh tắt máy lạnh và tự động giải phóng cảnh báo về trạng thái an toàn."

---

### Phân cảnh 6: Cập nhật Firmware OTA cho ESP2 (05:45 – 07:00)
- **Tiêu chí:** [T14] (2.0 điểm)
- **Khung hình:** Split screen:
  - Bên trái: CoreIOT Dashboard (Widget OTA Selector) hoặc Web Gateway.
  - Bên phải: Camera quay sát màn hình màu LCD của ESP2.
- **Thao tác thực tế:**
  1. Cho thấy màn hình ESP2 ban đầu hiển thị tiêu đề: ESP2 HUB - Ver: 1.0.3.
  2. Vào CoreIOT Dashboard -> mục OTA Version Selector: chọn phiên bản 1.0.4 và bấm nút 'Notify ESP2 (Prompt)'.
  3. CoreIOT gửi RPC ota_update xuống Gateway -> Gateway đẩy qua MQTT iot/esp2/command.
  4. Màn hình LCD ESP2 lập tức bật hộp thoại popup:
     FIRMWARE UPDATE: New version: 1.0.4 - Update now? [YES] [NO].
  5. Dùng ngón tay chạm nút [YES] trên màn hình cảm ứng:
     - Màn hình chuyển sang tiến trình nạp, thanh Progress Bar chạy 0% -> 100% trong vòng ~3.8 giây.
     - Thông báo Done! Rebooting... và thiết bị tự khởi động lại.
  6. Sau khi khởi động:
     - Màn hình LCD đổi tiêu đề sang: ESP2 HUB - Ver: 1.0.4.
     - Trên CoreIOT Dashboard, trường telemetry fw_version tự động cập nhật lên 1.0.4.
  7. Trình bày ngắn cấu trúc Dual Partition (app0 / app1 1.5MB) chống brick thiết bị.
- **Thuyết minh:**
  > "Tiêu chí T14 thể hiện tính năng cập nhật firmware OTA qua mạng với phân vùng Dual Partition an toàn cho vi điều khiển ESP2. Từ màn hình ban đầu hiển thị phiên bản 1.0.3, nhóm chọn phiên bản 1.0.4 trên CoreIOT Dashboard và nhấn Notify ESP2. Màn hình LCD bật popup cảm ứng hỏi xác nhận. Khi người vận hành chạm YES, ESP2 tải tệp nhị phân từ kho đệm Gateway nội bộ trong mạng LAN, thanh tiến trình nạp chạy từ 0 đến 100% trong 3.8 giây và tự động reboot. Sau khởi động, tiêu đề LCD đổi sang phiên bản 1.0.4 và trường telemetry fw_version trên CoreIOT đồng bộ ngay lập tức. Nhờ cơ chế hoán đổi giữa app0 và app1 cùng phân vùng otadata, thiết bị được bảo vệ tuyệt đối chống brick nếu xảy ra sự cố."

---

### Phân cảnh 7: Tổng kết & Đối chiếu bảng điểm (07:00 – 07:30)
- **Mục tiêu:** Tóm tắt kết quả, hiện bảng tự đánh giá để đối chiếu trực tiếp timestamp.
- **Khung hình:** Slide tổng kết kèm đường dẫn GitHub.
- **Nội dung hiển thị trên màn hình:**
  - Bảng tổng kết các mốc thời gian video khớp với Báo cáo:
    - 00:45 - T01: Cảm biến I²C Tasmota (0.5đ)
    - 01:45 - T02: Cảm biến Modbus RS485 (1.0đ)
    - 02:15 - T03: Giao tiếp BLE NUS (1.5đ)
    - 02:45 - T07: Webserver nhúng thiết bị (1.0đ)
    - 03:15 - T08: IoT Gateway PC Rust (1.0đ)
    - 03:30 - T10: Dashboard cục bộ Gateway (1.0đ)
    - 04:00 - T11: Template & Dashboard CoreIOT (1.0đ)
    - 04:30 - T12: RuleChain Hysteresis 24 nodes (1.0đ)
    - 04:30 - T13: Cảnh báo ngưỡng Alarms Table (0.5đ)
    - 05:45 - T14: Nạp Firmware OTA Dual Partition (2.0đ)
    - 01:45 - N03: Màn hình LCD CYD cảm ứng (1.0đ)
    - 04:00 - N07: Tự triển khai CoreIOT & VPN (1.0đ)
  - Tổng điểm tự đánh giá: **Công nghệ: 8.0/8.0 | Nâng cao: 1.0/1.0**.
  - Link GitHub: https://github.com/lovebmt/master-26-iot-CO5075-g3.
- **Thuyết minh:**
  > "Video thực nghiệm đã chứng minh hệ thống của Nhóm 3 hoạt động ổn định và đáp ứng trọn vẹn mọi yêu cầu. Nhóm đạt trần 8 trên 8 điểm công nghệ và 1 trên 1 điểm nâng cao. Toàn bộ mã nguồn firmware, trạm Gateway Rust, cấu hình RuleChain và tài liệu báo cáo kỹ thuật đã được công khai trên GitHub tại địa chỉ lovebmt/master-26-iot-CO5075-g3. Nhóm 3 xin chân thành cảm ơn thầy đã theo dõi phần trình bày video demo!"

---

## 3. Checklist Chuẩn Bị Trước Khi Quay

- [ ] Cắm nguồn cả 3 mạch ESP, kiểm tra kết nối Wi-Fi phòng lab/hotspot trùng với cấu hình (AAMM_ / 11111111).
- [ ] Chạy sẵn Rust Gateway trên PC: cd gateway && cargo run --release -> kiểm tra Web :8080 mở bình thường.
- [ ] Đăng nhập sẵn máy chủ CoreIOT https://fuvi-server.tailc75d93.ts.net trên trình duyệt qua Tailscale.
- [ ] Chuẩn bị một cốc nước ấm hoặc máy sấy tóc/bật lửa để tác động nhiệt lên đầu dò PT100 và cảm biến DHT20.
- [ ] Đảm bảo file gateway/firmware/esp2.bin và gói OTA v1.0.4 trên CoreIOT sẵn sàng để demo OTA.
- [ ] Sử dụng phần mềm OBS Studio để quay màn hình máy tính kết hợp webcam/điện thoại cắm qua Camo/DroidCam để quay góc mạch thật.

---

## 4. Lời Thoại Thuyết Minh Chi Tiết Từng Cảnh (Voiceover Script)

*Dùng để đọc trực tiếp khi quay hoặc dán vào công cụ chuyển văn bản thành giọng nói AI (CapCut / Clipchamp / Vbee) với tốc độ chuẩn 1.0x.*

---

### Phân cảnh 1: Giới thiệu tổng quan (00:00 – 00:45)
- **Badge góc màn hình:** `[TỔNG QUAN HỆ THỐNG]`
- **Hình ảnh hiển thị:** Camera lia toàn cảnh bàn thí nghiệm: trạm ESP1, trạm ESP2, trạm ESP3 và máy tính trạm Gateway.
- **Lời thoại đọc:**
> "Xin chào thầy và các bạn. Đây là video báo cáo và trình diễn thực nghiệm bài tập lớn môn Internet of Things của Nhóm 3. Đề tài của nhóm là: Hệ thống giám sát và điều khiển tự động nhà nấm thông minh trên nền tảng CoreIOT và IoT Gateway.
> 
> Hệ thống được thiết kế theo kiến trúc phân tầng biên hoàn chỉnh gồm bốn thành phần chính:
> Thứ nhất: Nút cảm biến ESP1 thu thập nhiệt độ và độ ẩm không khí qua chuẩn giao tiếp I2C chạy trên nền tảng firmware Tasmota mã nguồn mở.
> Thứ hai: Nút cảm biến ESP3 đo nhiệt độ giá thể nấm bằng cảm biến công nghiệp PT100 qua Modbus RS485 và truyền không dây qua Bluetooth Low Energy.
> Thứ ba: Nút trung tâm ESP2 trang bị màn hình cảm ứng LCD CYD, phục vụ webserver điều khiển tại chỗ, kích hoạt rơ-le máy lạnh và hỗ trợ nạp firmware OTA.
> Cuối cùng: Trạm PC IoT Gateway lập trình bằng Rust, tích hợp SQLite Outbox chống mất dữ liệu, đồng bộ hai chiều với máy chủ CoreIOT ThingsBoard tự triển khai qua mạng riêng ảo Tailscale Mesh VPN."

---

### Phân cảnh 2: Node ESP1 – Cảm biến I²C DHT20 trên Tasmota (00:45 – 01:45)
- **Badge góc màn hình:** `[T01] Cảm biến I2C trên Tasmota (0.5đ)`
- **Hình ảnh hiển thị:** Split screen: Web UI Tasmota ESP1 bên trái, cửa sổ Console Tasmota và Terminal nhận MQTT bên phải.
- **Lời thoại đọc:**
> "Tiếp theo là tiêu chí T01: Cảm biến I2C trên nền tảng Tasmota tại nút ESP1.
> 
> Tại trạm này, nhóm sử dụng bo mạch YoloUNO tích hợp vi điều khiển ESP32-S3, nạp bản dựng Tasmota 16.0.0. Cảm biến nhiệt ẩm số DHT20 được đấu nối trực tiếp vào bus I2C1 qua hai chân GPIO11 và GPIO12.
> Truy cập vào giao diện Web Tasmota Console, khi nhóm thực hiện lệnh I2CScan, hệ thống quét bus và nhận diện chính xác cảm biến DHT20 tại địa chỉ phần cứng không x ba tám (0x38).
> Để tự động hóa hoàn toàn mà không cần can thiệp thủ công, nhóm viết script Berry autoexec.be lưu trực tiếp trong bộ nhớ Flash của ESP32. Kịch bản này tự khởi động cùng hệ thống, thiết lập chu kỳ quét 5 giây một lần, kích hoạt cảm biến, đọc thanh ghi dữ liệu, tính toán mã kiểm tra CRC8 và hợp nhất số đo vào đối tượng telemetry gốc của Tasmota.
> Quan sát log thực tế qua lệnh Status 10 và cửa sổ Terminal MQTT bên cạnh, các gói tin JSON chuẩn mực liên tục được phát tán lên broker nội bộ qua topic tele/g3-esp1/SENSOR với độ ổn định cao và sai số dưới không phẩy năm phần trăm."

---

### Phân cảnh 3: ESP3 & ESP2 – Modbus RS485 & BLE NUS (01:45 – 02:45)
- **Badge góc màn hình:** `[T02] Modbus RS485 | [T03] BLE NUS | [N03] Màn hình LCD CYD (3.5đ)`
- **Hình ảnh hiển thị:** Camera quay cận cảnh đầu dò kim loại PT100 đang được làm nóng, góc nhỏ thể hiện Serial log ESP3 và màn hình màu LCD 2.8" của ESP2.
- **Lời thoại đọc:**
> "Bây giờ là chuỗi tiêu chí T02, T03 và tiêu chí nâng cao N03: Đo nhiệt độ đất qua Modbus RS485 và truyền dữ liệu không dây qua BLE đến màn hình cảm ứng LCD.
> 
> Tại trạm ESP3, môi trường giá thể nấm đòi hỏi độ bền cao nên nhóm trang bị đầu dò kim loại công nghiệp PT100 kết hợp module chuyển đổi XY-K485, giao tiếp với ESP3 qua cổng UART2 tốc độ 9600 baud. Như các bạn đang quan sát trên Serial log, thư viện ModbusMaster liên tục gửi frame truy vấn đọc thanh ghi Input Register. Khi chúng tôi áp tay làm nóng đầu dò kim loại, giá trị nhiệt độ đất đo được phản hồi tức thời và tăng đều từ 26 độ lên hơn 30 độ C.
> 
> Để loại bỏ hoàn toàn dây cáp vật lý phức tạp trong môi trường nhà nấm ẩm ướt, ESP3 được cấu hình làm BLE Server. Dữ liệu nhiệt độ đất được đóng gói JSON và phát broadcast qua kênh Nordic UART Service với Service UUID 6e400001.
> 
> Ở phía bên kia, nút trung tâm ESP2 hoạt động như một BLE Central, tự động quét nhận diện UUID và thiết lập kết nối mã hóa. Ngay khi nhận bản tin Notify, ESP2 bóc tách dữ liệu và vẽ trực tiếp số đo lên màn hình màu cảm ứng LCD CYD 2.8 inch theo thời gian thực với độ trễ dưới 100 mili giây. Nếu mất tín hiệu BLE quá 60 giây, ESP3 sẽ kích hoạt chế độ Wi-Fi MQTT dự phòng, đảm bảo luồng dữ liệu không bao giờ bị gián đoạn."

---

### Phân cảnh 4: Webserver nhúng ESP2 & Rust PC Gateway (02:45 – 04:00)
- **Badge góc màn hình:** `[T07] Webserver thiết bị | [T08] PC Gateway | [T10] Local Dashboard (3.0đ)`
- **Hình ảnh hiển thị:** Màn hình máy tính: duyệt web :8080 của ESP2, bấm nút toggle; sau đó chuyển sang Terminal chạy Rust Gateway và Web Dashboard 127.0.0.1:8080.
- **Lời thoại đọc:**
> "Tiếp theo, chúng ta cùng kiểm chứng tiêu chí T07: Webserver nhúng trên thiết bị, cùng tiêu chí T08 và T10 trên trạm PC Gateway.
> 
> Trước hết là Webserver nhúng trên ESP2. Truy cập trình duyệt đến cổng 8080 của vi điều khiển trong mạng LAN, trang web hiển thị trực quan thông số phiên bản firmware, nhiệt độ đất và trạng thái rơ-le máy lạnh. Khi thao tác nhấn nút Toggle Air.Con ngay trên trình duyệt, vi điều khiển xử lý ngắt và đóng tiếp điểm rơ-le phần cứng lập tức, đèn LED GPIO 4 bật sáng và nút cảm ứng trên màn hình LCD CYD tự động đảo trạng thái sang Air.Con ON.
> 
> Song song đó, trạm PC Gateway đóng vai trò hạt nhân biên được nhóm xây dựng hoàn toàn bằng ngôn ngữ Rust với runtime bất đồng bộ Tokio. Gateway đăng ký làm cầu nối chuyển tiếp giữa broker NanoMQ nội bộ và máy chủ đám mây CoreIOT.
> Giao diện Local Dashboard tại cổng 8080 của PC Gateway được thiết kế tinh gọn bằng Axum, MiniJinja và HTMX, hỗ trợ cơ chế tự cập nhật trạng thái kết nối, biểu đồ lịch sử và kiểm soát hàng đợi.
> Đặc biệt, nhằm đáp ứng độ tin cậy của tiêu chí T08, Gateway áp dụng mô hình SQLite Outbox Pattern. Khi chúng tôi chủ động rút dây mạng ngắt kết nối Internet, Gateway lập tức chuyển các bản tin telemetry mới vào bảng lưu đệm SQLite cục bộ. Ngay khi cắm lại dây mạng, luồng worker tự động kích hoạt đẩy bù toàn bộ hàng đợi lên ThingsBoard mà không suy hao hay xáo trộn thứ tự bất kỳ bản tin nào."

---

### Phân cảnh 5: CoreIOT Cloud & Tự động hóa Hysteresis (04:00 – 05:45)
- **Badge góc màn hình:** `[N07] Self-hosted CoreIOT | [T11] Dashboard | [T12] RuleChain | [T13] Cảnh báo (3.5đ)`
- **Hình ảnh hiển thị:** Split screen: Trình duyệt truy cập CoreIOT Dashboard (fuvi-server.tailc75d93.ts.net), camera quay cảm biến DHT20 và mạch ESP2.
- **Lời thoại đọc:**
> "Bước sang phần quan trọng nhất: Tiêu chí nâng cao N07 cùng các tiêu chí đám mây T11, T12 và T13 trên nền tảng CoreIOT ThingsBoard.
> 
> Về tiêu chí N07, nhóm tự triển khai toàn bộ máy chủ ThingsBoard Community Edition trên cụm máy chủ Minikube cá nhân, vận hành an toàn phía sau mạng riêng ảo Tailscale Mesh VPN với tên miền fuvi-server.tailc75d93.ts.net qua cổng 10000 an toàn. Giao diện Dashboard Group 3 hiển thị trực quan thông số từ ba trạm thiết bị.
> 
> Trái tim của hệ thống là Root Rule Chain gồm 24 nodes tích hợp giải thuật dải trễ Hysteresis 3 độ C nhằm bảo vệ máy lạnh khỏi hiện tượng đóng ngắt liên tục.
> Bây giờ, chúng tôi tác động nhiệt vào cảm biến DHT20:
> Khi nhiệt độ không khí vượt ngưỡng cài đặt 29 độ C: Trên giao diện Dashboard, bảng Alarms Table lập tức kích hoạt cảnh báo đỏ Critical Air Temp High. Cùng lúc đó, node Switch rẽ nhánh phát lệnh RPC set_light bật máy lạnh. Lệnh được Gateway định tuyến tức thì xuống ESP2, rơ-le đóng mạch và màn hình LCD chuyển sang Air.Con ON.
> Tiếp theo, khi nhiệt độ nguội dần và dao động trong dải trễ an toàn từ 27 đến 29 độ C: hệ thống vẫn duy trì máy lạnh hoạt động liên tục, triệt tiêu hoàn toàn hiện tượng rung lắc rơ-le khi nhiệt độ mấp mé quanh ngưỡng.
> Chỉ khi nhiệt độ hạ hẳn xuống dưới 27 độ C: RuleChain mới rẽ nhánh phát lệnh RPC tắt máy lạnh và tự động chuyển bảng Alarms sang trạng thái Clear màu xanh an toàn."

---

### Phân cảnh 6: Cập nhật Firmware OTA cho ESP2 (05:45 – 07:00)
- **Badge góc màn hình:** `[T14] Cập nhật Firmware OTA thiết bị (2.0đ)`
- **Hình ảnh hiển thị:** Màn hình CoreIOT Dashboard phần OTA Version Selector bên trái, camera zoom màn hình LCD ESP2 bên phải.
- **Lời thoại đọc:**
> "Tiếp theo là tiêu chí công nghệ nâng cao T14: Cập nhật Firmware qua mạng OTA với cơ chế xác thực kép và bộ nhớ Dual Partition an toàn trên ESP2.
> 
> Hiện tại trên góc màn hình LCD của ESP2 đang hiển thị phiên bản firmware 1.0.3. Nhóm đã đóng gói và tải bản dựng mới 1.0.4 lên kho lưu trữ CoreIOT OTA Packages.
> Từ widget OTA Selector trên Dashboard CoreIOT, người vận hành chọn phiên bản mục tiêu 1.0.4 và kích hoạt nút Notify ESP2.
> Máy chủ CoreIOT phát lệnh RPC xuống trạm Gateway, và Gateway gửi tiếp bản tin JSON chứa đường dẫn tải tệp nhị phân xuống ESP2 qua MQTT nội bộ.
> Ngay lập tức, màn hình cảm ứng LCD bật hộp thoại popup xác nhận: 'Phát hiện phiên bản mới 1.0.4, bạn có muốn cập nhật không?' kèm hai nút YES và NO.
> Khi người vận hành chạm tay vào nút YES trên màn hình: ESP2 lập tức kết nối đến Local Cache Server của Gateway qua mạng LAN để tải tệp nhị phân esp2.bin có dung lượng hơn 1 megabyte.
> Thanh Progress Bar trên màn hình LCD chuyển động mượt mà từ 0 đến 100% chỉ trong vỏn vẹn ba phẩy tám giây.
> Sau khi xác thực mã kiểm tra SHA256 hợp lệ, vi điều khiển đổi cờ khởi động trong phân vùng otadata và tự động khởi động lại.
> Sau khi reboot, màn hình LCD cập nhật tiêu đề mới sang ESP2 Hub phiên bản 1.0.4 và trường telemetry fw_version trên CoreIOT cũng tự động đồng bộ lên 1.0.4. Nhờ cơ chế hoán đổi giữa hai phân vùng đối xứng app0 và app1, thiết bị được bảo vệ tuyệt đối chống nguy cơ biến thành cục gạch nếu mất điện giữa chừng."

---

### Phân cảnh 7: Tổng kết & Đối chiếu bảng điểm (07:00 – 07:30)
- **Badge góc màn hình:** `[TỔNG KẾT ĐÁNH GIÁ & KHO MÃ NGUỒN]`
- **Hình ảnh hiển thị:** Slide tổng kết hiển thị bảng tự đánh giá tiêu chí và đường dẫn GitHub repo.
- **Lời thoại đọc:**
> "Tổng kết lại, video thực nghiệm đã chứng minh hệ thống của Nhóm 3 hoạt động ổn định và đáp ứng trọn vẹn toàn bộ các tiêu chí đề ra. Nhóm đạt trần 8 trên 8 điểm cho 10 tiêu chí công nghệ bắt buộc, cùng 1 trên 1 điểm trần cho 2 tiêu chí nâng cao là màn hình cảm ứng LCD và tự triển khai máy chủ CoreIOT qua VPN.
> 
> Toàn bộ sơ đồ nguyên lý mạch, mã nguồn firmware ESP, chương trình trạm biên Gateway Rust, tệp cấu hình RuleChain ThingsBoard và báo cáo kỹ thuật chi tiết đều được nhóm lưu trữ công khai trên kho lưu trữ GitHub tại địa chỉ lovebmt/master-26-iot-CO5075-g3.
> 
> Nhóm 3 xin chân thành cảm ơn thầy đã dành thời gian theo dõi toàn bộ phần trình bày video demo!"
