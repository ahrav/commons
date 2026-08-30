# Evidence: `bad-recipient-key-follows-resolved-deserializer`

- Discovery lenses: protocol contracts, failure recovery, dependencies.
- Trigger: public docs name invalid X25519 point/scalar errors, but this repository has no independent validity definition.
- Code trail: public-key mapping at `src/lib.rs:111-112`; private-key mapping at `:173-174`; earlier gates at `:104-109` and `:162-171`.
- Dependency trail: resolved `hpke 0.14.0/src/dhkex/x25519.rs:54-65,80-100` currently rejects serialized lengths other than 32 and accepts every 32-byte serialization at this parsing stage.
- Failure scenario: dependency validation semantics change but local mapping, docs, or tests continue assuming length-only behavior.
- Timing/configuration: build/dependency identity; no runtime schedule.
- Existing evidence: no test compares direct deserializer acceptance with public error classification.
- Instrumentation: direct `from_bytes` result and public API result for the same key bytes, with earlier gates forced to pass.
- Investigation log: this property preserves the mapping edge, not a particular validity policy. The stricter independent-policy question remains open.
