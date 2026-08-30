# `stale-writer-write-is-rejected`

- **Discovery:** protocol-contract and consumer passes.
- **Primary evidence:** claim at `crates/cortexkit-lease/src/lib.rs:11-16`; SQLite enforcement at `crates/cortexkit-store/src/lib.rs:155-216`.
- **Existing evidence:** `superseded_writer_is_fenced_out_after_handover` (`crates/cortexkit-store/src/lib.rs:631-670`) uses synthetic stores with independently supplied epochs. `equal_epoch_writer_is_not_fenced` (`:672-692`) proves duplicated epochs remain authorized.
- **Backend disagreement:** PostgreSQL increments and exposes an epoch (`crates/cortexkit-store-postgres/src/lib.rs:77-91,203-216`) but no fence comparison appears in its write API.
- **Failure scenario:** old connection persists after releasing lease; replacement claims newer epoch; old connection writes late.
- **Timing window:** handover through old-connection closure.
- **Instrumentation:** missing end-to-end handle-to-fence provenance and backend-wide write-site inventory.
- **Open-question log:** PostgreSQL design intent requires human input; its session lock may be considered sufficient, which would make its epoch informational despite current docs.
