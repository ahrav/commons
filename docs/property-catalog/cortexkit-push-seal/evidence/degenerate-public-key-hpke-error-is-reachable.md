# Evidence: `degenerate-public-key-hpke-error-is-reachable`

- Discovery lenses: reachability, dependencies, failure recovery.
- Trigger: `SealError::Hpke` is public but no repository test reaches it.
- Code trail: map at `src/lib.rs:115-123`; resolved `hpke 0.14.0/src/dhkex/x25519.rs:134-146` rejects an all-zero shared secret and `src/kem/dhkem.rs:111-115` lifts it to an encapsulation error.
- Failure scenario: degenerate or low-order recipient value passes 32-byte deserialization and fails during encapsulation.
- Timing/configuration: ordinary generated keypairs cannot witness this branch. A working entropy source is required because HPKE draws the ephemeral before reaching the all-zero DH rejection; entropy blocking or failure can prevent the witness.
- Existing evidence: none in the repository test suite.
- Instrumentation: a fixed dependency-version-compatible degenerate public value and an exact `Err(SealError::Hpke)` assertion.
- Investigation log: dependency mechanism is confirmed. The normative vector set should be selected with the dependency contract, not guessed from arbitrary invalid bytes.
