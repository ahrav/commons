# `shared-exclusive-exclusion-matrix`

- **Discovery:** concurrency and lifecycle passes.
- **Primary evidence:** contract at `crates/cortexkit-lease/src/lib.rs:169-187`; shared path at `:278-313`; exclusive path at `:240-276`.
- **Existing evidence:** `shared_holders_coexist_but_block_exclusive` (`crates/cortexkit-lease/src/lib.rs:547-579`), `exclusive_holder_blocks_shared` (`:581-597`), and `shared_lease_across_processes_blocks_exclusive` (`:630-691`), including the discriminating step where one of two shared holders drops and exclusive remains blocked.
- **Failure scenario:** process-scoped lock emulation or premature unlock lets exclusive coexist with a remaining shared holder.
- **Timing window:** exclusive attempt after the first shared holder drops but before the last drops.
- **Instrumentation:** partial; tests observe API outcomes but not live-holder counts or inode identity.
- **Open-question log:** locking uses the standard library's `File::try_lock` and `File::try_lock_shared`, with contention and other failures classified through `TryLockError` (`crates/cortexkit-lease/src/lib.rs:30-34,256-263,290-299`). These APIs set the workspace MSRV to Rust 1.89 (`Cargo.toml:14-15`). Deployment filesystem support beyond exercised platforms needs human confirmation.
