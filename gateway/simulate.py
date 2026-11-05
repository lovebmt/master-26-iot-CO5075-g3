import json
import ssl
import sys
import time
import paho.mqtt.client as mqtt
import paho.mqtt.publish as publish

# ponytail: simple simulation client; upgrade to scheduled cron/service when testing in continuous bench
TB_HOST = "fuvi-server.tailc75d93.ts.net"
TB_PORT = 10000
TB_TOKEN = "dd77ZqjaYCpv2F2sG71E"

LOCAL_HOST = "127.0.0.1"
LOCAL_PORT = 1883

def send_local():
    """Publish mock ESP telemetry to local gateway broker (NanoMQ)."""
    ts = int(time.time() * 1000)
    publish.single("iot/esp1/telemetry", json.dumps({"ts": ts, "values": {"co2_ppm": 620}}), hostname=LOCAL_HOST, port=LOCAL_PORT)
    publish.single("iot/esp2/telemetry", json.dumps({"ts": ts, "values": {"tds_ppm": 480, "light_on": True}}), hostname=LOCAL_HOST, port=LOCAL_PORT)
    print("Local MQTT sent: esp1, esp2")

def send_coreiot(co2=650, tds=490):
    """Publish to CoreIOT via MQTT TLS on port 10000 (insecure / os-certs)."""
    ts = int(time.time() * 1000)
    done = False

    def on_connect(c, userdata, flags, rc, properties=None):
        nonlocal done
        assert rc == 0, f"Connect failed rc={rc}"
        # Connect downstream devices
        c.publish("v1/gateway/connect", json.dumps({"device": "ESP1", "type": "CO2 Sensor"}), qos=1)
        c.publish("v1/gateway/connect", json.dumps({"device": "ESP2", "type": "TDS Sensor"}), qos=1)
        # Publish telemetry
        t_esp1 = json.dumps({"ESP1": [{"ts": ts, "values": {"co2_ppm": co2}}]})
        t_esp2 = json.dumps({"ESP2": [{"ts": ts, "values": {"tds_ppm": tds, "light_on": True}}]})
        c.publish("v1/gateway/telemetry", t_esp1, qos=1)
        c.publish("v1/gateway/telemetry", t_esp2, qos=1)
        done = True

    client = mqtt.Client(
        mqtt.CallbackAPIVersion.VERSION2 if hasattr(mqtt, "CallbackAPIVersion") else None,
        "sim-device"
    )
    client.tls_set(cert_reqs=ssl.CERT_NONE)
    client.tls_insecure_set(True)
    client.username_pw_set(TB_TOKEN, "")
    client.on_connect = on_connect

    client.connect(TB_HOST, TB_PORT, 10)
    client.loop_start()
    for _ in range(30):
        if done:
            break
        time.sleep(0.1)
    client.loop_stop()
    assert done, "MQTT TLS publish timed out"
    print(f"CoreIOT MQTT TLS port {TB_PORT} sent: ESP1 (co2={co2}), ESP2 (tds={tds})")

if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else "coreiot"
    send_local() if mode == "local" else send_coreiot()

