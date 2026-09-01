# `cortexkit-lease` property catalog

## Provenance and scope

- System: `crates/cortexkit-lease`
- Revision: `fa975843afd4b3122288149968ea5d6ff46322b3`
- Date: 2026-08-30
- External-evidence answer: **none**. No external design docs, related repositories, issue trackers, incident reports, or known failure reports were supplied.
- In-repo references consulted:
  - `crates/cortexkit-lease/src/lib.rs`: implementation, claims, and unit checks.
  - `crates/cortexkit-store/src/lib.rs`: the in-repo file-lease consumer and fence enforcement point.
  - `crates/cortexkit-store-postgres/src/lib.rs`: a second backend that shares the lease identity and FNV-1a derivation and persists its own epoch.
  - `crates/cortexkit-store-types/src/lib.rs`: provenance of lease-key fields.
  - `docs/lease-store-density.md`: measured file growth, the never-unlink rationale, and migration constraints.
  - `README.md` and `.github/workflows/ci.yml`: ownership, versioning, cross-process-test, and platform claims.
  - Git commits `8abefe8`, `16aed47`, `49bcaa2`, `bed0bb7`, `8da6d42`, `f2107e5`, and `94c65ec`: author-stated intent, regression history, standard-library lock migration, shared derivation, and version/MSRV changes.

All documentation and history statements are leads. They establish intended contracts, not implementation correctness. The target crate was clean at the recorded revision; unrelated untracked artifacts were left untouched.

Observation constraint: `LeaseKey::identity`, `fnv1a`, and `fnv1a_hex` are public (`src/lib.rs:112-123,341-358`); only `FileLeaseStore::lease_path` remains private (`src/lib.rs:204-210`). `LeaseKey` derives neither `Hash` nor `Ord` (`src/lib.rs:92`). Checks index logical keys by the field tuple `(module_id, backend, scope_key)`. External checks can verify identity and hash vectors directly, but exact path checks must live inside this crate or infer the fixed `.lease` convention from public outputs.

The parked dual-store migration sketch in `docs/lease-store-density.md:44-51` is outside this catalog because that system is not built. Its stated prerequisite depends on `returned-epoch-is-crash-durable`; the relationship map records that dependency.

Supporting artifacts:

- [System model](system-model.md)
- [Existing-check inventory](checks.md)
- [Fault-to-property map](fault-map.md)
- [Relationship map](relationships.md)
- [Portfolio evaluation](portfolio.md)
- [Per-property evidence](evidence/)

## Property catalog

### `at-most-one-exclusive-holder-per-key`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet. Existing checks cover sequential same-process contention, not a concurrent exclusive race across independent processes.
- **Guarantee:** Among cooperative participants using the same lease root and `LeaseKey`, at most one live handle returned by `LeaseStore::acquire` exists at every instant.
- **Check:** `always(exclusive_live_count[(physical_root_identity, module_id, backend, scope_key)] <= 1)`, where physical root identity is canonicalized and, on Unix, confirmed by device/inode rather than raw path spelling. Each successful holder records that identity, process, epoch, acquire-return time, and release time in a witness ledger outside the lease root; the oracle rejects overlapping intervals.
- **Fault/timing angle:** Two acquirers race from separate processes; path aliasing, inode replacement, or degraded filesystem lock semantics can let both return `Ok`.
- **Required faults and enabling state:** Two processes must have the same lease file open concurrently and both must attempt exclusive locking. For faulted histories, also inject path aliasing, file replacement, or the deployed filesystem's lock-degradation mode.
- **Confidence:** high. The contract is explicit at `src/lib.rs:3-5`; the implementation delegates it to `File::try_lock` at `src/lib.rs:239-276`.
- **Existing check:** `acquire_then_second_holder_is_rejected`, `src/lib.rs:491-506`, same process and sequential; status **unaudited**.
- **Impact:** Two writers can mutate one logical store. This is the crate's primary prohibited state.
- **Open questions:** Does the real-daemon two-process check claimed by `README.md:12-13` live in an external repository? `(needs human input)`
- **Evidence:** [evidence/at-most-one-exclusive-holder-per-key.md](evidence/at-most-one-exclusive-holder-per-key.md)

### `shared-exclusive-exclusion-matrix`

- **Type:** safety
- **Status:** active
- **Exercised:** yes, on the local Unix test path; platform and filesystem variants remain unexercised.
- **Guarantee:** At least two shared holders can coexist, but a live exclusive holder and any live shared holder never coexist for one root and key.
- **Check:** Safety: `always(exclusive_count <= 1 && (exclusive_count == 0 || shared_count == 0))`. Availability coverage: `sometimes(shared_count >= 2)`. The first forbids mixed modes; the second proves the documented positive shared-coexistence path occurred.
- **Fault/timing angle:** The last-of-many shared-holder drop is the discriminating transition; per-process lock semantics can release another handle's lock early.
- **Required faults and enabling state:** At least two simultaneous shared holders, an exclusive attempt while both live, another attempt after one drops, and the reverse exclusive-then-shared history.
- **Confidence:** high. The contract is explicit at `src/lib.rs:173-186`; the implementation uses shared and exclusive OS locks.
- **Existing check:** `shared_holders_coexist_but_block_exclusive`, `exclusive_holder_blocks_shared`, and `shared_lease_across_processes_blocks_exclusive`, `src/lib.rs:548-691`; status **unaudited**.
- **Impact:** A GC can delete a resource under a live reader, or readers can enter while an exclusive mutator is active.
- **Open questions:** Are Solaris or network filesystems supported, where the lock primitive may be process-scoped or host-scoped? `(needs human input)`
- **Evidence:** [evidence/shared-exclusive-exclusion-matrix.md](evidence/shared-exclusive-exclusion-matrix.md)

