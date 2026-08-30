# Fault-to-property map

Safety checks apply while faults are active. Liveness checks run after the stated bounded fault-free recovery window. Coverage records verify that vulnerable preconditions occurred.

| Fault or enabling state | Threatened properties | Required witness / non-vacuity condition | Occurs today |
|---|---|---|---|
| Two independent exclusive acquirers | `at-most-one-exclusive-holder-per-key`, `contention-is-classified-as-held` | `cross-process-exclusive-race-is-reached` fires. | no |
| Two shared holders, then one drops | `shared-exclusive-exclusion-matrix` | Exclusive attempted while exactly one of at least two shared holders remains. | yes, local unit test |
| Nonzero epoch with two simultaneous shared holders | `shared-acquisition-is-epoch-neutral` | Both shared holders coexist after a writer persisted a nonzero epoch. | no; current tests split these states |
| Holder killed without unwind | `dead-holder-lease-is-reclaimable` | Child exit is confirmed before recovery deadline starts. | no |
| Kill after truncate, before epoch rewrite | `writer-epoch-strictly-increases` | `epoch-update-interruption-window-is-reached` fires. | no |
| Power loss after acquisition acknowledgement | `returned-epoch-is-crash-durable`, `writer-epoch-strictly-increases` | Volatile cache is actually discarded; process kill alone is insufficient. | no |
| `ENOSPC`, `EDQUOT`, or returned `EIO` during rewrite | `failed-acquire-preserves-prior-epoch`, `writer-epoch-strictly-increases` | Failure occurs after truncation succeeds and `acquire` returns `Err`. | no |
| Valid-UTF-8 malformed epoch | `invalid-epoch-fails-closed`, `writer-epoch-strictly-increases` | File is non-empty and existed before acquisition. | no |
| Persisted epoch is `u64::MAX` | `writer-epoch-strictly-increases` | Parser observes the exact maximum, then two consecutive acquisitions both return. | no |
| Older lease file restored | `writer-epoch-strictly-increases`, `returned-epoch-is-crash-durable` | A previously acknowledged higher epoch exists before restore, then the same key is acquired. | no |
| Key contains `U+001F` | `distinct-lease-keys-do-not-alias` | Two structurally distinct keys produce the same joined identity. | no |
| FNV-1a collision | `distinct-lease-keys-do-not-alias` | Two distinct identities produce one digest; practical adversarial cost remains open. | no |
| Lease file unlinked/replaced while held | `lease-inode-remains-stable-while-held`, `at-most-one-exclusive-holder-per-key` | `live-lease-file-replacement-is-reached` fires. | no |
| Shared handle routed to write fence | `shared-epoch-never-authorizes-write`, `stale-writer-write-is-rejected` | Consumer records handle origin and a durable write attempt. | no in-repo caller; external unknown |
| Pre-existing permissive file | `unix-lease-file-is-owner-only` | Check both shared and exclusive acquisition. | partial: exclusive only |
| Permissive create-time umask | `lease-file-creation-is-never-permissive` | Observer opens between creation and hardening. | no |
| Symlink planted before open | `acquisition-does-not-follow-symlink` | Assert target existence, content, and mode remain unchanged through both acquisition methods. | partial: helper-only test |
| Path replaced after lease descriptor open or between metadata check and chmod | `permission-hardening-never-follows-replacement`, `unix-lease-file-is-owner-only` | Deterministic pauses open both windows; compare opened/locked inode with hardened inode. | no |
| Known contention with permissive incumbent file | `failed-acquisition-does-not-mutate-lease-state` | Snapshot content and metadata before and after `Held`. | no |
| Unsupported or differently scoped advisory lock | `filesystem-lock-scope-matches-deployment`, `contention-is-classified-as-held` | Real target mount and multi-host/process topology are used. | no deployment evidence |
| Oversized lease file | `epoch-input-size-is-bounded` | Both acquisition modes read a file over 32 bytes. | no |
| Sustained ephemeral keys | `lease-file-growth-trigger-is-observed` | Watcher reports size and acknowledges a threshold signal. | partial: historical measurement only |
| Old and new binaries overlap | `lease-path-format-is-version-stable`, `at-most-one-exclusive-holder-per-key` | Both versions derive and attempt the same logical key concurrently. | no |
| Same database, differing descriptors | `logical-store-has-single-lease-identity` | One logical path derives two root/key identities. | no |
| Sibling databases, equal descriptors | `logical-store-has-single-lease-identity` | Two independent files derive one root/key identity. | no |
| Last handle drops while competitor waits | `handle-drop-releases-lease` | Acquisition completes within configured bound. | partial: reacquire occurs, but no waiting competitor |
| Replacement acquired, fence not yet claimed | `replacement-fence-is-claimed-before-old-writer-writes` | Old connection attempts fenced write before replacement's first claim. | no |
| Protected mutation uses an unfenced API | `protected-write-set-is-fence-complete` | Every protected write site is inventoried and observed. | no inventory |
| Stale connection after completed fence claim | `stale-writer-write-is-rejected` | Stale attempt returns fenced and leaves state unchanged. | partial: synthetic SQLite only |

A `no` means every safety check threatened by that fault can pass without the fault ever occurring. `partial` names the missing arm so the gap remains explicit.
