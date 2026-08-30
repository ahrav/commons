# Evidence: `pinned-ciphersuite-codepoints`

- Discovery lenses: architecture, protocol contracts, version compatibility.
- Trigger: module documentation says type names are not wire facts.
- Code trail: suite table at `src/lib.rs:30-37`; type parameters at `:116` and `:179`; literal assertions at `:204-209`.
- Implemented mechanism: `X25519HkdfSha256`, `HkdfSha256`, and `ChaCha20Poly1305` are compile-time type arguments.
- Failure scenario: an implementation or dependency change preserves a local type name but changes the wire codepoint or chooses a different suite in the external opener.
- Timing/configuration: evaluated per build; no runtime fault needed.
- Existing evidence: all three local codepoints are asserted. Cross-repository equality is not verified.
- Instrumentation: complete locally; external conformance evidence is missing.
- Investigation log: no second local suite or runtime negotiation path was found.