### `dead-holder-lease-is-reclaimable`

- **Type:** liveness
- **Status:** active
- **Exercised:** not yet. The cross-process test lets its child exit cleanly.
- **Guarantee:** After a holder process dies without running `Drop`, another process can acquire the same key within the recovery bound.
- **Check:** With a configured recovery bound `B`, after process death is confirmed and no unrelated holder exists, attempt acquisition until deadline `death_confirmed + B`; assert `always(acquired_by_deadline)`. The configured deadline makes the eventual claim exact.
- **Fault/timing angle:** `SIGKILL`, abort, OOM kill, or equivalent termination while the handle is live.
- **Required faults and enabling state:** A child must hold the real OS lock and be terminated without unwind; the harness must confirm process exit before starting the recovery deadline.
- **Confidence:** high that this is intended (`src/lib.rs:8-10`); medium that every deployed filesystem supplies the promised behavior.
- **Existing check:** `shared_lease_across_processes_blocks_exclusive`, `src/lib.rs:632-691`, exits normally; status **unaudited**.
- **Impact:** A dead writer can permanently prevent module restart.
- **Open questions:** What recovery bound is operationally required? `(needs human input)`
- **Evidence:** [evidence/dead-holder-lease-is-reclaimable.md](evidence/dead-holder-lease-is-reclaimable.md)

### `writer-epoch-strictly-increases`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet under corruption, interruption, restore, or exhaustion; clean two-step examples exist.
- **Guarantee:** Every successful exclusive acquisition returns an epoch strictly greater than every epoch previously returned for the same physical root and key.
- **Check:** On each successful exclusive acquisition, `always(epoch > max_returned_epoch[(physical_root_identity, module_id, backend, scope_key)])`, then update the witness. This is the exact per-file fence-token contract at `src/lib.rs:135-137`.
- **Fault/timing angle:** Truncate-before-write interruption, unsynced write loss, valid-UTF-8 corruption, old-file restore, and a persisted `u64::MAX` all threaten strict increase.
- **Required faults and enabling state:** At least one prior successful acquisition, followed separately by each fault class. The maximum-value case requires the parser to observe `18446744073709551615` and then two consecutive successful acquisitions; counting to it is not required.
- **Confidence:** high. The contract is explicit; `parse().unwrap_or(0)` and `saturating_add(1)` at `src/lib.rs:328-337` provide discriminating contradictory code evidence.
- **Existing check:** `acquire_then_second_holder_is_rejected` and `epoch_persists_across_store_instances`, `src/lib.rs:491-506,694-706`; status **unaudited**.
- **Impact:** Reused or regressed epochs let a superseded writer pass an equal-or-older fence or can permanently reject legitimate writers.
- **Open questions:** Was saturation chosen deliberately? No rationale appears in code.
- **Evidence:** [evidence/writer-epoch-strictly-increases.md](evidence/writer-epoch-strictly-increases.md)

### `returned-epoch-is-crash-durable`

- **Type:** safety
- **Status:** active — provisional crash-model interpretation
- **Exercised:** not yet; store-instance recreation does not test device durability.
- **Guarantee:** Once `acquire` returns epoch `e`, a power loss cannot cause a later acquisition for the same physical root and key to return an epoch less than or equal to `e`.
- **Check:** Witness by `(physical_root_identity, module_id, backend, scope_key)`; after each acknowledged acquisition and crash-image recovery, assert `always(reacquired_epoch > acknowledged_epoch)`. This checks the durability promise at the acknowledgement boundary.
- **Fault/timing angle:** Power loss after file creation or epoch write but before file data and directory entry reach stable storage.
- **Required faults and enabling state:** A first-ever acquisition to exercise directory-entry durability, a later acquisition to exercise content durability, and volatile-cache loss rather than process death alone.
- **Confidence:** high that the docs claim durability; high that no `sync_data`, `sync_all`, or directory sync exists. `File::flush` at `src/lib.rs:337` is not a durability barrier.
- **Existing check:** `epoch_persists_across_store_instances`, `src/lib.rs:694-706`; status **unaudited**.
- **Impact:** A post-reboot writer can reuse a superseded writer's token.
- **Open questions:** Confirm whether the documented word “durable” includes machine power loss. This record adopts that conventional interpretation because no narrower crash model is documented. `(needs human input)`
- **Evidence:** [evidence/returned-epoch-is-crash-durable.md](evidence/returned-epoch-is-crash-durable.md)

### `invalid-epoch-fails-closed`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet. Existing tests only use an empty new file or valid decimal content.
- **Guarantee:** Non-empty lease content that is not a valid `u64` causes acquisition to fail without issuing an epoch.
- **Check:** For each invalid non-empty body, use the strongest current check `always(acquire(key).is_err() && bytes_after == bytes_before)`. A future diagnostic form requires a corruption-specific error distinguishable from device and permission failures; `LeaseError` currently exposes only `Held` and `Io` (`src/lib.rs:143-150`).
- **Fault/timing angle:** Valid-UTF-8 garbage, decimal overflow, foreign non-decimal writes, and future-format bytes. A torn body that remains valid decimal belongs to epoch monotonicity, not this parse-failure property.
- **Required faults and enabling state:** Place malformed content in the exact derived lease path while no holder is live; include invalid UTF-8 as a comparison because it already fails closed through `read_to_string`.
- **Confidence:** high. `read_epoch` and `bump_epoch` use `parse().unwrap_or(0)` at `src/lib.rs:323,332`, so valid-UTF-8 garbage currently resets to zero.
- **Existing check:** none.
- **Impact:** Silent reset to epoch 1 reissues old fence tokens.
- **Open questions:** Should an empty existing file also fail closed, or remain the representation of a new key? `(needs human input)`
- **Evidence:** [evidence/invalid-epoch-fails-closed.md](evidence/invalid-epoch-fails-closed.md)

