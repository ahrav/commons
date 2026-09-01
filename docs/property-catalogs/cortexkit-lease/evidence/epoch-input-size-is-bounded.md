# `epoch-input-size-is-bounded`

- **Discovery:** resource-boundary pass.
- **Primary evidence:** `read_epoch` uses `Read::take(21)`, preallocates 21 bytes, and rejects lengths above the 20-byte decimal maximum (`crates/cortexkit-lease/src/lib.rs:384-413`).
- **Existing evidence:** `invalid_epoch_states_fail_closed` exercises a 21-byte file through exclusive and shared acquisition (`crates/cortexkit-lease/src/lib.rs:601-644`).
- **Failure scenario:** oversized restored or hostile files fail without proportional allocation.
- **Timing window:** none; file contents alone enable it.
- **Instrumentation:** no read-byte counter is needed for the bounded reader; exact syscall read sizes are not asserted.
- **Open-question log:** a future format must revise the 20-byte limit deliberately.
