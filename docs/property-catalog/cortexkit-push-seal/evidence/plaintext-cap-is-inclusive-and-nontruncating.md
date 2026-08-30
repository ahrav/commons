# Evidence: `plaintext-cap-is-inclusive-and-nontruncating`

- Discovery lenses: resource boundaries, data integrity, failure recovery.
- Trigger: `MAX_PLAINTEXT_BYTES` is documented as normative and pre-seal.
- Code trail: constant and rationale at `src/lib.rs:52-56`; guard at `:104-109`; error fields at `:65-74`; test at `:244-259`.
- Implemented mechanism: size is checked before key parsing, RNG use, allocation of the envelope, or HPKE work.
- Failure scenario: off-by-one rejection at 2048, acceptance at 2049, truncation, or loss of the observed-size diagnostic.
- Timing/configuration: no timing dependence.
- Existing evidence: 2049 returns both numbers and 2048 succeeds. The exact-cap result is a positive control against a reject-all implementation.
- Instrumentation: complete in the public result.
- Investigation log: no other cap exists in this crate. Transport sizing remains a separate external obligation.
