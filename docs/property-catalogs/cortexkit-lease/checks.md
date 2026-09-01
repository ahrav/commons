# Existing-check inventory

All statuses are **unaudited**. Test adequacy belongs to `/testing:invariant-test-review`; production guard placement and strength belong to `/low-level-systems:defensive-assertions-and-invariant-guards`.

## Production checks and guards

No production `assert!`, `debug_assert!`, `panic!`, or equivalent invariant battery exists in `src/lib.rs:1-407`.

| Location | Check or branch | Semantics/message | Linked claims |
|---|---|---|---|
| `protect_file` (`src/lib.rs:25-54`) | Public path hardening | Unix `symlink_metadata` rejects non-regular paths; missing paths return `Ok`; `set_permissions` is path-based and does not open caller-owned files. | Static symlinks are not followed on Unix; trusted parent-directory ownership is a caller precondition. |
| `protect_open_file` (`src/lib.rs:56-77`) | Descriptor-relative lease-file checks | Non-regular descriptors return `InvalidInput`; Unix regular files are set to `0600`; Windows reparse points are rejected. | Lease acquisition validates and hardens its owned descriptor. |
| `lease_open_options`, `open_lease_file` (`src/lib.rs:89-143`) | Lease-file publication/open | Opens an existing final path first. On `NotFound`, it initializes a same-directory temporary inode to epoch zero and publishes with `persist_noclobber`; an `AlreadyExists` race reopens the winner within three attempts. | No empty final pathname is published; links, FIFOs, and reparse points fail closed. |
| `LeaseStore::acquire` (`src/lib.rs:280-303`) | Exclusive acquisition | Uses `try_lock`, classifies contention, and increments a validated epoch. | Exclusive liveness gate and error taxonomy. |
| `LeaseStore::acquire_shared` (`src/lib.rs:305-327`) | Shared acquisition | Uses `try_lock_shared` and reads without mutation. | Concurrent shared-first acquisition and exclusion matrix. |
| `FileLeaseHandle::drop` (`src/lib.rs:274-276`) | Handle `Drop` | Best-effort `File::unlock`; error discarded; descriptor then closes. | Drop releases lease. |
| `read_epoch` (`src/lib.rs:384-413`) | Bounded epoch parse | Reads at most 21 bytes into a bounded vector; existing empty state and anything except 1-20 ASCII digits in `u64` range are rejected. | Malformed, empty, oversized, and overflowing state fails closed. |
| `bump_epoch`, `persist_epoch` (`src/lib.rs:415-436`) | Epoch update | Checked increment; no truncate; fixed-width decimal overwrite with invalid-marker conversion only for 1-19 byte legacy states. | Exhaustion errors; ordered prefix writes cannot leave a lower parseable value in the injected model. |
| `LeaseKey::identity`, `FileLeaseStore::lease_path`, `fnv1a`, `fnv1a_hex` | Identity/path derivation | Public separator-joined identity and FNV functions feed the private `.lease` path helper. | Stable namespaced identity. |
| `LeaseHandle`, `LeaseStore` | Trait bounds | Handles and stores are `Send + Sync`. | Cross-thread use compiles. |

## In-crate claim-bearing tests (21)

