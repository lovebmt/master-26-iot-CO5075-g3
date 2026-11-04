#include <Arduino.h>
#include <WiFi.h>
#include <PubSubClient.h>
#include <Preferences.h>
#include <NimBLEDevice.h>
#include <TFT_eSPI.h>
#include <SPI.h>
#include <XPT2046_Touchscreen.h>
#include <ArduinoJson.h>

#define SERVICE_UUID "6e400001-b5a3-f393-e0a9-e50e24dcca9e"
#define TX_UUID      "6e400003-b5a3-f393-e0a9-e50e24dcca9e"
#define RX_UUID      "6e400002-b5a3-f393-e0a9-e50e24dcca9e"

#define MQTT_TOPIC_ESP2 "iot/esp2/telemetry"
#define MQTT_BROKER     "192.168.1.19"
#define MQTT_PORT       1883

// Touch SPI pins on CYD
#define XPT2046_IRQ  36
#define XPT2046_MOSI 32
#define XPT2046_MISO 39
#define XPT2046_CLK  25
#define XPT2046_CS   33

static TFT_eSPI tft = TFT_eSPI();
static SPIClass touchSPI(VSPI);
static XPT2046_Touchscreen touch(XPT2046_CS, XPT2046_IRQ);
static Preferences prefs;

static WiFiClient espClient;
static PubSubClient mqttClient(espClient);

static NimBLEServer *server = nullptr;
static NimBLECharacteristic *tx_char = nullptr;
static volatile bool ble_connected = false;

enum CommMode {
    MODE_BLE_AUTO = 0,
    MODE_MQTT_ONLY = 1,
    MODE_BLE_ONLY = 2
};

static CommMode current_mode = MODE_BLE_AUTO;
static bool active_fallback_mqtt = false; // active when auto-fallback after 60s
static uint32_t ble_unconnected_start = 0;

static char wifi_ssid[33] = "Fuvitech";
static char wifi_pass[65] = "fuvitech.vn";

// UI states
enum ScreenState {
    SCREEN_DASHBOARD = 0,
    SCREEN_KEYBOARD = 1
};
static ScreenState current_screen = SCREEN_DASHBOARD;
static int editing_field = 0; // 0: SSID, 1: PASS
static char edit_buffer[65] = "";
static bool shift_active = false;

static uint16_t current_tds = 500;
static uint32_t last_sensor_tick = 0;
static bool needs_ui_redraw = true;

// Calibration for XPT2046 on 320x240 landscape (Rotation 1)
static bool get_touch_point(uint16_t &tx, uint16_t &ty) {
    if (!touch.touched()) return false;
    TS_Point p = touch.getPoint();
    if (p.z < 200) return false;

    // Convert raw ADC (typically 200-3800) to 320x240
    int32_t x = map(p.x, 200, 3800, 0, 320);
    int32_t y = map(p.y, 240, 3800, 0, 240);
    if (x < 0) x = 0; if (x > 319) x = 319;
    if (y < 0) y = 0; if (y > 239) y = 239;
    tx = (uint16_t)x;
    ty = (uint16_t)y;
    return true;
}

class ServerCallbacks : public NimBLEServerCallbacks {
    void onConnect(NimBLEServer *s) override {
        ble_connected = true;
        ble_unconnected_start = 0;
        active_fallback_mqtt = false;
        needs_ui_redraw = true;
        Serial.println("ESP3 BLE connected to central");
    }
    void onDisconnect(NimBLEServer *s) override {
        ble_connected = false;
        ble_unconnected_start = millis();
        needs_ui_redraw = true;
        NimBLEDevice::startAdvertising();
        Serial.println("ESP3 BLE disconnected, advertising restarted");
    }
};

static void send_mqtt_telemetry(uint16_t tds) {
    if (WiFi.status() != WL_CONNECTED) {
        WiFi.begin(wifi_ssid, wifi_pass);
        uint32_t t0 = millis();
        while (WiFi.status() != WL_CONNECTED && millis() - t0 < 4000) {
            delay(100);
        }
    }
    if (WiFi.status() == WL_CONNECTED) {
        if (!mqttClient.connected()) {
            mqttClient.setServer(MQTT_BROKER, MQTT_PORT);
            mqttClient.connect("esp3-client");
        }
        if (mqttClient.connected()) {
            char payload[64];
            snprintf(payload, sizeof(payload), "{\"values\":{\"tds_ppm\":%u}}", tds);
            mqttClient.publish(MQTT_TOPIC_ESP2, payload);
            Serial.printf("ESP3 sent via MQTT to %s: %s\n", MQTT_TOPIC_ESP2, payload);
        }
    }
}

