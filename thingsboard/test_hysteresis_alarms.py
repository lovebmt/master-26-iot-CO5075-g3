import requests
import json
import time
import urllib3

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

SESSION = requests.Session()
BASE_URL = "https://fuvi-server.tailc75d93.ts.net"

def login():
    res = SESSION.post(f"{BASE_URL}/api/auth/login", json={"username": "master.iot@gmail.com", "password": "master.iot"}, verify=False)
    res.raise_for_status()
    token = res.json()["token"]
    return {"X-Authorization": f"Bearer {token}", "Content-Type": "application/json"}

def test_alarms():
    headers = login()
    
    # ESP1 device ID: 4fb8f7f0-d7e9-11f1-9301-8d833790fa1a
    # ESP2 device ID: cf2cd650-d7e9-11f1-9301-8d833790fa1a
    esp1_id = "4fb8f7f0-d7e9-11f1-9301-8d833790fa1a"
    esp2_id = "cf2cd650-d7e9-11f1-9301-8d833790fa1a"
    
    print("=== Step 1: Send High Temperature to ESP1 (31.5°C > 30°C) ===")
    r = SESSION.post(f"{BASE_URL}/api/plugins/telemetry/DEVICE/{esp1_id}/timeseries/ANY", headers=headers, json={"temperature": 31.5, "humidity": 88.0}, verify=False)
    print("Post telemetry result:", r.status_code)
    time.sleep(1.5)

    # Check active alarms
    alarms = SESSION.get(f"{BASE_URL}/api/alarm/DEVICE/{esp1_id}?status=ACTIVE_UNACK&pageSize=10&page=0", headers=headers, verify=False).json()
    print("ESP1 Active Alarms count:", alarms.get("totalElements"))
    for a in alarms.get("data", []):
        print(f" -> Type: {a.get('type')}, Severity: {a.get('severity')}, Status: {a.get('status')}")

    print("\n=== Step 2: Send Temperature in Hysteresis Deadband (29.5°C, between 29°C and 30°C) ===")
    r = SESSION.post(f"{BASE_URL}/api/plugins/telemetry/DEVICE/{esp1_id}/timeseries/ANY", headers=headers, json={"temperature": 29.5}, verify=False)
    time.sleep(1.5)
    alarms = SESSION.get(f"{BASE_URL}/api/alarm/DEVICE/{esp1_id}?status=ACTIVE_UNACK&pageSize=10&page=0", headers=headers, verify=False).json()
    print("ESP1 Active Alarms count (should still remain active):", alarms.get("totalElements"))

    print("\n=== Step 3: Send Temperature within Safe Cleared zone (27.0°C <= 29°C) ===")
    r = SESSION.post(f"{BASE_URL}/api/plugins/telemetry/DEVICE/{esp1_id}/timeseries/ANY", headers=headers, json={"temperature": 27.0}, verify=False)
    time.sleep(1.5)
    alarms = SESSION.get(f"{BASE_URL}/api/alarm/DEVICE/{esp1_id}?status=CLEARED_UNACK&pageSize=10&page=0", headers=headers, verify=False).json()
    print("ESP1 Cleared Alarms count (should now be CLEARED):", alarms.get("totalElements"))
    for a in alarms.get("data", []):
        print(f" -> Cleared Alarm: {a.get('type')}, Status: {a.get('status')}")

if __name__ == "__main__":
    test_alarms()

