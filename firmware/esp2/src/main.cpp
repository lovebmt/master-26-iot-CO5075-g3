#include <Arduino.h>
#include <WiFi.h>
#include <WebServer.h>
#include <Update.h>
#include <HTTPClient.h>
#include <esp_ota_ops.h>
#include <PubSubClient.h>
#include <NimBLEDevice.h>
#include <TFT_eSPI.h>
#include <SPI.h>
#include <XPT2046_Touchscreen.h>
#include <ArduinoJson.h>

#define FW_VERSION   "1.0.3"
#define TDS_TOPIC    "iot/esp2/telemetry"
#define CMD_TOPIC    "iot/esp2/command"
#define RESULT_TOPIC "iot/esp2/result"

#define SERVICE_UUID "6e400001-b5a3-f393-e0a9-e50e24dcca9e"
#define TX_UUID      "6e400003-b5a3-f393-e0a9-e50e24dcca9e"

// CYD RGB LED (Active-Low)
#define PIN_RED   4
#define PIN_GREEN 16
#define PIN_BLUE  17

// CYD Touch XPT2046 (SPI3)
#define XPT2046_IRQ  36
#define XPT2046_MOSI 32
#define XPT2046_MISO 39
#define XPT2046_CLK  25
#define XPT2046_CS   33

static TFT_eSPI tft = TFT_eSPI();
static SPIClass touchSPI(VSPI);
static XPT2046_Touchscreen touch(XPT2046_CS, XPT2046_IRQ);

static WiFiClient espClient;
static PubSubClient mqttClient(espClient);
static WebServer server(8080);

static volatile int16_t current_tds = -1;
static volatile bool current_light = false;
static volatile bool current_ble = false;
static bool current_wifi = false;

static int16_t last_tds_shown = -2;
static int8_t  last_light_shown = -1;
static int8_t  last_ble_shown = -1;
static int8_t  last_wifi_shown = -1;

static bool touch_down = false;
static NimBLEClient *bleClient = nullptr;
static NimBLERemoteCharacteristic *pRemoteChar = nullptr;

// OTA UI and State
enum OtaUiState {
    UI_NORMAL,
    UI_PROMPT,
    UI_PROGRESS
};
static volatile OtaUiState ota_ui_state = UI_NORMAL;
static char pending_ota_url[192] = "";
static char pending_ota_ver[32] = "";
static volatile bool ota_confirmed = false;
static volatile bool ota_rejected = false;
static volatile bool ota_in_progress = false;

static bool get_touch_point(uint16_t &tx, uint16_t &ty) {
    if (!touch.touched()) return false;
    TS_Point p = touch.getPoint();
    if (p.z < 200) return false;
    // Map raw ADC (200-3800) to 320x240 landscape (Rotation 1)
    int32_t x = map(p.x, 200, 3800, 0, 320);
    int32_t y = map(p.y, 240, 3800, 0, 240);
    tx = (uint16_t)constrain(x, 0, 319);
    ty = (uint16_t)constrain(y, 0, 239);
    return true;
}

void send_telemetry() {
    if (mqttClient.connected()) {
        char payload[128];
        int tds = (current_tds >= 0) ? current_tds : 0;
        snprintf(payload, sizeof(payload), "{\"values\":{\"tds_ppm\":%d,\"light_on\":%s,\"fw_version\":\"%s\"}}",
                 tds, current_light ? "true" : "false", FW_VERSION);
        mqttClient.publish(TDS_TOPIC, payload);
        Serial.printf("Telemetry sent: %s\n", payload);
    }
}

void set_light(bool on) {
    bool changed = (current_light != on);
    current_light = on;
    digitalWrite(PIN_RED, on ? LOW : HIGH);
    digitalWrite(PIN_GREEN, on ? LOW : HIGH);
    digitalWrite(PIN_BLUE, on ? LOW : HIGH);
    if (changed) {
        send_telemetry();
    }
}

static void draw_normal_screen() {
    tft.fillScreen(TFT_BLACK);
    tft.setTextColor(TFT_YELLOW, TFT_BLACK);
    char hdr[32];
    snprintf(hdr, sizeof(hdr), "ESP2 - Ver: %s", FW_VERSION);
    tft.drawString(hdr, 12, 20, 4);

    last_ble_shown = -1;
    last_tds_shown = -2;
    last_light_shown = -1;
    last_wifi_shown = -1;
}

