# ThingsBoard / CoreIOT

Store exportable device templates, dashboards, RuleChains, and setup notes here. Planned rubric items: T11 (templates/dashboard), T12 (RuleChain), and T13 (threshold notifications). Do not store access tokens or passwords.
- Swagger UI: https://fuvi-server.tailc75d93.ts.net/swagger-ui/index.html
- API base URL: https://fuvi-server.tailc75d93.ts.net
- Authentication endpoint: `POST /api/auth/login`
- Development login: `master.iot@gmail.com` / `master.iot`
- mqtt port: 10000 (--tls-use-os-certs , insecure)

## 1. Device Profiles
Gateway tự động kết nối các thiết bị hạ tầng lên CoreIOT qua topic `v1/gateway/connect`:
- `ESP1`: Profile **`esp_temp_humi`**
- `ESP2`: Profile **`esp_hub`**
- `ESP3`: Profile **`esp_ble`**

## 2. Thiết lập OTA Firmware trên CoreIOT
### Cách 1: Qua Dashboard Widget "OTA Version Selector & Dispatcher"
Dashboard `IoT System Dashboard — Group 3` tích hợp widget điều khiển OTA động:
- Dropdown chọn phiên bản từ kho CoreIOT OTA Packages (`v1.0.4`, `v1.0.5`, `v1.0.6`, `v1.0.3`).
- Nút **Notify ESP2 (Prompt)**: gửi RPC `ota_update` với `force: false`. ESP2 hiển thị popup `[YES] [NO]` trên màn hình cảm ứng để người vận hành xác nhận.
- Nút **Force OTA**: gửi RPC `ota_update` với `force: true`. ESP2 tự động tải nạp OTA và reboot ngay lập tức.

### Cách 2: Qua Kho lưu trữ OTA Updates của ThingsBoard (Khép kín không dùng External URL)
1. Đăng nhập trang quản trị CoreIOT -> **OTA Updates**.
2. Package chính thức đã được tải lên:
   - **Device profile**: **`esp_hub`**
   - **Type**: `Firmware`
   - **Title**: `ESP2 Hub Firmware`
   - **Version**: `1.0.3`
   - **File**: `esp2.bin` (kích thước 1.26MB, Checksum SHA-256).
   - **Download URL trên CoreIOT**: `https://fuvi-server.tailc75d93.ts.net/api/otaPackage/{packageId}/download`
3. Gán Firmware cho thiết bị **`ESP2`** hoặc Device Profile **`esp_hub`**.
4. Gateway kéo file từ URL CoreIOT về lưu đệm cục bộ tại `gateway/firmware/esp2.bin` và cung cấp cho ESP2 trong mạng LAN nội bộ (`http://<IP_GATEWAY>:8080/firmware/esp2.bin`).

## 3. Cấu hình Nginx & Minikube trên máy chủ CoreIOT
* Máy chủ: `fuvi-server.tailc75d93.ts.net` (`justin@192.168.1.32`).
* Cụm: Minikube (`profile: tb2`), Ingress: `tb-ingress` (namespace `thingboard-dev`).
* Nginx Reverse Proxy Container: `mega-nginx-minikube-tb2` ánh xạ cổng `28880 -> 80`.
* Đã cấu hình `client_max_body_size 100M;` trong `/home/justin/dienmega/thingsboard_2/nginx/nginx.conf` và Kubernetes Ingress annotation `nginx.ingress.kubernetes.io/proxy-body-size: "100m"` để giải quyết dứt điểm lỗi `HTTP 413 Request Entity Too Large` khi upload các bản build firmware lớn (> 1MB).
