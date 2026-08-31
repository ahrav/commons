# Evidence: `open-error-precedence-is-stable`

- Discovery lenses: failure recovery, protocol contracts, version compatibility.
- Trigger: local and external implementations can agree on error mappings but disagree on which error wins for multi-defect input.
- Code trail: gate order in [`src/lib.rs`](../../../../crates/cortexkit-push-seal/src/lib.rs); [`open_error_precedence_is_stable`](../../../../crates/cortexkit-push-seal/src/lib.rs#L284).
- Implemented mechanism: first-return ordering is length, version, private-key length, then dependency decapsulation or authenticated open.
- Failure scenario: a truncated future-version envelope is `malformed` here but `unsupported_version` in an opener that checks the leading byte first.
- Timing/configuration: version rollout plus truncation is the highest-risk combination; no concurrency timing is involved.
- Existing evidence: a 32-byte wrong-version envelope with an invalid private key returns exact `Malformed`; every unsupported version on a full-length envelope with an invalid key returns exact `UnknownVersion`; the same corrupt, valid-version envelope returns exact `BadRecipientKey` with an invalid key and exact `Aead` with a valid key; the unchanged envelope opens.
- Audit status: audited for local length, version, key-parse, and authenticated-open precedence with multi-defect inputs and a valid control. External opener precedence remains unknown.
- Instrumentation: result variant and wire code are directly observable.
- Investigation log: local order is resolved from code. External opener order remains unknown because its repository was not supplied.
