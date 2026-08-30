# `shared-acquisition-is-epoch-neutral`

- **Discovery:** concurrency and protocol passes.
- **Primary evidence:** contract at `crates/cortexkit-lease/src/lib.rs:173-187`; `read_epoch` at `:316-324`; shared call site at `:301-306`.
- **Existing evidence:** `shared_acquisition_does_not_bump_the_write_epoch` (`crates/cortexkit-lease/src/lib.rs:599-624`) observes equal parsed epochs across shared acquisitions; `shared_holders_coexist_but_block_exclusive` (`:547-579`) holds two shared handles concurrently.
- **Failure scenario:** a refactor calls `bump_epoch` or writes reader metadata, consuming or racing the writer fence.
- **Enabling state:** prior nonzero writer epoch and simultaneous shared holders.
- **Instrumentation:** partial; no byte-level before/after observation.
- **Open-question log:** `protect_file` can mutate mode on the shared path. The docs do not define whether epoch-neutrality excludes all file metadata writes.
