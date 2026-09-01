# `stale-writer-write-is-rejected`

- **Discovery:** protocol-contract and consumer passes.
- **Primary evidence:** claim at `crates/cortexkit-lease/src/lib.rs:2-6`; SQLite enforcement at `crates/cortexkit-store/src/lib.rs:172-187,324-344`; PostgreSQL enforcement at `crates/cortexkit-store-postgres/src/lib.rs:116-126,189-207`.
- **Existing evidence:** `superseded_writer_is_fenced_out_after_handover` (`crates/cortexkit-store/src/lib.rs:1328-1367`) uses synthetic stores with independently supplied epochs. `equal_epoch_writer_is_not_fenced` (`:1402-1423`) proves equal epochs remain authorized on the write path, and `open_claim_rejects_an_epoch_the_database_already_stores` (`:749-786`) proves the open path refuses them. PostgreSQL separately checks callback rollback, equal-epoch repeats, and synthetic stale rejection (`crates/cortexkit-store-postgres/src/lib.rs:821-911`). Both backends reject stale migrations before schema SQL.
- **Failure scenario:** old connection persists after releasing lease; replacement claims newer epoch; old connection writes late.
- **Timing window:** handover through old-connection closure.
- **Instrumentation:** missing end-to-end retained-connection handover and a complete protected write-site inventory.
- **Open-question log:** unfenced SQLite consumer mutations prevent a backend-wide guarantee.
