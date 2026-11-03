# PC Gateway

The PC application receives ESP telemetry from the local MQTT broker, aggregates it, provides the local dashboard (T10), and synchronizes telemetry and control commands with CoreIOT (T08). Keep broker configuration and credentials outside tracked files.

Set `DATABASE_URL` to a create-capable SQLite URL, such as `sqlite://gateway.db?mode=rwc`, so first startup creates the database before applying migrations.

Start the local test broker with `docker compose -f compose.yaml up -d`. Its MQTT port is bound to localhost only. Run the intake integration test with `NANOMQ_TEST_URL=mqtt://127.0.0.1:1883 cargo test --test nanomq_flow`; without `NANOMQ_TEST_URL`, the test reports a skip and exits successfully.
