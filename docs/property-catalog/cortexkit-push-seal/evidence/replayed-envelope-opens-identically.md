# Evidence: `replayed-envelope-opens-identically`

- Discovery lenses: idempotency and replay, state and persistence.
- Trigger: the crate adds no framing field for a message identifier, counter, timestamp, or expiry and has no consumed-state store. Arbitrary encrypted plaintext may contain such metadata.
- Code trail: `seal` documents the layout and `seal_with_rng` assembles `version || enc || ciphertext`; `open` holds no state across calls.
- Implemented mechanism: pure deterministic open over private-key and envelope bytes.
- Failure scenario: transport duplicates or delays a valid notification and no payload or device layer rejects stale content.
- Timing/configuration: duplicate delivery and arbitrary delay are the relevant conditions.
- Existing evidence: existing round-trip test opens once only.
- Instrumentation: repeat the same valid envelope and observe identical result; system-level replay safety requires payload/device evidence outside this crate.
- Investigation log: local replay behavior is resolved from code. Whether a higher layer rejects duplicates is tagged `(needs human input)`.
