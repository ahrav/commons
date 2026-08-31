# Evidence: `wire-error-vocabulary-is-stable`

- Discovery lenses: protocol contracts, version compatibility, security boundaries.
- Trigger: `wire_code` is documented as the sole mapping to the conformance vocabulary.
- Code trail: mapping rationale and implementation in [`src/lib.rs`](../../../../crates/cortexkit-push-seal/src/lib.rs); [`every_open_failure_maps_to_the_wire_vocabulary`](../../../../crates/cortexkit-push-seal/src/lib.rs#L322) and [`open_error_precedence_is_stable`](../../../../crates/cortexkit-push-seal/src/lib.rs#L284).
- Implemented mechanism: exhaustive match over the closed `OpenError` enum.
- Failure scenario: literal rename, new variant in the wrong bucket, or opener-side key failure classified differently across repositories.
- Timing/configuration: no timing dependence.
- Existing evidence: a finite table constructs every current `OpenError` variant and pins exact strings; the precedence test reaches `BadRecipientKey` through the public API.
- Audit status: audited for the local closed enum and exact literals. The external opener's vocabulary remains unaudited.
- Instrumentation: complete once every variant is constructed.
- Investigation log: code establishes a two-string image. Whether `BadRecipientKey -> malformed` is the intended external contract needs the missing opener/corpus.
