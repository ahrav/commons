# Evidence: `key-error-classification-matches-key-shape`

- Discovery lenses: dependencies, failure recovery, protocol contracts.
- Trigger: docs describe point/scalar validity while the dependency parser behavior is narrower.
- Code trail: error docs at `src/lib.rs:75-90`; public-key parse at `:111-112`; private-key parse at `:173-174`; resolved `hpke 0.14.0/src/dhkex/x25519.rs:54-65,80-100` X25519 deserializers enforce serialized length.
- Competing explanation: `from_bytes` might validate X25519 points or scalars. Dependency source states conversion is infallible once length is correct, which discriminates against that explanation.
- Failure scenario: caller interprets `BadRecipientKey` as proof of semantic validation, or misdiagnoses a degenerate 32-byte public value reported as `Hpke`.
- Timing/configuration: no timing dependence.
- Existing evidence: no test constructs wrong-length or degenerate keys.
- Instrumentation: direct public API result over lengths 0, 31, 32, and 33 plus a dependency-approved degenerate vector.
- Investigation log: implemented classification is confirmed only after earlier plaintext, envelope-length, and version gates pass. Whether docs or validation should change is a design decision, so this is retained as an unresolved contract rather than an active property.
