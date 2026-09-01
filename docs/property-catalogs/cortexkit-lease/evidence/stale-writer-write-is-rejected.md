# `stale-writer-write-is-rejected`

- **Discovery:** protocol-contract and consumer passes.
- **Primary evidence:** claim at `crates/cortexkit-lease/src/lib.rs:2-6`; SQLite enforcement at `crates/cortexkit-store/src/lib.rs:144-205`.
- **Existing evidence:** `superseded_writer_is_fenced_out_after_handover` (`crates/cortexkit-store/src/lib.rs:629-667`) uses synthetic stores with independently supplied epochs. `equal_epoch_writer_is_not_fenced` (`:670-689`) proves duplicated epochs remain authorized.
- **Backend disagreement:** PostgreSQL increments and exposes an epoch (`crates/cortexkit-store-postgres/src/lib.rs:65-80,182-195`) but no fence comparison appears in its write API.
- **Failure scenario:** old connection persists after releasing lease; replacement claims newer epoch; old connection writes late.
- **Timing window:** handover through old-connection closure.
- **Instrumentation:** missing end-to-end handle-to-fence provenance and backend-wide write-site inventory.
- **Open-question log:** PostgreSQL design intent requires human input; its session lock may be considered sufficient, which would make its epoch informational despite current docs.
