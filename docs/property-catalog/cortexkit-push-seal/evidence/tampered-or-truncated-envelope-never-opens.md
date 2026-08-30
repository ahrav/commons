# Evidence: `tampered-or-truncated-envelope-never-opens`

- Discovery lenses: data integrity, failure recovery, security boundaries.
- Trigger: docs state truncation cannot yield a plaintext fragment and AAD binds the cleartext version.
- Code trail: truncation rationale at `src/lib.rs:65-70`; length gate at `:162-166`; AEAD open at `:178-187`; tests at `:261-282`, `:286-296`, and `:330-356`.
- Implemented mechanism: inputs below 33 bytes fail structurally; longer corrupt inputs fail version or AEAD authentication except with negligible forgery probability under the primitive's security assumptions.
- Failure scenario: short read, partial write, field mutation, or tag mutation produces accepted bytes.
- Timing/configuration: no timing dependence. Every envelope field and every proper-prefix length is relevant.
- Existing evidence: one 32-byte prefix and two changed versions are rejected. `enc`, ciphertext, tag, and all other prefix lengths are not systematically exercised.
- Instrumentation: direct result observation is sufficient.
- Investigation log: no evidence of an accepted mutation was found; confidence comes from mechanism, not comprehensive campaign evidence.
