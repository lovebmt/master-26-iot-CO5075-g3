#!/usr/bin/env python3
"""
Automated Figure Capture for Report
Captures high-resolution screenshots for:
1. CoreIOT Dashboard (Telemetry, Air.Con Switch, OTA Selector, Alarms Table)
2. CoreIOT Root Rule Chain (24 nodes Hysteresis Alarms & Auto RPC)
3. CoreIOT Devices List (ESP1, ESP2, ESP3, PC-Gateway)
4. CoreIOT Alarms Page (Active alarms)
5. PC Gateway Dashboard (Local Edge UI at 127.0.0.1:8080)
"""

import os
import time
import requests
import urllib3
from playwright.sync_api import sync_playwright

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

COREIOT_URL = "https://fuvi-server.tailc75d93.ts.net"
FIGURES_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "figures"))
os.makedirs(FIGURES_DIR, exist_ok=True)

def get_auth_tokens():
    res = requests.post(
        f"{COREIOT_URL}/api/auth/login",
        json={"username": "master.iot@gmail.com", "password": "master.iot"},
        verify=False,
        timeout=10
    )
    res.raise_for_status()
    data = res.json()
    return data["token"], data.get("refreshToken", "")

def capture_figures():
    print(f"[*] Destination figures directory: {FIGURES_DIR}")
    token, refresh = get_auth_tokens()
    print("[*] Successfully authenticated with CoreIOT API.")

    with sync_playwright() as p:
        browser = p.chromium.launch(
            executable_path="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            headless=True,
            args=["--ignore-certificate-errors", "--no-sandbox", "--disable-gpu"]
        )
        context = browser.new_context(
            ignore_https_errors=True,
            viewport={"width": 1920, "height": 1080},
            device_scale_factor=2 # Retina crisp resolution
        )
        page = context.new_page()

        # 1. Initialize session with JWT token
        page.goto(f"{COREIOT_URL}/login", timeout=30000)
        page.evaluate(f"""() => {{
            localStorage.setItem("jwt_token", "{token}");
            localStorage.setItem("refreshToken", "{refresh}");
        }}""")

        # ----------------------------------------------------
        # 1. CAPTURE PC GATEWAY DASHBOARD (127.0.0.1:8080)
        # ----------------------------------------------------
        print("\n[1/5] Capturing PC Gateway Dashboard...")
        try:
            page.goto("http://127.0.0.1:8080/devices", timeout=15000)
            time.sleep(2)
            gw_path = os.path.join(FIGURES_DIR, "fig_gateway_dashboard.png")
            page.screenshot(path=gw_path, full_page=True)
            print(f"  -> Saved: {gw_path}")
        except Exception as e:
            print(f"  [!] Failed to capture Gateway: {e}")

        # ----------------------------------------------------
        # 2. CAPTURE COREIOT DEVICES LIST
        # ----------------------------------------------------
        print("\n[2/5] Capturing CoreIOT Devices List...")
        try:
            page.goto(f"{COREIOT_URL}/devices", timeout=30000)
            time.sleep(4)
            dev_path = os.path.join(FIGURES_DIR, "fig_coreiot_devices.png")
            page.screenshot(path=dev_path, full_page=True)
            print(f"  -> Saved: {dev_path}")
        except Exception as e:
            print(f"  [!] Failed to capture Devices: {e}")

        # ----------------------------------------------------
        # 3. CAPTURE COREIOT ROOT RULE CHAIN (48e0fd00-d678-11f1-9301-8d833790fa1a)
        # ----------------------------------------------------
        print("\n[3/5] Capturing CoreIOT Root Rule Chain...")
        try:
            rc_url = f"{COREIOT_URL}/ruleChains/48e0fd00-d678-11f1-9301-8d833790fa1a"
            page.goto(rc_url, timeout=30000)
            time.sleep(6) # Wait for canvas rendering
            rc_path = os.path.join(FIGURES_DIR, "fig_root_rule_chain.png")
            page.screenshot(path=rc_path, full_page=True)
            print(f"  -> Saved: {rc_path}")
        except Exception as e:
            print(f"  [!] Failed to capture Rule Chain: {e}")

        # ----------------------------------------------------
        # 4. CAPTURE COREIOT ALARMS VIEW
        # ----------------------------------------------------
        print("\n[4/5] Capturing CoreIOT Alarms Table...")
        try:
            page.goto(f"{COREIOT_URL}/alarms", timeout=30000)
            time.sleep(4)
            alarm_path = os.path.join(FIGURES_DIR, "fig_coreiot_alarms.png")
            page.screenshot(path=alarm_path, full_page=True)
            print(f"  -> Saved: {alarm_path}")
        except Exception as e:
            print(f"  [!] Failed to capture Alarms: {e}")

        # ----------------------------------------------------
        # 5. CAPTURE COREIOT MAIN DASHBOARD (ce9732c0-d7f9-11f1-9301-8d833790fa1a)
        # ----------------------------------------------------
        print("\n[5/5] Capturing CoreIOT Main Dashboard...")
        try:
            db_url = f"{COREIOT_URL}/dashboards/ce9732c0-d7f9-11f1-9301-8d833790fa1a"
            page.goto(db_url, timeout=30000)
            time.sleep(6) # Wait for timeseries charts, switches, and tables to populate
            db_path = os.path.join(FIGURES_DIR, "fig_coreiot_dashboard.png")
            page.screenshot(path=db_path, full_page=True)
            print(f"  -> Saved: {db_path}")
        except Exception as e:
            print(f"  [!] Failed to capture Dashboard: {e}")

        browser.close()

    print("\n" + "=" * 60)
    print("✅ TẤT CẢ CÁC HÌNH ẢNH ĐÃ ĐƯỢC CHỤP VÀ LƯU VÀO present/report/figures!")
    print("=" * 60)

if __name__ == "__main__":
    capture_figures()

