# Evidence: `hpke-info-remains-empty`

- Discovery lenses: protocol contracts, security boundaries, version compatibility.
- Trigger: module docs identify `info` as a cross-implementation wire fact.
- Code trail: rationale at `src/lib.rs:39-42`; empty slices passed at `:119` and `:183`.
- Implemented mechanism: no public parameter or configuration can alter `info`.
- Failure scenario: only one repository introduces a domain string, causing opaque authentication failures.
- Timing/configuration: no runtime timing dependence; release skew is the relevant window.
- Existing evidence: source inspection only. No test opens one envelope with both empty and non-empty `info`.
- Instrumentation: requires a direct dependency-level open because the public API hardcodes the value.
- Investigation log: the value is confirmed locally. The reason it is safe depends on the separate key-dedication property.
