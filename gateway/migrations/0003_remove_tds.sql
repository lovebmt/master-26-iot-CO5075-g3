DELETE FROM telemetry_history WHERE metric = 'tds_ppm';

UPDATE device_state
SET payload_json = json_remove(payload_json, '$.values.tds_ppm')
WHERE json_valid(payload_json)
  AND json_type(payload_json, '$.values.tds_ppm') IS NOT NULL;

UPDATE telemetry_outbox
SET payload_json = json_remove(payload_json, '$.values.tds_ppm')
WHERE json_valid(payload_json)
  AND json_type(payload_json, '$.values.tds_ppm') IS NOT NULL;
