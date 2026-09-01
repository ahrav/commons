# `protected-write-set-is-fence-complete`

- **Discovery:** targeted consumer write-surface pass after portfolio evaluation.
- **Primary evidence:** SQLite documents `with_conn` as unfenced and read-only by contract but cannot enforce that restriction (`cortexkit-store/src/lib.rs:132-145`). PostgreSQL exposes enforced read-only and fenced transactions (`cortexkit-store-postgres/src/lib.rs:84-102,104-121`). Both migration implementations check the fence in the same transaction as schema SQL (`cortexkit-store/src/lib.rs:361-418`; `cortexkit-store-postgres/src/lib.rs:293-344`).
- **Existing evidence:** the [durable consumer inventory](../durable-consumer-inventory.md) records unfenced durable mutations in Magic Context. Backend tests cover fenced writes, migrations, rollback, and PostgreSQL read-only enforcement, not consumer completeness.
- **Failure scenario:** domain code performs a protected durable mutation through an unfenced closure or migration path.
- **Timing window:** stale connection after handover.
- **Instrumentation:** authoritative inventory of durable-write sites plus at least one authoritative fence check atomically bound to each protected commit.
- **Open-question log:** companion SQLite consumer migrations and an authoritative protected write-set classification remain unavailable.
