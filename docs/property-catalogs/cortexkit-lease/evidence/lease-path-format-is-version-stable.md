# `lease-path-format-is-version-stable`

- **Discovery:** version-compatibility and protocol passes.
- **Primary evidence:** public `LeaseKey::identity`, `fnv1a`, and `fnv1a_hex` form the compatibility contract; private `FileLeaseStore::lease_path` appends the `.lease` suffix.
- **Cross-crate evidence:** PostgreSQL imports public `fnv1a` and hashes public `LeaseKey::identity` at `cortexkit-store-postgres/src/lib.rs:11,54-59`.
- **Existing evidence:** `identity_hash_derivation_is_stable` provides golden identity/hash coverage, and PostgreSQL pins the resulting advisory key in `advisory_key_derivation_is_stable`.
- **Residual gaps:** no automated SemVer gate, mixed-version overlap test, full-filename golden, or adversarial vectors.
- **Failure scenario:** rolling restart or rollback overlaps binaries using different separators, field order, normalization, hash, or suffix.
- **Timing window:** from first new-version acquisition until every old process is gone.
- **Instrumentation:** artifact-version and derived-path observations remain missing.
- **Residual risk:** one edit to the shared identity or FNV-1a derivation can remap both file and PostgreSQL lock domains.
- **Open-question log:** mixed-version overlap policy is not documented. The versioning rule remains at `README.md:43-52`; `cortexkit-lease/Cargo.toml:2-3` records version `0.3.0`, with no path-derivation change since `0.2.0`.
