#!/usr/bin/env python3
import sys
import os
import json
import ssl
import urllib.request
import urllib.error

ctx = ssl._create_unverified_context()
TB_BASE = os.environ.get("TB_BASE_URL", "https://fuvi-server.tailc75d93.ts.net")
TB_USER = os.environ.get("TB_ADMIN_USER", "master.iot@gmail.com")
TB_PASS = os.environ.get("TB_ADMIN_PASS", "master.iot")

def main():
    if len(sys.argv) < 2:
        print("Usage: sync_coreiot_firmware.py <target_version> [destination_bin] [version_json]")
        sys.exit(1)

    target_ver = sys.argv[1].lstrip("v")
    script_dir = os.path.dirname(os.path.abspath(__file__))
    workspace = os.path.abspath(os.path.join(script_dir, ".."))
    dest_bin = sys.argv[2] if len(sys.argv) > 2 else os.path.join(workspace, "firmware/esp2.bin")
    version_json_path = sys.argv[3] if len(sys.argv) > 3 else os.path.join(workspace, "firmware/version.json")

    print(f"[CoreIOT Sync] Connecting to CoreIOT server at {TB_BASE}...")
    # 1. Login to CoreIOT
    login_data = json.dumps({"username": TB_USER, "password": TB_PASS}).encode()
    req = urllib.request.Request(f"{TB_BASE}/api/auth/login", data=login_data, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, context=ctx) as res:
            token = json.loads(res.read().decode())["token"]
    except Exception as e:
        print(f"[CoreIOT Sync Error] Failed to login to CoreIOT: {e}", file=sys.stderr)
        sys.exit(1)

    # 2. Fetch OTA packages list
    req = urllib.request.Request(f"{TB_BASE}/api/otaPackages?pageSize=50&page=0", headers={"X-Authorization": f"Bearer {token}"})
    try:
        with urllib.request.urlopen(req, context=ctx) as res:
            pkgs = json.loads(res.read().decode()).get("data", [])
    except Exception as e:
        print(f"[CoreIOT Sync Error] Failed to list packages: {e}", file=sys.stderr)
        sys.exit(1)

    matched = None
    for p in pkgs:
        if p.get("version", "").lstrip("v") == target_ver:
            matched = p
            break

    if not matched:
        print(f"[CoreIOT Sync Error] Version {target_ver} not found in CoreIOT OTA Packages repository!", file=sys.stderr)
        print("Available in CoreIOT: " + ", ".join([p.get("version", "") for p in pkgs]), file=sys.stderr)
        sys.exit(1)

    pkg_id = matched["id"]["id"]
    pkg_title = matched.get("title", "ESP2 Hub Firmware")
    data_size = matched.get("dataSize", 0)
    print(f"[CoreIOT Sync] Found Package: {pkg_title} v{target_ver} (ID: {pkg_id}, Size: {data_size:,} bytes)")

    # 3. Download binary directly from CoreIOT
    download_url = f"{TB_BASE}/api/otaPackage/{pkg_id}/download"
    print(f"[CoreIOT Sync] Downloading binary from {download_url}...")
    down_req = urllib.request.Request(download_url, headers={"X-Authorization": f"Bearer {token}"})
    with urllib.request.urlopen(down_req, context=ctx) as res:
        bin_data = res.read()

    os.makedirs(os.path.dirname(dest_bin), exist_ok=True)
    with open(dest_bin, "wb") as f:
        f.write(bin_data)
    print(f"[CoreIOT Sync] Successfully cached {len(bin_data):,} bytes to {dest_bin}")

    # 4. Generate version.json dynamically from CoreIOT metadata
    vinfo = {
        "version": target_ver,
        "package_id": pkg_id,
        "title": pkg_title,
        "data_size": len(bin_data),
        "checksum": matched.get("checksum"),
        "checksum_algorithm": matched.get("checksumAlgorithm"),
        "source": "CoreIOT OTA Package Repository",
        "url": "http://192.168.1.19:8080/firmware/esp2.bin"
    }
    with open(version_json_path, "w") as f:
        json.dump(vinfo, f, indent=2)
    print(f"[CoreIOT Sync] Successfully written {version_json_path} with CoreIOT metadata.")

if __name__ == "__main__":
    main()

