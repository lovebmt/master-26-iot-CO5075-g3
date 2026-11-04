---
title: Đề xuất kiến trúc và phân công bài tập lớn IoT Nhóm 3
tags:
  - iot
  - bai-tap-lon
status: cho-vu-duyet
---

# Tổng quan và phân công bài tập lớn IoT Nhóm 3

Hệ thống theo dõi nồng độ CO₂ theo thời gian tại các khu vực trong nhà nấm. Theo phương án của nhóm, mô hình hồi quy tuyến tính (linear regression) ước tính thời điểm nồng độ CO₂ có thể giảm xuống dưới ngưỡng cấu hình. Kết quả dự báo dùng để phát cảnh báo sớm. Nhóm sử dụng ThingsBoard tự triển khai thay CoreIOT để truy cập API và lưu căn cứ xác nhận phương án thay thế cùng hồ sơ bài tập.

Một cảm biến CO₂ hiện chỉ đủ để kiểm chứng một khu vực tại một thời điểm. Bản tin cần có mã khu vực `zone_id` để giữ riêng lịch sử từng khu. Muốn giám sát đồng thời nhiều khu vực cần bố trí thêm điểm đo. Xu hướng giảm và ngưỡng CO₂ cần được xác nhận bằng dữ liệu và yêu cầu vận hành thực tế. Vai trò của TDS và đèn trong nhà nấm còn chờ nhóm chốt. Trong phạm vi tích hợp ban đầu, TDS là luồng đo qua Modbus/BLE và đèn là tải minh họa cho lệnh điều khiển.

Cảnh báo là đầu ra của bản đầu. Phương án bơm oxy là ý tưởng mở rộng, cần xác nhận yêu cầu vận hành, thiết bị chấp hành và điều kiện an toàn trước khi đưa vào phạm vi triển khai.

## Thành phần hệ thống

| Thành phần | Vai trò |
|---|---|
| ESP1 — Vũ | YoloUNO chạy Tasmota, đọc CO₂ qua I²C và gửi MQTT. |
| ESP3 — Tâm | YoloUNO đọc TDS qua Modbus RS485 và gửi bản tin qua BLE NUS. |
| ESP2 — Phương | YoloUNO nhận TDS qua BLE, chuyển tiếp MQTT, điều khiển đèn và phục vụ web tại chỗ. LVGL và OTA triển khai theo tiến độ. |
| PC | Dịch vụ môi giới MQTT (broker) chuyển bản tin. Ứng dụng Gateway nhận và chuẩn hóa dữ liệu, cung cấp giao diện tại chỗ, kết nối ThingsBoard hai chiều. |
| ThingsBoard tự triển khai | Máy chủ nhóm dùng cho mẫu thiết bị, bảng điều khiển, chuỗi quy tắc (RuleChain), thông báo và minh chứng N07. |

```text
CO₂ ── I²C ── ESP1 ── MQTT ───────────────────┐
                                               ▼
TDS ── RS485 ── ESP3 ── BLE ── ESP2 ── MQTT ── Broker ↔ Gateway ↔ ThingsBoard
                                  ├─ Web T07             └─ Web T10
                                  └─ Đèn
```

Lệnh từ ThingsBoard đi ngược qua Gateway và MQTT đến ESP2. Web T07 gọi bộ xử lý đèn ngay trên ESP2. Web T10 gọi Gateway. Cả ba giao diện dùng trạng thái ESP2 công bố.

## Phân công và bàn giao

| Người | Phần chính | Bàn giao |
|---|---|---|
| **Vũ** | ESP1, biên tập PDF và ghép video demo | Cấu hình Tasmota, bản tin MQTT thật, minh chứng T01, PDF và video hoàn chỉnh. |
| **Phương** | ESP2 và ThingsBoard. Phụ trách BLE/MQTT, đèn, web T07, mẫu thiết bị, bảng điều khiển, RuleChain và thông báo. LVGL và OTA theo tiến độ. | Firmware, mẫu lệnh và phản hồi, cấu hình ThingsBoard có thể nhập lại và minh chứng T07/T11–T13/N07. |
| **Tâm** | ESP3, chương trình Gateway trên PC, giao diện T10 và dự báo CO₂ trên Gateway | Firmware, log đọc Modbus, bản tin BLE mẫu, hướng dẫn chạy Gateway và dữ liệu thử dự báo. |

Phương và Tâm cùng kiểm thử BLE và kết nối ThingsBoard hai chiều. Vũ và Tâm đối chiếu dữ liệu ESP1 qua MQTT. Để giữ tải việc cho Tâm, T10 chỉ cần hiển thị số đo, trạng thái kết nối và nút điều khiển đèn. Mỗi người tự viết phần kỹ thuật, giao số liệu và quay đoạn demo phần mình. Vũ biên tập báo cáo theo mẫu, ghép video và gắn mốc video vào bảng tự đánh giá. Tỷ lệ đóng góp chốt theo công việc thực tế.

## Đầu ra và điều kiện nghiệm thu

