# `returned-epoch-is-crash-durable`

- **Discovery:** state/persistence and crash-recovery passes.
- **Primary evidence:** “durable” and “persisted” claims at `Cargo.toml:9` and `src/lib.rs:1,11-16`; write sequence at `:328-338`.
- **Contradictory code evidence:** no `sync_data`, `sync_all`, or directory sync; `flush` at `:337` is not a stable-storage barrier for `File`.
- **Existing evidence:** `epoch_persists_across_store_instances` (`src/lib.rs:693-706`) recreates `FileLeaseStore` in a live process, which preserves page cache and directory state.
- **Failure scenario:** acknowledged epoch or newly created directory entry is lost on power failure; next writer reuses an old value.
- **Timing window:** after `acquire` returns and before kernel writeback.
- **Instrumentation:** missing crash-image or power-cut replay and acknowledgement witness keyed by physical root plus `LeaseKey` fields.
- **Open-question log:** all crate docs were checked; none defines whether “durable” excludes machine failure. Owner intent is required.
