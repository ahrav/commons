# Evidence: `low-order-encapsulation-aead-path-is-reachable`

- Discovery lenses: reachability, security boundaries, dependencies.
- Trigger: bounded review of `open` found the real X25519 validation path occurs during decapsulation, not encapsulated-key parsing.
- Code trail: serialized `enc` extraction at `src/lib.rs:175`; `single_shot_open` and collapse at `:179-187`.
- Dependency trail: `hpke 0.14.0/src/dhkex/x25519.rs:134-146` rejects an all-zero shared secret; receiver decapsulation in `src/kem/dhkem.rs:291-294` lifts that to `DecapError`, which the crate maps to `OpenError::Aead`.
- Competing explanation: malformed low-order points might fail at `from_bytes`. Dependency source refutes this; deserialization is length-only.
- Failure scenario: attacker supplies a known low-order X25519 point as the 32-byte `enc` field.
- Timing/configuration: no timing dependence; ordinary random input is unlikely to reach this semantic state.
- Existing evidence: none. The empty-ciphertext test reaches a separate short-tail cause collapsed into the same variant.
- Instrumentation: fixed dependency-approved low-order `enc` vector, valid recipient private key, and direct dependency decapsulation result or branch witness proving `DecapError`; then assert the public collapse to `OpenError::Aead` plus wire code `malformed`. The public result alone cannot distinguish low-order rejection from tag or short-tail failure.
- Investigation log: mechanism is confirmed. A normative vector should come from the dependency/RFC contract rather than an arbitrary guessed byte string.
