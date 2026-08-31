# Evidence: `tampered-or-truncated-envelope-never-opens`

- Discovery lenses: data integrity, failure recovery, security boundaries.
- Trigger: docs state truncation cannot yield a plaintext fragment and AAD binds the cleartext version.
- Code trail: truncation rationale at `src/lib.rs:65-70`; length gate at `:162-166`; AEAD open at `:175-187`; campaigns at `:334-418`; precedence and version sweep at `:284-331`.
- Implemented mechanism: inputs below 33 bytes fail structurally; longer corrupt inputs fail version or AEAD authentication except with negligible forgery probability under the primitive's security assumptions. HPKE binds the serialized encapsulated key into the KEM context, so even an X25519 DH-equivalent masked-bit mutation changes authentication keys.
- Failure scenario: short read, partial write, field mutation, or tag mutation produces accepted bytes.
- Timing/configuration: no timing dependence. Every envelope field and every proper-prefix length is relevant.
- Existing evidence: `every_proper_prefix_of_a_valid_envelope_is_rejected` checks every proper prefix of one valid anchor with exact `Malformed` or `Aead` errors. `single_bit_mutations_have_field_specific_outcomes` checks every bit in the version, encapsulated key, ciphertext, and tag with exact outcomes. Both first open the valid anchor and use local reach counters. `open_error_precedence_is_stable` additionally rejects every non-`0x01` version byte and the 33-byte minimum-length envelope.
- Instrumentation: direct result observation is sufficient.
- Investigation log: no prefix or contributory field mutation opened. This is finite local evidence for one generated anchor, not universal mutation coverage or cross-implementation evidence.
