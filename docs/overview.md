# Tổng quan bài tập lớn IoT Nhóm 3

Hệ thống giám sát nhiệt độ và độ ẩm không khí trong nhà nấm, bật/tắt quạt thông gió theo ngưỡng nhiệt độ và gửi cảnh báo khi độ ẩm ra ngoài dải cấu hình. Nhóm dùng ThingsBoard tự triển khai làm nền tảng giám sát và điều khiển. Loài nấm, giai đoạn trồng và ngưỡng vận hành được chốt sau khi đối chiếu tài liệu và điều kiện thử nghiệm.

## Kiến trúc và phân công

| Thành phần | Phụ trách | Đầu ra và điều kiện nghiệm thu |
|---|---|---|
| ESP1 | Vũ | DHT20 đo nhiệt độ và độ ẩm qua I²C, gửi bản tin MQTT thật. Bàn giao cấu hình, mẫu bản tin và minh chứng đọc cảm biến. T01 cần xác nhận đọc được trên Tasmota. |
| ESP2 | Phương | Điều khiển quạt và phục vụ web tại chỗ (T07). Bàn giao firmware, hướng dẫn nạp và log cho thấy lệnh được áp dụng, trạng thái được báo lại. |
| PC | Tâm | Dịch vụ môi giới MQTT chuyển bản tin. Gateway chuẩn hóa số đo, xử lý quy tắc bật/tắt tự động, chuyển lệnh đến ESP2 và đồng bộ hai chiều với ThingsBoard. Bàn giao giao diện tại chỗ (T10), hướng dẫn chạy và log tích hợp. |
| ThingsBoard | Phương | Bàn giao cấu hình mẫu thiết bị, bảng điều khiển, chuỗi quy tắc (RuleChain), thông báo và minh chứng tự triển khai (N07). Bảng điều khiển hiển thị số đo, điều khiển quạt qua Gateway và nhận trạng thái phản hồi. |
| Báo cáo và video | Vũ tổng hợp | Mỗi thành viên viết phần mình, giao số liệu và đoạn quay. Vũ biên tập PDF theo mẫu, ghép video và đưa mốc minh chứng vào bảng tự đánh giá. |

Luồng số đo đi từ DHT20 → ESP1 → dịch vụ môi giới MQTT → Gateway → ThingsBoard. Lệnh từ ThingsBoard đi qua Gateway và MQTT đến ESP2. Web T07 gọi bộ xử lý quạt ngay trên ESP2. Web T10 gọi Gateway. Các giao diện dùng trạng thái do ESP2 công bố.

ESP3 do Tâm phụ trách nếu nhóm chọn thêm một điểm đo **nhiệt độ và độ ẩm không khí** qua cảm biến Modbus RS485, rồi chuyển dữ liệu qua BLE đến ESP2. Cần chốt phần cứng và vai trò điểm đo trước khi đưa đường này vào phạm vi nghiệm thu.

## Quy trình cập nhật tài liệu

Soạn và cập nhật tài liệu trong thư mục `BaiNhom`. Vũ duyệt nội dung trước khi đồng bộ bản đã duyệt sang repo. Mọi lần sửa sau áp dụng cùng quy trình.

## Phân công viết báo cáo theo mẫu của giảng viên

Bản Word làm việc của nhóm nằm trong repo tại `present/report/source/BaoCao_BaiTapLon_IoT_Nhom3.docx` và được sao chép trực tiếp từ mẫu **BaoCaoMau_BaiTapLon_IoT.docx**. Thành viên trao đổi trước khi sửa để tránh ghi đè thay đổi của nhau. Cả nhóm viết trực tiếp vào các mục được giao. Vũ ghi người phụ trách bằng bình luận trong từng mục. Khi dán nội dung từ nguồn khác, dùng tùy chọn **Chỉ giữ văn bản** để áp dụng định dạng của mẫu tại vị trí dán. Giữ bố cục, thứ tự chương mục, kiểu tiêu đề, bảng và chú thích của mẫu. Khi mục đã có nội dung và minh chứng, người viết để bình luận **Sẵn sàng duyệt**. Lưu ảnh gốc, số liệu và log trong thư mục minh chứng chung.

