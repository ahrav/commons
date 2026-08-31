# Evidence: `low-order-encapsulation-aead-path-is-reachable`

- Discovery lenses: reachability, security boundaries, dependencies.
- Trigger: bounded review of `open` found the real X25519 validation path occurs during decapsulation, not encapsulated-key parsing.
- Code trail: `open` extracts the fixed-size `enc`, then collapses HPKE decapsulation and authenticated-open errors to `OpenError::Aead`.
- Dependency trail: `hpke 0.14.0/src/dhkex/x25519.rs:134-146` rejects an all-zero shared secret; receiver decapsulation in `src/kem/dhkem.rs:291-294` lifts that to `DecapError`, which the crate maps to `OpenError::Aead`.
- Competing explanation: malformed low-order points might fail at `from_bytes`. Dependency source refutes this; deserialization is length-only.
- Failure scenario: attacker supplies a known low-order X25519 point as the 32-byte `enc` field.
- Timing/configuration: no timing dependence; ordinary random input is unlikely to reach this semantic state.
- Existing evidence: `low_order_encapsulation_reaches_decap_error` replaces a valid envelope's encapsulated key with 32 zero bytes. Direct `setup_receiver` returns exact `HpkeError::DecapError`; public `open` returns exact `OpenError::Aead` and wire code `malformed`. The original envelope opens as the enabling control.
- Instrumentation: fixed dependency-approved low-order `enc` vector, valid recipient private key, and direct dependency decapsulation result or branch witness proving `DecapError`; then assert the public collapse to `OpenError::Aead` plus wire code `malformed`. The public result alone cannot distinguish low-order rejection from tag or short-tail failure.
- Investigation log: the fixed all-zero low-order input is audited against `hpke 0.14.0`. This is one dependency-semantic witness, not a claim over all low-order aliases or the external opener.
