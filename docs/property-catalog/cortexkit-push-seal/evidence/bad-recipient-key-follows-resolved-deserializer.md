# Evidence: `bad-recipient-key-follows-resolved-deserializer`

- Discovery lenses: protocol contracts, failure recovery, dependencies.
- Trigger: public docs name invalid X25519 point/scalar errors, but this repository has no independent validity definition.
- Code trail: `seal` maps public-key deserialization failure to `SealError::BadRecipientKey`; `open` maps private-key deserialization failure after its length and version gates.
- Dependency trail: resolved `hpke 0.14.0/src/dhkex/x25519.rs:54-65,80-100` currently rejects serialized lengths other than 32 and accepts every 32-byte serialization at this parsing stage.
- Failure scenario: dependency validation semantics change but local mapping, docs, or tests continue assuming length-only behavior.
- Timing/configuration: build/dependency identity; no runtime schedule.
- Existing evidence: `key_deserialization_and_degenerate_public_key_paths_are_reachable` checks direct `IncorrectInputLength(32, observed)` results and exact public errors for the same 31-byte and 33-byte public and private keys, then uses generated 32-byte keys as successful direct and public controls.
- Instrumentation: direct `from_bytes` result and public API result for the same key bytes, with earlier gates forced to pass.
- Investigation log: lengths 31 and 33 plus generated 32-byte controls are sampled evidence. The universal mapping remains source-backed and unaudited, and this repository defines no independent X25519 validity policy.
