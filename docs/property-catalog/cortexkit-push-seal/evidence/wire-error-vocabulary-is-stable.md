# Evidence: `wire-error-vocabulary-is-stable`

- Discovery lenses: protocol contracts, version compatibility, security boundaries.
- Trigger: `wire_code` is documented as the sole mapping to the conformance vocabulary.
- Code trail: mapping rationale and implementation at `src/lib.rs:133-153`; mapping test at `:286-316`.
- Implemented mechanism: exhaustive match over the closed `OpenError` enum.
- Failure scenario: literal rename, new variant in the wrong bucket, or opener-side key failure classified differently across repositories.
- Timing/configuration: no timing dependence.
- Existing evidence: `UnknownVersion`, `Malformed`, and `Aead` are constructed. `BadRecipientKey` is not reached by the existing test.
- Instrumentation: complete once every variant is constructed.
- Investigation log: code establishes a two-string image. Whether `BadRecipientKey -> malformed` is the intended external contract needs the missing opener/corpus.
