# `protected-write-set-is-fence-complete`

- **Discovery:** targeted consumer write-surface pass after portfolio evaluation.
- **Primary evidence:** SQLite exposes `with_conn`, `with_conn_fenced`, and `migrate` (`cortexkit-store/src/lib.rs:130-220`); PostgreSQL exposes `with_client` and `migrate` without comparing epoch (`cortexkit-store-postgres/src/lib.rs:72-89`).
- **Existing evidence:** fence tests cover `with_conn_fenced` only.
- **Failure scenario:** domain code performs a protected durable mutation through an unfenced closure or migration path.
- **Timing window:** stale connection after handover.
- **Instrumentation:** authoritative inventory of durable-write sites plus at least one authoritative fence check atomically bound to each protected commit.
- **Open-question log:** repository docs do not classify which data and write APIs require stale-writer protection.
