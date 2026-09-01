# Existing-check inventory

All statuses are **unaudited**. Test adequacy belongs to `/testing:invariant-test-review`; production guard placement and strength belong to `/low-level-systems:defensive-assertions-and-invariant-guards`.

## Production checks and guards

No production `assert!`, `debug_assert!`, `panic!`, or equivalent invariant battery exists in `src/lib.rs:1-359`.

| Location | Check or branch | Semantics/message | Linked claims |
|---|---|---|---|
| `src/lib.rs:62-66` | `symlink_metadata` errors | Missing path returns `Ok`; other metadata errors propagate. | Optional sidecars may be absent. |
| `src/lib.rs:67-75` | Non-regular-file refusal | `InvalidInput`: “is not a regular file; refusing to change its permissions”. | Symlinks and other non-files are refused. |
| `src/lib.rs:76-78` | Mode normalization | If low permission bits differ, set `0600`. | Unix owner-only file. |
| `src/lib.rs:80-82` | Non-Unix branch | Returns `Ok` without checking or changing the path. | Platform contract remains open. |
| `src/lib.rs:240-254` | Directory/open/protect error propagation | Maps failures to `LeaseError::Io`; uses `truncate(false)`. | Preserve epoch on open; harden every acquisition. |
| `src/lib.rs:256-263` | Exclusive try-lock classification | Standard-library `File::try_lock`; `TryLockError::WouldBlock` becomes `Held`, while `TryLockError::Error` becomes `Io`. | Exclusive liveness gate and error taxonomy. |
| `src/lib.rs:265-269` | Epoch-bump error cleanup | Best-effort standard-library `File::unlock`, then `Io`. | Failed acquisition does not leak lock; state restoration is absent. |
| `src/lib.rs:278-287` | Shared directory/open/protect propagation | Same as exclusive path. | Shared reader setup. |
| `src/lib.rs:293-299` | Shared try-lock classification | Standard-library `File::try_lock_shared`; `TryLockError::WouldBlock` becomes `Held`, while `TryLockError::Error` becomes `Io`. | Shared/exclusive matrix. |
| `src/lib.rs:301-306` | Epoch-read error cleanup | Best-effort standard-library `File::unlock`, then `Io`. | Read failure does not leak lock. |
| `src/lib.rs:233-236` | Handle `Drop` | Best-effort standard-library `File::unlock`; error discarded; descriptor then closes. | Drop releases lease. |
| `src/lib.rs:319-324` | Epoch parse on shared path | Any valid-UTF-8 parse failure becomes zero. | New/empty file reads as zero; corruption is not guarded. |
| `src/lib.rs:328-339` | Epoch update | Parse-or-zero, saturating increment, truncate, write, flush. | Monotonic persisted epoch; overflow and interruption are unguarded. |
| `src/lib.rs:112-123,204-210,341-358` | Identity/path derivation | Public separator-joined identity and FNV functions feed the private `.lease` path helper. | Stable namespaced identity. |
| `src/lib.rs:134,168` | Trait bounds | Handles and stores are `Send + Sync`. | Cross-thread use compiles. |

## In-crate claim-bearing tests (13)

