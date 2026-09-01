# `invalid-epoch-fails-closed`

- **Discovery:** data-integrity and protocol-format passes.
- **Primary evidence:** `read_epoch` accepts only 1-20 ASCII digits in `u64` range (`src/lib.rs:395-423`).
- **Discriminating fact:** ordinary and shared acquisition reject empty state, and every acquisition mode rejects nonempty, non-decimal, oversized, or out-of-range state with `InvalidData`; `open_lease_file` publishes only initialized canonical zero.
- **Existing evidence:** `invalid_epoch_states_fail_closed` exercises ordinary and floor-based acquisition for parser-invalid states and preserves bytes (`src/lib.rs:642-687`). `maximum_epoch_is_readable_but_exhausted` separates valid shared reads from exclusive exhaustion (`src/lib.rs:718-741`). Floor-based empty-state recovery is separately pinned at `src/lib.rs:522-540`.
- **Failure scenario:** future formats without an explicit migration fail closed rather than silently issuing epoch 1.
- **Instrumentation:** a corruption-specific public error remains absent; callers see `LeaseError::Io`.
- **Open-question log:** none for the current decimal format.
