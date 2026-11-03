# PC Gateway

The PC application receives ESP telemetry from the local MQTT broker, aggregates it, provides the local dashboard (T10), and synchronizes telemetry and control commands with CoreIOT (T08). Keep broker configuration and credentials outside tracked files.

Set `DATABASE_URL` to a create-capable SQLite URL, such as `sqlite://gateway.db?mode=rwc`, so first startup creates the database before applying migrations.