### `failed-acquire-preserves-prior-epoch`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet; no write-path error is injected.
- **Guarantee:** An exclusive acquisition that returns `Err` does not lower, erase, or corrupt the prior persisted epoch.
- **Check:** Around each forced post-lock failure, `always(after_epoch_bytes parse to a value >= before_epoch)`. This checks durable state, not only that the lock was released.
- **Fault/timing angle:** `ENOSPC`, `EDQUOT`, or returned `EIO` after `set_len(0)` and before `write_all` completes. Non-returning termination belongs to crash recovery, not this property.
- **Required faults and enabling state:** A prior nonzero epoch and an injected I/O error after `src/lib.rs:334` succeeds, with `acquire` returning `Err`.
- **Confidence:** high. The error path unlocks at `src/lib.rs:265-269` but does not restore bytes truncated at `src/lib.rs:334`.
- **Existing check:** none.
- **Impact:** A transient storage error turns into permanent fence regression.
- **Open questions:** A replace-by-rename repair conflicts with the documented stable-inode requirement; what update protocol is intended? `(needs human input)`
- **Evidence:** [evidence/failed-acquire-preserves-prior-epoch.md](evidence/failed-acquire-preserves-prior-epoch.md)

### `distinct-lease-keys-do-not-alias`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet for adversarial values; simple separator-free examples exist.
- **Guarantee:** Distinct `LeaseKey` values never map to the same lease file unless a collision is detected and rejected.
- **Check:** Inside the crate, `always(k1 == k2 || lease_path(k1) != lease_path(k2))` for generated and adversarial pairs. The file stores no identity, so collision detection by stored-key verification is a design follow-up, not an available check.
- **Fault/timing angle:** No timing fault is needed. A field containing `U+001F` makes the tuple encoding ambiguous; FNV-1a-64 also has no collision handling.
- **Required faults and enabling state:** Construct keys containing the separator in different fields. A targeted FNV collision is a separate enabling state whose practical cost remains open.
- **Confidence:** high for the separator witness; high that collision handling is absent; low on practical targeted-FNV cost.
- **Existing check:** `distinct_scopes_do_not_conflict`, `distinct_modules_do_not_conflict_on_same_scope`, and `distinct_backends_do_not_conflict_on_same_scope`, `src/lib.rs:509-545`; status **unaudited**.
- **Impact:** Unrelated stores falsely contend and share one epoch sequence.
- **Open questions:** Are key fields attacker-controlled in any external consumer? `(needs human input)`
- **Evidence:** [evidence/distinct-lease-keys-do-not-alias.md](evidence/distinct-lease-keys-do-not-alias.md)

### `lease-inode-remains-stable-while-held`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** Replacing a held lease path never permits a competing acquisition to succeed on a different inode for the same logical root and key.
- **Check:** During replacement histories, `always(competing_acquire_succeeded => competing_inode == incumbent_inode)`. Path/inode divergence is the enabling state, not itself the forbidden outcome.
- **Fault/timing angle:** Unlink, rename, restore, bind-mount replacement, or cleanup while the old inode remains locked through an open descriptor.
- **Required faults and enabling state:** A live holder, external replacement of its lease path, then a second acquisition.
- **Confidence:** high. The hazard follows from descriptor-bound locks, and `docs/lease-store-density.md:22-24` explicitly names the unlink-inode race.
- **Existing check:** none.
- **Impact:** Two exclusive locks can succeed on two inodes for one logical key, and both epoch sequences can restart.
- **Open questions:** Which deployed actors can unlink or replace lease files? `(needs human input)`
- **Evidence:** [evidence/lease-inode-remains-stable-while-held.md](evidence/lease-inode-remains-stable-while-held.md)

### `shared-acquisition-is-epoch-neutral`

- **Type:** safety
- **Status:** active
- **Exercised:** yes, for serial shared acquisitions on the local filesystem.
- **Guarantee:** Shared acquisition never changes the persisted writer epoch.
- **Check:** `always(epoch_bytes_after_shared_acquire == epoch_bytes_before_shared_acquire)`. Byte equality is stronger and more direct than comparing parsed handle values.
- **Fault/timing angle:** Concurrent shared holders matter because a future refactor that writes metadata into the file can create lost updates or consume fence values.
- **Required faults and enabling state:** A nonzero writer epoch and at least two simultaneous shared holders in the same history; no injected fault is required.
- **Confidence:** high. `read_epoch` at `src/lib.rs:319-324` performs no content write.
- **Existing check:** `shared_acquisition_does_not_bump_the_write_epoch`, `src/lib.rs:600-624`; status **unaudited**.
- **Impact:** Readers consuming writer epochs can prematurely fence legitimate writers.
- **Open questions:** None. This record is explicitly limited to epoch bytes; metadata effects are covered by permission and failed-acquisition records.
- **Evidence:** [evidence/shared-acquisition-is-epoch-neutral.md](evidence/shared-acquisition-is-epoch-neutral.md)