| Người phụ trách | Đầu ra cần bàn giao | Điều kiện nghiệm thu |
|---|---|---|
| Vũ | Cấu hình ESP1, mẫu MQTT thật, PDF và video | Gateway nhận CO₂ thật kèm mã khu vực. PDF dẫn tới video và các mốc minh chứng. |
| Phương | Firmware ESP2, hướng dẫn nạp, web T07, log lệnh và cấu hình ThingsBoard | ESP2 nhận đúng TDS qua BLE, thực hiện lệnh và báo trạng thái. ThingsBoard hiển thị số đo, điều khiển được đèn qua Gateway và gửi thông báo thử. |
| Tâm | Firmware ESP3, ứng dụng Gateway, trang T10, chức năng dự báo và hướng dẫn chạy | Đọc TDS thật qua Modbus, gửi BLE đến ESP2. Gateway đưa dữ liệu lên ThingsBoard và chuyển lệnh về ESP2. Với dữ liệu thử có xu hướng giảm, Gateway công bố thời điểm dự kiến xuống ngưỡng. |
| Cả nhóm | Bộ số liệu thử, đoạn quay của từng phần và kịch bản demo tích hợp | Truy được đường đi của một số đo và một lệnh. Đo được độ trễ và ghi được kết quả khi mất rồi phục hồi BLE. Mỗi mục tự đánh giá có minh chứng cụ thể. |

Các đầu ra dự báo có thể làm với dữ liệu giả lập trước. Đánh giá độ chính xác cần dùng dữ liệu thật và so thời điểm dự báo với thời điểm đo thực tế xuống ngưỡng. Nếu chưa đủ dữ liệu thật, báo cáo ghi rõ mới kiểm thử chức năng.

## Quy ước giao tiếp đề xuất

| Đường giao tiếp | Quy ước |
|---|---|
| ESP1 → Gateway | Giữ bản tin MQTT gốc của Tasmota. Chốt chủ đề và trường dữ liệu bằng bản tin thật. Gateway ánh xạ CO₂ thành `co2_ppm`. |
| ESP3 → ESP2 | BLE NUS, JSON UTF-8 kết thúc bằng ký tự xuống dòng. ESP2 ghép đủ bản tin rồi mới phân tích. |
| ESP2 → Gateway | `g3/esp2/telemetry` chuyển tiếp TDS với `source_id: "esp3"` và giữ `seq`. ESP2 công bố trạng thái đèn tại `g3/esp2/status`. |
| Gateway ↔ ESP2 | Gửi lệnh tại `g3/esp2/command` và nhận kết quả tại `g3/esp2/ack`. |
| Gateway ↔ ThingsBoard | Dùng các khóa chuẩn hóa cho dữ liệu đo. Ánh xạ lệnh từ dashboard sang `set_light` và chuyển kết quả về đúng yêu cầu ban đầu. Phương và Tâm chốt giao thức, thông tin xác thực và cơ chế gọi lệnh trên máy chủ thực tế. |

Ví dụ bản tin BLE nếu đầu đo trả TDS theo ppm:

```json
{"v":1,"seq":125,"tds_ppm":720}
```

Ví dụ lệnh và phản hồi trên hai chủ đề MQTT tương ứng:

```json
{"request_id":"cmd-0042","method":"set_light","params":{"on":true}}
```

```json
{"request_id":"cmd-0042","status":"applied","light_on":true}
```

Lệnh đặt trạng thái đích và không bật lưu lệnh (MQTT retain). `applied` xác nhận firmware đã áp dụng đầu ra. Giao diện báo chưa xác nhận nếu hết thời gian chờ và báo dữ liệu cũ khi mất mẫu mới, thay vì gán giá trị 0. Nhóm chốt chu kỳ lấy mẫu, thời hạn chờ và mã lỗi sau khi chạy thử.

## Chuẩn dữ liệu tại Gateway

Tâm chuẩn hóa dữ liệu tại Gateway. Phương dùng cùng tên trường và đơn vị trên ThingsBoard. Vũ cung cấp bản tin Tasmota thật để đối chiếu phần ánh xạ.

| Trường | Quy ước |
|---|---|
| `source_id`, `zone_id` | Nguồn phát sinh dữ liệu và mã khu vực đo CO₂. Dữ liệu TDS vẫn mang nguồn `esp3` khi chuyển tiếp. |
| `ts_ms`, `time_source` | Thời gian Unix theo mili giây, ưu tiên thời điểm đo nếu đáng tin cậy. Nếu dùng thời điểm Gateway nhận, ghi `time_source: "gateway_received"`. |
| `co2_ppm`, `temperature_c`, `humidity_pct` | CO₂ theo ppm, nhiệt độ theo °C và độ ẩm tương đối theo %. |
| `tds_ppm` | TDS theo ppm sau khi xác nhận đơn vị và hệ số quy đổi từ đầu đo. |
| `simulated` | `true` với dữ liệu giả lập, `false` với dữ liệu đo thật. Tách hai nguồn khi đánh giá dự báo. |
| `light_on` | Giá trị đúng/sai phản ánh trạng thái đầu ra do ESP2 công bố. |
| `forecast_status`, `predicted_crossing_at_ms` | Trạng thái dự báo và thời điểm Unix theo mili giây. Chỉ có thời điểm khi dự báo hợp lệ. |

