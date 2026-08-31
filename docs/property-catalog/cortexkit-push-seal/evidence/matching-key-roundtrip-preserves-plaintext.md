# Evidence: `matching-key-roundtrip-preserves-plaintext`

- Discovery lenses: architecture, data integrity, idempotency and replay.
- Trigger: the crate's stated purpose and the one existing round-trip test.
- Code trail: `seal` and `open` in [`src/lib.rs`](../../../../crates/cortexkit-push-seal/src/lib.rs); [`a_sealed_payload_opens_to_the_same_plaintext`](../../../../crates/cortexkit-push-seal/src/lib.rs).
- Implemented mechanism: matching HPKE base-mode contexts use the same suite, empty `info`, and `[VERSION]` AAD.
- Failure scenario: a boundary length, binary payload, or dependency change causes self-produced bytes to open incorrectly.
- Timing/configuration: no timing dependence. Accepted domain is `0..=2048` plaintext bytes.
- Existing evidence: exact byte round trips at lengths 0, 2047, and 2048. The two non-empty fixtures are asserted to be non-UTF-8.
- Audit status: audited for these finite local boundaries and one generated matching keypair. This is not universal coverage or external-opener evidence.
- Instrumentation: sufficient; both result bytes and original plaintext are directly observable.
- Investigation log: code, tests, and crate history were inspected. No conflicting implementation behavior was found. The external opener remains outside scope.
