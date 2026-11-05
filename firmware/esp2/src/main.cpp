#include <Arduino.h>
#include <WiFi.h>
#include <WebServer.h>
#include <Update.h>
#include <PubSubClient.h>
#include <NimBLEDevice.h>
#include <TFT_eSPI.h>
#include <SPI.h>
#include <XPT2046_Touchscreen.h>
#include <ArduinoJson.h>

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

void send_telemetry() {
    if (mqttClient.connected()) {
        char payload[96];
        int tds = (current_tds >= 0) ? current_tds : 0;
        snprintf(payload, sizeof(payload), "{\"values\":{\"tds_ppm\":%d,\"light_on\":%s}}",
                 tds, current_light ? "true" : "false");
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

static void update_screen() {
    if (last_ble_shown != (int8_t)current_ble) {
        last_ble_shown = current_ble;
        tft.setTextColor(current_ble ? TFT_GREEN : TFT_RED, TFT_BLACK);
        tft.drawString(current_ble ? "BLE: CONNECTED          " : "BLE: SCANNING           ", 12, 70, 4);
    }
    if (last_tds_shown != current_tds) {
        last_tds_shown = current_tds;
        tft.setTextColor(TFT_CYAN, TFT_BLACK);
        char buf[32];
        if (current_tds >= 0) snprintf(buf, sizeof(buf), "TDS: %d ppm            ", current_tds);
        else snprintf(buf, sizeof(buf), "TDS: waiting BLE       ");
        tft.drawString(buf, 12, 110, 4);
    }
    if (last_light_shown != (int8_t)current_light) {
        last_light_shown = current_light;
        uint16_t bg = current_light ? tft.color565(0, 70, 20) : tft.color565(15, 15, 15);
        uint16_t stroke = current_light ? TFT_GREEN : tft.color565(40, 40, 40);
        uint16_t fg = current_light ? TFT_WHITE : tft.color565(120, 120, 120);

        tft.fillRoundRect(12, 138, 296, 38, 6, bg);
        tft.drawRoundRect(12, 138, 296, 38, 6, stroke);
        tft.setTextColor(fg, bg);
        tft.drawString(current_light ? "[TOUCH] LED1: ON  (TAP)" : "[TOUCH] LED1: OFF (TAP)", 24, 148, 4);
    }
    if (last_wifi_shown != (int8_t)current_wifi) {
        last_wifi_shown = current_wifi;
        tft.setTextColor(current_wifi ? TFT_GREEN : TFT_RED, TFT_BLACK);
        tft.drawString(current_wifi ? "Wi-Fi: CONNECTED        " : "Wi-Fi: OFFLINE          ", 12, 190, 4);
    }
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

        char body[512];
        snprintf(body, sizeof(body),
            "<html><meta name='viewport' content='width=device-width'><h1>ESP2</h1>"
            "<p>TDS: %s ppm</p><p>Light: %s</p><p>BLE: %s</p>"
            "<form method='post' action='/light?state=%s'><button>Toggle light</button></form></html>",
            tds_str, current_light ? "true" : "false", current_ble ? "connected" : "scanning",
            current_light ? "false" : "true"
        );
        server.send(200, "text/html", body);
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
    tft.fillScreen(TFT_BLACK);
    tft.setTextColor(TFT_YELLOW, TFT_BLACK);
    tft.drawString("ESP2 / CYD", 12, 30, 4);

    touchSPI.begin(XPT2046_CLK, XPT2046_MISO, XPT2046_MOSI, XPT2046_CS);
    touch.begin(touchSPI);
    touch.setRotation(1);

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
    bool touched = (!digitalRead(XPT2046_IRQ)) || touch.touched();
    if (touched) {
        if (!touch_down) {
            touch_down = true;
            set_light(!current_light);
            Serial.printf("Touch pressed -> Light is %s\n", current_light ? "ON" : "OFF");
        }
    } else {
        touch_down = false;
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
