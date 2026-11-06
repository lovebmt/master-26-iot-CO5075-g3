import requests
import json
import urllib3

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

SESSION = requests.Session()
BASE_URL = "https://fuvi-server.tailc75d93.ts.net"

def login():
    res = SESSION.post(f"{BASE_URL}/api/auth/login", json={"username": "master.iot@gmail.com", "password": "master.iot"}, verify=False)
    res.raise_for_status()
    token = res.json()["token"]
    return {"X-Authorization": f"Bearer {token}", "Content-Type": "application/json"}

def setup():
    headers = login()
    rc_id = "48e0fd00-d678-11f1-9301-8d833790fa1a"
    meta = SESSION.get(f"{BASE_URL}/api/ruleChain/{rc_id}/metadata", headers=headers, verify=False).json()
    
    # Keep only the original 7 system nodes
    original_nodes = []
    for n in meta["nodes"]:
        if n.get("name") in [
            "Save Timeseries",
            "Save Client Attributes",
            "Message Type Switch",
            "Log RPC from Device",
            "Log Other",
            "RPC Call Request",
            "Device Profile Node"
        ]:
            original_nodes.append(n)
    
    meta["nodes"] = original_nodes
    
    # Original 6 system connections
    meta["connections"] = [
        {"fromIndex": 2, "toIndex": 0, "type": "Post telemetry"},
        {"fromIndex": 2, "toIndex": 1, "type": "Post attributes"},
        {"fromIndex": 2, "toIndex": 3, "type": "RPC Request from Device"},
        {"fromIndex": 2, "toIndex": 4, "type": "Other"},
        {"fromIndex": 2, "toIndex": 5, "type": "RPC Request to Device"},
        {"fromIndex": 6, "toIndex": 2, "type": "Success"}
    ]

    save_timeseries_idx = 0
    switch_idx = len(meta["nodes"]) # 7

    # JavaScript Switch function for Hysteresis + Air.Con Automatic Control
    # 1. Air Temp Alarms: Low < 24°C (Clear >= 25°C), High > 30°C (Clear <= 29°C) -> 1°C Hysteresis
    # 2. Air Humi Alarms: Low < 80% (Clear >= 82%), High > 95% (Clear <= 93%) -> 2% RH Hysteresis
    # 3. Soil Temp Alarms: Low < 20°C (Clear >= 21°C), High > 32°C (Clear <= 31°C) -> 1°C Hysteresis
    # 4. Air.Con Control (ESP2): On > 29°C, Off < 26°C -> 3°C Hysteresis deadband
    js_switch_script = """
var relations = [];

function getNum(val) {
    if (val === null || val === undefined) return null;
    var n = parseFloat(val);
    return isNaN(n) ? null : n;
}

// 1. Air Temperature (ESP1)
var t = getNum(msg.temperature);
if (t === null && msg.values) t = getNum(msg.values.temperature);
if (t !== null) {
    // Alarm High/Low
    if (t > 30.0) {
        relations.push('AirTempHigh');
    } else if (t <= 29.0) {
        relations.push('ClearAirTempHigh');
    }
    
    if (t < 24.0) {
        relations.push('AirTempLow');
    } else if (t >= 25.0) {
        relations.push('ClearAirTempLow');
    }

    // Air Conditioner Automatic Control
    if (t > 29.0) {
        relations.push('AirConOn');
    } else if (t < 26.0) {
        relations.push('AirConOff');
    }
}

// 2. Air Humidity (ESP1)
var h = getNum(msg.humidity);
if (h === null && msg.values) h = getNum(msg.values.humidity);
if (h !== null) {
    if (h > 95.0) {
        relations.push('AirHumiHigh');
    } else if (h <= 93.0) {
        relations.push('ClearAirHumiHigh');
    }
    
    if (h < 80.0) {
        relations.push('AirHumiLow');
    } else if (h >= 82.0) {
        relations.push('ClearAirHumiLow');
    }
}

// 3. Soil Temperature (ESP2 / ESP3)
var st = getNum(msg.soil_temp);
if (st === null && msg.values) st = getNum(msg.values.soil_temp);
if (st !== null) {
    if (st > 32.0) {
        relations.push('SoilTempHigh');
    } else if (st <= 31.0) {
        relations.push('ClearSoilTempHigh');
    }
    
    if (st < 20.0) {
        relations.push('SoilTempLow');
    } else if (st >= 21.0) {
        relations.push('ClearSoilTempLow');
    }
}

return relations;
"""

    switch_node = {
        "type": "org.thingsboard.rule.engine.filter.TbJsSwitchNode",
        "name": "Hysteresis Alarm Switch",
        "configuration": {
            "scriptLang": "JS",
            "jsScript": js_switch_script.strip(),
            "tbelScript": js_switch_script.strip()
        },
        "additionalInfo": {
            "description": "Checks Hysteresis thresholds for Alarms and Air.Con automatic control",
            "layoutX": 1120,
            "layoutY": 260
        }
    }

    def make_create_config(alarm_type, severity):
        return {
            "alarmType": alarm_type,
            "scriptLang": "JS",
            "alarmDetailsBuildJs": "var details = {};\nif (metadata.prevAlarmDetails) {\n    details = JSON.parse(metadata.prevAlarmDetails);\n    delete metadata.prevAlarmDetails;\n}\ndetails.temperature = msg.temperature;\ndetails.humidity = msg.humidity;\ndetails.soil_temp = msg.soil_temp;\ndetails.reported_at = new Date().toISOString();\nreturn details;",
            "alarmDetailsBuildTbel": "var details = {};\nif (metadata.prevAlarmDetails != null) {\n    details = JSON.parse(metadata.prevAlarmDetails);\n    metadata.remove('prevAlarmDetails');\n}\ndetails.temperature = msg.temperature;\ndetails.humidity = msg.humidity;\ndetails.soil_temp = msg.soil_temp;\ndetails.reported_at = new Date().toISOString();\nreturn details;",
            "severity": severity,
            "propagate": False,
            "propagateToOwner": False,
            "propagateToTenant": False,
            "useMessageAlarmData": False,
            "overwriteAlarmDetails": True,
            "dynamicSeverity": False,
            "relationTypes": []
        }

    def make_clear_config(alarm_type):
        return {
            "alarmType": alarm_type,
            "scriptLang": "JS",
            "alarmDetailsBuildJs": "var details = {};\nif (metadata.prevAlarmDetails) {\n    details = JSON.parse(metadata.prevAlarmDetails);\n    delete metadata.prevAlarmDetails;\n}\nreturn details;",
            "alarmDetailsBuildTbel": "var details = {};\nif (metadata.prevAlarmDetails != null) {\n    details = JSON.parse(metadata.prevAlarmDetails);\n    metadata.remove('prevAlarmDetails');\n}\nreturn details;"
        }

    alarms = [
        # Air Temp High
        ("AirTempHigh", "Create Air Temp High", "Air Temperature High", True, "CRITICAL", 1450, 60),
        ("ClearAirTempHigh", "Clear Air Temp High", "Air Temperature High", False, None, 1750, 60),
        # Air Temp Low
        ("AirTempLow", "Create Air Temp Low", "Air Temperature Low", True, "WARNING", 1450, 140),
        ("ClearAirTempLow", "Clear Air Temp Low", "Air Temperature Low", False, None, 1750, 140),
        # Air Humi High
        ("AirHumiHigh", "Create Air Humi High", "Air Humidity High", True, "CRITICAL", 1450, 220),
        ("ClearAirHumiHigh", "Clear Air Humi High", "Air Humidity High", False, None, 1750, 220),
        # Air Humi Low
        ("AirHumiLow", "Create Air Humi Low", "Air Humidity Low", True, "WARNING", 1450, 300),
        ("ClearAirHumiLow", "Clear Air Humi Low", "Air Humidity Low", False, None, 1750, 300),
        # Soil Temp High
        ("SoilTempHigh", "Create Soil Temp High", "Soil Temperature High", True, "CRITICAL", 1450, 380),
        ("ClearSoilTempHigh", "Clear Soil Temp High", "Soil Temperature High", False, None, 1750, 380),
        # Soil Temp Low
        ("SoilTempLow", "Create Soil Temp Low", "Soil Temperature Low", True, "WARNING", 1450, 460),
        ("ClearSoilTempLow", "Clear Soil Temp Low", "Soil Temperature Low", False, None, 1750, 460),
    ]

    new_nodes = [switch_node]
    new_connections = [
        {"fromIndex": save_timeseries_idx, "toIndex": switch_idx, "type": "Success"}
    ]

    # 1. Add Alarm Nodes
    for rel, node_name, alarm_type, is_create, severity, lx, ly in alarms:
        node_idx = switch_idx + len(new_nodes)
        if is_create:
            node = {
                "type": "org.thingsboard.rule.engine.action.TbCreateAlarmNode",
                "name": node_name,
                "configuration": make_create_config(alarm_type, severity),
                "additionalInfo": {"layoutX": lx, "layoutY": ly}
            }
        else:
            node = {
                "type": "org.thingsboard.rule.engine.action.TbClearAlarmNode",
                "name": node_name,
                "configuration": make_clear_config(alarm_type),
                "additionalInfo": {"layoutX": lx, "layoutY": ly}
            }
        new_nodes.append(node)
        new_connections.append({
            "fromIndex": switch_idx,
            "toIndex": node_idx,
            "type": rel
        })

    # 2. Add Air.Con RPC Automation Nodes
    # 2.1 Transform to RPC ON
    rpc_on_idx = switch_idx + len(new_nodes)
    node_rpc_on = {
        "type": "org.thingsboard.rule.engine.transform.TbTransformMsgNode",
        "name": "Prepare RPC AirCon ON",
        "configuration": {
            "scriptLang": "JS",
            "jsScript": "var newMsg = { method: 'set_light', params: { state: true } }; metadata.oneway = 'true'; return {msg: newMsg, metadata: metadata, msgType: 'POST_TELEMETRY_REQUEST'};",
            "tbelScript": "var newMsg = { method: 'set_light', params: { state: true } }; metadata.oneway = 'true'; return {msg: newMsg, metadata: metadata, msgType: 'POST_TELEMETRY_REQUEST'};"
        },
        "additionalInfo": {"layoutX": 1450, "layoutY": 540}
    }
    new_nodes.append(node_rpc_on)
    new_connections.append({"fromIndex": switch_idx, "toIndex": rpc_on_idx, "type": "AirConOn"})

    # 2.2 Transform to RPC OFF
    rpc_off_idx = switch_idx + len(new_nodes)
    node_rpc_off = {
        "type": "org.thingsboard.rule.engine.transform.TbTransformMsgNode",
        "name": "Prepare RPC AirCon OFF",
        "configuration": {
            "scriptLang": "JS",
            "jsScript": "var newMsg = { method: 'set_light', params: { state: false } }; metadata.oneway = 'true'; return {msg: newMsg, metadata: metadata, msgType: 'POST_TELEMETRY_REQUEST'};",
            "tbelScript": "var newMsg = { method: 'set_light', params: { state: false } }; metadata.oneway = 'true'; return {msg: newMsg, metadata: metadata, msgType: 'POST_TELEMETRY_REQUEST'};"
        },
        "additionalInfo": {"layoutX": 1450, "layoutY": 620}
    }
    new_nodes.append(node_rpc_off)
    new_connections.append({"fromIndex": switch_idx, "toIndex": rpc_off_idx, "type": "AirConOff"})

    # 2.3 Change Originator to ESP2
    ch_orig_idx = switch_idx + len(new_nodes)
    node_ch_orig = {
        "type": "org.thingsboard.rule.engine.transform.TbChangeOriginatorNode",
        "name": "Change Originator to ESP2",
        "configuration": {
            "originatorSource": "ENTITY",
            "entityType": "DEVICE",
            "entityNamePattern": "ESP2"
        },
        "additionalInfo": {"layoutX": 1750, "layoutY": 580}
    }
    new_nodes.append(node_ch_orig)
    new_connections.append({"fromIndex": rpc_on_idx, "toIndex": ch_orig_idx, "type": "Success"})
    new_connections.append({"fromIndex": rpc_off_idx, "toIndex": ch_orig_idx, "type": "Success"})

    # 2.4 RPC Call Request Node
    rpc_send_idx = switch_idx + len(new_nodes)
    node_rpc_send = {
        "type": "org.thingsboard.rule.engine.rpc.TbSendRPCRequestNode",
        "name": "Send AirCon RPC to ESP2",
        "configuration": {
            "timeoutInSeconds": 60
        },
        "additionalInfo": {"layoutX": 2050, "layoutY": 580}
    }
    new_nodes.append(node_rpc_send)
    new_connections.append({"fromIndex": ch_orig_idx, "toIndex": rpc_send_idx, "type": "Success"})

    meta["nodes"].extend(new_nodes)
    meta["connections"].extend(new_connections)

    print(f"Submitting rule chain with {len(meta['nodes'])} nodes and {len(meta['connections'])} connections...")
    res = SESSION.post(f"{BASE_URL}/api/ruleChain/metadata", headers=headers, json=meta, verify=False)
    if res.status_code == 200:
        print("SUCCESS! Root Rule Chain with Hysteresis Alarms + Air.Con RPC has been saved!")
    else:
        print("Error:", res.status_code, res.text)
        res.raise_for_status()

if __name__ == "__main__":
    setup()
