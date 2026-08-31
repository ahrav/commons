# Evidence: `version-byte-is-exact-aad`

- Discovery lenses: data integrity, security boundaries, protocol contracts.
- Trigger: docs say an unbound cleartext version could select a different parse.
- Code trail: claim at `src/lib.rs:97-102`; AAD creation at `:114` and `:178`; direct dependency test at `:364-389`.
- Implemented mechanism: both functions use `[VERSION]`, while `open` first requires `envelope[0] == VERSION`.
- Failure scenario: AAD becomes empty, gains bytes, or remains tied to the build constant while a second observed version is accepted.
- Timing/configuration: multi-version rollout is the dangerous transition. It is not present at this revision.
- Existing evidence: opening without AAD fails and opening with current AAD succeeds. Other incorrect non-empty AAD values are not exercised.
- Instrumentation: sufficient through direct dependency-level open.
- Investigation log: the current invariant depends transitively on the version gate; this coupling should remain explicit.
