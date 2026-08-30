# Evidence: `each-seal-uses-fresh-ephemeral`

- Discovery lenses: data integrity, concurrency, idempotency and replay.
- Trigger: the test comment states HPKE base-mode confidentiality requires a fresh ephemeral for every message.
- Code trail: ambient-random seal at `src/lib.rs:115-123`; test at `:227-242`; `hpke 0.14.0` routes this API through `SysRng`.
- Implemented mechanism: each `single_shot_seal` creates a sender context and requests ephemeral randomness from `SysRng` (`hpke-0.14.0/src/single_shot.rs:102-125`). Random outputs can collide; the requirement is a fresh draw and context, not mathematical uniqueness.
- Failure scenario: deterministic or degraded custom RNG, or cached sender context, reuses ephemeral state and therefore the AEAD key/nonce pair. Fork duplication is not claimed for the resolved default `SysRng`, which obtains OS bytes per call.
- Timing/configuration: repeated successful calls with accepted plaintext, valid recipient key, and working entropy matter. Resolved `getrandom 0.4.3` exposes a custom backend that can observe or control draw calls at build time.
- Existing evidence: two sequential calls have different `enc` values. This is a weak statistical canary, not proof that each call drew independently or that degraded entropy is detected.
- Instrumentation: a recorded custom backend counts draws and supplies distinct fixed byte streams; a negative-control backend repeats bytes and must repeat `enc`. Envelope bytes expose `enc` at offsets 1..33.
- Investigation log: a corpus cannot cover this property because each vector is independent; it needs an in-process campaign.
