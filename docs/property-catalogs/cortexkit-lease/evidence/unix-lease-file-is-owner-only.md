# `unix-lease-file-is-owner-only`

- **Discovery:** security and bug-history passes.
- **Primary evidence:** `protect_file` at `crates/cortexkit-lease/src/lib.rs:36-83`; call sites at `:254,288`; commit `49bcaa2` records measured permissive deployment files.
- **Existing evidence:** `an_acquired_lease_file_is_owner_only` (`crates/cortexkit-lease/src/lib.rs:383-424`) checks exclusive acquisition over a pre-existing `0644` file. `protect_file_refuses_a_symlink_and_leaves_its_target_untouched` (`:426-466`) checks static symlink refusal.
- **Failure scenario:** a restored permissive file remains permissive on the shared path, or the pathname is replaced after the lease descriptor opens so `protect_file` hardens another inode while acquisition succeeds on the original permissive inode. Creation-time exposure is cataloged separately.
- **Timing window:** descriptor open through path-based hardening, with replacement between them.
- **Instrumentation:** compare opened/locked inode identity with the inode whose mode is checked; shared-path outcome is also missing.
- **Open-question log:** non-Unix branch returns `Ok` without work (`crates/cortexkit-lease/src/lib.rs:80-82`); no Windows ACL contract is documented.