Ví dụ CO₂ giả lập đã chuẩn hóa:

```json
{"source_id":"esp1","zone_id":"zone_01","ts_ms":1793764800000,"time_source":"gateway_received","co2_ppm":700,"simulated":true}
```

## Dự báo CO₂ theo khu vực

Gateway lưu `zone_id`, thời điểm đo và `co2_ppm`. Mô hình khớp đường hồi quy trên các mẫu gần đây của từng khu vực. Khi nồng độ đang trên ngưỡng và độ dốc âm, Gateway tính thời điểm dự kiến xuống ngưỡng và gửi kết quả lên ThingsBoard. Nếu thiếu mẫu, dữ liệu đã cũ hoặc xu hướng không giảm, trả trạng thái không có dự báo. Nếu số đo thực tế đã dưới ngưỡng, xử lý bằng quy tắc cảnh báo theo số đo hiện tại.

Đầu ra tối thiểu gồm `zone_id`, `threshold_ppm`, `forecast_status` và `predicted_crossing_at_ms` khi có dự báo. Số mẫu tối thiểu, cửa sổ đo và khoảng thời gian báo trước sẽ được chốt khi có dữ liệu. Cảnh báo sớm từ dự báo và thông báo vượt ngưỡng từ số đo thật cần được phân biệt trên giao diện. Dự báo là chức năng ứng dụng. Việc tự đánh giá T09 cần đối chiếu tiêu chí riêng và minh chứng thực tế.

## Thang điểm, nghiệm thu và video

| Ưu tiên | Hạng mục | Minh chứng |
|---|---|---|
| Đường chính | T02 (1), T03 (1,5), T07 (1), T08 (1), T10 (1), T11 (1), T12 (1), T13 (0,5) | **8 điểm dự kiến** nếu hoàn thành đủ. Quay Modbus và BLE riêng, ba đường điều khiển đèn, dashboard và thông báo thực nhận. |
| Dự phòng | T01 (0,5) | Đấu nối I²C, cấu hình Tasmota và dữ liệu thật. |
| Bổ sung | T14 (2) | Chỉ làm khi đã chạy ổn đường chính. Cần quy trình gán gói, thiết bị cập nhật và phiên bản trước/sau. |
| Nâng cao | N07 hoặc N03 | Tối đa 1 điểm cộng. Minh chứng triển khai ThingsBoard trên máy local và nhận dữ liệu thật, hoặc màn hình LVGL hoạt động. |
| Báo cáo | Tối đa 2 điểm | PDF theo mẫu, bảng phân công, bảng tự đánh giá, ánh xạ tính năng Fxx với Txx/Nxx, đo đạc, hướng dẫn chạy và tối thiểu 5 tài liệu tham khảo. |

Điểm công nghệ áp trần 8. Điểm bài tập lớn tối đa 10. Chỉ tự đánh giá các mục đã thực hiện và có minh chứng.

Video dự kiến 5–10 phút. Quay lần lượt đọc cảm biến thật, giám sát và điều khiển, cảnh báo và minh chứng N07. Đo lặp lại theo các mốc bên dưới. Ghi số lần đo, điều kiện thử, trung bình, nhỏ nhất và lớn nhất cho các phép đo độ trễ. Thử thêm mất BLE rồi kết nối lại để kiểm tra trạng thái dữ liệu cũ và phục hồi.

## Chỉ số nghiệm thu

| Chỉ số | Cách đo |
|---|---|
| Độ trễ Gateway → ThingsBoard | Từ lúc Gateway nhận bản tin đến khi thấy đúng mẫu trên bảng điều khiển. Đối chiếu mã mẫu hoặc dấu thời gian. Phép đo này bắt đầu tại Gateway. |
| Độ trễ lệnh MQTT | Từ lúc Gateway gửi lệnh đến lúc nhận `applied` với cùng `request_id`. Ghi riêng số lần hết thời gian chờ. |
| Tỷ lệ nhận BLE | Số bản tin hợp lệ, không trùng mà ESP2 nhận chia cho số bản tin ESP3 gửi trong cùng lượt thử. Đối chiếu `seq`. |
| Sai số dự báo | Độ lệch tuyệt đối giữa thời điểm dự báo và thời điểm mẫu thật đầu tiên xác nhận CO₂ xuống dưới ngưỡng trong cùng khu vực. Ghi chu kỳ lấy mẫu và thời điểm phát dự báo. |

## Tài liệu đối chiếu

- [Thông báo bài tập lớn](ThongBao_BaiTapLon_IoT.docx)
- [Mẫu báo cáo bài tập lớn](BaoCaoMau_BaiTapLon_IoT.docx)
