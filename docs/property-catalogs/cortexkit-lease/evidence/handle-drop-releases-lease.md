# `handle-drop-releases-lease`

- **Discovery:** targeted lifecycle pass after portfolio evaluation.
- **Primary evidence:** handle contract at `crates/cortexkit-lease/src/lib.rs:126-140`; file-handle `Drop` calls standard-library `File::unlock` (`crates/cortexkit-lease/src/lib.rs:213-236`). Acquisition and read failures also call `File::unlock` before returning (`crates/cortexkit-lease/src/lib.rs:265-269,301-306`). Reacquisition tests cover normal drops (`crates/cortexkit-lease/src/lib.rs:490-506,547-597,630-705`).
- **Toolchain mechanism:** standard-library `File::try_lock`, `File::try_lock_shared`, and `File::unlock` provide acquisition and release (`crates/cortexkit-lease/src/lib.rs:30-34,233-235,256-263,290-299`). Descriptor close follows the best-effort explicit unlock when `Drop` returns.
- **Failure scenario:** best-effort explicit unlock fails and descriptor close also does not release the lock promptly; a successor remains blocked.
- **Timing window:** last-handle drop while a competitor that has observed `Held` continues retrying.
- **Instrumentation:** retry-attempt timestamps, explicit last-handle event, and scheduler-fairness assumption.
- **Open-question log:** workspace and crate manifests declare MSRV 1.89 (`Cargo.toml:9-15`; `crates/cortexkit-lease/Cargo.toml:1-9`), when these standard-library file-locking APIs stabilized. CI installs only the moving stable toolchain (`.github/workflows/ci.yml:21-42,64-73`), so compatibility with the declared MSRV is unverified.