### `shared-epoch-never-authorizes-write`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet. No in-repo production caller uses `acquire_shared`.
- **Guarantee:** An epoch obtained from a shared handle is never used to authorize or stamp a durable write.
- **Check:** No runtime mode check exists because `LeaseHandle` exposes no mode. The available check is source-level: `always(no value returned by acquire_shared reaches a durable write fence or stamp)` across every consumer. If the interface later carries mode, add `always(handle.mode == Exclusive)` at every write-fence boundary.
- **Fault/timing angle:** No fault is needed. Both acquisition methods return the same erased `Box<dyn LeaseHandle>`, while shared handles report the incumbent writer epoch.
- **Required faults and enabling state:** A consumer that accepts both handle modes and routes `epoch()` to a durable write path.
- **Confidence:** high that the type cannot distinguish modes (`src/lib.rs:134-187`); low that a current unseen consumer misuses it.
- **Existing check:** `shared_acquisition_does_not_bump_the_write_epoch`, `src/lib.rs:600-624`, pins equal epoch values but not their use; status **unaudited**.
- **Impact:** A reader can present the live writer's epoch as write authority.
- **Open questions:** Which external repository consumes shared mode, and should the interface split reader and writer handles or expose mode? `(needs human input)`
- **Evidence:** [evidence/shared-epoch-never-authorizes-write.md](evidence/shared-epoch-never-authorizes-write.md)

### `unix-lease-file-is-owner-only`

- **Type:** safety
- **Status:** active
- **Exercised:** yes on Unix for an exclusive acquisition over a pre-existing permissive file; shared acquisition and creation-window coverage remain absent.
- **Guarantee:** After a successful Unix acquisition, the locked lease file's permission bits are exactly `0600`.
- **Check:** `always(mode(locked_inode) & 0o777 == 0o600)` after both exclusive and shared acquisition. Platform qualification is explicit; non-Unix behavior is not imported into this claim.
- **Fault/timing angle:** Pre-existing `0644` files, restores, and copies. Creation-window exposure is a separate property.
- **Required faults and enabling state:** Exercise exclusive and shared acquisition against pre-existing permissive files, including replacement between descriptor open and path-based hardening. Confirm the opened/locked inode is the inode whose mode is checked.
- **Confidence:** high for the intended Unix outcome; commit `49bcaa2` records the observed permissive deployment state and the fix rationale.
- **Existing check:** `an_acquired_lease_file_is_owner_only`, `src/lib.rs:398-424`, Unix-only; status **unaudited**.
- **Impact:** A writable lease file allows fence-token forgery. A readable file exposes key activity, though filenames are hashed.
- **Open questions:** What security contract, if any, applies on Windows, where `protect_file` returns success without work? `(needs human input)`
- **Evidence:** [evidence/unix-lease-file-is-owner-only.md](evidence/unix-lease-file-is-owner-only.md)

### `permission-hardening-never-follows-replacement`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet for a concurrent replacement; static symlink refusal is covered.
- **Guarantee:** `protect_file` changes permissions only on the regular inode it inspected and never on a replacement target.
- **Check:** Whenever the chmod branch executes, `always(inspected_inode == chmod_target_inode)`, and assert every unrelated target's mode is unchanged. Pre-open symlink following is a separate property.
- **Fault/timing angle:** Replace the final path component with a symlink between `symlink_metadata` at `src/lib.rs:62` and path-based `set_permissions` at `src/lib.rs:77`.
- **Required faults and enabling state:** Directory mutation permission plus a deterministic pause after metadata inspection and before chmod.
- **Confidence:** high that the check-then-act window exists; medium on exploitability in deployment.
- **Existing check:** `protect_file_refuses_a_symlink_and_leaves_its_target_untouched`, `src/lib.rs:435-466`, static case; status **unaudited**.
- **Impact:** Permission changes can land on a file the caller never named; acquisition may also create a symlink target before refusal.
- **Open questions:** Is the process ever privileged, and are lease directories writable by another principal? `(needs human input)`
- **Evidence:** [evidence/permission-hardening-never-follows-replacement.md](evidence/permission-hardening-never-follows-replacement.md)

### `contention-is-classified-as-held`

- **Type:** safety
- **Status:** active
- **Exercised:** yes on the local platform for ordinary contention; non-contention lock errors and all supported targets remain incomplete.
- **Guarantee:** A contended try-lock returns `LeaseError::Held`, while every other lock failure returns `LeaseError::Io`.
- **Check:** Positive arm: while a known live holder exists, `always(matches!(result, Err(LeaseError::Held { .. })))`. Negative arm: for each injected non-contention lock error, `always(matches!(result, Err(LeaseError::Io(_))))`. The arms use different ground-truth mechanisms and are not collapsed into one biconditional.
- **Fault/timing angle:** Platform-specific raw OS codes and filesystems that report unsupported/exhausted lock resources rather than the normal contention code.
- **Required faults and enabling state:** Genuine contention for the positive arm; injected `EACCES`, `ENOLCK`, `EOPNOTSUPP`, or target equivalents for the negative arm.
- **Confidence:** high on Linux/macOS/Windows ordinary contention; lower on unsupported targets and filesystems.
- **Existing check:** contention tests at `src/lib.rs:491-691`; status **unaudited**.
- **Impact:** Callers can mistake a live holder for storage failure or a broken lock facility for ordinary contention.
- **Open questions:** Is the CI matrix the complete supported platform set? `(needs human input)`
- **Evidence:** [evidence/contention-is-classified-as-held.md](evidence/contention-is-classified-as-held.md)

### `filesystem-lock-scope-matches-deployment`

