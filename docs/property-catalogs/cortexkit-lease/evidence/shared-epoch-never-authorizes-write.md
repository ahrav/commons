# `shared-epoch-never-authorizes-write`

- **Discovery:** protocol-contract and interface wildcard passes.
- **Primary evidence:** `LeaseHandle::epoch` writer-fence contract at `crates/cortexkit-lease/src/lib.rs:126-137`; shared carve-out at `:183-186`; identical return types at `:171,187`.
- **Existing evidence:** `shared_acquisition_does_not_bump_the_write_epoch` (`crates/cortexkit-lease/src/lib.rs:599-624`) confirms a shared handle returns the current writer epoch. No production in-repo caller uses `acquire_shared`.
- **Failure scenario:** consumer loses acquisition-mode provenance and passes shared `epoch()` into a write fence that accepts equal epochs.
- **Timing window:** no fault; misuse at consumer boundary.
- **Instrumentation:** missing handle-mode tag and write-site provenance assertion.
- **Open-question log:** external consumers named in the density doc were not supplied. Whether shared mode is used remains `(needs human input)`.
