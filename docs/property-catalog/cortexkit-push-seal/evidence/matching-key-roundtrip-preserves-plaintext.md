# Evidence: `matching-key-roundtrip-preserves-plaintext`

- Discovery lenses: architecture, data integrity, idempotency and replay.
- Trigger: the crate's stated purpose and the one existing round-trip test.
- Code trail: `seal` at `src/lib.rs:103-131`; `open` at `src/lib.rs:161-188`; test at `src/lib.rs:211-216`.
- Implemented mechanism: matching HPKE base-mode contexts use the same suite, empty `info`, and `[VERSION]` AAD.
- Failure scenario: a boundary length, binary payload, or dependency change causes self-produced bytes to open incorrectly.
- Timing/configuration: no timing dependence. Accepted domain is `0..=2048` plaintext bytes.
- Existing evidence: one ten-byte ASCII sample. Empty, maximum-size, and non-UTF-8 inputs are not exercised by a claim-bearing test.
- Instrumentation: sufficient; both result bytes and original plaintext are directly observable.
- Investigation log: code, tests, and crate history were inspected. No conflicting implementation behavior was found. The external opener remains outside scope.