- **Type:** safety
- **Status:** active
- **Exercised:** cannot be exercised from this repository; mount and host topology are deployment evidence.
- **Guarantee:** Every cooperative writer using the lease protocol and able to access a shared lease root participates in one lock domain for that root.
- **Check:** For each deployed mount configuration, `always(acquire_on_B == Held)` while A holds the same key; run B on another host whenever the mount is shared.
- **Fault/timing angle:** Node-local advisory locking, unsupported locking, overlay replacement, or process-scoped emulation.
- **Required faults and enabling state:** The real deployment filesystem and mount options, with concurrent acquirers in every host/process topology that can access it.
- **Confidence:** medium. The crate accepts arbitrary paths and documents no filesystem contract.
- **Existing check:** local-temp-directory cross-process shared contention, `src/lib.rs:632-691`; status **unaudited**.
- **Impact:** Both hosts can believe they exclusively own one store.
- **Open questions:** Where does each external consumer place its lease root, and with what mount options? `(needs human input)`
- **Evidence:** [evidence/filesystem-lock-scope-matches-deployment.md](evidence/filesystem-lock-scope-matches-deployment.md)

### `epoch-input-size-is-bounded`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** Acquisition reads at most the maximum valid encoded epoch size plus bounded framing, independent of file size.
- **Check:** `always(bytes_read_for_epoch <= 32)` and reject files larger than 32 bytes before proportional allocation. The current decimal `u64` needs at most 20 bytes; the remainder permits bounded surrounding whitespace accepted by `trim()`.
- **Fault/timing angle:** A corrupt, restored, or hostile multi-gigabyte lease file.
- **Required faults and enabling state:** Replace a key's lease file with progressively oversized content while no holder is live; exercise both exclusive and shared acquisition paths.
- **Confidence:** high that the current read is unbounded: both paths call `read_to_string` with no length check at `src/lib.rs:322,331`.
- **Existing check:** none.
- **Impact:** Opening a store can exhaust process memory before the database is opened.
- **Open questions:** A future versioned format must revise the 32-byte bound deliberately.
- **Evidence:** [evidence/epoch-input-size-is-bounded.md](evidence/epoch-input-size-is-bounded.md)

### `lease-file-growth-trigger-is-observed`

- **Type:** liveness
- **Status:** active
- **Exercised:** cannot be exercised from this repository; watcher and acknowledgement evidence live in the deployment owner.
- **Guarantee:** When a lease directory crosses the configured physical-size trigger, the assigned owner receives and acknowledges a re-open signal within a configured bound.
- **Check:** Coverage: `reachable(watcher_evaluated_and_reported_size)` once per monitoring interval. Convergence: after crossing a configurable campaign threshold and a bounded signal window with no injected watcher fault, `always(owner_acknowledged_reopen_signal)`. Production uses 1 GiB; campaigns use a smaller constructible threshold.
- **Fault/timing angle:** Long-running growth from ephemeral identities, watcher failure, ownership drift, and inode exhaustion before the byte threshold.
- **Required faults and enabling state:** Sustained unique-key creation through an actual configured-threshold crossing, with the watcher healthy for the bounded acknowledgement check. Injected watcher failure is evaluated separately by heartbeat coverage.
- **Confidence:** medium. The measurement and ownership assignment are documented at `docs/lease-store-density.md:3-42`; watcher operation is outside this repository.
- **Existing check:** none in the crate.
- **Impact:** Unbounded file and inode growth can exhaust the filesystem; an unsafe cleanup can then trigger the unlink-inode race.
- **Open questions:** Is the watcher still armed, and who watches inode availability? `(needs human input)`
- **Evidence:** [evidence/lease-file-growth-trigger-is-observed.md](evidence/lease-file-growth-trigger-is-observed.md)

### `lease-path-format-is-version-stable`

- **Type:** safety
- **Status:** active
- **Exercised:** partial. One identity/hash vector and one PostgreSQL advisory-key vector pin the shared identity/FNV-1a derivation; the private `.lease` path assembly and cross-version overlap remain unchecked.
- **Guarantee:** Binaries that may overlap in one deployment derive the same lease path for the same key, or reject mixed-version operation before either acquires.
- **Check:** Expand the checked-in vectors to representative keys, including empty, non-ASCII, and `U+001F` fields, and assert `always(derived_filename == golden_filename)`. Pin the PostgreSQL advisory bigint from the same public `LeaseKey::identity` and `fnv1a` derivation.
- **Fault/timing angle:** Changing field order, separator, hash, suffix, or normalization while old and new processes overlap.
- **Required faults and enabling state:** Two versions running concurrently against one lease root, including rolling restart and rollback.
- **Confidence:** high that the path is a de facto persisted protocol (`src/lib.rs:204-210`). Crate version `0.1.1` records the public derivation and MSRV change, but compatibility remains a manual versioning convention rather than an automated gate.
- **Existing check:** `identity_hash_derivation_is_stable`, `src/lib.rs:482-488`, pins one identity and filename hash; `advisory_key_derivation_is_stable`, `cortexkit-store-postgres/src/lib.rs:375-381`, pins the corresponding shared derivation for a PostgreSQL key; status **unaudited**.
- **Impact:** Old and new binaries lock different files and can both write.
- **Open questions:** Is mixed-version overlap supported for all consumers? `(needs human input)`
- **Evidence:** [evidence/lease-path-format-is-version-stable.md](evidence/lease-path-format-is-version-stable.md)

### `stale-writer-write-is-rejected`

