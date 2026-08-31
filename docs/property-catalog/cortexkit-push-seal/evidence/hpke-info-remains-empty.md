# Evidence: `hpke-info-remains-empty`

- Discovery lenses: protocol contracts, security boundaries, version compatibility.
- Trigger: module docs identify `info` as a cross-implementation wire fact.
- Code trail: module rationale and both HPKE call sites use empty `info`; `aad_and_info_are_exact` calls HPKE directly.
- Implemented mechanism: no public parameter or configuration can alter `info`.
- Failure scenario: only one repository introduces a domain string, causing opaque authentication failures.
- Timing/configuration: no runtime timing dependence; release skew is the relevant window.
- Existing evidence: direct and public opens with empty `info` and exact AAD succeed; the same envelope with fixed non-empty `info` returns exact `HpkeError::OpenError`.
- Instrumentation: requires a direct dependency-level open because the public API hardcodes the value.
- Investigation log: local HPKE semantics are audited. External opener agreement and the separate key-dedication property remain unaudited.
