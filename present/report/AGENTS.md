# Hướng dẫn cho AI hỗ trợ báo cáo IoT Nhóm 3

Tâm phụ trách hoàn thiện báo cáo chung, bao gồm nội dung kỹ thuật do Phương phụ trách. Bảo toàn phần Vũ đã viết và bổ sung phần còn thiếu vào đúng vị trí trong mẫu.

## Tệp làm việc và định dạng

- Chỉ chỉnh sửa bản báo cáo chính tại `present/report/source/BaoCao_BaiTapLon_IoT_Nhom3.docx`, tính từ gốc repo. Kiểm tra nhánh và trạng thái Git trước khi sửa để tránh ghi đè công việc của thành viên khác.
- Dùng `docs/BaoCaoMau_BaiTapLon_IoT.docx` làm mẫu định dạng và `docs/ThongBao_BaiTapLon_IoT.docx` để đối chiếu yêu cầu chấm. Hai tệp này dùng để tham chiếu.
- Sửa trực tiếp trên bản sao của báo cáo hiện có rồi lưu vào đúng đường dẫn bản chính sau khi kiểm tra. Giữ cấu trúc chương mục, kiểu đoạn và tiêu đề, phông chữ, lề, khổ giấy, bảng biểu, chú thích, đầu và chân trang, mục lục của mẫu. Dùng các kiểu định dạng có sẵn khi thêm nội dung.
- Chỉ thay chỗ giữ chỗ khi đã có nội dung hoặc hình tương ứng. Đoạn văn đã điền không thay thế cho ảnh còn thiếu. Khi chèn hình và bảng, kiểm tra số thứ tự, chú thích và các chỗ dẫn trong phần được phép sửa.

## Phần Vũ đã viết cần giữ nguyên

- Toàn bộ Chương 1 và Bảng 1.1. Giữ các mã F01–F03 khi viết tiếp.
- Các mục 2.1, 2.2, 2.3.4, 2.4, 2.5, gồm Hình 2.1, Hình 2.2 và Bảng 2.1. Giữ đoạn về ESP1 trong 2.3.1 và các ô đã điền trong Bảng 2.2.
- Đoạn đấu nối ESP1 và Hình 3.1 ở mục 3.1. Giữ toàn bộ mục 3.2.1 và Hình 3.4 về Console Tasmota.
- Các đoạn thử nghiệm ESP1 ở 4.1–4.3, các đoạn hiện có ở 5.1–5.3, hướng dẫn ESP1 ở 6.2–6.4, dòng tự đánh giá T01 và tài liệu tham khảo [1]–[6].
- Bảo toàn cả câu chữ, hình, chú thích, số liệu, mã và định dạng trong các phần trên. Với mục viết chung, thêm đoạn hoặc hàng mới bên cạnh nội dung hiện có. Nếu phát hiện mâu thuẫn cần sửa phần của Vũ hoặc số hình mà Vũ đã dẫn, nêu rõ vị trí và đề xuất để Vũ duyệt trước khi sửa.

## Phần cần hoàn thiện

| Khu vực | Việc cần làm |
|---|---|
| Chương 2 | Điền 2.3.2, 2.3.3 và 2.3.5 theo cách triển khai thực tế. Bổ sung thông tin cảm biến ESP3 và phần cứng ESP2/ESP3 trong các vị trí còn trống. |
| Chương 3 | Tại 3.1, thay hai chỗ giữ hình bằng ảnh thực tế và sơ đồ đấu nối ESP2 (Hình 3.2), ESP3 (Hình 3.3). Bổ sung firmware ESP2/ESP3 theo cấu trúc mẫu mà vẫn giữ mục ESP1. Viết phần web trên thiết bị, đường Modbus/BLE/MQTT, Gateway và ThingsBoard. Ghi trạng thái thực tế của cảnh báo, tính năng nâng cao và OTA. |
| Chương 4 và 5 | Bổ sung điều kiện thử, kết quả tích hợp và đánh giá toàn hệ thống cạnh các đoạn ESP1 có sẵn. Điền bảng đo đạc và vị trí biểu đồ khi có số liệu thực tế. |
| Chương 6, đầu báo cáo và phụ lục | Bổ sung cách chạy các thành phần còn lại, thông tin nộp bài, phân công thực tế, các dòng tự đánh giá có bằng chứng và mốc video đã xác nhận. Cập nhật mục lục bằng Word khi các mục đã ổn định. |

## Nguồn, số liệu và kiểm tra trước khi bàn giao

- Đọc `docs/overview.md`, mã trong `firmware/` và `gateway/`, cấu hình trong `thingsboard/` và minh chứng do thành viên bàn giao. Khi tài liệu thiết kế và hiện thực khác nhau, ghi lại điểm cần nhóm xác nhận. Dùng tên nền tảng, thiết bị điều khiển, ngưỡng và vị trí xử lý quy tắc theo hệ thống thực tế. Giữ tên các mục theo mẫu của thầy.
- Mô tả mỗi tính năng bằng luồng dữ liệu hoặc lệnh, cấu hình chính và kết quả kiểm chứng. Trích đoạn mã ngắn khi cần giải thích xử lý chính. Để mã đầy đủ trong repo.
- Phân biệt dữ liệu giả lập, phép thử từng bo và nghiệm thu toàn hệ thống. Hai bản tin ESP1 nhận ở broker thử trên Mac chứng minh đường MQTT trong phép thử riêng. Khoảng cách 10 giây là chu kỳ công bố. Chỉ ghi kết quả Gateway/ThingsBoard, độ trễ, cảnh báo, OTA và điểm tự đánh giá khi có minh chứng tương ứng.
- Gắn minh chứng với mã Txx/Nxx, số mục, hình hoặc log, đường dẫn mã và mốc video nếu đã có. Khi thiếu dữ kiện như mã linh kiện, cấu hình, số đo hoặc nguồn, giữ chỗ tương ứng và liệt kê mục cần Tâm hoặc Phương bổ sung. Giữ các đề mục trong mẫu và ghi rõ phạm vi thực hiện đối với mục không áp dụng.
- Giữ nguyên các trích dẫn đã có. Nếu cần thêm nguồn, kiểm tra thứ tự IEEE. Khi phải đánh lại số trích dẫn thuộc phần Vũ, trình Vũ duyệt. Che mật khẩu, mã truy cập và thông tin xác thực trong ảnh, log hoặc cấu hình đưa vào báo cáo.
- Trước khi lưu, đối chiếu nội dung, hình, chú thích và các ô đã điền của Vũ với bản trước khi sửa. Mở hoặc kết xuất bản chỉnh để kiểm tra trang bị ngắt, hình và chú thích tách trang, bảng tràn lề và chỗ giữ chỗ đã được thay. Khi bàn giao, ghi rõ các mục đã hoàn tất và những mục còn thiếu minh chứng.