- **Type:** safety
- **Status:** active
- **Exercised:** partial: SQLite's synthetic fenced path is covered; real handover, pre-claim window, unfenced SQLite paths, and PostgreSQL are missing.
- **Guarantee:** On write paths declared fence-protected, after a replacement writer claims epoch `n`, every write attempt from epoch `< n` is rejected before effects.
- **Check:** `always(effects_begin => holder_epoch >= authoritative_epoch)`. For every stale attempt, assert an explicit fenced result and unchanged application state.
- **Fault/timing angle:** A stale connection remains usable after its lease is released and a replacement acquires a newer epoch.
- **Required faults and enabling state:** Real handover, retained old connection, replacement fence claim, then a late old-writer mutation. Run for every path declared fence-protected; fence-coverage completeness is a separate property.
- **Confidence:** high that the crate claims it at `src/lib.rs:11-16`; high that SQLite implements one fenced path; high that PostgreSQL exposes an epoch but no in-repo fence check was found.
- **Existing check:** `superseded_writer_is_fenced_out_after_handover`, `cortexkit-store/src/lib.rs:629-667`, synthetic `for_test` handles; status **unaudited**.
- **Impact:** A superseded process can overwrite state owned by its replacement.
- **Open questions:** Is PostgreSQL's session lock intentionally considered sufficient, making its epoch informational, or is the fence path missing? `(needs human input)`
- **Evidence:** [evidence/stale-writer-write-is-rejected.md](evidence/stale-writer-write-is-rejected.md)

### `logical-store-has-single-lease-identity`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet for same-database descriptor disagreement or sibling databases in one directory.
- **Guarantee:** All cooperative writers for one logical store derive the same `(base_dir, LeaseKey)`, while distinct stores that must write independently derive different identities.
- **Check:** `always(same_logical_store => lease_identity_a == lease_identity_b)` and `always(independent_stores => lease_identity_a != lease_identity_b)`, where identity includes canonical root plus all three key fields.
- **Fault/timing angle:** The lease key excludes the SQLite database path, while the root is only its parent. Sibling databases with equal descriptors alias; one database opened with differing module or namespace values splits into independent locks.
- **Required faults and enabling state:** Open the same SQLite database through descriptors differing in module or namespace; open it through cross-parent symlink or hardlink aliases; open two sibling database files under one parent with equal key fields.
- **Confidence:** high on derivation facts (`cortexkit-store/src/lib.rs:69-75,245-260`); unknown whether descriptor authority prevents these combinations in deployment.
- **Existing check:** `distinct_databases_do_not_falsely_contend`, `cortexkit-store/src/lib.rs:493-501`, uses different parent directories and does not exercise sibling files; status **unaudited**.
- **Impact:** One store can have two writers, or independent stores can falsely block each other.
- **Open questions:** What component guarantees descriptor uniqueness and canonical database paths? `(needs human input)`
- **Evidence:** [evidence/logical-store-has-single-lease-identity.md](evidence/logical-store-has-single-lease-identity.md)

### `failed-acquisition-does-not-mutate-lease-state`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** An attempt rejected as `Held` does not change the incumbent lease file's bytes, mode, owner, or modification time.
- **Check:** Around every known-contended attempt, `always(after_state == before_state)` for content and metadata of the incumbent inode.
- **Fault/timing angle:** Both acquisition paths create/open and call `protect_file` before try-lock, so a non-holder can chmod the file before learning it is contended.
- **Required faults and enabling state:** A live holder plus a competing process that sees a deliberately permissive mode before attempting acquisition.
- **Confidence:** high on operation order at `src/lib.rs:239-263,278-299`.
- **Existing check:** none.
- **Impact:** A rejected actor mutates state it never owned; foreign-owned or read-only roots also collapse into undifferentiated `Io` failures.
- **Open questions:** Is single-UID ownership a supported precondition or merely a deployment habit? `(needs human input)`
- **Evidence:** [evidence/failed-acquisition-does-not-mutate-lease-state.md](evidence/failed-acquisition-does-not-mutate-lease-state.md)

### `handle-drop-releases-lease`

- **Type:** liveness
- **Status:** active
- **Exercised:** yes on current CI-style local filesystems; toolchain and target variants remain partial.
- **Guarantee:** After the last handle for a root and key is dropped, a retrying cooperative acquirer succeeds within configured bound `B`.
- **Check:** A competitor first observes `Held`, then retries on a fixed campaign schedule. Drop the last handle and assert `always(acquired_by(drop_time + B))`, under the stated scheduler-fairness assumption.
- **Fault/timing angle:** `Drop` discards errors from standard-library `File::unlock`; descriptor close is the final release mechanism.
- **Required faults and enabling state:** A competitor that has observed `Held` and continues retrying, last-handle drop, injected unlock error where possible, and every supported target/toolchain family.
- **Confidence:** high for current Linux/macOS/Windows close semantics. Rust 1.89 is the declared MSRV because it stabilized `File::try_lock`, `File::try_lock_shared`, and `File::unlock`, but CI has no MSRV job.
- **Existing check:** T5, T9, T10, and T12 reacquire after clean drop/exit (`src/lib.rs:491-691`); status **unaudited**.
- **Impact:** A cleanly stopped module can leave its successor unable to start.
- **Open questions:** Should CI add an explicit Rust 1.89 job to enforce the declared MSRV? `(needs human input)`
- **Evidence:** [evidence/handle-drop-releases-lease.md](evidence/handle-drop-releases-lease.md)

