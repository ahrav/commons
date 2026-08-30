# Evidence: `envelope-layout-and-overhead-stay-fixed`

- Discovery lenses: architecture, data integrity, protocol contracts, version compatibility.
- Trigger: layout is a cross-repository byte contract and `open` hardcodes a split offset.
- Code trail: `ENC_LEN` at `src/lib.rs:58-59`; assembly at `:125-130`; parsing at `:162-185`; layout test at `:218-225`; manual split at `:335-355`.
- Implemented mechanism: one version byte, serialized X25519 encapsulated key, then ChaCha20-Poly1305 ciphertext and tag.
- Failure scenario: field reorder, changed KEM size, changed tag size, or stale `ENC_LEN` would cause the separately documented opener to reject envelopes if it retains the old contract.
- Timing/configuration: no timing dependence. Boundary plaintext lengths matter for size arithmetic.
- Existing evidence: one-byte plaintext has expected total length. The manual split at `src/lib.rs:335-351` is used only for a negative empty-AAD open; public `open` is the success control at `:355`. No test directly ties `ENC_LEN` to the KEM serialized size or checks the 2097-byte maximum envelope.
- Instrumentation: output bytes and lengths are directly observable.
- Investigation log: `seal` uses `enc.len()` while `open` uses `ENC_LEN`; this duplicate fact is the main drift point.
