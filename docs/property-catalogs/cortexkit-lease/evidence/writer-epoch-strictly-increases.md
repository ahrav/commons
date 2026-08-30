# `writer-epoch-strictly-increases`

- **Discovery:** data-integrity, protocol, and wildcard passes.
- **Primary evidence:** contract at `crates/cortexkit-lease/src/lib.rs:11-16,135-137`; `bump_epoch` implementation at `:326-339`.
- **Contradictory code evidence:** `parse().unwrap_or(0)` (`crates/cortexkit-lease/src/lib.rs:332`) can reset; `saturating_add(1)` (`:333`) reissues `u64::MAX`; truncate-before-write (`:334-337`) opens a regression window.
- **Existing evidence:** `acquire_then_second_holder_is_rejected` (`crates/cortexkit-lease/src/lib.rs:490-506`) and `epoch_persists_across_store_instances` (`:693-706`) cover clean acquisitions only.
- **Failure scenario:** crash, write failure, restore, malformed body, or maximum body causes a non-increasing returned token.
- **Instrumentation:** missing external maximum-ever-returned witness per physical root and key tuple.
- **Open-question log:** no code comment or commit explains saturation. No repair protocol exists for regression.