| Mục trong mẫu | Người viết chính | Nội dung cần bàn giao |
|---|---|---|
| Trang đầu, thông tin nộp bài, bảng phân công, bảng tự đánh giá | Vũ tổng hợp | Thông tin nhóm, đường dẫn GitHub và video, tỷ lệ đóng góp thực tế, mã công nghệ và minh chứng. Phương và Tâm cung cấp minh chứng phần mình. |
| Chương 1. Giới thiệu | Vũ | Bài toán nhà nấm, các công trình liên quan, mục tiêu đo được, danh sách tính năng Fxx và phạm vi. |
| 2.1–2.2 và 2.4–2.6 | Vũ tổng hợp | Sơ đồ kiến trúc, phân rã chức năng, ánh xạ Fxx với Txx/Nxx, công nghệ mới nếu có và bảng phần cứng. Mỗi người xác nhận thành phần mình làm. |
| 2.3.1 Lớp cảm biến | Vũ | DHT20, đại lượng đo, đơn vị, chu kỳ và căn cứ chọn cảm biến. Tâm bổ sung điểm đo Modbus nếu được triển khai. |
| 2.3.2–2.3.3 Lớp giao tiếp và Gateway | Tâm | Luồng MQTT, cấu trúc số đo và lệnh, xử lý tại Gateway. Phương đối chiếu đầu nhận ESP2 và ThingsBoard. Bổ sung BLE nếu triển khai ESP3. |
| 2.3.4 Lớp trí tuệ nhân tạo và 3.5 Trí tuệ nhân tạo | Vũ | Giữ các mục theo mẫu và ghi rõ phạm vi hiện tại dùng điều khiển theo ngưỡng. |
| 2.3.5 Lớp ứng dụng và nền tảng | Phương | Bảng điều khiển, RuleChain, thông báo và phần thay thế CoreIOT bằng ThingsBoard đã được xác nhận. |
| 3.1 Phần cứng và sơ đồ kết nối | Vũ tổng hợp | Mỗi người giao sơ đồ nối dây, mã thiết bị và ảnh phần cứng thật của mình. |
| 3.2 Firmware trên thiết bị | Vũ, Phương và Tâm theo thiết bị | Vũ viết ESP1, Phương viết ESP2 và web T07. Tâm viết ESP3 nếu có. Vũ ghép vào các mục tương ứng của mẫu. |
| 3.3–3.4 Truyền thông và Gateway | Tâm | Luồng bản tin, xử lý lệnh tự động, giao diện T10, xác nhận lệnh và log tích hợp. Phương cung cấp log từ ESP2. |
| 3.6–3.7 Nền tảng và tính năng nâng cao | Phương | Cấu hình ThingsBoard, kết quả thông báo, minh chứng N07, LVGL hoặc OTA nếu thực hiện. Giữ các mục theo mẫu và ghi trạng thái thực tế. |
| Chương 4. Đo đạc và thử nghiệm | Tâm lập bảng số liệu, Vũ biên tập | Vũ giao số đo cảm biến. Phương giao log trạng thái quạt, dashboard và cảnh báo. Tâm tổng hợp độ trễ, thử đúng ngưỡng và đánh giá từ dữ liệu đã đo. |
| Chương 5. Kết luận và hướng mở rộng | Vũ | Tổng hợp kết quả, hạn chế và hướng phát triển dựa trên minh chứng cả nhóm. |
| Chương 6. Hướng dẫn sử dụng mã nguồn | Tâm tổng hợp | Vũ cung cấp cấu hình ESP1. Phương cung cấp hướng dẫn nạp ESP2 và nhập cấu hình ThingsBoard. Tâm hướng dẫn chạy Gateway và ghép thành quy trình tái lập. |
| Tài liệu tham khảo và phụ lục | Vũ tổng hợp | Dẫn theo IEEE, đối chiếu ít nhất năm nguồn được trích trong nội dung, lưu số liệu chi tiết và khai báo phạm vi sử dụng AI theo yêu cầu. |

Trước khi nộp, Vũ đối chiếu bản Word với mẫu gốc theo từng chương, cập nhật mục lục và kiểm tra bảng, hình, chú thích, số trang và liên kết sau khi xuất PDF. Giữ bố cục và định dạng của mẫu khi nội dung làm thay đổi số trang. Tên PDF theo quy cách `[Mã lớp]_[Mã nhóm]_BaoCaoIoT.pdf`.

## Danh sách bàn giao theo thành viên

- **Vũ:** Cấu hình ESP1, sơ đồ đấu nối DHT20, bản tin MQTT thật và số liệu nhiệt độ–độ ẩm. Viết các mục được giao trong Word chung, quay ESP1, chốt PDF và ghép video.
- **Phương:** Firmware và hướng dẫn nạp ESP2, cấu hình ThingsBoard có thể nhập lại, ảnh hoặc log của web T07, trạng thái quạt, bảng điều khiển và thông báo thực nhận. Viết các mục ESP2 và ThingsBoard trong Word chung, quay phần điều khiển và cảnh báo.
- **Tâm:** Mã nguồn và hướng dẫn chạy Gateway, giao diện T10, bản tin mẫu, log lệnh và phản hồi, bảng số liệu thử nghiệm. Viết các mục giao tiếp, Gateway, thử nghiệm và hướng dẫn chạy trong Word chung, quay phần tích hợp. Nếu triển khai ESP3, bàn giao thêm firmware, sơ đồ đấu nối và log Modbus/BLE.

