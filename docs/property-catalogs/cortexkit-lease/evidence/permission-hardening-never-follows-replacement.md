# `permission-hardening-never-follows-replacement`

- **Discovery:** security wildcard pass.
- **Primary evidence:** no-follow claim at `src/lib.rs:51-53`; metadata check at `:62-76`; path-based chmod at `:77`; acquisition opens before checking at `:240-254,278-288`.
- **Existing evidence:** T2 (`:426-466`) presents a symlink before the call and asserts target mode. It cannot exercise a swap between check and act.
- **Failure scenario:** final path component changes from inspected regular file to symlink before `set_permissions`; chmod follows the replacement target.
- **Timing window:** metadata lookup to chmod.
- **Instrumentation:** missing deterministic pause and inode identity capture.
- **Open-question log:** directory permissions and process privilege in deployment are unknown.
