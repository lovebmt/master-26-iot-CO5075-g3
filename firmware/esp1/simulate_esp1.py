#!/usr/bin/env python3
"""
ESP1 Simulation Script (DHT20 Sensor Node)
Simulates ESP1 publishing temperature and humidity to NanoMQ (iot/esp1/telemetry).
Tests Hysteresis Alarms and automatic Air Conditioner (Air.Con) control on ESP2.
"""

import sys
import time
import json
import argparse
import requests
import urllib3
import paho.mqtt.client as mqtt

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

COREIOT_URL = "https://fuvi-server.tailc75d93.ts.net"
ESP1_TOPIC = "iot/esp1/telemetry"
ESP2_DEVICE_ID = "cf2cd650-d7e9-11f1-9301-8d833790fa1a"

def get_coreiot_token():
    try:
        res = requests.post(
            f"{COREIOT_URL}/api/auth/login",
            json={"username": "master.iot@gmail.com", "password": "master.iot"},
            verify=False,
            timeout=5
        )
        if res.status_code == 200:
            return res.json().get("token")
    except Exception as e:
        print(f"[!] Warning: Cannot login to CoreIOT API: {e}")
    return None

def fetch_active_alarms(token):
    if not token:
        return []
    try:
        headers = {"X-Authorization": f"Bearer {token}"}
        res = requests.get(
            f"{COREIOT_URL}/api/alarms?pageSize=10&page=0&status=ACTIVE_UNACK",
            headers=headers,
            verify=False,
            timeout=5
        )
        if res.status_code == 200:
            return res.json().get("data", [])
    except Exception:
        pass
    return []

def fetch_esp2_aircon_state(token):
    if not token:
        return None
    try:
        headers = {"X-Authorization": f"Bearer {token}"}
        res = requests.get(
            f"{COREIOT_URL}/api/plugins/telemetry/DEVICE/{ESP2_DEVICE_ID}/values/timeseries?keys=light_on",
            headers=headers,
            verify=False,
            timeout=5
        )
        if res.status_code == 200:
            entries = res.json().get("light_on", [])
            if entries:
                return entries[0].get("value") == "true"
    except Exception:
        pass
    return None

def publish_telemetry(client, temp, humi):
    payload = {
        "values": {
            "temperature": round(float(temp), 2),
            "humidity": round(float(humi), 2)
        }
    }
    client.publish(ESP1_TOPIC, json.dumps(payload))
    print(f"\n[ESP1 -> NanoMQ] Published: Temp={temp}°C, Humidity={humi}% (Topic: {ESP1_TOPIC})")

