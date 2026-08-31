# Evidence: `wrong-recipient-never-opens`

- Discovery lenses: data integrity, security boundaries.
- Trigger: crate purpose is recipient-only confidentiality.
- Code trail: recipient key input, private-key parsing, and opening in [`src/lib.rs`](../../../../crates/cortexkit-push-seal/src/lib.rs); [`the_wrong_recipient_cannot_open`](../../../../crates/cortexkit-push-seal/src/lib.rs#L482).
- Implemented mechanism: HPKE decapsulation and AEAD authentication are bound to the recipient keypair; the claim is probabilistic under HPKE/AEAD security assumptions, not mathematical impossibility.
- Failure scenario: environment, tenant, or recipient key mix-up yields plaintext instead of `Aead`.
- Timing/configuration: no timing dependence. Two keypairs with asserted-distinct public keys are the required enabling state.
- Existing evidence: one pair of generated recipients whose public keys are asserted unequal before the second private key returns exact `OpenError::Aead`.
- Audit status: audited for this sampled local pair and enabling state. The probabilistic cryptographic claim and external key-selection paths are not universal evidence.
- Instrumentation: public result is sufficient.
- Investigation log: no alternate key-selection path exists in the library. Operator selection hazards exist in `handseal` and are tracked separately.
