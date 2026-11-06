# Hướng Dẫn Tự Triển Khai Nền Tảng CoreIOT (ThingsBoard CE) & Mạng Riêng Ảo Tailscale

Tài liệu hướng dẫn kỹ thuật chi tiết từng bước xây dựng, cấu hình và vận hành hệ thống máy chủ IoT Cloud tự lưu trữ (**Self-hosted CoreIOT / ThingsBoard Platform**) kết hợp mạng riêng ảo bảo mật **Tailscale Mesh VPN** cho dự án giám sát nhà nấm thông minh (Nhóm 3 — CO5075).

> [!NOTE]
> Tài liệu này đồng thời phục vụ làm minh chứng kỹ thuật đầy đủ cho tiêu chí đánh giá **N07 (Tự triển khai nền tảng / Self-hosted IoT Platform)** theo đề cương bài tập lớn môn học CO5075.

---

## 1. Kiến Trúc Hạ Tầng Máy Chủ (Infrastructure Architecture)

Toàn bộ nền tảng đám mây CoreIOT được tự triển khai trên máy chủ vật lý độc lập, kết nối an toàn với máy tính trạm Gateway qua mạng Tailscale Mesh VPN mà không cần mở cổng công khai (port forwarding) trên router Internet:

```mermaid
flowchart TD
    subgraph EDGE["Lớp Trạm Cục Bộ (Local Edge Farm)"]
        ESP1["ESP1 (DHT20 / Tasmota)"] -->|WiFi MQTT: 1883| BROKER["NanoMQ Broker"]
        ESP2["ESP2 (LCD Display Hub)"] <-->|WiFi MQTT: 1883| BROKER
        ESP3["ESP3 (PT100 RS485)"] -->|BLE NUS| ESP2
        BROKER <--> GW["Rust PC Gateway"]
    end

    subgraph TAILSCALE["Mạng Bảo Mật Tailscale Mesh VPN (Encrypted WireGuard)"]
        GW <==>|Tailscale Interface (100.x.y.z)| TS_NET["Mesh Network: *.tailc75d93.ts.net"]
    end

    subgraph SERVER["Máy Chủ Vật Lý CoreIOT (fuvi-server.tailc75d93.ts.net)"]
        TS_NET <==> TS_SRV["Tailscale Daemon / MagicDNS"]
        TS_SRV --> NGINX["Nginx Reverse Proxy Container<br/>(mega-nginx-minikube-tb2)"]
        
        subgraph K8S["Cụm Minikube Kubernetes (profile: tb2)"]
            NGINX -->|HTTP: Port 28880 -> 80| INGRESS["Kubernetes Ingress (tb-ingress)<br/>Namespace: thingboard-dev"]
            NGINX -->|MQTT: Port 10000 -> 1883| SVC_MQTT["ThingsBoard MQTT Service"]
            INGRESS --> TB_POD["ThingsBoard CE Pods (CoreIOT Application)"]
            TB_POD <--> DB["PostgreSQL Database Pod (PersistentVolume)"]
        end
    end
```

### Thông số triển khai thực tế:
* **Địa chỉ máy chủ (FQDN):** `https://fuvi-server.tailc75d93.ts.net`
* **IP nội bộ Tailscale:** `192.168.1.32` / Tailscale IP (`100.x.y.z`).
* **Tài khoản quản trị máy chủ:** `justin@192.168.1.32`
* **Cụm Kubernetes:** Minikube (`profile: tb2`, Kubernetes v1.28+).
* **Namespace:** `thingboard-dev`
* **Ingress Controller:** Ingress-Nginx (`tb-ingress`).
* **Cổng dịch vụ mở ra mạng Tailscale:**
  * Cổng `443` / `80`: Web Dashboard & REST API (`https://fuvi-server.tailc75d93.ts.net`).
  * Cổng `10000`: ThingsBoard MQTT Gateway Transport (kết nối trực tiếp từ PC Gateway).

---

## 2. Phần 1: Cài Đặt và Cấu Hình Tailscale Mesh VPN

### 2.1. Cài đặt Tailscale trên máy chủ Ubuntu/Debian
Đăng nhập SSH vào máy chủ và cài đặt gói chính thức:

```bash
# 1. Cài đặt Tailscale qua script chính thức
curl -fsSL https://tailscale.com/install.sh | sh

# 2. Khởi động và kiểm tra dịch vụ
sudo systemctl enable --now tailscaled
sudo systemctl status tailscaled
```

