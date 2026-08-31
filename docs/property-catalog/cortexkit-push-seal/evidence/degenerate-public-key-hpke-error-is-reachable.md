# Evidence: `degenerate-public-key-hpke-error-is-reachable`

- Discovery lenses: reachability, dependencies, failure recovery.
- Trigger: `SealError::Hpke` is public but no repository test reaches it.
- Code trail: `seal_with_rng` maps the HPKE sealing failure to `SealError::Hpke`; resolved `hpke 0.14.0/src/dhkex/x25519.rs:134-146` rejects an all-zero shared secret and `src/kem/dhkem.rs:111-115` lifts it to an encapsulation error.
- Failure scenario: degenerate or low-order recipient value passes 32-byte deserialization and fails during encapsulation.
- Timing/configuration: ordinary generated keypairs cannot witness this branch. A working entropy source is required because HPKE draws the ephemeral before reaching the all-zero DH rejection; entropy blocking or failure can prevent the witness.
- Existing evidence: `key_deserialization_and_degenerate_public_key_paths_are_reachable` proves an all-zero 32-byte public value passes length parsing and returns exact `SealError::Hpke` under working ambient entropy.
- Instrumentation: a fixed dependency-version-compatible degenerate public value and an exact `Err(SealError::Hpke)` assertion.
- Investigation log: one fixed low-order value is audited for reachability under `hpke 0.14.0`; this does not claim coverage of every low-order encoding or external implementation.
