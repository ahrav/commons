# Existing-check inventory

All statuses are **unaudited**. Test adequacy belongs to `/testing:invariant-test-review`; production guard placement and strength belong to `/low-level-systems:defensive-assertions-and-invariant-guards`.

## Production checks and guards

No production `assert!`, `debug_assert!`, `panic!`, or equivalent invariant battery exists in `src/lib.rs:1-471`.

| Location | Check or branch | Semantics/message | Linked claims |
|---|---|---|---|
| `protect_file` (`src/lib.rs:35-54`) | Public path hardening | Unix `symlink_metadata` rejects non-regular paths; missing paths return `Ok`; `set_permissions` is path-based and does not open caller-owned files. | Static symlinks are not followed on Unix; trusted parent-directory ownership is a caller precondition. |
| `protect_open_file` (`src/lib.rs:56-77`) | Descriptor-relative lease-file checks | Non-regular descriptors return `InvalidInput`; Unix regular files are set to `0600`; Windows reparse points are rejected. | Lease acquisition validates and hardens its owned descriptor. |
| `lease_open_options`, `open_lease_file` (`src/lib.rs:89-104,106-143`) | Lease-file publication/open | Opens an existing final path first. On `NotFound`, it initializes a same-directory temporary inode to epoch zero and publishes with `persist_noclobber`; an `AlreadyExists` race reopens the winner within three attempts. | No empty final pathname is published; links, FIFOs, and reparse points fail closed. |
| `FileLeaseStore::acquire_above`, `LeaseStore::acquire` (`src/lib.rs:260-286,316-318`) | Exclusive acquisition | Uses `try_lock`, classifies contention, and issues an epoch above the persisted value and a caller-supplied floor. Trait acquisition reuses floor zero. | Exclusive liveness gate, error taxonomy, and durable-resource epoch recovery. |
| `LeaseStore::acquire_shared` (`src/lib.rs:320-342`) | Shared acquisition | Uses `try_lock_shared` and reads without mutation. | Concurrent shared-first acquisition and exclusion matrix. |
| `FileLeaseHandle::drop` (`src/lib.rs:310-312`) | Handle `Drop` | Best-effort `File::unlock`; error discarded; descriptor then closes. | Drop releases lease. |
| `read_epoch` (`src/lib.rs:400-428`) | Bounded epoch parse | Reads at most 21 bytes into a bounded vector; existing empty state and anything except 1-20 ASCII digits in `u64` range are rejected. | Malformed, empty, oversized, and overflowing state fails closed. |
| `bump_epoch_above`, `persist_epoch` (`src/lib.rs:431-439,444-452`) | Epoch update | Checked increment above both persisted state and floor; no truncate; fixed-width decimal overwrite with invalid-marker conversion only for 1-19 byte legacy states. | Exhaustion errors; ordered prefix writes cannot leave a lower parseable value in the injected model. |
| `LeaseKey::identity`, `FileLeaseStore::lease_path`, `fnv1a`, `fnv1a_hex` | Identity/path derivation | Public separator-joined identity and FNV functions feed the private `.lease` path helper. | Stable namespaced identity. |
| `LeaseHandle`, `LeaseStore` | Trait bounds | Handles and stores are `Send + Sync`. | Cross-thread use compiles. |

## In-crate claim-bearing tests (22)

