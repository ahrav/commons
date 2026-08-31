# Evidence: `entropy-failure-does-not-unwind`

- Discovery lenses: failure recovery, dependencies, unproven assumptions.
- Trigger: docs say sealing failures preserve their cause, while the public signature returns `Result`; this record narrows the claim to the confirmed entropy-failure mechanism and assumes sufficient memory.
- Code trail: public `seal` explicitly supplies `hpke::rand_core::UnwrapErr(getrandom::SysRng)` to `seal_with_rng` and documents the panic. The crate pins direct `getrandom = 0.4.3` and `hpke = 0.14.0`.
- Competing explanation: all dependency failures might be converted into `HpkeError`. Dependency source discriminates: RNG failure occurs through a panicking wrapper before an `HpkeError` can be returned.
- Failure scenario: `getrandom` is blocked or fails in a sandbox, early boot, or constrained runtime.
- Timing/configuration: environment-specific, not input-specific. The `getrandom` path may block, retry, or fall back before returning or failing. Resolved `getrandom 0.4.3` supplies build-time `unsupported` and `custom` backends; `unsupported` deterministically forces failure, while `custom` can instrument draws.
- Existing evidence: no panic-expecting test was added. Happy-path RNG observation cannot establish entropy-failure behavior.
- Instrumentation: build the test target with `getrandom_backend="unsupported"`; use accepted plaintext and a fixed valid non-low-order key so earlier guards pass; confirm the backend fired. A custom backend can observe draw calls. These backend flags are part of build identity.
- Investigation log: the current panic mechanism is documented, not approved as the desired contract. Deployment reachability, consumer policy, and the no-unwind property remain unaudited.
