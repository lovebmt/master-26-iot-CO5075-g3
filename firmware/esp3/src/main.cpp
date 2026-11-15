#include "soc/rtc_cntl_reg.h"
#include "soc/soc.h"
#include <Arduino.h>
#include <ArduinoJson.h>
#include <ModbusMaster.h>
#include <NimBLEDevice.h>
#include <Preferences.h>
#include <PubSubClient.h>
#include <SPI.h>
#include <TFT_eSPI.h>
#include <WiFi.h>
#include <XPT2046_Touchscreen.h>

// ==========================================
// BLE NUS Configuration
// ==========================================
#define SERVICE_UUID "6e400001-b5a3-f393-e0a9-e50e24dcca9e"
#define TX_UUID "6e400003-b5a3-f393-e0a9-e50e24dcca9e"
#define RX_UUID "6e400002-b5a3-f393-e0a9-e50e24dcca9e"

// ==========================================
// WiFi & MQTT Fallback Configuration
// ==========================================
#define MQTT_TOPIC_ESP3 "iot/esp3/telemetry"
#define MQTT_BROKER "192.168.1.19"
#define MQTT_PORT 1883

static char wifi_ssid[33] = "Fuvitech";
static char wifi_pass[65] = "fuvitech.vn";

// ==========================================
// RS485 Modbus RTU Pinout (XY-K485 + PTA9B01)
// ==========================================
#define RS485_RX_PIN 16 // XY-K485 RXD -> GPIO16
#define RS485_TX_PIN 17 // XY-K485 TXD -> GPIO17
#define RS485_BAUD 9600
#define RS485_SLAVE_ID 1

static ModbusMaster modbusNode;
static bool rs485_hardware_connected = false;

static Preferences prefs;
static WiFiClient espClient;
static PubSubClient mqttClient(espClient);

static NimBLEServer *server = nullptr;
static NimBLECharacteristic *tx_char = nullptr;
static volatile bool ble_connected = false;

enum CommMode { MODE_BLE_AUTO = 0, MODE_MQTT_ONLY = 1, MODE_BLE_ONLY = 2 };

static CommMode current_mode = MODE_BLE_AUTO;
static bool active_fallback_mqtt = false;
static uint32_t ble_unconnected_start = 0;

static float current_soil_temp = 25.0f;
static uint16_t current_tds = 530;
static uint32_t last_sensor_tick = 0;

// BLE Server Callbacks
class ServerCallbacks : public NimBLEServerCallbacks {
  void onConnect(NimBLEServer *s) override {
    ble_connected = true;
    ble_unconnected_start = 0;
    active_fallback_mqtt = false;
    Serial.println("[BLE] Central connected to ESP3!");
  }
  void onDisconnect(NimBLEServer *s) override {
    ble_connected = false;
    ble_unconnected_start = millis();
    NimBLEDevice::startAdvertising();
    Serial.println("[BLE] Disconnected, advertising restarted");
  }
};

static void send_mqtt_telemetry(float temp, uint16_t tds) {
  if (WiFi.status() != WL_CONNECTED) {
    Serial.printf("[WIFI] Connecting to %s...\n", wifi_ssid);
    WiFi.begin(wifi_ssid, wifi_pass);
    uint32_t t0 = millis();
    while (WiFi.status() != WL_CONNECTED && millis() - t0 < 5000) {
      delay(100);
      Serial.print(".");
    }
    Serial.println();
  }

  if (WiFi.status() == WL_CONNECTED) {
    if (!mqttClient.connected()) {
      mqttClient.setServer(MQTT_BROKER, MQTT_PORT);
      mqttClient.connect("esp3-client");
    }
    if (mqttClient.connected()) {
      char payload[96];
      snprintf(payload, sizeof(payload),
               "{\"values\":{\"soil_temp\":%.1f,\"tds_ppm\":%u}}", temp, tds);
      mqttClient.publish(MQTT_TOPIC_ESP3, payload);
      Serial.printf("[MQTT FALLBACK] Sent to %s: %s\n", MQTT_TOPIC_ESP3,
                    payload);
    } else {
      Serial.println("[MQTT] Connect failed");
    }
  } else {
    Serial.println("[WIFI] Connection failed");
  }
}

