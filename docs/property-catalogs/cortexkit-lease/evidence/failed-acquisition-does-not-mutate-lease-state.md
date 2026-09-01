# `failed-acquisition-does-not-mutate-lease-state`

- **Discovery:** targeted pre-lock side-effect pass after portfolio evaluation.
- **Primary evidence:** file-backed exclusive acquisition opens or creates and calls `protect_file` before `File::try_lock` (`crates/cortexkit-lease/src/lib.rs:240-263`); shared acquisition does the same before `File::try_lock_shared` (`crates/cortexkit-lease/src/lib.rs:278-299`). PostgreSQL tries its session advisory lock before creating infrastructure tables or bumping the epoch (`crates/cortexkit-store-postgres/src/lib.rs:116-142`).
- **Existing evidence:** file contention tests assert returned errors and later acquisition behavior, not file bytes or metadata (`crates/cortexkit-lease/src/lib.rs:490-506,547-597,630-691`).
- **Failure scenario:** losing acquirer changes mode or creates the file before returning `Held`; foreign ownership or read-only access returns undifferentiated `Io` before contention is known.
- **Timing window:** incumbent live; competitor reaches hardening before try-lock.
- **Instrumentation:** content, mode, owner, and mtime snapshot around rejected acquisition.
- **Open-question log:** commit `49bcaa2` assumes a single-account host but the public crate contract does not state that precondition.
