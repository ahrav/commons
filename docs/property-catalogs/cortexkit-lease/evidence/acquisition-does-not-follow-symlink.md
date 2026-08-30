# `acquisition-does-not-follow-symlink`

- **Discovery:** targeted security refinement after portfolio evaluation.
- **Primary evidence:** exclusive acquisition opens or creates the path before calling `protect_file` (`crates/cortexkit-lease/src/lib.rs:240-254`), as does shared acquisition (`crates/cortexkit-lease/src/lib.rs:278-288`); `protect_file` is a no-op off Unix (`crates/cortexkit-lease/src/lib.rs:80-82`).
- **Existing evidence:** `protect_file_refuses_a_symlink_and_leaves_its_target_untouched` invokes `protect_file` directly on a static symlink (`crates/cortexkit-lease/src/lib.rs:426-466`), bypassing acquisition's open-before-protect ordering.
- **Failure scenario:** Unix dangling target is created before refusal; non-Unix target can be locked and epoch-written through the link.
- **Timing window:** symlink exists before open; no race is required.
- **Instrumentation:** target existence/content/mode snapshots plus syscall or descriptor tracing proving no acquisition-owned descriptor resolves to the target inode.
- **Open-question log:** Windows deployment support is not declared, though CI and lock-specific code include Windows.