| Test | Location | Claim and exact oracle | Platform | Status |
|---|---|---|---|---|
| `fresh_exclusive_initializes_to_one` | `src/lib.rs:492-516` | A fresh key returns epoch 1, writes exactly 20 decimal digits, and the published Unix file is `0600`. | All | unaudited |
| `exclusive_epoch_exceeds_resource_floor` | `src/lib.rs:518-530` | Persisted epoch 41 with floor 100 issues 101; ordinary reacquisition then issues 102. | All | unaudited |
| `shared_first_initializes_canonical_zero` | `src/lib.rs:532-549` | Shared-first creation observes canonical zero, blocks exclusive, then permits writer epoch 1 after drop. | All | unaudited |
| `concurrent_shared_first_acquisitions_coexist` | `src/lib.rs:551-614` | Eight synchronized fresh-key shared acquisitions all coexist at epoch zero. Report collection and holder release are both deadline-bounded, so a holder that dies before reporting fails the check instead of hanging the suite. | All | unaudited |
| `legacy_decimal_epoch_is_canonicalized` | `src/lib.rs:616-629` | Variable-width decimal 41 becomes epoch 42 in fixed-width form. | All | unaudited |
| `invalid_epoch_states_fail_closed` | `src/lib.rs:631-674` | Empty, malformed, oversized, and overflowing states return `LeaseError::Io(InvalidData)` through both acquisition modes and preserve bytes. | All | unaudited |
| `epoch_errors_keep_the_underlying_os_error` | `src/lib.rs:676-702` | Epoch error context preserves the original `io::Error` and raw OS error through the source chain. | All | unaudited |
| `maximum_epoch_is_readable_but_exhausted` | `src/lib.rs:704-728` | Shared acquisition reads `u64::MAX`; exclusive acquisition reports exhaustion and preserves bytes. | All | unaudited |
| `interrupted_persist_never_leaves_a_lower_parseable_epoch` | `src/lib.rs:730-842` | Injected ordered prefix-write failures exercise production `persist_epoch` and `read_epoch` for legacy-width and canonical-width prior states, including a carry; any parseable aftermath is not lower, completion is fixed-width, and the count of parseable aftermaths is asserted per case. | All, in-memory `Read + Write + Seek` seam | unaudited |
| `acquisition_refuses_symlink_and_leaves_target_untouched` | `src/lib.rs:844-871` | Exclusive and shared acquisition fail; target content and mode remain unchanged. | Unix | unaudited |
| `acquisition_refuses_fifo_without_blocking` | `src/lib.rs:873-893` | Both modes reject a Unix FIFO opened with `O_NONBLOCK`. | Unix | unaudited |
| `an_acquired_lease_file_is_owner_only` | `src/lib.rs:895-923` | `mode == 0600`; message: lease stayed group/world writable. | Unix | unaudited |
| `protect_file_refuses_a_symlink_and_leaves_its_target_untouched` | `src/lib.rs:925-960` | `protect_file` returns `InvalidInput` and target remains `0644`. | Unix | unaudited |
| `protect_file_ignores_a_missing_path` | `src/lib.rs:962-971` | Missing path returns `Ok`. | All; trivial on non-Unix | unaudited |
| `identity_hash_derivation_is_stable` | `src/lib.rs:973-979` | Pins one public identity string and exact filename digest. | All | unaudited |
| `acquire_then_second_holder_is_rejected` | `src/lib.rs:981-996` | Second live exclusive is `Held`; re-acquired epoch is greater. | All | unaudited |
| `distinct_identity_axes_do_not_conflict` | `src/lib.rs:998-1018` | Distinct scopes, modules, and backends acquire independently at epoch 1. | All | unaudited |
| `shared_holders_coexist_but_block_exclusive` | `src/lib.rs:1020-1051` | Two shared holders coexist; exclusive remains `Held` until last drop. | All | unaudited |
| `exclusive_holder_blocks_shared` | `src/lib.rs:1053-1068` | Shared is `Held` under exclusive, then succeeds after drop. | All | unaudited |
| `shared_acquisition_does_not_bump_the_write_epoch` | `src/lib.rs:1070-1094` | Writer 1, shared 1/1, writer 2. | All | unaudited |
| `shared_lease_across_processes_blocks_exclusive` | `src/lib.rs:1096-1160` | Python child holds shared lock; parent exclusive is `Held`, shared succeeds, exclusive succeeds after child exits. | Unix | unaudited |
| `epoch_persists_across_store_instances` | `src/lib.rs:1162-1174` | Fresh store instance observes epochs 1 then 2. | All | unaudited |

## Adjacent in-repo checks

These are outside the target crate but explicitly exercise or consume its contract.