### `replacement-fence-is-claimed-before-old-writer-writes`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** On declared fence-protected paths, from the instant a replacement's exclusive acquisition succeeds, no write from a prior epoch commits, even if that write began earlier.
- **Check:** `always(old_epoch_effect_commits => replacement_not_yet_acquired_at_commit)`. After replacement acquisition, every old-epoch attempt or in-flight transaction must abort as fenced and leave application state unchanged.
- **Fault/timing angle:** SQLite claims the database fence lazily in `with_conn_fenced`, not during `open_sqlite`; before the replacement's first fenced write, the stored fence can still equal the old writer's epoch.
- **Required faults and enabling state:** Old connection retained after releasing its lease, replacement acquires a newer epoch but performs no fenced write, then old connection calls the fenced path.
- **Confidence:** high from `cortexkit-store/src/lib.rs:162-205,245-284` and equal-epoch behavior at `:670-689`.
- **Existing check:** none; the synthetic handover test claims epoch 2 before the stale attempt.
- **Impact:** A superseded writer can commit during the handover window the fence is meant to close.
- **Open questions:** Is fence claim intended at open, or is a pre-claim stale-write window accepted? `(needs human input)`
- **Evidence:** [evidence/replacement-fence-is-claimed-before-old-writer-writes.md](evidence/replacement-fence-is-claimed-before-old-writer-writes.md)

### `protected-write-set-is-fence-complete`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet; no write-site inventory exists.
- **Guarantee:** Every durable mutation declared protected by lease fencing commits only after at least one authoritative fence check atomically bound to that mutation.
- **Check:** For the enumerated protected write-site set, `always(protected_effect_commits => authoritative_atomic_fence_checks >= 1)`, with a source-level inventory proving no protected write path bypasses the checked transaction.
- **Fault/timing angle:** `cortexkit-store` exposes unfenced `with_conn` and `migrate` alongside `with_conn_fenced`; PostgreSQL exposes `with_client` without an epoch comparison.
- **Required faults and enabling state:** Exercise or inspect every public durable-write boundary and classify whether fencing is required by its contract.
- **Confidence:** high that unfenced paths exist; their intended protection scope needs human authority.
- **Existing check:** none. Existing fence tests cover only `with_conn_fenced`.
- **Impact:** The module-level claim “every durable write carries the holder's epoch” can be false even when the fenced helper works.
- **Open questions:** Which SQLite and PostgreSQL writes are contractually fence-protected? `(needs human input)`
- **Evidence:** [evidence/protected-write-set-is-fence-complete.md](evidence/protected-write-set-is-fence-complete.md)

### `lease-file-creation-is-never-permissive`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** A newly created Unix lease file is never observable with permission bits wider than `0600`.
- **Check:** `always(mode_observed_from_creation & 0o077 == 0)` and, structurally, assert the create operation requests exactly `0600`; a numeric comparison or post-acquisition check is insufficient.
- **Fault/timing angle:** `OpenOptions::create(true)` uses umask-derived mode before `protect_file` corrects it.
- **Required faults and enabling state:** First acquisition under permissive umasks plus a concurrent observer that opens during the create-before-chmod window.
- **Confidence:** high on operation order at `src/lib.rs:243-254,281-288`; deployment umasks are unknown.
- **Existing check:** T1 checks only post-acquisition steady state; status **unaudited**.
- **Impact:** A racing process can retain access acquired before chmod.
- **Open questions:** What umasks do supported deployments use? `(needs human input)`
- **Evidence:** [evidence/lease-file-creation-is-never-permissive.md](evidence/lease-file-creation-is-never-permissive.md)

### `acquisition-does-not-follow-symlink`

- **Type:** safety
- **Status:** active
- **Exercised:** not yet through either acquisition path.
- **Guarantee:** Exclusive and shared acquisition never create, open, lock, write, or chmod a symlink target as the lease file.
- **Check:** With the derived lease path replaced by symlinks to existing and absent targets, assert `always(acquire_returns_error)`, `always(target_content_mode_and_existence_unchanged)`, and `unreachable("acquisition-owned-fd-resolved-to-target-inode")` using syscall or descriptor tracing.
- **Fault/timing angle:** Acquisition opens the path before `protect_file` inspects it; on Unix a dangling target can be created before refusal, while the non-Unix hardening branch is a no-op.
- **Required faults and enabling state:** Existing-target and dangling-target symlinks through both shared and exclusive methods on every supported platform.
- **Confidence:** high from `src/lib.rs:243-254,281-288,80-82`.
- **Existing check:** T2 calls `protect_file` directly and never traverses acquisition; status **unaudited**.
- **Impact:** The lease can protect and overwrite an attacker-chosen inode or create an unintended file.
- **Open questions:** Is Windows a supported deployment target? `(needs human input)`
- **Evidence:** [evidence/acquisition-does-not-follow-symlink.md](evidence/acquisition-does-not-follow-symlink.md)

### `cross-process-exclusive-race-is-reached`

- **Type:** reachability
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** Every campaign for exclusive exclusion executes at least one history where two independent processes have the same lease file open and concurrently attempt exclusive acquisition.
- **Check:** `sometimes(distinct_processes >= 2 && same_root_and_logical_key && same_inode && both_waiting_at_pre_lock_barrier_before_either_try_lock)`. This is situation coverage, so `sometimes` fits.
- **Fault/timing angle:** Scheduler ordering can otherwise serialize every attempt and let the safety check pass vacuously.
- **Required faults and enabling state:** A barrier after both opens and before both try-lock calls, then concurrent release of the barrier.
- **Confidence:** high that this is reachable; the current test machinery already spawns a child for shared mode.
- **Existing check:** none; `src/lib.rs:632-691` reaches cross-process shared contention only.
- **Impact:** Without this witness, `at-most-one-exclusive-holder-per-key` can pass without exercising its primary contention state.
- **Open questions:** None.
- **Evidence:** [evidence/cross-process-exclusive-race-is-reached.md](evidence/cross-process-exclusive-race-is-reached.md)

### `epoch-update-interruption-window-is-reached`