static void send_tds_data(uint16_t tds) {
    char payload[32];
    snprintf(payload, sizeof(payload), "{\"tds_ppm\":%u}", tds);

    bool should_mqtt = false;
    if (current_mode == MODE_MQTT_ONLY) {
        should_mqtt = true;
    } else if (current_mode == MODE_BLE_ONLY) {
        if (ble_connected && tx_char) {
            tx_char->setValue((uint8_t*)payload, strlen(payload));
            tx_char->notify();
        }
    } else { // MODE_BLE_AUTO
        if (ble_connected && tx_char) {
            tx_char->setValue((uint8_t*)payload, strlen(payload));
            tx_char->notify();
        } else if (active_fallback_mqtt) {
            should_mqtt = true;
        }
    }

    if (should_mqtt) {
        send_mqtt_telemetry(tds);
    }
}

// ---------------- UI & Keyboard ----------------

static void draw_dashboard() {
    tft.fillScreen(TFT_BLACK);

    // Title
    tft.setTextColor(TFT_YELLOW, TFT_BLACK);
    tft.drawString("ESP3 / TDS Sensor Node", 12, 10, 4);

    // TDS Reading
    tft.setTextColor(TFT_CYAN, TFT_BLACK);
    char buf[32];
    snprintf(buf, sizeof(buf), "TDS: %u ppm", current_tds);
    tft.drawString(buf, 12, 45, 4);

    // Mode Button: [AUTO (60s)] / [MQTT ONLY] / [BLE ONLY]
    tft.fillRoundRect(12, 85, 140, 36, 6, tft.color565(30, 40, 70));
    tft.drawRoundRect(12, 85, 140, 36, 6, TFT_WHITE);
    tft.setTextColor(TFT_WHITE, tft.color565(30, 40, 70));
    const char* mode_str = (current_mode == MODE_BLE_AUTO) ? "Mode: AUTO" :
                           (current_mode == MODE_MQTT_ONLY) ? "Mode: MQTT" : "Mode: BLE";
    tft.drawString(mode_str, 20, 95, 2);

    // Connection Status
    tft.setTextColor(TFT_GREEN, TFT_BLACK);
    if (current_mode == MODE_BLE_AUTO) {
        if (ble_connected) {
            tft.drawString("Status: BLE Connected", 160, 95, 2);
        } else if (active_fallback_mqtt) {
            tft.drawString("Status: MQTT Fallback", 160, 95, 2);
        } else {
            uint32_t remain = (millis() - ble_unconnected_start) / 1000;
            char s[32];
            snprintf(s, sizeof(s), "BLE Adv (%lus/60s)", remain < 60 ? (60 - remain) : 0);
            tft.drawString(s, 160, 95, 2);
        }
    } else if (current_mode == MODE_MQTT_ONLY) {
        bool w = (WiFi.status() == WL_CONNECTED);
        tft.drawString(w ? "Status: WiFi/MQTT OK" : "Status: WiFi Conn...", 160, 95, 2);
    } else {
        tft.drawString(ble_connected ? "Status: BLE Connected" : "Status: BLE Adv...", 160, 95, 2);
    }

    // Config Wi-Fi Box
    tft.drawRect(12, 135, 296, 95, tft.color565(60, 60, 60));
    tft.setTextColor(TFT_LIGHTGREY, TFT_BLACK);
    tft.drawString("Wi-Fi Settings (Tap to Config)", 20, 142, 2);

    // SSID button
    tft.fillRoundRect(20, 165, 135, 30, 4, tft.color565(40, 40, 40));
    tft.setTextColor(TFT_WHITE, tft.color565(40, 40, 40));
    char ssid_disp[20];
    snprintf(ssid_disp, sizeof(ssid_disp), "SSID: %.8s", wifi_ssid);
    tft.drawString(ssid_disp, 26, 172, 2);

    // PASS button
    tft.fillRoundRect(165, 165, 135, 30, 4, tft.color565(40, 40, 40));
    tft.drawString("PASS: ********", 171, 172, 2);

    // Fallback notice
    tft.setTextColor(tft.color565(120, 120, 120), TFT_BLACK);
    tft.drawString("Default: Fuvitech / fuvitech.vn", 20, 205, 1);
}

// Full onscreen keyboard layout
static const char *kb_rows[] = {
    "1234567890-=",
    "qwertyuiop",
    "asdfghjkl",
    "zxcvbnm._"
};
static const char *kb_rows_shift[] = {
    "!@#$%^&*()_+",
    "QWERTYUIOP",
    "ASDFGHJKL",
    "ZXCVBNM/?"
};

