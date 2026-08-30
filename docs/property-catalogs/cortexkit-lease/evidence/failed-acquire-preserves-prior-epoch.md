# `failed-acquire-preserves-prior-epoch`

- **Discovery:** lifecycle and I/O-failure passes.
- **Primary evidence:** exclusive acquisition calls `bump_epoch` and uses `File::unlock` on error (`crates/cortexkit-lease/src/lib.rs:265-269`); `bump_epoch` truncates before writing and flushing the replacement epoch (`crates/cortexkit-lease/src/lib.rs:328-338`).
- **Discriminating fact:** if `set_len(0)` succeeds and `write_all` fails, acquisition returns `Io` after unlocking but leaves the previous epoch erased.
- **Existing evidence:** no failpoint or storage-error test.
- **Failure scenario:** full disk, quota, or returned I/O error converts an `Err` result into persistent fence damage. Non-returning process termination is covered by crash recovery.
- **Timing window:** after `set_len(0)` at `crates/cortexkit-lease/src/lib.rs:334`, before `write_all` at `:336` completes.
- **Instrumentation:** missing write failpoint after truncate and before bytes persist.
- **Open-question log:** a temp-file rename would preserve old bytes but replace the inode, conflicting with `docs/lease-store-density.md:22-24`; no intended protocol is documented.