- **Type:** reachability
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** Every crash-recovery campaign interrupts at least one acquisition after the old epoch is truncated and before the replacement is fully written.
- **Check:** `reachable("process-terminated-after-set-len-before-write-complete")`. The event fires only after the harness confirms termination occurred inside the named code window.
- **Fault/timing angle:** Random process kills are unlikely to land in a two-syscall window.
- **Required faults and enabling state:** A deterministic failpoint immediately after `set_len(0)` at `src/lib.rs:334`, followed by non-unwinding process termination.
- **Confidence:** high that the point is reachable in code; no current injection mechanism exposes it.
- **Existing check:** none.
- **Impact:** Without this witness, crash-recovery properties can pass vacuously. Returned-I/O-error preservation needs a separate injected error witness.
- **Open questions:** None.
- **Evidence:** [evidence/epoch-update-interruption-window-is-reached.md](evidence/epoch-update-interruption-window-is-reached.md)

### `live-lease-file-replacement-is-reached`

- **Type:** reachability
- **Status:** active
- **Exercised:** not yet.
- **Guarantee:** Every inode-stability campaign executes at least one history where a lease path is replaced while its old inode remains locked by a live holder.
- **Check:** `sometimes(holder_live && path_identity != holder_inode_identity)`. This asserts the vulnerable precondition, not the forbidden outcome of two successful writers.
- **Fault/timing angle:** Cleanup and restore actions may otherwise occur only before or after the holder lifetime.
- **Required faults and enabling state:** A live holder, external unlink or rename of the path, and creation of a new file at the same path before holder release.
- **Confidence:** high that the state is constructible on local Unix filesystems; deployment reachability remains open.
- **Existing check:** none.
- **Impact:** Without this witness, inode-stability checks say nothing about the race documented in `docs/lease-store-density.md:22-24`.
- **Open questions:** Which production actor can create this state? `(needs human input)`
- **Evidence:** [evidence/live-lease-file-replacement-is-reached.md](evidence/live-lease-file-replacement-is-reached.md)

## Handoff list

All active records go to `/testing:test-strategy` for test-form, oracle, and boundary selection. Additional routing:

| Property | Additional handoff |
|---|---|
| `at-most-one-exclusive-holder-per-key` | `/testing:test-strategy` for a real multi-process barrier race; a simulation must not replace the kernel lock under test |
| `shared-exclusive-exclusion-matrix` | `/testing:invariant-test-review` for T9-T12 |
| `dead-holder-lease-is-reclaimable` | `/testing:crash-consistency-and-failpoint-testing` for non-unwinding termination and restart |
| `writer-epoch-strictly-increases` | `/testing:crash-consistency-and-failpoint-testing`; `/testing:invariant-test-review` for T5/T13 |
| `returned-epoch-is-crash-durable` | `/testing:crash-consistency-and-failpoint-testing` for power-loss and crash-image evidence |
| `invalid-epoch-fails-closed` | `/low-level-systems:defensive-assertions-and-invariant-guards` for production corruption handling |
| `failed-acquire-preserves-prior-epoch` | `/testing:crash-consistency-and-failpoint-testing` for short writes and I/O failpoints |
| `distinct-lease-keys-do-not-alias` | `/testing:invariant-test-review` for T6-T8 |
| `lease-inode-remains-stable-while-held` | `/testing:test-strategy` for a real two-process replacement ordering test |
| `shared-acquisition-is-epoch-neutral` | `/testing:invariant-test-review` for T11 |
| `shared-epoch-never-authorizes-write` | `/testing:test-strategy` at consumer boundary; interface design is a separate follow-up |
| `unix-lease-file-is-owner-only` | `/testing:invariant-test-review` for T1 |
| `permission-hardening-never-follows-replacement` | `/testing:crash-consistency-and-failpoint-testing` for a deterministic check-act pause; `/testing:invariant-test-review` for T2 |
| `contention-is-classified-as-held` | `/testing:invariant-test-review` for contention tests |
| `filesystem-lock-scope-matches-deployment` | `/operational-resilience:production-readiness-review`; it needs deployment mount and host evidence |
| `epoch-input-size-is-bounded` | `/low-level-systems:defensive-assertions-and-invariant-guards` for production input bounds |
| `lease-file-growth-trigger-is-observed` | `/operational-resilience:production-readiness-review` for watcher and inode-headroom evidence |
| `lease-path-format-is-version-stable` | `/testing:test-strategy`; SemVer tooling is a separate follow-up |
| `stale-writer-write-is-rejected` | `/testing:invariant-test-review` for the SQLite check; `/testing:test-strategy` for real handover histories |
| `logical-store-has-single-lease-identity` | `/testing:test-strategy` at the descriptor-to-store boundary |
| `failed-acquisition-does-not-mutate-lease-state` | `/testing:test-strategy`; production enforcement routes to defensive assertions |
| `handle-drop-releases-lease` | `/testing:invariant-test-review` for current drop/reacquire checks |
| `replacement-fence-is-claimed-before-old-writer-writes` | `/testing:test-strategy` at the SQLite handover boundary |
| `protected-write-set-is-fence-complete` | `/testing:test-strategy` for source-level write-site inventory |
| `lease-file-creation-is-never-permissive` | `/testing:test-strategy` for a real creation-window observer |
| `acquisition-does-not-follow-symlink` | `/testing:test-strategy`; `/testing:invariant-test-review` for the narrower direct helper test |
| `epoch-update-interruption-window-is-reached` | `/testing:crash-consistency-and-failpoint-testing` |
| Other reachability records | `/testing:test-strategy` for real process/filesystem scheduling |

Existing production guards and tests remain **unaudited** until their owning review skills return verdicts.