static void draw_keyboard() {
    tft.fillScreen(TFT_BLACK);
    tft.setTextColor(TFT_YELLOW, TFT_BLACK);
    tft.drawString(editing_field == 0 ? "Edit Wi-Fi SSID:" : "Edit Wi-Fi Password:", 10, 10, 2);

    // Text input display box
    tft.fillRect(10, 30, 300, 30, tft.color565(20, 20, 50));
    tft.drawRect(10, 30, 300, 30, TFT_WHITE);
    tft.setTextColor(TFT_WHITE, tft.color565(20, 20, 50));
    tft.drawString(edit_buffer, 15, 37, 2);

    // Keys
    int start_y = 68;
    int key_h = 28;

    for (int r = 0; r < 4; r++) {
        const char *row = shift_active ? kb_rows_shift[r] : kb_rows[r];
        int len = strlen(row);
        int key_w = 300 / len;
        for (int c = 0; c < len; c++) {
            int kx = 10 + c * key_w;
            int ky = start_y + r * (key_h + 4);
            tft.fillRoundRect(kx, ky, key_w - 2, key_h, 3, tft.color565(50, 50, 50));
            tft.drawRoundRect(kx, ky, key_w - 2, key_h, 3, TFT_DARKGREY);
            tft.setTextColor(TFT_WHITE, tft.color565(50, 50, 50));
            char s[2] = {row[c], 0};
            tft.drawString(s, kx + key_w / 2 - 4, ky + 6, 2);
        }
    }

    // Bottom Action Row: [SHIFT] [SPACE] [BACKSPACE] [SAVE & EXIT]
    int by = start_y + 4 * (key_h + 4);
    // SHIFT
    tft.fillRoundRect(10, by, 50, 30, 4, shift_active ? TFT_BLUE : tft.color565(60, 60, 60));
    tft.setTextColor(TFT_WHITE);
    tft.drawString("SHIFT", 15, by + 7, 2);

    // SPACE
    tft.fillRoundRect(65, by, 70, 30, 4, tft.color565(60, 60, 60));
    tft.drawString("SPACE", 78, by + 7, 2);

    // BKSP
    tft.fillRoundRect(140, by, 60, 30, 4, tft.color565(120, 30, 30));
    tft.drawString("DEL", 155, by + 7, 2);

    // OK / SAVE
    tft.fillRoundRect(205, by, 105, 30, 4, tft.color565(0, 120, 40));
    tft.drawString("SAVE/EXIT", 215, by + 7, 2);
}

static void handle_keyboard_touch(uint16_t tx, uint16_t ty) {
    int start_y = 68;
    int key_h = 28;

    // Check rows 0-3
    for (int r = 0; r < 4; r++) {
        int ky = start_y + r * (key_h + 4);
        if (ty >= ky && ty <= ky + key_h) {
            const char *row = shift_active ? kb_rows_shift[r] : kb_rows[r];
            int len = strlen(row);
            int key_w = 300 / len;
            int c = (tx - 10) / key_w;
            if (c >= 0 && c < len) {
                size_t blen = strlen(edit_buffer);
                if (blen < sizeof(edit_buffer) - 1) {
                    edit_buffer[blen] = row[c];
                    edit_buffer[blen + 1] = '\0';
                    draw_keyboard();
                }
                return;
            }
        }
    }

    // Bottom Action Row
    int by = start_y + 4 * (key_h + 4);
    if (ty >= by && ty <= by + 32) {
        if (tx >= 10 && tx < 60) { // SHIFT
            shift_active = !shift_active;
            draw_keyboard();
        } else if (tx >= 65 && tx < 135) { // SPACE
            size_t blen = strlen(edit_buffer);
            if (blen < sizeof(edit_buffer) - 1) {
                edit_buffer[blen] = ' ';
                edit_buffer[blen + 1] = '\0';
                draw_keyboard();
            }
        } else if (tx >= 140 && tx < 200) { // BKSP
            size_t blen = strlen(edit_buffer);
            if (blen > 0) {
                edit_buffer[blen - 1] = '\0';
                draw_keyboard();
            }
        } else if (tx >= 205 && tx <= 310) { // SAVE & EXIT
            if (editing_field == 0) {
                strncpy(wifi_ssid, edit_buffer, sizeof(wifi_ssid) - 1);
                prefs.putString("ssid", wifi_ssid);
            } else {
                strncpy(wifi_pass, edit_buffer, sizeof(wifi_pass) - 1);
                prefs.putString("pass", wifi_pass);
            }
            WiFi.disconnect();
            current_screen = SCREEN_DASHBOARD;
            needs_ui_redraw = true;
        }
    }
}