| Test | Location | Claim and exact oracle | Platform | Status |
|---|---|---|---|---|
| `an_acquired_lease_file_is_owner_only` | `src/lib.rs:398-424` | `mode == 0600`; message: lease stayed group/world writable. | Unix | unaudited |
| `protect_file_refuses_a_symlink_and_leaves_its_target_untouched` | `src/lib.rs:435-466` | `protect_file` errors and target remains `0644`; messages cover refusal and target chmod. | Unix | unaudited |
| `protect_file_ignores_a_missing_path` | `src/lib.rs:473-480` | Missing path returns `Ok`. | All; trivial on non-Unix | unaudited |
| `identity_hash_derivation_is_stable` | `src/lib.rs:482-488` | Pins one public identity string and its exact `fnv1a_hex` filename digest. | All | unaudited |
| `acquire_then_second_holder_is_rejected` | `src/lib.rs:491-506` | Second live exclusive is `Held`; re-acquired epoch is greater. | All | unaudited |
| `distinct_scopes_do_not_conflict` | `src/lib.rs:509-517` | Both acquire and both start at epoch 1. | All | unaudited |
| `distinct_modules_do_not_conflict_on_same_scope` | `src/lib.rs:520-532` | Both `.expect` calls succeed; second message says modules must not conflict. | All | unaudited |
| `distinct_backends_do_not_conflict_on_same_scope` | `src/lib.rs:535-545` | Both `.expect` calls succeed. | All | unaudited |
| `shared_holders_coexist_but_block_exclusive` | `src/lib.rs:548-579` | Two shared holders coexist; exclusive remains `Held` until last drop. | All | unaudited |
| `exclusive_holder_blocks_shared` | `src/lib.rs:582-597` | Shared is `Held` under exclusive, then succeeds after drop. | All | unaudited |
| `shared_acquisition_does_not_bump_the_write_epoch` | `src/lib.rs:600-624` | Writer 1, shared 1/1, writer 2. | All | unaudited |
| `shared_lease_across_processes_blocks_exclusive` | `src/lib.rs:632-691` | Python child holds shared lock; parent exclusive is `Held`, shared succeeds, exclusive succeeds after child exits. | Unix | unaudited |
| `epoch_persists_across_store_instances` | `src/lib.rs:694-706` | Fresh store instance observes epochs 1 then 2. | All | unaudited |

## Adjacent in-repo checks

These are outside the target crate but explicitly exercise or consume its contract.

| Test | Location | Claim | Status |
|---|---|---|---|
| `reopening_a_permissive_store_protects_the_database_and_its_wal` | `cortexkit-store/src/lib.rs:364-418` | Database and WAL are `0600` on reopen. | unaudited |
| `open_runs_migrations_and_seeds_once` | `cortexkit-store/src/lib.rs:457-478` | Store epochs are 1 then 2 across clean opens. | unaudited |
| `second_live_writer_is_rejected` | `cortexkit-store/src/lib.rs:481-490` | Second same-process store open is rejected as a lease error. | unaudited |
| `distinct_databases_do_not_falsely_contend` | `cortexkit-store/src/lib.rs:493-501` | Distinct database paths coexist. | unaudited |
| `superseded_writer_is_fenced_out_after_handover` | `cortexkit-store/src/lib.rs:629-667` | Synthetic epoch-1 writer cannot overwrite epoch-2 state. | unaudited |
| `equal_epoch_writer_is_not_fenced` | `cortexkit-store/src/lib.rs:670-689` | Equal epoch can continue writing. | unaudited |
| `open_migrate_and_single_writer` | `cortexkit-store-postgres/src/lib.rs:305-341` | Live PostgreSQL session lock rejects a second open; first epoch is `>= 1` and reopened epoch is `>= 2` after clean release. It does not compare the values directly. Skips locally unless `CORTEXKIT_TEST_PG_DSN` is set; CI has a required live job. | unaudited |
| `advisory_key_derivation_is_stable` | `cortexkit-store-postgres/src/lib.rs:375-381` | Pins one advisory bigint derived through public `LeaseKey::identity` and `fnv1a`. | unaudited |

The handover checks use `SqliteStore::for_test` (`cortexkit-store/src/lib.rs:105-128`), which bypasses real lease acquisition. They check fence logic, not an end-to-end handover.

## Explicitly absent checks

- Process death without unwind.
- Power-loss durability or directory-entry durability.
- I/O failure after epoch truncation.
- Malformed, oversized, restored, or maximum epoch files.
- Live lease-file unlink or replacement.
- Cross-process exclusive-versus-exclusive contention.
- Adversarial key fields or hash collisions.
- Deployed network/overlay filesystem semantics.
- Shared-handle epoch use at consumer write sites.
- Golden lease-path vectors or cross-version overlap.
- Situation-coverage assertions (`sometimes`/`reachable` equivalents).
- Property, fuzz, model-checking, Miri, or failpoint harnesses.
