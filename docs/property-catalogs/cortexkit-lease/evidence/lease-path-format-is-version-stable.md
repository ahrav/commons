# `lease-path-format-is-version-stable`

- **Discovery:** version-compatibility and protocol passes.
- **Primary evidence:** public identity and compatibility contract at `src/lib.rs:112-123`; file paths derive from the shared FNV-1a hash at `:204-210,341-358`.
- **Cross-crate evidence:** PostgreSQL imports public `fnv1a` and hashes public `LeaseKey::identity` at `cortexkit-store-postgres/src/lib.rs:24,62-67`.
- **Existing evidence:** golden identity/hash coverage exists at `src/lib.rs:482-488`, and PostgreSQL pins the resulting advisory key at `cortexkit-store-postgres/src/lib.rs:396-402`.
- **Residual gaps:** no automated SemVer gate, mixed-version overlap test, full-filename golden, or adversarial vectors.
- **Failure scenario:** rolling restart or rollback overlaps binaries using different separators, field order, normalization, hash, or suffix.
- **Timing window:** from first new-version acquisition until every old process is gone.
- **Instrumentation:** artifact-version and derived-path observations remain missing.
- **Residual risk:** one edit to the shared identity or FNV-1a derivation can remap both file and PostgreSQL lock domains.
- **Open-question log:** mixed-version overlap policy is not documented. The versioning rule remains at `README.md:42-51`, and `cortexkit-lease/Cargo.toml:2-3` now records the `0.1.0` to `0.1.1` bump.
