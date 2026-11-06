#!/usr/bin/env bash
# Script tiện ích chuyển đổi phiên bản firmware OTA cho ESP2
# Sử dụng: ./switch_version.sh <version>
# Ví dụ:   ./switch_version.sh 1.0.5

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Available pre-built versions in gateway/firmware/:"
    ls gateway/firmware/esp2_v*.bin 2>/dev/null | sed -E 's/.*esp2_v(.*)\.bin/  - \1/'
    exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="$ROOT_DIR/gateway/firmware/esp2_v${VERSION}.bin"
TARGET_BIN="$ROOT_DIR/gateway/firmware/esp2.bin"
VERSION_JSON="$ROOT_DIR/gateway/firmware/version.json"

# Lấy IP mạng LAN của máy Mac (hoặc mặc định 192.168.1.19)
HOST_IP=$(ipconfig getifaddr en0 2>/dev/null || echo "192.168.1.19")

# Nếu chưa có file .bin của version này, tự động build mới bằng PlatformIO
if [ ! -f "$BIN_PATH" ]; then
    echo "=> Chưa có bản build cho version $VERSION. Đang tiến hành build mới..."
    sed -i '' -E "s/#define FW_VERSION .*/#define FW_VERSION   \"$VERSION\"/" "$ROOT_DIR/firmware/esp2/src/main.cpp"
    /Users/mac/.platformio/penv/bin/pio run -d "$ROOT_DIR/firmware/esp2"
    if [ $? -ne 0 ]; then
        echo "=> Build thất bại!"
        exit 1
    fi
    cp "$ROOT_DIR/firmware/esp2/.pio/build/esp32dev/firmware.bin" "$BIN_PATH"
    echo "=> Đã tạo file: $BIN_PATH"
fi

# Copy file binary được chọn thành esp2.bin để Gateway phục vụ
cp "$BIN_PATH" "$TARGET_BIN"

# Cập nhật version.json
cat <<EOF > "$VERSION_JSON"
{
  "version": "$VERSION",
  "url": "http://${HOST_IP}:8080/firmware/esp2.bin"
}
EOF

echo "=========================================================="
echo "  ĐÃ THIẾT LẬP THÀNH CÔNG TARGET OTA: v$VERSION"
echo "=========================================================="
echo " - File phục vụ: gateway/firmware/esp2.bin (nguồn: esp2_v${VERSION}.bin)"
echo " - Nội dung version.json:"
cat "$VERSION_JSON"
echo ""
echo "=> Bây giờ bạn có thể:"
echo "   1. Bấm nút [CHECK FW] trên màn hình LCD ESP2 để cập nhật."
echo "   2. Hoặc gửi lệnh OTA từ ThingsBoard / Web Gateway."
echo "=========================================================="

