# System model

System path: `crates/cortexkit-lease` at `fa975843afd4b3122288149968ea5d6ff46322b3`.

## Architecture and data flow

`LeaseKey` contains `(module_id, backend, scope_key)` (`src/lib.rs:92-123`). The fields are joined with `U+001F`, hashed with FNV-1a-64, and mapped to `<base_dir>/<16hex>.lease` (`src/lib.rs:204-210,341-358`).

Exclusive acquisition creates the directory, opens or creates the file, calls `protect_file`, calls the standard-library `File::try_lock`, increments the epoch, and returns a handle that owns the file descriptor (`src/lib.rs:239-276`). Shared acquisition follows the same path but calls `File::try_lock_shared` and only reads the epoch (`src/lib.rs:278-313`). Both methods classify `TryLockError::WouldBlock` as `LeaseError::Held` and unwrap `TryLockError::Error` into `LeaseError::Io`.

The crate has no network or database boundary. Its authority boundaries are the filesystem path and kernel lock table. `cortexkit-store` consumes the exclusive handle and uses its epoch at a SQLite write fence (`cortexkit-store/src/lib.rs:144-205,245-284`).

## State and persistence

One file per derived key stores a bare decimal `u64`. There is no magic, key binding, checksum, length bound, format version, or generation. `bump_epoch` reads the whole file, parses with `unwrap_or(0)`, uses `saturating_add`, truncates, rewrites, and calls `flush` (`src/lib.rs:328-339`). No stable-storage sync exists in the crate.

Lease files are not removed by production code. `docs/lease-store-density.md:22-24` says this avoids an unlink-inode race; the source does not enforce the assumption against external actors.

## Concurrency model

There is no internal shared mutable state. Inter-process coordination is entirely the OS advisory lock held by `FileLeaseHandle.file` (`src/lib.rs:213-236`). The crate uses standard-library `File::try_lock`, `File::try_lock_shared`, and `File::unlock`; lock semantics still depend on the target OS and backing filesystem.

The epoch read-modify-write runs only after the exclusive lock succeeds. Shared readers each use a separate file descriptor and perform no epoch write.

## Claimed safety guarantees

- No two live writers for one logical store (`src/lib.rs:3-5`).
- Persisted, monotonically increasing writer epochs; stale writes are rejected (`src/lib.rs:11-16,135-137`).
- Distinct module/backend/scope keys do not collide on a shared root (`src/lib.rs:22-28,204-210`).
- Shared and exclusive holder modes exclude each other (`src/lib.rs:173-186`).
- Permission hardening refuses non-regular paths and does not follow symlinks (`src/lib.rs:51-78`).
- Genuine contention maps to `LeaseError::Held` through `TryLockError::WouldBlock` (`src/lib.rs:256-263,293-299`).

These remain claims under test. Code disagreements are retained in the catalog.

## Claimed liveness guarantees

- Process death releases the advisory lock and permits reclaim without stale-PID cleanup (`src/lib.rs:8-10`).
- Dropping a handle releases its lease (`src/lib.rs:126-128,233-236`).
- The parked density watcher is claimed to trigger when physical size crosses 1 GiB (`docs/lease-store-density.md:30-42`).

## Bug history and density

- `8abefe8` extracted the lease and names a prior Windows contention-classification bug class.
- `16aed47` added shared mode.
- `49bcaa2` hardens file modes after a measured deployment had permissive files. Its commit message records an initial adjacent `cortexkit-store` WAL test that could not fail, direct evidence that vacuity has occurred in this subsystem's check history.
- `bed0bb7` migrated file locking to the standard library, declared Rust 1.89 as the workspace MSRV, and added the lease identity/hash stability vector.
- `8da6d42` made lease identity and hexadecimal hashing public so PostgreSQL could share the derivation, then added the advisory-key stability vector.
- `f2107e5` exposed numeric `fnv1a`, retained `fnv1a_hex` for filenames, and removed PostgreSQL's hex format/parse round trip.
- `94c65ec` bumped `cortexkit-lease` to 0.1.1 for the public API and MSRV change.
- `docs/lease-store-density.md:7-13` reports 20,484 lease files and 80 MiB physical use from about 20 KiB logical content.

