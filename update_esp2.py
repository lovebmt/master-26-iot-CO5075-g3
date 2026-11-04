with open('firmware/esp2/src/main.cpp', 'r') as f:
    lines = f.readlines()

new_lines = []
skip_send = False
skip_wifi = False
skip_loop = False

for i, line in enumerate(lines):
    if "void send_telemetry() {" in line:
        new_lines.append(line)
        new_lines.append("    if (mqttClient.connected()) {
")
        new_lines.append("        char payload[96];
")
        new_lines.append("        int tds = (current_tds >= 0) ? current_tds : 0;
")
        new_lines.append('        snprintf(payload, sizeof(payload), "{\"values\":{\"tds_ppm\":%d,\"light_on\":%s}}",
')
        new_lines.append('                 tds, current_light ? "true" : "false");
')
        new_lines.append("        mqttClient.publish(TDS_TOPIC, payload);
")
        new_lines.append('        Serial.printf("Telemetry sent: %s\n", payload);
')
        new_lines.append("    }
")
        new_lines.append("}
")
        skip_send = True
        continue
    if skip_send:
        if line.strip() == "}":
            skip_send = False
        continue

    if 'WiFi.begin("Fuvitech", "fuvitech.vn");' in line and i < 280:
        new_lines.append('    WiFi.mode(WIFI_STA);
')
        new_lines.append('    WiFi.setAutoReconnect(true);
')
        new_lines.append('    WiFi.begin("Fuvitech", "fuvitech.vn");
')
        new_lines.append('    Serial.print("Connecting to Wi-Fi");
')
        new_lines.append('    uint32_t t0 = millis();
')
        new_lines.append('    while (WiFi.status() != WL_CONNECTED && millis() - t0 < 8000) {
')
        new_lines.append('        delay(250);
')
        new_lines.append('        Serial.print(".");
')
        new_lines.append('    }
')
        new_lines.append('    Serial.println();
')
        new_lines.append('    if (WiFi.status() == WL_CONNECTED) {
')
        new_lines.append('        current_wifi = true;
')
        new_lines.append('        Serial.printf("Wi-Fi connected, IP: %s\n", WiFi.localIP().toString().c_str());
')
        new_lines.append('    } else {
')
        new_lines.append('        Serial.println("Wi-Fi not ready yet; will auto-reconnect in loop");
')
        new_lines.append('    }
')
        skip_wifi = True
        continue
    if skip_wifi:
        if 'broker = "192.168.1.19"' in line:
            skip_wifi = False
            new_lines.append(line)
        continue

    if '// Wi-Fi & MQTT loop' in line:
        new_lines.append('    // Wi-Fi & MQTT loop
')
        new_lines.append('    current_wifi = (WiFi.status() == WL_CONNECTED);
')
        new_lines.append('    if (!current_wifi) {
')
        new_lines.append('        static uint32_t last_wifi_try = 0;
')
        new_lines.append('        if (millis() - last_wifi_try > 8000) {
')
        new_lines.append('            last_wifi_try = millis();
')
        new_lines.append('            Serial.println("Reconnecting Wi-Fi...");
')
        new_lines.append('            WiFi.disconnect();
')
        new_lines.append('            WiFi.begin("Fuvitech", "fuvitech.vn");
')
        new_lines.append('        }
')
        new_lines.append('    } else {
')
        new_lines.append('        if (!mqttClient.connected()) {
')
        new_lines.append('            static uint32_t last_reconnect = 0;
')
        new_lines.append('            if (millis() - last_reconnect > 3000) {
')
        new_lines.append('                last_reconnect = millis();
')
        new_lines.append('                if (mqttClient.connect("esp2")) {
')
        new_lines.append('                    mqttClient.subscribe(CMD_TOPIC);
')
        new_lines.append('                    Serial.println("MQTT connected & subscribed");
')
        new_lines.append('                    send_telemetry();
')
        new_lines.append('                }
')
        new_lines.append('            }
')
        new_lines.append('        } else {
')
        new_lines.append('            mqttClient.loop();
')
        new_lines.append('        }
')
        new_lines.append('    }
')
        skip_loop = True
        continue
    if skip_loop:
        if 'update_screen();' in line:
            skip_loop = False
            new_lines.append(line)
        continue

    new_lines.append(line)

with open('firmware/esp2/src/main.cpp', 'w') as f:
    f.writelines(new_lines)

print("Updated main.cpp successfully")
