# `invalid-epoch-fails-closed`

- **Discovery:** data-integrity and protocol-format passes.
- **Primary evidence:** `read_epoch` accepts only 1-20 ASCII digits in `u64` range (`src/lib.rs:342-370`).
- **Discriminating fact:** every existing empty, non-decimal, oversized, or out-of-range state returns `InvalidData`; `open_lease_file` publishes only initialized canonical zero.
- **Existing evidence:** `invalid_epoch_states_fail_closed` exercises both acquisition modes for parser-invalid states and preserves bytes (`src/lib.rs:559-601`). `maximum_epoch_is_readable_but_exhausted` separates valid shared reads from exclusive exhaustion (`src/lib.rs:604-627`).
- **Failure scenario:** future formats without an explicit migration fail closed rather than silently issuing epoch 1.
- **Instrumentation:** a corruption-specific public error remains absent; callers see `LeaseError::Io`.
- **Open-question log:** none for the current decimal format.
