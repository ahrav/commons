# `stale-writer-write-is-rejected`

- **Discovery:** protocol-contract and consumer passes.
- **Primary evidence:** claim at `crates/cortexkit-lease/src/lib.rs:2-6`; SQLite enforcement at `crates/cortexkit-store/src/lib.rs:164-179,312-343`; PostgreSQL enforcement at `crates/cortexkit-store-postgres/src/lib.rs:111-121,156-175`.
- **Existing evidence:** `superseded_writer_is_fenced_out_after_handover` (`crates/cortexkit-store/src/lib.rs:833-871`) uses synthetic stores with independently supplied epochs. `equal_epoch_writer_is_not_fenced` (`:907-927`) proves equal epochs remain authorized. PostgreSQL separately checks callback rollback, equal-epoch repeats, and synthetic stale rejection (`crates/cortexkit-store-postgres/src/lib.rs:476-565`). Both backends reject stale migrations before schema SQL.
- **Failure scenario:** old connection persists after releasing lease; replacement claims newer epoch; old connection writes late.
- **Timing window:** handover through old-connection closure.
- **Instrumentation:** missing end-to-end retained-connection handover and a complete protected write-site inventory.
- **Open-question log:** unfenced SQLite consumer mutations prevent a backend-wide guarantee.