static void update_screen() {
    if (ota_ui_state != UI_NORMAL) return;

    if (last_ble_shown != (int8_t)current_ble) {
        last_ble_shown = current_ble;
        tft.setTextColor(current_ble ? TFT_GREEN : TFT_RED, TFT_BLACK);
        tft.drawString(current_ble ? "BLE: CONNECTED          " : "BLE: SCANNING           ", 12, 65, 4);
    }
    if (last_tds_shown != current_tds) {
        last_tds_shown = current_tds;
        tft.setTextColor(TFT_CYAN, TFT_BLACK);
        char buf[32];
        if (current_tds >= 0) snprintf(buf, sizeof(buf), "TDS: %d ppm            ", current_tds);
        else snprintf(buf, sizeof(buf), "TDS: waiting BLE       ");
        tft.drawString(buf, 12, 105, 4);
    }
    if (last_light_shown != (int8_t)current_light) {
        last_light_shown = current_light;
        uint16_t bg = current_light ? tft.color565(0, 70, 20) : tft.color565(15, 15, 15);
        uint16_t stroke = current_light ? TFT_GREEN : tft.color565(40, 40, 40);
        uint16_t fg = current_light ? TFT_WHITE : tft.color565(120, 120, 120);

        // LED Button (Left)
        tft.fillRoundRect(12, 138, 142, 40, 6, bg);
        tft.drawRoundRect(12, 138, 142, 40, 6, stroke);
        tft.setTextColor(fg, bg);
        tft.drawString(current_light ? "LED1: ON" : "LED1: OFF", 32, 148, 4);

        // Check FW Button (Right)
        tft.fillRoundRect(166, 138, 142, 40, 6, tft.color565(15, 35, 90));
        tft.drawRoundRect(166, 138, 142, 40, 6, TFT_CYAN);
        tft.setTextColor(TFT_WHITE, tft.color565(15, 35, 90));
        tft.drawString("CHECK FW", 178, 148, 4);
    }
    if (last_wifi_shown != (int8_t)current_wifi) {
        last_wifi_shown = current_wifi;
        tft.setTextColor(current_wifi ? TFT_GREEN : TFT_RED, TFT_BLACK);
        tft.drawString(current_wifi ? "Wi-Fi: CONNECTED        " : "Wi-Fi: OFFLINE          ", 12, 190, 4);
    }
}

static void draw_ota_prompt() {
    tft.fillScreen(tft.color565(20, 20, 35));
    tft.drawRoundRect(10, 10, 300, 220, 8, TFT_CYAN);
    tft.setTextColor(TFT_YELLOW, tft.color565(20, 20, 35));
    tft.drawString("FIRMWARE UPDATE", 50, 25, 4);

    tft.setTextColor(TFT_WHITE, tft.color565(20, 20, 35));
    char buf[48];
    snprintf(buf, sizeof(buf), "New version: %s", pending_ota_ver);
    tft.drawString(buf, 25, 75, 4);
    tft.drawString("Update now?", 25, 115, 4);

    // YES button (Green)
    tft.fillRoundRect(25, 160, 120, 48, 6, tft.color565(0, 120, 40));
    tft.drawRoundRect(25, 160, 120, 48, 6, TFT_GREEN);
    tft.setTextColor(TFT_WHITE, tft.color565(0, 120, 40));
    tft.drawString("YES", 65, 172, 4);

    // NO button (Red)
    tft.fillRoundRect(175, 160, 120, 48, 6, tft.color565(140, 20, 20));
    tft.drawRoundRect(175, 160, 120, 48, 6, TFT_RED);
    tft.setTextColor(TFT_WHITE, tft.color565(140, 20, 20));
    tft.drawString("NO", 220, 172, 4);
}