### 2.2. Khởi chạy và kích hoạt thiết bị vào Tailnet
```bash
# Kích hoạt máy chủ vào mạng với tên host fuvi-server
sudo tailscale up --hostname=fuvi-server --accept-routes
```
* Trình duyệt sẽ xuất hiện đường dẫn xác thực OAuth (hoặc đăng nhập bằng tài khoản Google/GitHub quản trị mạng Tailscale).
* Sau khi xác thực, máy chủ sẽ nhận được một địa chỉ IP cố định thuộc dải `100.x.y.z` và tên miền nội bộ MagicDNS.

### 2.3. Cấu hình MagicDNS & HTTPS Certificates
1. Trên trang quản trị **Tailscale Admin Console** (`login.tailscale.com`):
   * Bật tính năng **MagicDNS** trong mục *DNS*.
   * Đặt tên miền Tailnet: `tailc75d93.ts.net`.
   * Bật tính năng **HTTPS Certificates** (Tailscale tự động cấp chứng chỉ Let's Encrypt cho tên miền `fuvi-server.tailc75d93.ts.net`).
2. Trên máy chủ, cấp chứng chỉ SSL cục bộ:
   ```bash
   sudo tailscale cert fuvi-server.tailc75d93.ts.net
   # Sẽ sinh ra 2 file: fuvi-server.tailc75d93.ts.net.crt và fuvi-server.tailc75d93.ts.net.key
   ```

---

## 3. Phần 2: Triển Khai Cụm Minikube / Kubernetes Cho ThingsBoard

### 3.1. Cài đặt Docker & Minikube
```bash
# 1. Cài đặt Docker Engine
sudo apt-get update && sudo apt-get install -y docker.io conntrack
sudo usermod -aG docker $USER && newgrp docker

# 2. Cài đặt Minikube
curl -LO https://storage.googleapis.com/minikube/releases/latest/minikube-linux-amd64
sudo install minikube-linux-amd64 /usr/local/bin/minikube

# 3. Khởi tạo cụm với profile tb2
minikube start --profile=tb2 --driver=docker --cpus=4 --memory=8192 --disk-size=50g
```

### 3.2. Bật Addon Ingress trên Minikube
```bash
minikube addons enable ingress -p tb2
```

### 3.3. Tạo Namespace và Phân bổ Bộ Nhớ Lưu Trữ
```bash
kubectl create namespace thingboard-dev
```

Tạo file lưu trữ cơ sở dữ liệu `thingsboard-db-pvc.yaml`:
```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: tb-postgres-pvc
  namespace: thingboard-dev
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 20Gi
```
Áp dụng:
```bash
kubectl apply -f thingsboard-db-pvc.yaml
```

### 3.4. Triển khai Cơ sở dữ liệu PostgreSQL & ThingsBoard CE
Áp dụng cấu hình triển khai ThingsBoard Community Edition (PostgreSQL DB, ThingsBoard Core Engine, Web UI):

```bash
# Cài đặt PostgreSQL StatefulSet và ThingsBoard Deployment
kubectl apply -f https://raw.githubusercontent.com/thingsboard/thingsboard-ce-k8s/master/minikube/tb-node.yml -n thingboard-dev
```

Kiểm tra trạng thái các Pods:
```bash
kubectl get pods -n thingboard-dev
# Kết quả mong đợi:
# NAME                         READY   STATUS    RESTARTS   AGE
# tb-node-0                    1/1     Running   0          5d
# tb-postgres-0                1/1     Running   0          5d
```

### 3.5. Cấu hình Kubernetes Ingress (`tb-ingress`)
Tạo file cấu hình `tb-ingress.yaml`:
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: tb-ingress
  namespace: thingboard-dev
  annotations:
    kubernetes.io/ingress.class: nginx
    nginx.ingress.kubernetes.io/proxy-body-size: "100m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
spec:
  rules:
  - host: fuvi-server.tailc75d93.ts.net
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: tb-node
            port:
              number: 8080
```
Áp dụng:
```bash
kubectl apply -f tb-ingress.yaml
```

---

## 4. Phần 3: Cấu Hình Nginx Reverse Proxy Container & Tối Ưu Hóa OTA Package Upload

### 4.1. Vai trò của Container `mega-nginx-minikube-tb2`
Minikube chạy cô lập trong mạng bridge của Docker, do đó container Nginx đứng ngoài máy chủ vật lý làm cầu nối chuyển tiếp:
1. Chuyển tiếp cổng Web HTTP/HTTPS (`80` / `443`) vào Ingress NodePort `28880`.
2. Chuyển tiếp cổng MQTT TCP `10000` vào dịch vụ MQTT NodePort của ThingsBoard.

### 4.2. Cấu hình `/home/justin/dienmega/thingsboard_2/nginx/nginx.conf`
Nội dung file cấu hình hoàn chỉnh:

```nginx
user nginx;
worker_processes auto;
error_log /var/log/nginx/error.log warn;
pid /var/run/nginx.pid;

events {
    worker_connections 4096;
}

# 1. Cấu hình Stream Proxy cho giao thức MQTT (TCP Port 10000)
stream {
    upstream thingsboard_mqtt {
        server 192.168.49.2:31883; # IP Minikube Node và NodePort của TB MQTT
    }

    server {
        listen 10000;
        proxy_pass thingsboard_mqtt;
        proxy_connect_timeout 10s;
        proxy_timeout 3600s;
    }
}

# 2. Cấu hình HTTP Reverse Proxy & Xử lý Upload Firmware Lớn
http {
    include /etc/nginx/mime.types;
    default_type application/octet-stream;
    
    # Quan trọng: Cho phép tải file nhị phân OTA firmware dung lượng lớn
    client_max_body_size 100M;
    client_body_buffer_size 16M;

    upstream minikube_ingress {
        server 192.168.49.2:28880; # Ingress HTTP NodePort của Minikube profile tb2
    }

    server {
        listen 80;
        server_name fuvi-server.tailc75d93.ts.net;
        return 301 https://$host$request_uri;
    }

    server {
        listen 443 ssl http2;
        server_name fuvi-server.tailc75d93.ts.net;

        ssl_certificate /etc/tailscale/fuvi-server.tailc75d93.ts.net.crt;
        ssl_certificate_key /etc/tailscale/fuvi-server.tailc75d93.ts.net.key;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;

        location / {
            proxy_pass http://minikube_ingress;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            
            # Tăng timeout hỗ trợ stream file binary
            proxy_connect_timeout 60s;
            proxy_read_timeout 600s;
            proxy_send_timeout 600s;
        }
    }
}
```

Khởi chạy container Nginx:
```bash
docker run -d \
  --name mega-nginx-minikube-tb2 \
  --restart always \
  -p 80:80 \
  -p 443:443 \
  -p 10000:10000 \
  -v /home/justin/dienmega/thingsboard_2/nginx/nginx.conf:/etc/nginx/nginx.conf:ro \
  -v /var/lib/tailscale/certs:/etc/tailscale:ro \
  nginx:alpine
```

### 4.3. Xử lý triệt để lỗi `HTTP 413 Request Entity Too Large`
File nhị phân biên dịch từ PlatformIO của ESP32 (kèm thư viện GUI TFT LCD, Touch, NimBLE, Update) có dung lượng khoảng **1.26MB**, vượt quá mức mặc định 1MB của Nginx.
Nhờ 2 cấu hình đồng bộ sau:
1. Trong file Nginx Reverse Proxy: `client_max_body_size 100M;`
2. Trong Ingress Kubernetes: `nginx.ingress.kubernetes.io/proxy-body-size: "100m"`

Toàn bộ các gói binary OTA kích thước lớn (1MB – 50MB) đều được upload qua REST API thành công tuyệt đối mà không bị ngắt kết nối.

---

## 5. Phần 4: Khởi Tạo Cấu Hình Nền Tảng (Initial Provisioning)

Sau khi máy chủ trực tuyến, truy cập `https://fuvi-server.tailc75d93.ts.net` và đăng nhập:
* **Email:** `master.iot@gmail.com`
* **Mật khẩu:** `master.iot`

### 5.1. Tạo các Device Profiles
Truy cập **Device Profiles** $\rightarrow$ Thêm mới các hồ sơ thiết bị tương ứng:
1. **`esp_temp_humi`**: Dành cho ESP1 (Node cảm biến nhiệt độ & độ ẩm không khí DHT20).
2. **`esp_hub`**: Dành cho ESP2 (Node Hub trung tâm, màn hình hiển thị LCD CYD, điều khiển máy lạnh, OTA).
3. **`esp_ble`**: Dành cho ESP3 (Node đo nhiệt độ giá thể PT100 Modbus truyền qua BLE).

### 5.2. Nhập Root Rule Chain Đầy Đủ (24 Nodes)
1. Truy cập **Rule Chains** $\rightarrow$ chọn **Root Rule Chain**.
2. Hoặc sử dụng script nạp tự động qua API có sẵn trong repo:
   ```bash
   python thingsboard/setup_hysteresis_rulechain.py
   ```
3. Chuỗi quy tắc kích hoạt:
   * Node `Hysteresis Alarm Switch`: tự động giám sát ngưỡng và phát hiện dải trễ Hysteresis chống chập chờn.
   * 6 Node `Create Alarm` + 6 Node `Clear Alarm`: Quản lý báo động cho nhiệt độ không khí, độ ẩm không khí và nhiệt độ giá thể.
   * Cụm node RPC `Prepare RPC AirCon ON/OFF` $\rightarrow$ `Change Originator to ESP2` $\rightarrow$ `Send AirCon RPC to ESP2`: Tự động gửi lệnh điều khiển máy lạnh xuống ESP2 khi nhiệt độ vượt ngưỡng $29^\circ\text{C}$ và tắt khi hạ dưới $26^\circ\text{C}$.

### 5.3. Nhập Bảng Điều Khiển Dashboard
1. Truy cập **Dashboards** $\rightarrow$ Nhập file [`thingsboard/dashboard_group3.json`](../thingsboard/dashboard_group3.json).
2. Dashboard bao gồm:
   * **Bảng Cảnh Báo Ngưỡng Hysteresis (`alarms_table`):** Giám sát trạng thái hoạt động, hỗ trợ xác nhận (Ack) và xóa (Clear).
   * **Bảng Telemetry ESP2:** Hiển thị trực quan chỉ số nhiệt độ giá thể PT100, trạng thái máy lạnh và phiên bản Firmware hiện hành.
   * **Công tắc Air.Con:** Điều khiển bật/tắt máy lạnh thủ công từ xa.
   * **OTA Packages Version Selector & Dispatcher:** Chọn phiên bản từ kho CoreIOT và kích hoạt OTA tức thì.

### 5.4. Thiết lập Kho Lưu Trữ OTA Updates
1. Truy cập **OTA Updates** $\rightarrow$ **Add Package**.
2. Chọn Device Profile: **`esp_hub`**, Type: **`Firmware`**, Title: **`ESP2 Hub Firmware`**.
3. Upload file nhị phân `firmware/esp2/.pio/build/esp32dev/firmware.bin` với phiên bản tương ứng (`1.0.3`, `1.0.4`, `1.0.5`, `1.0.6`).
4. CoreIOT tự động lưu trữ và cấp phát URL tải an toàn:
   ```text
   https://fuvi-server.tailc75d93.ts.net/api/otaPackage/{packageId}/download
   ```

---

## 6. Minh Chứng Nghiệm Thu Tiêu Chí N07 (Báo Cáo Môn Học)

| Hạng mục kiểm tra | Tiêu chuẩn đánh giá | Trạng thái thực tế | Bằng chứng kiểm tra |
|---|---|:---:|---|
| **Cụm máy chủ độc lập** | Máy chủ vật lý riêng, không dùng Cloud công cộng phụ thuộc | **ĐẠT** | Máy chủ `fuvi-server` chạy Minikube Kubernetes `tb2` tại `192.168.1.32`. |
| **Mạng riêng ảo bảo mật** | Mã hóa WireGuard, không lộ cổng ra Internet công cộng | **ĐẠT** | Mạng Tailscale MagicDNS `fuvi-server.tailc75d93.ts.net` bảo mật 100%. |
| **Chứng chỉ số SSL/TLS** | Hỗ trợ kết nối HTTPS và TLS bảo mật | **ĐẠT** | Chứng chỉ hợp lệ cấp phát qua Tailscale / Let's Encrypt. |
| **Dịch vụ MQTT Gateway** | Mở cổng TCP 10000 cho Gateway kết nối 2 chiều | **ĐẠT** | Cổng 10000 mở qua Nginx stream proxy tới cụm ThingsBoard MQTT. |
| **Quy mô xử lý OTA Firmware** | Upload và phân phối firmware lớn > 1MB | **ĐẠT** | Cấu hình `100M` body size, lưu trữ trọn vẹn firmware ESP2 1.26MB. |
| **Chuỗi quy tắc tự động hóa** | Xử lý Hysteresis và phát lệnh RPC trực tiếp | **ĐẠT** | Root Rule Chain 24 nodes hoạt động ổn định và chính xác. |

