# Tài liệu dự án (Documentation)

Thư mục này chứa toàn bộ tài liệu kiến trúc kỹ thuật, đặc tả giao tiếp phần cứng, quy tắc vận hành và hướng dẫn triển khai của hệ thống IoT Nhóm 3.

---

## 1. Danh mục tài liệu

* **[`overview.md`](overview.md)**:
  - Tổng quan bài toán nhà nấm thông minh và kiến trúc tổng thể.
  - Phân công nhiệm vụ chi tiết và kế hoạch viết báo cáo theo mẫu.
  - Quy tắc điều khiển tự động và cảnh báo bằng RuleChain (ngưỡng trễ Hysteresis).
  - Chi tiết kiến trúc OTA Firmware khép kín: kho lưu trữ CoreIOT OTA Packages, cơ chế Gateway Cache, và cấu hình Nginx/Minikube Ingress.
* **[`hardware-architecture.md`](hardware-architecture.md)**:
  - Sơ đồ kiến trúc phần cứng các node: ESP1 (DHT20 I²C), ESP2 (ESP32-2432S028R ILI9341 + XPT2046 Touch + Light), ESP3 (PT100 RS485 Modbus + BLE).
  - Ma trận tiêu chí chấm điểm (Rubric T01–T14, N03).
  - Luồng dữ liệu và điều khiển giữa các phân hệ.
  - Thông tin kết nối máy chủ phát triển CoreIOT (`fuvi-server.tailc75d93.ts.net`).

---

## 2. Liên kết mã nguồn các phân hệ

* Firmware vi điều khiển: [`firmware/README.md`](../firmware/README.md)
  - ESP1: [`firmware/esp1/README.md`](../firmware/esp1/README.md)
  - ESP2: [`firmware/esp2/README.md`](../firmware/esp2/README.md)
  - ESP3: [`firmware/esp3/README.md`](../firmware/esp3/README.md)
* PC Gateway: [`gateway/README.md`](../gateway/README.md)
* ThingsBoard / CoreIOT: [`thingsboard/README.md`](../thingsboard/README.md)
* Báo cáo và thuyết trình: [`present/README.md`](../present/README.md)