Mỗi người lưu tệp minh chứng và clip gốc trong thư mục chung, ghi mã Txx/Nxx và mục báo cáo tương ứng để Vũ đối chiếu khi tổng hợp.

## Quy tắc điều khiển

- Ở chế độ tự động, Gateway bật quạt khi nhiệt độ đạt ngưỡng bật và tắt khi nhiệt độ giảm xuống ngưỡng tắt. Ngưỡng tắt thấp hơn ngưỡng bật để tránh đóng ngắt liên tục.
- Độ ẩm ra ngoài dải cấu hình kích hoạt cảnh báo trên ThingsBoard. Ở chế độ thủ công, người dùng đặt trạng thái quạt từ giao diện. ESP2 công bố chế độ và trạng thái hiện tại để Gateway chỉ gửi lệnh tự động khi chế độ tự động đang bật.
- Có thể thử chức năng với ngưỡng bật 27 °C và tắt 25 °C. Đây là giá trị minh họa cho nguyên mẫu. Dải tham khảo cho nuôi trồng được trình bày riêng theo loài và giai đoạn.

## Chuẩn dữ liệu và điều khiển

| Dữ liệu | Quy ước |
|---|---|
| Bản tin ESP1 | Giữ bản tin gốc từ thiết bị. Vũ và Tâm dùng một bản tin thật để chốt chủ đề MQTT và ánh xạ trường. |
| Số đo chuẩn hóa | `source_id`, `zone_id`, `temperature_c` theo °C, `humidity_pct` theo %RH, `ts_ms` theo thời gian Unix mili giây, `time_source` và `simulated`. Nếu Gateway gắn thời điểm nhận, dùng `time_source: "gateway_received"`. |
| Lệnh quạt | Gateway gửi đến `g3/esp2/command`, ESP2 trả kết quả tại `g3/esp2/ack` với cùng `request_id`. Lệnh đặt trạng thái đích và tắt lưu lệnh (MQTT retain). |
| Trạng thái | ESP2 công bố `fan_on` và `mode` tại `g3/esp2/status`. Khi hết hạn phản hồi, giao diện báo chưa xác nhận. Khi mất mẫu đo, đánh dấu dữ liệu cũ và tạm dừng phát lệnh tự động dựa trên mẫu đó. |

```json
{"source_id":"esp1","zone_id":"zone_01","temperature_c":27.1,"humidity_pct":88.0,"ts_ms":1793764800000,"time_source":"gateway_received","simulated":true}
```

```json
{"request_id":"cmd-0042","method":"set_fan","params":{"on":true}}
```

```json
{"request_id":"cmd-0042","status":"applied","fan_on":true}
```

`applied` xác nhận firmware đã áp dụng trạng thái đầu ra. Video nghiệm thu cho thấy quạt thực sự đổi trạng thái. Phương và Tâm chốt khóa ThingsBoard, giao thức kết nối và cách ánh xạ lệnh vào cùng cấu trúc trên.

## Nghiệm thu và thang điểm

| Chỉ số | Cách đo |
|---|---|
| Đúng ngưỡng | Thử tăng qua ngưỡng bật, giữ giữa hai ngưỡng, giảm qua ngưỡng tắt. Ghi số đo và trạng thái quạt theo từng bước. |
| Độ trễ dữ liệu | Từ lúc Gateway nhận bản tin đến lúc mẫu tương ứng hiển thị trên ThingsBoard. |
| Độ trễ điều khiển | Từ lúc Gateway gửi lệnh MQTT đến lúc nhận `applied` với cùng `request_id`. Ghi riêng các lần hết hạn. |
| Phản ứng môi trường | Ghi nhiệt độ và độ ẩm trước và sau khi quạt chạy, kèm điều kiện thử. |
| BLE nếu triển khai ESP3 | So số bản tin hợp lệ, không trùng tại ESP2 với số bản tin ESP3 gửi trong cùng lượt thử. |

