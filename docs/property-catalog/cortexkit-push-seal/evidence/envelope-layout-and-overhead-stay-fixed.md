# Evidence: `envelope-layout-and-overhead-stay-fixed`

- Discovery lenses: architecture, data integrity, protocol contracts, version compatibility.
- Trigger: layout is a cross-repository byte contract and `open` hardcodes a split offset.
- Code trail: `ENC_LEN`, assembly, and parsing in [`src/lib.rs`](../../../../crates/cortexkit-push-seal/src/lib.rs); [`the_envelope_has_version_one_and_fixed_overhead`](../../../../crates/cortexkit-push-seal/src/lib.rs#L228).
- Implemented mechanism: one version byte, serialized X25519 encapsulated key, then ChaCha20-Poly1305 ciphertext and tag.
- Failure scenario: field reorder, changed KEM size, changed tag size, or stale `ENC_LEN` would cause the separately documented opener to reject envelopes if it retains the old contract.
- Timing/configuration: no timing dependence. Boundary plaintext lengths matter for size arithmetic.
- Existing evidence: plaintext lengths 0, 1, and 2048 pin literal version `0x01`, encapsulated-key length 32, overhead 49, and maximum local envelope length 2097.
- Audit status: audited for the leading version and local size literals at these lengths. Field ordering beyond the version, external opener framing, and production transport sizing remain unaudited.
- Instrumentation: output bytes and lengths are directly observable.
- Investigation log: `seal` uses `enc.len()` while `open` uses `ENC_LEN`; this duplicate fact is the main drift point.
