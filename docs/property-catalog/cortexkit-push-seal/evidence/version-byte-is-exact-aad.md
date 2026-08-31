# Evidence: `version-byte-is-exact-aad`

- Discovery lenses: data integrity, security boundaries, protocol contracts.
- Trigger: docs say an unbound cleartext version could select a different parse.
- Code trail: `seal` and `open` both construct `[VERSION]`; `aad_and_info_are_exact` splits a valid envelope and calls HPKE directly.
- Implemented mechanism: both functions use `[VERSION]`, while `open` first requires `envelope[0] == VERSION`.
- Failure scenario: AAD becomes empty, gains bytes, or remains tied to the build constant while a second observed version is accepted.
- Timing/configuration: multi-version rollout is the dangerous transition. It is not present at this revision.
- Existing evidence: direct and public correct opens succeed for the same envelope; direct opens with empty, wrong one-byte, and extra-byte AAD return exact `HpkeError::OpenError` against empty `info`.
- Instrumentation: sufficient through direct dependency-level open.
- Investigation log: local HPKE semantics are audited. Agreement with the external opener remains unaudited.
