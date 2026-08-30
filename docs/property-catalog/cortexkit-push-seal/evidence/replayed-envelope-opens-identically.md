# Evidence: `replayed-envelope-opens-identically`

- Discovery lenses: idempotency and replay, state and persistence.
- Trigger: the crate adds no framing field for a message identifier, counter, timestamp, or expiry and has no consumed-state store. Arbitrary encrypted plaintext may contain such metadata.
- Code trail: layout at `src/lib.rs:97-102,126-130`; stateless `open` at `:161-188`.
- Implemented mechanism: pure deterministic open over private-key and envelope bytes.
- Failure scenario: transport duplicates or delays a valid notification and no payload or device layer rejects stale content.
- Timing/configuration: duplicate delivery and arbitrary delay are the relevant conditions.
- Existing evidence: existing round-trip test opens once only.
- Instrumentation: repeat the same valid envelope and observe identical result; system-level replay safety requires payload/device evidence outside this crate.
- Investigation log: local replay behavior is resolved from code. Whether a higher layer rejects duplicates is tagged `(needs human input)`.