static void draw_ota_progress(int percent, const char* status_text) {
    if (ota_ui_state != UI_PROGRESS) {
        ota_ui_state = UI_PROGRESS;
        tft.fillScreen(TFT_BLACK);
        tft.setTextColor(TFT_CYAN, TFT_BLACK);
        tft.drawString("OTA UPDATING...", 60, 40, 4);
    }

    tft.setTextColor(TFT_WHITE, TFT_BLACK);
    char buf[48];
    snprintf(buf, sizeof(buf), "%s: %d%%      ", status_text, percent);
    tft.drawString(buf, 40, 95, 4);

    // Progress bar frame & fill
    tft.drawRoundRect(30, 140, 260, 28, 4, TFT_WHITE);
    int fill_w = map(constrain(percent, 0, 100), 0, 100, 0, 256);
    if (fill_w > 0) {
        tft.fillRect(32, 142, fill_w, 24, TFT_GREEN);
    }
    if (fill_w < 256) {
        tft.fillRect(32 + fill_w, 142, 256 - fill_w, 24, TFT_BLACK);
    }
}

static bool perform_http_ota(const char* url) {
    ota_in_progress = true;
    draw_ota_progress(0, "Connecting");

    HTTPClient http;
    http.begin(url);
    int httpCode = http.GET();
    if (httpCode != HTTP_CODE_OK) {
        Serial.printf("OTA HTTP error: %d\n", httpCode);
        draw_ota_progress(0, "HTTP Error");
        http.end();
        ota_in_progress = false;
        delay(2000);
        ota_ui_state = UI_NORMAL;
        draw_normal_screen();
        return false;
    }

    int contentLength = http.getSize();
    Serial.printf("OTA Firmware size: %d bytes\n", contentLength);

    // Verify partition size
    const esp_partition_t *update_partition = esp_ota_get_next_update_partition(NULL);
    if (!update_partition) {
        Serial.println("OTA Error: No update partition found");
        draw_ota_progress(0, "No Partition");
        http.end();
        ota_in_progress = false;
        delay(2000);
        ota_ui_state = UI_NORMAL;
        draw_normal_screen();
        return false;
    }

    if (contentLength > (int)update_partition->size) {
        Serial.printf("OTA Error: Firmware size %d exceeds partition size %u\n",
                      contentLength, update_partition->size);
        draw_ota_progress(0, "Flash Insufficient");
        http.end();
        ota_in_progress = false;
        delay(2500);
        ota_ui_state = UI_NORMAL;
        draw_normal_screen();
        return false;
    }

    bool canBegin = Update.begin(contentLength > 0 ? contentLength : UPDATE_SIZE_UNKNOWN);
    if (!canBegin) {
        Serial.println("OTA Error: Update.begin failed");
        draw_ota_progress(0, "Begin Failed");
        http.end();
        ota_in_progress = false;
        delay(2000);
        ota_ui_state = UI_NORMAL;
        draw_normal_screen();
        return false;
    }

    WiFiClient *stream = http.getStreamPtr();
    size_t written = 0;
    uint8_t buff[1024];
    int lastPercent = -1;

    while (http.connected() && (written < (size_t)contentLength || contentLength <= 0)) {
        size_t available = stream->available();
        if (available) {
            size_t toRead = available > sizeof(buff) ? sizeof(buff) : available;
            int bytesRead = stream->readBytes(buff, toRead);
            if (bytesRead > 0) {
                Update.write(buff, bytesRead);
                written += bytesRead;
                if (contentLength > 0) {
                    int percent = (written * 100) / contentLength;
                    if (percent != lastPercent) {
                        lastPercent = percent;
                        draw_ota_progress(percent, "Downloading");
                    }
                }
            }
        }
        if (contentLength > 0 && written >= (size_t)contentLength) break;
        delay(1);
    }

    if (Update.end()) {
        if (Update.isFinished()) {
            Serial.println("OTA Update successfully completed. Rebooting...");
            draw_ota_progress(100, "Done! Rebooting");
            delay(1500);
            ESP.restart();
            return true;
        } else {
            Serial.println("OTA Error: Update not finished");
        }
    } else {
        Serial.printf("OTA Error #: %u\n", Update.getError());
    }

    draw_ota_progress(0, "Update Failed");
    http.end();
    ota_in_progress = false;
    delay(2000);
    ota_ui_state = UI_NORMAL;
    draw_normal_screen();
    return false;
}

