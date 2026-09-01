# `at-most-one-exclusive-holder-per-key`

- **Discovery:** architecture, concurrency, claimed-safety, and failure-recovery passes.
- **Primary evidence:** headline contract at `crates/cortexkit-lease/src/lib.rs:2-6`; the file backend opens one per-key path and calls `File::try_lock`, mapping `TryLockError::WouldBlock` to `Held` and other lock errors to `Io` (`crates/cortexkit-lease/src/lib.rs:150-153,181-218`). PostgreSQL derives one advisory key and treats a false `pg_try_advisory_lock` result as `Held` (`crates/cortexkit-store-postgres/src/lib.rs:41-54,116-126`).
- **Existing evidence:** `acquire_then_second_holder_is_rejected` (`crates/cortexkit-lease/src/lib.rs:429-445`) and PostgreSQL's `open_migrate_and_single_writer` (`crates/cortexkit-store-postgres/src/lib.rs:305-341`) are same-process and sequential. `README.md:12-13` claims a real-daemon two-process check, but none exists in this repository.
- **Failure scenario:** independent processes race; path aliasing, replacement, or lock-scope mismatch creates separate lock domains; both return `Ok`.
- **Timing window:** both contenders reach backend lock acquisition before either holder releases.
- **Instrumentation:** missing live-exclusive-holder identity and cross-process exclusive-versus-exclusive race barrier.
- **Open-question log:** searched the target crate, `cortexkit-store`, `cortexkit-store-postgres`, README, and CI. No cross-process exclusive-versus-exclusive check found. External claustrum evidence is needed to resolve the README claim.
