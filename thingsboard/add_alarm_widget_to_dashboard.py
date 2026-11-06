import requests
import json
import uuid
import urllib3

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

SESSION = requests.Session()
BASE_URL = "https://fuvi-server.tailc75d93.ts.net"

def login():
    res = SESSION.post(f"{BASE_URL}/api/auth/login", json={"username": "master.iot@gmail.com", "password": "master.iot"}, verify=False)
    res.raise_for_status()
    token = res.json()["token"]
    return {"X-Authorization": f"Bearer {token}", "Content-Type": "application/json"}

def add_alarm_widget():
    headers = login()
    db_id = "ce9732c0-d7f9-11f1-9301-8d833790fa1a"
    db = SESSION.get(f"{BASE_URL}/api/dashboard/{db_id}", headers=headers, verify=False).json()
    
    cfg = db["configuration"]
    
    # 1. Add All Devices Alias
    alias_id = "all-farm-devices-alias-id"
    cfg["entityAliases"][alias_id] = {
        "id": alias_id,
        "alias": "All Farm Devices",
        "filter": {
            "type": "entityList",
            "entityType": "DEVICE",
            "entityList": [
                "4fb8f7f0-d7e9-11f1-9301-8d833790fa1a", # ESP1
                "cf2cd650-d7e9-11f1-9301-8d833790fa1a", # ESP2
                "5fa8f7f0-d7e9-11f1-9301-8d833790fa1b"  # ESP3
            ]
        }
    }

    # 2. Add Alarms Table Widget
    widget_id = "alarm-table-widget-group3"
    
    widget_cfg = {
        "type": "alarm",
        "sizeX": 24,
        "sizeY": 6,
        "config": {
            "timewindow": {
                "realtime": {"interval": 1000, "timewindowMs": 86400000},
                "aggregation": {"type": "NONE", "limit": 200}
            },
            "showTitle": True,
            "backgroundColor": "rgb(255, 255, 255)",
            "color": "rgba(0, 0, 0, 0.87)",
            "padding": "4px",
            "settings": {
                "enableSelection": True,
                "enableSearch": True,
                "displayDetails": True,
                "allowAcknowledgment": True,
                "allowClear": True,
                "allowAssign": True,
                "displayActivity": True,
                "displayPagination": True,
                "defaultPageSize": 10,
                "defaultSortOrder": "-createdTime",
                "enableSelectColumnDisplay": True,
                "enableStickyAction": False,
                "enableFilter": True,
                "entitiesTitle": None,
                "alarmsTitle": "Bảng Theo Dõi Cảnh Báo Ngưỡng (Hysteresis Alarms)"
            },
            "title": "🚨 Bảng Cảnh Báo Ngưỡng Hysteresis (CoreIOT Alarms)",
            "dropShadow": True,
            "enableFullscreen": True,
            "titleStyle": {"fontSize": "16px", "fontWeight": 600, "padding": "5px 10px"},
            "useDashboardTimewindow": False,
            "showLegend": False,
            "alarmSource": {
                "type": "entity",
                "entityAliasId": alias_id,
                "name": "alarms",
                "dataKeys": [
                    {
                        "name": "createdTime",
                        "type": "alarm",
                        "label": "Thời gian tạo",
                        "color": "#2196f3",
                        "settings": {"useCellStyleFunction": False, "useCellContentFunction": False}
                    },
                    {
                        "name": "originator",
                        "type": "alarm",
                        "label": "Thiết bị",
                        "color": "#4caf50",
                        "settings": {"useCellStyleFunction": False, "useCellContentFunction": False}
                    },
                    {
                        "name": "type",
                        "type": "alarm",
                        "label": "Loại cảnh báo",
                        "color": "#f44336",
                        "settings": {"useCellStyleFunction": False, "useCellContentFunction": False}
                    },
                    {
                        "name": "severity",
                        "type": "alarm",
                        "label": "Mức độ",
                        "color": "#ff9800",
                        "settings": {"useCellStyleFunction": False, "useCellContentFunction": False}
                    },
                    {
                        "name": "status",
                        "type": "alarm",
                        "label": "Trạng thái",
                        "color": "#607d8b",
                        "settings": {"useCellStyleFunction": False, "useCellContentFunction": False}
                    }
                ]
            },
            "alarmSearchStatus": "ANY",
            "alarmsPollingInterval": 5,
            "showTitleIcon": True,
            "titleIcon": "warning",
            "iconColor": "#d32f2f",
            "iconSize": "24px",
            "titleTooltip": "Giám sát cảnh báo nhiệt độ, độ ẩm không khí và nhiệt độ giá thể",
            "widgetStyle": {},
            "displayTimewindow": True,
            "actions": {},
            "alarmStatusList": [],
            "alarmSeverityList": [],
            "alarmTypeList": [],
            "searchPropagatedAlarms": False,
            "configMode": "basic",
            "alarmFilterConfig": None
        },
        "id": widget_id,
        "typeAlias": "alarms_table",
        "bundleAlias": "alarm_widgets"
    }

    cfg["widgets"][widget_id] = widget_cfg

    # Layout positioning
    layouts = cfg["states"]["default"]["layouts"]["main"]
    layouts["widgets"][widget_id] = {
        "row": 10,
        "col": 0,
        "sizeX": 24,
        "sizeY": 6
    }

    res = SESSION.post(f"{BASE_URL}/api/dashboard", headers=headers, json=db, verify=False)
    if res.status_code == 200:
        print("Successfully added Alarms Table widget to dashboard!")
    else:
        print("Failed to update dashboard:", res.status_code, res.text)
        res.raise_for_status()

if __name__ == "__main__":
    add_alarm_widget()