static void trigger_ota(const char* url, const char* ver, bool force) {
    if (ota_in_progress) return;

    strncpy(pending_ota_url, url, sizeof(pending_ota_url) - 1);
    strncpy(pending_ota_ver, ver, sizeof(pending_ota_ver) - 1);

    if (force) {
        Serial.printf("Force OTA triggered for %s from %s\n", ver, url);
        perform_http_ota(pending_ota_url);
    } else {
        Serial.printf("OTA prompt triggered for %s from %s\n", ver, url);
        ota_ui_state = UI_PROMPT;
        ota_confirmed = false;
        ota_rejected = false;
        draw_ota_prompt();
    }
}

static void check_new_firmware() {
    if (ota_in_progress) return;
    Serial.println("Checking for new firmware from server...");

    HTTPClient http;
    http.begin("http://192.168.1.19:8080/firmware/version.json");
    http.setTimeout(3500);
    int code = http.GET();
    if (code == HTTP_CODE_OK) {
        String payload = http.getString();
        JsonDocument doc;
        DeserializationError err = deserializeJson(doc, payload);
        if (!err) {
            const char* ver = doc["version"] | "";
            const char* url = doc["url"] | "http://192.168.1.19:8080/firmware/esp2.bin";
            Serial.printf("Server FW version: %s (Current: %s)\n", ver, FW_VERSION);
            if (strlen(ver) > 0 && strcmp(ver, FW_VERSION) != 0) {
                trigger_ota(url, ver, false);
                http.end();
                return;
            } else {
                Serial.println("Already on latest version!");
                tft.setTextColor(TFT_GREEN, TFT_BLACK);
                tft.drawString("FW IS UP-TO-DATE!       ", 12, 190, 4);
                delay(1200);
                last_wifi_shown = -1;
            }
        }
    } else {
        Serial.printf("Check FW failed, HTTP code: %d\n", code);
    }
    http.end();
}

static void mqtt_callback(char* topic, byte* payload, unsigned int length) {
    if (strcmp(topic, CMD_TOPIC) != 0) return;
    JsonDocument doc;
    DeserializationError err = deserializeJson(doc, payload, length);
    if (err) return;

    const char* method = doc["method"];
    const char* id = doc["id"];

    if (method && strcmp(method, "set_light") == 0 && id) {
        bool state = doc["params"]["state"] | false;
        set_light(state);

        JsonDocument res;
        res["id"] = id;
        res["success"] = true;
        char out[64];
        serializeJson(res, out);
        mqttClient.publish(RESULT_TOPIC, out);
        Serial.printf("MQTT set_light -> %s, replied ID=%s\n", state ? "ON" : "OFF", id);
    } else if (method && (strcmp(method, "ota_update") == 0 || strcmp(method, "set_firmware") == 0)) {
        const char* url = doc["params"]["url"] | "http://192.168.1.19:8080/firmware/esp2.bin";
        const char* ver = doc["params"]["version"] | "v1.0.1";
        bool force = doc["params"]["force"] | false;

        JsonDocument res;
        if (id) res["id"] = id;
        res["status"] = "accepted";
        char out[64];
        serializeJson(res, out);
        mqttClient.publish(RESULT_TOPIC, out);

        trigger_ota(url, ver, force);
    }
}

static void on_ble_notify(NimBLERemoteCharacteristic* pChar, uint8_t* pData, size_t length, bool isNotify) {
    JsonDocument doc;
    DeserializationError err = deserializeJson(doc, pData, length);
    if (!err && doc["tds_ppm"].is<uint16_t>()) {
        current_tds = doc["tds_ppm"].as<uint16_t>();
        Serial.printf("BLE TDS received: %d ppm\n", current_tds);
        send_telemetry();
    }
}

class ClientCallbacks : public NimBLEClientCallbacks {
    void onConnect(NimBLEClient* pClient) override {
        current_ble = true;
        Serial.println("ESP2 BLE connected to ESP3");
    }
    void onDisconnect(NimBLEClient* pClient) override {
        current_ble = false;
        pRemoteChar = nullptr;
        Serial.println("ESP2 BLE disconnected from ESP3");
    }
};

