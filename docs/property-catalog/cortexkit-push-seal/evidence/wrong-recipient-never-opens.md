# Evidence: `wrong-recipient-never-opens`

- Discovery lenses: data integrity, security boundaries.
- Trigger: crate purpose is recipient-only confidentiality.
- Code trail: recipient key input at `src/lib.rs:103`; private key parse and open at `:173-187`; test at `:318-324`.
- Implemented mechanism: HPKE decapsulation and AEAD authentication are bound to the recipient keypair; the claim is probabilistic under HPKE/AEAD security assumptions, not mathematical impossibility.
- Failure scenario: environment, tenant, or recipient key mix-up yields plaintext instead of `Aead`.
- Timing/configuration: no timing dependence. Two keypairs with asserted-distinct public keys are the required enabling state.
- Existing evidence: one pair of independently generated recipients, but the existing test does not assert public-key distinctness.
- Instrumentation: public result is sufficient.
- Investigation log: no alternate key-selection path exists in the library. Operator selection hazards exist in `handseal` and are tracked separately.