Các phép đo độ trễ cần lặp lại và báo cáo số lần thử, trung bình, nhỏ nhất và lớn nhất. Video 5–10 phút nên thể hiện số đo thật, một chu kỳ bật/tắt tự động, thao tác thủ công, thông báo thực nhận và minh chứng triển khai ThingsBoard.

Theo thang điểm trong thông báo, T07/T08/T10/T11/T12/T13 có tổng dự kiến **5,5 điểm** nếu hoàn thành và có minh chứng. T01 thêm 0,5 điểm khi đọc I²C trên Tasmota. T02/T03 thêm tối đa 2,5 điểm nếu hiện thực đường Modbus/BLE. T14 là phương án bổ sung nếu triển khai OTA có minh chứng. Công nghệ áp trần 8 điểm, báo cáo tối đa 2 điểm, N07 và N03 cùng thuộc phần điểm cộng tối đa 1 điểm. Tổng điểm bài tập lớn tối đa 10.

## Nguồn tham khảo cho báo cáo

| Bài và năm xuất bản | Tạp chí và xếp hạng | DOI hoặc nguồn | Dùng cho phần nào |
|---|---|---|---|
| Evaluation of Different Pasteurization and Sterilization Methods for Oyster Mushroom Substrates (2024) | Journal of Microbiology, Biotechnology and Food Sciences. [SJR 2025 Q3](https://journalsbase.com/journals/journal-of-microbiology-biotechnology-and-food-sciences). | [10.55251/jmbfs.10428](https://doi.org/10.55251/jmbfs.10428) | Bối cảnh nuôi trồng. Bảng 3 dẫn lại điều kiện theo giai đoạn từ Stamets. Nghiên cứu chính đánh giá xử lý giá thể. |
| Design and Evaluation of a Compact IoT-Enabled Microfarm for Decentralized Urban Agriculture Applied to the Cultivation of Pleurotus ostreatus (2025) | Sustainability. [SJR 2025 Q1 ở Geography, Planning and Development, Q2 ở Computer Networks and Communications](https://researchcloud.net/journal/2071-1050). | [10.3390/su172210332](https://doi.org/10.3390/su172210332) | Công trình liên quan, cách đo nhiệt độ và độ ẩm trong buồng trồng và phương pháp đánh giá. |
| Design and Implementation of a Cost-Aware and Smart Oyster Mushroom Cultivation System (2024) | Smart Agricultural Technology. [SJR 2025 Q1 ở Agricultural and Biological Sciences (miscellaneous)](https://www.resurchify.com/impact/details/21101111783). | [10.1016/j.atech.2024.100439](https://doi.org/10.1016/j.atech.2024.100439) | Công trình liên quan và so sánh phạm vi đo lường. |
| The Development of an Automatic Environment Control System in Greenhouses for Phoenix Oyster Mushrooms Using the Internet of Things (2026, số 8(1)) | Journal of Applied Informatics and Technology. [Có trong TCI](https://ph01.tci-thaijo.org/index.php/jait/journal-insight). [Danh sách của trường ghi Group 1](https://it.msu.ac.th/wp-content/uploads/2023/10/Collaborative-journals-AJCC-AUCC-2024-edited-Oct.5-1.pdf). Kỳ xếp hạng 2026 và quartile SJR chưa xác minh. | [10.14456/jait.2026.2](https://doi.org/10.14456/jait.2026.2) | Tham khảo thiết kế quạt và bộ phun ẩm riêng cho nhà nấm bào ngư Phoenix. |
| DHT20 Data Sheet (2021) | Tài liệu kỹ thuật ASAIR, không áp dụng xếp hạng tạp chí. | [PDF của hãng](https://www.aosong.com/userfiles/files/media/Data%20Sheet%20DHT20%20%20A1.pdf), không có DOI. | Giao tiếp I²C, điều kiện đo và sai số công bố của cảm biến. |

Các bản PDF đã tải được lưu trong [thư mục tài liệu tham khảo của nhóm trên Google Drive](https://drive.google.com/drive/folders/1QgDkBjaxtsZ0zJZVQ1gTQvduthe19O6d?usp=sharing).

Dẫn tối thiểu năm nguồn phù hợp trong nội dung theo kiểu IEEE. Chọn dải nhiệt–ẩm theo đúng loài và giai đoạn. Đánh giá nguyên mẫu bằng số liệu do nhóm đo. Có thể bổ sung [đặc tả MQTT 3.1.1 của OASIS](https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html) nếu đây là phiên bản nhóm triển khai.

- [Thông báo bài tập lớn](ThongBao_BaiTapLon_IoT.docx)
- [Mẫu báo cáo bài tập lớn](BaoCaoMau_BaiTapLon_IoT.docx)
