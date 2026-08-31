# Evidence: `each-seal-uses-fresh-ephemeral`

- Discovery lenses: data integrity, concurrency, idempotency and replay.
- Trigger: the test comment states HPKE base-mode confidentiality requires a fresh ephemeral for every message.
- Code trail: public `seal` supplies `UnwrapErr(SysRng)` to the private `seal_with_rng`; the test calls that same private path with a tiny recording RNG and also calls the public `seal` twice.
- Implemented mechanism: each `single_shot_seal_with_rng` creates a sender context and requests ephemeral randomness from its RNG (`hpke-0.14.0/src/single_shot.rs:128-158`). Public `seal` supplies `UnwrapErr(SysRng)`. Random outputs can collide; the requirement is a fresh draw and context, not mathematical uniqueness.
- Failure scenario: deterministic or degraded custom RNG, or cached sender context, reuses ephemeral state and therefore the AEAD key/nonce pair. Fork duplication is not claimed for the resolved default `SysRng`, which obtains OS bytes per call.
- Timing/configuration: repeated successful calls with accepted plaintext and a valid recipient key matter. The private seam permits deterministic observation without changing the public entropy path.
- Existing evidence: `each_seal_uses_a_fresh_ephemeral` records exactly one 32-byte fill for each successful `seal_with_rng` call. Distinct deterministic draws produce distinct `enc` fields; a fixed repeated draw produces repeated `enc` fields and proves the canary discriminates degraded entropy. The same test also seals twice through the public `seal` and requires distinct `enc`, which is the only check covering the RNG that `seal` selects. Rewiring `seal` to a constant RNG was confirmed to fail that assertion and no other.
- Instrumentation: test-only `RecordingRng` implements `TryRng<Error = Infallible>` and `TryCryptoRng`; no trait abstraction or mocking dependency was added.
- Investigation log: local call and context behavior is audited through `hpke 0.14.0` and observed `rand_core 0.10.1`. Production entropy quality and external vectors remain separate obligations.