static void handle_dashboard_touch(uint16_t tx, uint16_t ty) {
    // Mode button: (12, 85, 140, 36)
    if (tx >= 12 && tx <= 152 && ty >= 85 && ty <= 121) {
        current_mode = (CommMode)((current_mode + 1) % 3);
        if (current_mode == MODE_BLE_AUTO) {
            ble_unconnected_start = millis();
            active_fallback_mqtt = false;
        }
        needs_ui_redraw = true;
        return;
    }

    // SSID button: (20, 165, 135, 30)
    if (tx >= 20 && tx <= 155 && ty >= 165 && ty <= 195) {
        editing_field = 0;
        strncpy(edit_buffer, wifi_ssid, sizeof(edit_buffer) - 1);
        current_screen = SCREEN_KEYBOARD;
        draw_keyboard();
        return;
    }

    // PASS button: (165, 165, 135, 30)
    if (tx >= 165 && tx <= 300 && ty >= 165 && ty <= 195) {
        editing_field = 1;
        strncpy(edit_buffer, wifi_pass, sizeof(edit_buffer) - 1);
        current_screen = SCREEN_KEYBOARD;
        draw_keyboard();
        return;
    }
}

void setup() {
    Serial.begin(115200);

    // Display backlight
    pinMode(21, OUTPUT);
    digitalWrite(21, HIGH);

    tft.init();
    tft.setRotation(1);
    tft.fillScreen(TFT_BLACK);

    touchSPI.begin(XPT2046_CLK, XPT2046_MISO, XPT2046_MOSI, XPT2046_CS);
    touch.begin(touchSPI);
    touch.setRotation(1);

    prefs.begin("esp3_cfg", false);
    String s = prefs.getString("ssid", "Fuvitech");
    String p = prefs.getString("pass", "fuvitech.vn");
    strncpy(wifi_ssid, s.c_str(), sizeof(wifi_ssid) - 1);
    strncpy(wifi_pass, p.c_str(), sizeof(wifi_pass) - 1);

    NimBLEDevice::init("ESP3-TDS");
    server = NimBLEDevice::createServer();
    server->setCallbacks(new ServerCallbacks());

    NimBLEService *service = server->createService(SERVICE_UUID);
    tx_char = service->createCharacteristic(TX_UUID, NIMBLE_PROPERTY::NOTIFY);
    service->createCharacteristic(RX_UUID, NIMBLE_PROPERTY::WRITE);
    service->start();

    NimBLEAdvertising *adv = NimBLEDevice::getAdvertising();
    adv->addServiceUUID(SERVICE_UUID);
    adv->setName("ESP3-TDS");
    adv->start();

    ble_unconnected_start = millis();
    draw_dashboard();
    Serial.println("ESP3 started with BLE + Touch + WiFi failover");
}

static bool was_touched = false;

void loop() {
    uint32_t now = millis();

    // 1. Touch handling
    uint16_t tx = 0, ty = 0;
    bool touched = get_touch_point(tx, ty);
    if (touched) {
        if (!was_touched) {
            was_touched = true;
            if (current_screen == SCREEN_DASHBOARD) {
                handle_dashboard_touch(tx, ty);
            } else if (current_screen == SCREEN_KEYBOARD) {
                handle_keyboard_touch(tx, ty);
            }
        }
    } else {
        was_touched = false;
    }

    // 2. 60-second BLE Timeout check in AUTO mode
    if (current_mode == MODE_BLE_AUTO && !ble_connected) {
        if (ble_unconnected_start > 0 && (now - ble_unconnected_start >= 60000)) {
            if (!active_fallback_mqtt) {
                active_fallback_mqtt = true;
                needs_ui_redraw = true;
                Serial.println("ESP3 BLE 60s timeout -> switched to WiFi/MQTT fallback!");
            }
        }
    }

    // 3. Sensor sampling every 5 seconds (500-570 ppm)
    if (now - last_sensor_tick >= 5000) {
        last_sensor_tick = now;
        uint32_t step = (now / 5000) % 8;
        current_tds = 500 + step * 10;
        send_tds_data(current_tds);
        if (current_screen == SCREEN_DASHBOARD) {
            needs_ui_redraw = true;
        }
    }

    // 4. Redraw dashboard if needed
    if (current_screen == SCREEN_DASHBOARD && needs_ui_redraw) {
        needs_ui_redraw = false;
        draw_dashboard();
    }

    delay(20);
}