def run_test_sequence(broker, port):
    print("=" * 70)
    print("🚀 BẮT ĐẦU TEST TOÀN DIỆN HYSTERESIS ALARM & AIR.CON TRÊN ESP2")
    print("=" * 70)

    token = get_coreiot_token()
    client = mqtt.Client()
    try:
        client.connect(broker, port, 60)
    except Exception as e:
        print(f"[-] Không thể kết nối MQTT Broker tại {broker}:{port}: {e}")
        sys.exit(1)

    steps = [
        {
            "desc": "BƯỚC 1: Nhiệt độ bình thường (25.5°C, 85% RH)",
            "temp": 25.5, "humi": 85.0, "wait": 4,
            "expected_alarm": "Không có cảnh báo",
            "expected_aircon": "Air.Con OFF"
        },
        {
            "desc": "BƯỚC 2: Nhiệt độ vượt ngưỡng làm mát & cảnh báo cao (31.5°C > 30°C và > 29°C)",
            "temp": 31.5, "humi": 85.0, "wait": 5,
            "expected_alarm": "Cảnh báo [Air Temperature High] (CRITICAL)",
            "expected_aircon": "Air.Con BẬT (ON)"
        },
        {
            "desc": "BƯỚC 3: Nhiệt độ hạ vào vùng trễ (28.0°C <= 29°C clear alarm, nhưng trong vùng 26°C-29°C)",
            "temp": 28.0, "humi": 85.0, "wait": 5,
            "expected_alarm": "Alarm High được CLEAR",
            "expected_aircon": "Air.Con DUY TRÌ BẬT (ON) - Hysteresis chống ngắt sớm"
        },
        {
            "desc": "BƯỚC 4: Nhiệt độ hạ sâu dưới 26.0°C (24.5°C < 26°C) -> Tắt máy lạnh",
            "temp": 24.5, "humi": 85.0, "wait": 5,
            "expected_alarm": "Không có alarm",
            "expected_aircon": "Air.Con TẮT (OFF)"
        },
        {
            "desc": "BƯỚC 5: Nhiệt độ quá thấp < 24.0°C (22.0°C < 24°C) -> Cảnh báo thấp",
            "temp": 22.0, "humi": 85.0, "wait": 5,
            "expected_alarm": "Cảnh báo [Air Temperature Low] (WARNING)",
            "expected_aircon": "Air.Con giữ TẮT (OFF)"
        },
        {
            "desc": "BƯỚC 6: Trở lại an toàn (27.0°C >= 25°C clear low alarm)",
            "temp": 27.0, "humi": 85.0, "wait": 5,
            "expected_alarm": "Alarm Low được CLEAR",
            "expected_aircon": "Air.Con TẮT (OFF)"
        }
    ]

    for i, s in enumerate(steps, 1):
        print(f"\n--- {s['desc']} ---")
        print(f"[*] Kỳ vọng: {s['expected_alarm']} | {s['expected_aircon']}")
        publish_telemetry(client, s["temp"], s["humi"])
        
        print(f"[*] Đang chờ CoreIOT RuleChain xử lý ({s['wait']}s)...")
        time.sleep(s["wait"])

        # Kiểm tra trạng thái thực tế từ CoreIOT
        alarms = fetch_active_alarms(token)
        aircon = fetch_esp2_aircon_state(token)

        alarm_str = ", ".join([f"{a.get('type')} ({a.get('severity')})" for a in alarms]) if alarms else "None"
        aircon_str = "ON" if aircon else ("OFF" if aircon is False else "Unknown")

        print(f"[CoreIOT Thực Tế] Alarms đang Active: {alarm_str}")
        print(f"[ESP2 Thực Tế] Trạng thái Air.Con: {aircon_str}")

    client.disconnect()
    print("\n" + "=" * 70)
    print("✅ HOÀN TẤT KỊCH BẢN THỬ NGHIỆM!")
    print("=" * 70)

def main():
    parser = argparse.ArgumentParser(description="ESP1 Simulator for Mushroom Farm IoT System")
    parser.add_argument("--broker", default="127.0.0.1", help="MQTT Broker host (default: 127.0.0.1)")
    parser.add_argument("--port", type=int, default=1883, help="MQTT Broker port (default: 1883)")
    parser.add_argument("--test", action="store_true", help="Chạy kịch bản kiểm tra toàn diện Hysteresis & Air.Con")
    parser.add_argument("--temp", type=float, default=None, help="Gửi 1 bản tin với nhiệt độ chỉ định")
    parser.add_argument("--humi", type=float, default=85.0, help="Độ ẩm không khí (mặc định 85.0%%)")
    parser.add_argument("--continuous", action="store_true", help="Chạy liên tục gửi dữ liệu mỗi 5 giây")

    args = parser.parse_args()

    if args.test:
        run_test_sequence(args.broker, args.port)
        return

    client = mqtt.Client()
    try:
        client.connect(args.broker, args.port, 60)
    except Exception as e:
        print(f"[-] Không thể kết nối MQTT Broker tại {args.broker}:{args.port}: {e}")
        sys.exit(1)

    if args.temp is not None:
        publish_telemetry(client, args.temp, args.humi)
        client.disconnect()
        return

    if args.continuous:
        print(f"[*] Đang chạy ESP1 Simulation liên tục tới {args.broker}:{args.port} (Ctrl+C để dừng)...")
        import random
        base_temp = 27.5
        base_humi = 86.0
        try:
            while True:
                t = base_temp + random.uniform(-1.0, 1.0)
                h = base_humi + random.uniform(-2.0, 2.0)
                publish_telemetry(client, t, h)
                time.sleep(5)
        except KeyboardInterrupt:
            print("\nĐã dừng simulation.")
            client.disconnect()
        return

    # Mặc định: chạy test sequence
    run_test_sequence(args.broker, args.port)

if __name__ == "__main__":
    main()

