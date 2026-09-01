# `contention-is-classified-as-held`

- **Discovery:** failure-degradation and history passes.
- **Primary evidence:** error taxonomy at `crates/cortexkit-lease/src/lib.rs:143-165`; exclusive `File::try_lock` and shared `File::try_lock_shared` map `TryLockError::WouldBlock` to `Held` and `TryLockError::Error` to `Io` (`crates/cortexkit-lease/src/lib.rs:256-263,290-299`). PostgreSQL maps a false `pg_try_advisory_lock` result to `Held` and query errors to `Backend` (`crates/cortexkit-store-postgres/src/lib.rs:118-135`).
- **History:** commit `8abefe8` names Windows contention misclassification as a prior bug class.
- **Existing evidence:** same-process exclusive and shared contention tests assert `Held` (`crates/cortexkit-lease/src/lib.rs:490-506,547-597`), and the Unix cross-process shared-versus-exclusive test does too (`crates/cortexkit-lease/src/lib.rs:630-691`). CI runs the workspace tests on Ubuntu, macOS, and Windows (`.github/workflows/ci.yml:21-42`) and a live PostgreSQL job on Ubuntu (`.github/workflows/ci.yml:47-73`).
- **Failure scenario:** target returns a different contention code, or unsupported-lock failure is mistaken for contention.
- **Instrumentation:** missing injection of non-contention lock errors and unsupported-filesystem behavior.
- **Open-question log:** supported targets beyond the CI matrix are not declared.