// Read real PT100 sensor via RS485 Modbus RTU (Holding Registers 0x0000, 2
// registers)
static bool read_rs485_pt100(float &temperature, float &resistance) {
  // Clear buffer before transaction
  modbusNode.clearResponseBuffer();

  // Read 2 holding registers starting at 0x0000
  // [0] = Temperature in 0.1 deg C (signed int16)
  // [1] = Resistance in 0.1 Ohm (uint16)
  uint8_t result = modbusNode.readHoldingRegisters(0x0000, 2);

  if (result == modbusNode.ku8MBSuccess) {
    int16_t rawTemp = (int16_t)modbusNode.getResponseBuffer(0);
    uint16_t rawRes = modbusNode.getResponseBuffer(1);

    temperature = rawTemp / 10.0f;
    resistance = rawRes / 10.0f;

    Serial.printf("[MODBUS SUCCESS] PT100 Temp: %.1f C | Res: %.1f Ohm\n",
                  temperature, resistance);
    return true;
  } else {
    Serial.printf("[MODBUS ERROR 0x%02X]: ", result);
    switch (result) {
    case modbusNode.ku8MBResponseTimedOut:
      Serial.println("Timeout - Module PTA9B01 khong tra loi (kiem tra "
                     "nguon 12V, day A/B, GND chung)");
      break;
    case modbusNode.ku8MBInvalidCRC:
      Serial.println("Sai CRC - nhieu hoac sai baudrate");
      break;
    case modbusNode.ku8MBInvalidSlaveID:
      Serial.println("Sai Slave ID");
      break;
    default:
      Serial.println("Loi Modbus khac");
      break;
    }
    return false;
  }
}

static void send_sensor_data(float temp, uint16_t tds) {
  char payload[64];
  snprintf(payload, sizeof(payload), "{\"soil_temp\":%.1f,\"tds_ppm\":%u}",
           temp, tds);

  bool should_mqtt = false;
  if (current_mode == MODE_MQTT_ONLY) {
    should_mqtt = true;
  } else if (current_mode == MODE_BLE_ONLY) {
    if (ble_connected && tx_char) {
      tx_char->setValue((uint8_t *)payload, strlen(payload));
      tx_char->notify();
      Serial.printf("[BLE SENT] %s\n", payload);
    }
  } else { // MODE_BLE_AUTO
    if (ble_connected && tx_char) {
      tx_char->setValue((uint8_t *)payload, strlen(payload));
      tx_char->notify();
      Serial.printf("[BLE SENT] %s\n", payload);
    } else if (active_fallback_mqtt) {
      should_mqtt = true;
    }
  }

  if (should_mqtt) {
    send_mqtt_telemetry(temp, tds);
  }
}

void setup() {
  // Disable brownout detector to prevent reboot loop during radio bursts
  WRITE_PERI_REG(RTC_CNTL_BROWN_OUT_REG, 0);

  Serial.begin(115200);
  delay(1000);
  Serial.println("\n==============================================");
  Serial.println("  ESP3: RS485 Modbus RTU (PT100) + BLE + WiFi ");
  Serial.println("  (LCD disabled - Headless Sensor Node)       ");
  Serial.println("==============================================");

  prefs.begin("esp3_cfg", false);
  String s = prefs.getString("ssid", "AAMM_");
  String p = prefs.getString("pass", "11111111");
  strncpy(wifi_ssid, s.c_str(), sizeof(wifi_ssid) - 1);
  strncpy(wifi_pass, p.c_str(), sizeof(wifi_pass) - 1);

  // Initialize BLE NUS
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
  Serial.println("[BLE] Advertising started as 'ESP3-TDS'");

  ble_unconnected_start = millis();

  // Initialize RS485 Modbus RTU interface on UART2 (GPIO16=RX,
  // GPIO17=TX)
  Serial2.begin(RS485_BAUD, SERIAL_8N1, RS485_RX_PIN, RS485_TX_PIN);
  modbusNode.begin(RS485_SLAVE_ID, Serial2);
  // Note: XY-K485 is auto-direction transceiver, no DE/RE toggle
  // required
  Serial.printf("[RS485] Serial2 initialized at 9600 8N1 (RX:%d, "
                "TX:%d, SlaveID:%d)\n",
                RS485_RX_PIN, RS485_TX_PIN, RS485_SLAVE_ID);

  Serial.println("[SYSTEM] Ready! Polling PT100 sensor every 2 seconds...\n");
}

void loop() {
  uint32_t now = millis();

  // 1. 60-second BLE Timeout check in AUTO mode
  if (current_mode == MODE_BLE_AUTO && !ble_connected) {
    if (ble_unconnected_start > 0 && (now - ble_unconnected_start >= 60000)) {
      if (!active_fallback_mqtt) {
        active_fallback_mqtt = true;
        Serial.println("[FAILOVER] BLE 60s timeout -> Switched to "
                       "WiFi/MQTT fallback!");
      }
    }
  }

  // 2. Poll PT100 sensor every 2 seconds
  if (now - last_sensor_tick >= 2000) {
    last_sensor_tick = now;

    float real_temp = 0.0f;
    float real_res = 0.0f;
    if (read_rs485_pt100(real_temp, real_res)) {
      current_soil_temp = real_temp;
      rs485_hardware_connected = true;
    } else {
      // error: preserve last valid reading or default
      rs485_hardware_connected = false;
    }

    // Send telemetry (BLE or WiFi fallback)
    send_sensor_data(current_soil_temp, current_tds);
  }

  delay(20);
}
