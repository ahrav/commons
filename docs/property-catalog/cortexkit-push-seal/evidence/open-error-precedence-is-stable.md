# Evidence: `open-error-precedence-is-stable`

- Discovery lenses: failure recovery, protocol contracts, version compatibility.
- Trigger: local and external implementations can agree on error mappings but disagree on which error wins for multi-defect input.
- Code trail: length gate at `src/lib.rs:162-166`; version gate at `:167-171`; private-key parse at `:173-174`; authenticated open at `:175-187`.
- Implemented mechanism: first-return ordering is length, version, private-key length, then dependency decapsulation or authenticated open.
- Failure scenario: a truncated future-version envelope is `malformed` here but `unsupported_version` in an opener that checks the leading byte first.
- Timing/configuration: version rollout plus truncation is the highest-risk combination; no concurrency timing is involved.
- Existing evidence: length, version, and authenticated-open gates are tested separately; the wrong-length private-key gate is not. No claim-bearing test combines defects.
- Instrumentation: result variant and wire code are directly observable.
- Investigation log: local order is resolved from code. External opener order remains unknown because its repository was not supplied.
