# Evidence: `tampered-or-truncated-envelope-never-opens`

- Discovery lenses: data integrity, failure recovery, security boundaries.
- Trigger: docs state truncation cannot yield a plaintext fragment and AAD binds the cleartext version.
- Code trail: truncation rationale at `src/lib.rs:65-70`; length gate at `:162-166`; AEAD open at `:179-187`; tests at `:284-331` and `:364-389`.
- Implemented mechanism: inputs below 33 bytes fail structurally; longer corrupt inputs fail version or AEAD authentication except with negligible forgery probability under the primitive's security assumptions.
- Failure scenario: short read, partial write, field mutation, or tag mutation produces accepted bytes.
- Timing/configuration: no timing dependence. Every envelope field and every proper-prefix length is relevant.
- Existing evidence: one 32-byte prefix, the 33-byte minimum-length envelope, every non-`0x01` version byte, and one corrupted trailing tag byte are rejected. `enc`, other ciphertext positions, and other prefix lengths are not systematically exercised.
- Instrumentation: direct result observation is sufficient.
- Investigation log: no evidence of an accepted mutation was found; confidence comes from mechanism, not comprehensive campaign evidence.