| Test | Location | Claim and exact oracle | Platform | Status |
|---|---|---|---|---|
| `fresh_exclusive_initializes_to_one` | `src/lib.rs:476-500` | A fresh key returns epoch 1, writes exactly 20 decimal digits, and the published Unix file is `0600`. | All | unaudited |
| `shared_first_initializes_canonical_zero` | `src/lib.rs:502-519` | Shared-first creation observes canonical zero, blocks exclusive, then permits writer epoch 1 after drop. | All | unaudited |
| `concurrent_shared_first_acquisitions_coexist` | `src/lib.rs:521-584` | Eight synchronized fresh-key shared acquisitions all coexist at epoch zero. Report collection and holder release are both deadline-bounded, so a holder that dies before reporting fails the check instead of hanging the suite. | All | unaudited |
| `legacy_decimal_epoch_is_canonicalized` | `src/lib.rs:586-599` | Variable-width decimal 41 becomes epoch 42 in fixed-width form. | All | unaudited |
| `invalid_epoch_states_fail_closed` | `src/lib.rs:601-644` | Empty, malformed, oversized, and overflowing states return `LeaseError::Io(InvalidData)` through both acquisition modes and preserve bytes. | All | unaudited |
| `epoch_errors_keep_the_underlying_os_error` | `src/lib.rs:646-672` | Epoch read and persist failures expose the originating OS error through `Error::source`, so an errno is not flattened into the lease error's message. | All | unaudited |
| `maximum_epoch_is_readable_but_exhausted` | `src/lib.rs:674-698` | Shared acquisition reads `u64::MAX`; exclusive acquisition reports exhaustion and preserves bytes. | All | unaudited |
| `interrupted_persist_never_leaves_a_lower_parseable_epoch` | `src/lib.rs:700-812` | Injected ordered prefix-write failures exercise production `persist_epoch` and `read_epoch` for legacy-width and canonical-width prior states, including a carry; any parseable aftermath is not lower, completion is fixed-width, and the count of parseable aftermaths is asserted per case. | All, in-memory `Read + Write + Seek` seam | unaudited |
| `acquisition_refuses_symlink_and_leaves_target_untouched` | `src/lib.rs:815-841` | Exclusive and shared acquisition fail; target content and mode remain unchanged. | Unix | unaudited |
| `acquisition_refuses_fifo_without_blocking` | `src/lib.rs:844-863` | Both modes reject a Unix FIFO opened with `O_NONBLOCK`. | Unix | unaudited |
| `an_acquired_lease_file_is_owner_only` | `src/lib.rs:868-893` | `mode == 0600`; message: lease stayed group/world writable. | Unix | unaudited |
| `protect_file_refuses_a_symlink_and_leaves_its_target_untouched` | `src/lib.rs:903-930` | `protect_file` returns `InvalidInput` and target remains `0644`. | Unix | unaudited |
| `protect_file_ignores_a_missing_path` | `src/lib.rs:936-941` | Missing path returns `Ok`. | All; trivial on non-Unix | unaudited |
| `identity_hash_derivation_is_stable` | `src/lib.rs:945-949` | Pins one public identity string and exact filename digest. | All | unaudited |
| `acquire_then_second_holder_is_rejected` | `src/lib.rs:951-966` | Second live exclusive is `Held`; re-acquired epoch is greater. | All | unaudited |
| `distinct_identity_axes_do_not_conflict` | `src/lib.rs:968-988` | Distinct scopes, modules, and backends acquire independently at epoch 1. | All | unaudited |
| `shared_holders_coexist_but_block_exclusive` | `src/lib.rs:990-1021` | Two shared holders coexist; exclusive remains `Held` until last drop. | All | unaudited |
| `exclusive_holder_blocks_shared` | `src/lib.rs:1023-1038` | Shared is `Held` under exclusive, then succeeds after drop. | All | unaudited |
| `shared_acquisition_does_not_bump_the_write_epoch` | `src/lib.rs:1040-1064` | Writer 1, shared 1/1, writer 2. | All | unaudited |
| `shared_lease_across_processes_blocks_exclusive` | `src/lib.rs:1071-1130` | Python child holds shared lock; parent exclusive is `Held`, shared succeeds, exclusive succeeds after child exits. | Unix | unaudited |
| `epoch_persists_across_store_instances` | `src/lib.rs:1132-1144` | Fresh store instance observes epochs 1 then 2. | All | unaudited |

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
- Real `File` I/O failure after a positive write prefix.
- Runtime Windows reparse-point and lock-conversion behavior; Windows is compile-checked only.
- Restored older valid epoch files.
- Live lease-file unlink or replacement.
- Cross-process exclusive-versus-exclusive contention.
- Adversarial key fields or hash collisions.
- Deployed network/overlay filesystem semantics.
- Shared-handle epoch use at consumer write sites.
- Expanded golden lease-path vectors or cross-version overlap.
- Situation-coverage assertions (`sometimes`/`reachable` equivalents).
- Property, fuzz, model-checking, Miri, or failpoint harnesses.