static void setup_webserver() {
    server.on("/", HTTP_GET, []() {
        char tds_str[16];
        if (current_tds >= 0) snprintf(tds_str, sizeof(tds_str), "%d", current_tds);
        else snprintf(tds_str, sizeof(tds_str), "waiting");

        char body[750];
        snprintf(body, sizeof(body),
            "<html><meta name='viewport' content='width=device-width'><h1>ESP2 (Ver %s)</h1>"
            "<p>TDS: %s ppm</p><p>Light: %s</p><p>BLE: %s</p>"
            "<form method='post' action='/light?state=%s' style='display:inline;'><button>Toggle light</button></form> "
            "<form method='post' action='/check_fw' style='display:inline;'><button style='background:#2563eb;color:#fff;padding:6px 14px;'>Check for New FW</button></form>"
            "<hr><h3>Local OTA Firmware Upload</h3>"
            "<form method='post' action='/ota' enctype='multipart/form-data'>"
            "<input type='file' name='update'><button>Upload</button></form></html>",
            FW_VERSION, tds_str, current_light ? "true" : "false", current_ble ? "connected" : "scanning",
            current_light ? "false" : "true"
        );
        server.send(200, "text/html", body);
    });

    server.on("/check_fw", HTTP_POST, []() {
        server.sendHeader("Location", "/");
        server.send(303, "text/plain", "Checking...");
        check_new_firmware();
    });

    server.on("/light", HTTP_POST, []() {
        if (server.hasArg("state")) {
            set_light(server.arg("state") == "true");
        }
        server.send(200, "text/plain", "OK");
    });

    server.on("/ota", HTTP_POST, []() {
        server.sendHeader("Connection", "close");
        server.send(200, "text/plain", Update.hasError() ? "FAIL" : "restarting");
        delay(500);
        ESP.restart();
    }, []() {
        HTTPUpload& upload = server.upload();
        if (upload.status == UPLOAD_FILE_START) {
            Serial.println("OTA starting");
            if (!Update.begin(UPDATE_SIZE_UNKNOWN)) {
                Update.printError(Serial);
            }
        } else if (upload.status == UPLOAD_FILE_WRITE) {
            if (Update.write(upload.buf, upload.currentSize) != upload.currentSize) {
                Update.printError(Serial);
            }
        } else if (upload.status == UPLOAD_FILE_END) {
            if (Update.end(true)) {
                Serial.println("OTA Success");
            } else {
                Update.printError(Serial);
            }
        }
    });

    server.begin();
}

static void ble_task(void *pvParameters) {
    while (true) {
        if (!current_ble) {
            NimBLEScan *scan = NimBLEDevice::getScan();
            NimBLEScanResults results = scan->start(2, false);
            for (int i = 0; i < results.getCount(); ++i) {
                NimBLEAdvertisedDevice dev = results.getDevice(i);
                if (dev.haveName() && dev.getName().find("ESP3-TDS") != std::string::npos) {
                    if (!bleClient) {
                        bleClient = NimBLEDevice::createClient();
                        bleClient->setClientCallbacks(new ClientCallbacks());
                    }
                    if (bleClient->connect(&dev)) {
                        NimBLERemoteService* pSvc = bleClient->getService(SERVICE_UUID);
                        if (pSvc) {
                            pRemoteChar = pSvc->getCharacteristic(TX_UUID);
                            if (pRemoteChar && pRemoteChar->canNotify()) {
                                pRemoteChar->subscribe(true, on_ble_notify);
                                current_ble = true;
                                Serial.println("Subscribed to ESP3 TDS notify characteristic");
                                break;
                            }
                        }
                        bleClient->disconnect();
                    }
                }
            }
            vTaskDelay(pdMS_TO_TICKS(2000));
        } else {
            vTaskDelay(pdMS_TO_TICKS(1000));
        }
    }
}

