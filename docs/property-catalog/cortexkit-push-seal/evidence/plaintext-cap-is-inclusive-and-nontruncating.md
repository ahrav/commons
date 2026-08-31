# Evidence: `plaintext-cap-is-inclusive-and-nontruncating`

- Discovery lenses: resource boundaries, data integrity, failure recovery.
- Trigger: `MAX_PLAINTEXT_BYTES` is documented as normative and pre-seal.
- Code trail: constant, guard, and error fields in [`src/lib.rs`](../../../../crates/cortexkit-push-seal/src/lib.rs); [`an_oversized_plaintext_is_refused_with_both_numbers`](../../../../crates/cortexkit-push-seal/src/lib.rs#L262) and [`a_sealed_payload_opens_to_the_same_plaintext`](../../../../crates/cortexkit-push-seal/src/lib.rs#L212).
- Implemented mechanism: size is checked before key parsing, RNG use, allocation of the envelope, or HPKE work.
- Failure scenario: off-by-one rejection at 2048, acceptance at 2049, truncation, or loss of the observed-size diagnostic.
- Timing/configuration: no timing dependence.
- Existing evidence: 2049 with an invalid key returns literal `PlaintextTooLarge { limit: 2048, observed: 2049 }`, proving the cap precedes key parsing. Lengths 2047 and 2048 round-trip, and the cap test separately opens its 2048-byte positive control.
- Audit status: audited for the neighboring 2047-2049 local boundary and exact classification. Larger oversize samples and caller behavior are not claimed.
- Instrumentation: complete in the public result.
- Investigation log: no other cap exists in this crate. Transport sizing remains a separate external obligation.