No issue or incident tracker was supplied, so history cannot establish additional reported defects.

## Existing test strategy

Thirteen inline unit tests cover ordinary exclusivity, shared/exclusive behavior, simple key separation, clean epoch increments, permissions, one identity/hash stability vector, and one cross-process shared-lock case. There are no failpoints, crash-image tests, property tests, fuzz targets, model checkers, or situation-coverage assertions. See [checks.md](checks.md).

## Failure and degradation

- The persisted update has a truncate-before-write interruption window.
- Valid-UTF-8 malformed state silently becomes epoch zero; invalid UTF-8 errors.
- `u64::MAX` saturates and is reissued.
- Replacing a locked path creates a new lock domain.
- Unsupported or differently scoped filesystem locks can fail closed or fail open depending on behavior.
- `protect_file` opens before checking and uses separate path-resolution operations for check and chmod.
- Shared acquisition needs write access because it creates and hardens the lease file.

## Dependencies

The crate has no runtime dependency. Its lock API requires Rust 1.89, which the workspace declares in `Cargo.toml`; CI installs floating stable and has no MSRV job. The test suite also assumes `python3` on Unix for the cross-process test (`src/lib.rs:632-691`). The standard-library lock implementation and `TryLockError` behavior remain platform contracts.

## Product context

The lease guards module-owned durable stores. SQLite holds the exclusive handle for the store lifetime and exposes an epoch-fenced write API. Shared mode's documented example protects a blob reader against exclusive GC, but no production in-repo caller uses `acquire_shared`.

The density decision explicitly prioritizes single-writer correctness over reclaiming small files (`docs/lease-store-density.md:16-28`).

## Unproven assumptions

- Lease roots are on filesystems whose advisory-lock scope matches all writers.
- Lease paths and inodes are not replaced while held.
- Key fields never contain the tuple separator and hash collisions need no handling.
- The intended epoch crash model is narrower than machine power loss, despite the unconditional “durable” wording.
- Malformed state can safely mean epoch zero.
- Shared-handle epochs are never routed to writes despite no mode distinction in the type.
- External consumers preserve the lease-path format across overlapping versions.
- One OS account owns and can write each lease root.

Targeted portfolio follow-up found two additional boundary assumptions: every logical store has one canonical `(lease root, LeaseKey)`, and every durable mutation claimed as fence-protected reaches the fenced write API.

## Wildcard findings

- The trait abstraction promises interchangeable future cloud leases, but the actual PostgreSQL backend does not implement `LeaseStore` and has no shared mode.
- `LeaseHandle::epoch` documents a writer fence, while shared handles expose an observability-only value through the same method and trait object.
- Rust 1.89 is declared as the MSRV, but CI does not compile or test with 1.89.
- The README claims a real-daemon, two-process single-writer test; none exists in this repository.

## Property-lens coverage

| Lens | Result |
|---|---|
| Data integrity | Epoch monotonicity, crash durability, malformed state, failed-write preservation, key binding, and input bounds cataloged. |
| Concurrency | Exclusive uniqueness, mode matrix, inode stability, contention classification, and coverage races cataloged. |
| Failure recovery | Dead-holder reclaim, epoch interruption, and file replacement cataloged. |
| Protocol contracts | Fence use, key/path format, and backend write fencing cataloged. |
| Resource boundaries | Unbounded reads and lease-file growth cataloged. |
| Security boundaries | File mode and symlink/TOCTOU properties cataloged. |
| Distributed coordination | Consensus/quorum/election are not present. Filesystem lock scope is cataloged because it controls cross-host exclusion. |
| Lifecycle transitions | Acquire, held, drop, process death, restart, and rolling-version overlap represented. |
| Idempotency and replay | No request/replay protocol exists. Failed-attempt state preservation is cataloged. |
| Version compatibility | Lease-path persistence and shared cross-crate identity/FNV-1a derivation represented. |
| Wildcard | Trait-mode ambiguity, external backend divergence, untested MSRV, and stale README claim retained as leads. |
