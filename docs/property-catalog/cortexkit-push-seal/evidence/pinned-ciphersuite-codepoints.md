# Evidence: `pinned-ciphersuite-codepoints`

- Discovery lenses: architecture, protocol contracts, version compatibility.
- Trigger: module documentation says type names are not wire facts.
- Code trail: the module docs carry the suite table; `seal_with_rng` and `open` instantiate `ChaCha20Poly1305`, `HkdfSha256`, and `X25519HkdfSha256`; `the_pinned_suite_is_the_one_the_opener_agreed_to` asserts the three literal codepoints.
- Implemented mechanism: `X25519HkdfSha256`, `HkdfSha256`, and `ChaCha20Poly1305` are compile-time type arguments.
- Failure scenario: an implementation or dependency change preserves a local type name but changes the wire codepoint or chooses a different suite in the external opener.
- Timing/configuration: evaluated per build; no runtime fault needed.
- Existing evidence: all three local codepoints are asserted. Cross-repository equality is not verified.
- Instrumentation: complete locally; external conformance evidence is missing.
- Investigation log: no second local suite or runtime negotiation path was found.