| Test | Location | Claim | Status |
|---|---|---|---|
| `reopening_a_permissive_store_protects_the_database_and_its_wal` | `cortexkit-store/src/lib.rs:476-531` | Database and WAL are `0600` on reopen. | unaudited |
| `open_claims_fence_before_return` | `cortexkit-store/src/lib.rs:569-582` | Open stamps the lease epoch before exposing the store. | unaudited |
| `open_claim_rejects_an_epoch_the_database_already_stores` | `cortexkit-store/src/lib.rs:588-625` | The open claim rejects an epoch equal to the stored fence; `claim_fence` still authorizes it. | unaudited |
| `migrations_seed_once_across_reopen` | `cortexkit-store/src/lib.rs:627-649` | Migrations and seeds run once; clean reopen issues a greater epoch. | unaudited |
| `database_epoch_survives_repeated_lease_sidecar_loss` | `cortexkit-store/src/lib.rs:651-678` | Two repeated sidecar losses each issue an epoch above the database fence. | unaudited |
| `second_live_writer_is_rejected` | `cortexkit-store/src/lib.rs:692-702` | Second same-process store open is rejected as a lease error. | unaudited |
| `distinct_databases_do_not_falsely_contend` | `cortexkit-store/src/lib.rs:704-713` | Distinct database paths coexist. | unaudited |
| `fenced_write_rolls_back_on_error` | `cortexkit-store/src/lib.rs:819-853` | Callback failure rolls back both domain mutation and a newer fence claim. | unaudited |
| `legacy_database_without_fence_table_uses_zero_floor` | `cortexkit-store/src/lib.rs:855-878` | A pre-fence-table database opens at floor zero and receives epoch 1. | unaudited |
| `legacy_negative_database_fence_fails_closed` | `cortexkit-store/src/lib.rs:880-908` | A pre-constraint negative fence is rejected and remains unchanged. | unaudited |
| `superseded_writer_is_fenced_out_after_handover` | `cortexkit-store/src/lib.rs:910-949` | Synthetic epoch-1 writer cannot overwrite epoch-2 state. | unaudited |
| `superseded_writer_cannot_migrate` | `cortexkit-store/src/lib.rs:951-982` | Synthetic stale migration is fenced before its schema SQL executes. | unaudited |
| `equal_epoch_writer_is_not_fenced` | `cortexkit-store/src/lib.rs:984-1005` | Equal epoch can continue writing. | unaudited |
| `epoch_above_sqlite_integer_range_fails` | `cortexkit-store/src/lib.rs:1007-1023` | Epochs above SQLite's signed integer range fail instead of wrapping. | unaudited |
| `open_migrate_and_single_writer` | `cortexkit-store-postgres/src/lib.rs:431-463` | Live PostgreSQL covers migration and session exclusion. Requires `CORTEXKIT_TEST_PG_DSN`; CI has a required live job. | unaudited |
| `read_only_callback_rejects_mutation_without_rows` | `cortexkit-store-postgres/src/lib.rs:487-520` | Read-only mutation reports SQLSTATE `25006` and leaves rows unchanged. | unaudited |
| `unfenced_callback_runs_statements_a_transaction_forbids` | `cortexkit-store-postgres/src/lib.rs:465-485` | `VACUUM` reports SQLSTATE `25001` inside a fenced transaction and succeeds through the autocommit callback. | unaudited |
| `fenced_callback_error_rolls_back_rows` | `cortexkit-store-postgres/src/lib.rs:522-556` | Callback failure rolls back domain rows. | unaudited |
| `repeated_fenced_writes_at_current_epoch_succeed` | `cortexkit-store-postgres/src/lib.rs:558-578` | Repeated writes at the current lease epoch succeed. | unaudited |
| `superseded_writer_is_rejected_after_reopen` | `cortexkit-store-postgres/src/lib.rs:580-612` | Synthetic stale callback is rejected after reopen. | unaudited |
| `superseded_writer_cannot_migrate` | `cortexkit-store-postgres/src/lib.rs:614-650` | Synthetic stale migration is fenced before its schema SQL executes. | unaudited |
| `independent_namespace_chains` | `cortexkit-store-postgres/src/lib.rs:652-682` | Independent migrations both apply. | unaudited |
| `advisory_key_derivation_is_stable` | `cortexkit-store-postgres/src/lib.rs:686-690` | Pins one advisory bigint derived through public `LeaseKey::identity` and `fnv1a`. | unaudited |

The handover checks use synthetic stores that bypass real lease acquisition. They check fence logic against real database transactions, not an end-to-end retained-connection handover.

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
