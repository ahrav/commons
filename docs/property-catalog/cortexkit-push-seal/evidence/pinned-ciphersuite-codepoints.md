# Evidence: `pinned-ciphersuite-codepoints`

- Discovery lenses: architecture, protocol contracts, version compatibility.
- Trigger: module documentation says type names are not wire facts.
- Code trail: suite table at `src/lib.rs:30-37`; type parameters at `:116` and `:179`; literal assertions at `:204-209`.
- Implemented mechanism: `X25519HkdfSha256`, `HkdfSha256`, and `ChaCha20Poly1305` are compile-time type arguments.
- Failure scenario: an implementation or dependency change preserves a local type name but changes the wire codepoint or chooses a different suite in the external opener.
- Timing/configuration: evaluated per build; no runtime fault needed.
- Existing evidence: `the_pinned_suite_has_the_documented_codepoints` independently asserts literal KEM `0x0020`, KDF `0x0001`, and AEAD `0x0003`; `wire_v1_fixture_matches_local_bytes_and_classifications` also checks those fixture fields before regenerating the frozen local envelope.
- Instrumentation: complete locally; external conformance evidence is missing.
- Audit: local build-wide codepoint coverage is non-vacuous and audited. Cross-repository equality remains unaudited because the external opener was not supplied.
- Investigation log: no second local suite or runtime negotiation path was found. The fixture is a local regression oracle, not independent opener evidence.