void setup() {
    Serial.begin(115200);

    pinMode(PIN_RED, OUTPUT);
    pinMode(PIN_GREEN, OUTPUT);
    pinMode(PIN_BLUE, OUTPUT);
    set_light(false);

    pinMode(21, OUTPUT);
    digitalWrite(21, HIGH);

    tft.init();
    tft.setRotation(1);

    touchSPI.begin(XPT2046_CLK, XPT2046_MISO, XPT2046_MOSI, XPT2046_CS);
    touch.begin(touchSPI);
    touch.setRotation(1);

    draw_normal_screen();
    update_screen();

    WiFi.mode(WIFI_STA);
    WiFi.setAutoReconnect(true);
    WiFi.begin("Fuvitech", "fuvitech.vn");
    Serial.print("Connecting to Wi-Fi");
    uint32_t t0 = millis();
    while (WiFi.status() != WL_CONNECTED && millis() - t0 < 8000) {
        delay(250);
        Serial.print(".");
    }
    Serial.println();
    if (WiFi.status() == WL_CONNECTED) {
        current_wifi = true;
        Serial.printf("Wi-Fi connected, IP: %s\n", WiFi.localIP().toString().c_str());
    } else {
        Serial.println("Wi-Fi not ready yet; will auto-reconnect in loop");
    }

    const char* broker = "192.168.1.19";
    mqttClient.setServer(broker, 1883);
    mqttClient.setCallback(mqtt_callback);

    NimBLEDevice::init("");
    NimBLEScan *pScan = NimBLEDevice::getScan();
    pScan->setActiveScan(true);
    pScan->setInterval(100);
    pScan->setWindow(99);

    xTaskCreatePinnedToCore(ble_task, "ble_task", 4096, NULL, 1, NULL, 0);

    setup_webserver();
    Serial.println("ESP2 ready; Webserver :8080");
}

void loop() {
    server.handleClient();

    // Touch handle
    uint16_t tx = 0, ty = 0;
    bool touched = get_touch_point(tx, ty);

    if (touched) {
        if (!touch_down) {
            touch_down = true;

            if (ota_ui_state == UI_PROMPT) {
                // Check YES button: 25 <= tx <= 145, 160 <= ty <= 208
                if (tx >= 25 && tx <= 145 && ty >= 150 && ty <= 215) {
                    Serial.println("OTA confirmed by touch (YES)");
                    ota_confirmed = true;
                }
                // Check NO button: 175 <= tx <= 295, 160 <= ty <= 208
                else if (tx >= 175 && tx <= 295 && ty >= 150 && ty <= 215) {
                    Serial.println("OTA rejected by touch (NO)");
                    ota_rejected = true;
                }
            } else if (ota_ui_state == UI_NORMAL) {
                // LED toggle button (Left half: x 10..156, y 130..185)
                if (tx >= 10 && tx <= 156 && ty >= 130 && ty <= 185) {
                    set_light(!current_light);
                    Serial.printf("Touch pressed -> Light is %s\n", current_light ? "ON" : "OFF");
                }
                // CHECK FW button (Right half: x 164..310, y 130..185)
                else if (tx >= 164 && tx <= 310 && ty >= 130 && ty <= 185) {
                    Serial.println("Touch pressed -> CHECK FW button triggered");
                    check_new_firmware();
                }
            }
        }
    } else {
        touch_down = false;
    }

    // Handle OTA prompt actions
    if (ota_ui_state == UI_PROMPT) {
        if (ota_confirmed) {
            ota_confirmed = false;
            perform_http_ota(pending_ota_url);
        } else if (ota_rejected) {
            ota_rejected = false;
            ota_ui_state = UI_NORMAL;
            draw_normal_screen();
        }
    }

    // Wi-Fi & MQTT loop
    current_wifi = (WiFi.status() == WL_CONNECTED);
    if (!current_wifi) {
        static uint32_t last_wifi_try = 0;
        if (millis() - last_wifi_try > 8000) {
            last_wifi_try = millis();
            Serial.println("Reconnecting Wi-Fi...");
            WiFi.disconnect();
            WiFi.begin("Fuvitech", "fuvitech.vn");
        }
    } else {
        if (!mqttClient.connected()) {
            static uint32_t last_reconnect = 0;
            if (millis() - last_reconnect > 3000) {
                last_reconnect = millis();
                if (mqttClient.connect("esp2")) {
                    mqttClient.subscribe(CMD_TOPIC);
                    Serial.println("MQTT connected & subscribed");
                    send_telemetry();
                }
            }
        } else {
            mqttClient.loop();
        }
    }

    update_screen();
    delay(20);
}

