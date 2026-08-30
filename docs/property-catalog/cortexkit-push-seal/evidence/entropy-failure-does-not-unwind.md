# Evidence: `entropy-failure-does-not-unwind`

- Discovery lenses: failure recovery, dependencies, unproven assumptions.
- Trigger: docs say sealing failures preserve their cause, while the public signature returns `Result`; this record narrows the claim to the confirmed entropy-failure mechanism and assumes sufficient memory.
- Code trail: claim at `src/lib.rs:61-62`; call at `:115-123`; `hpke 0.14.0/src/single_shot.rs:102-125` wraps `SysRng` in `rand_core::UnwrapErr` and documents a panic on entropy failure.
- Competing explanation: all dependency failures might be converted into `HpkeError`. Dependency source discriminates: RNG failure occurs through a panicking wrapper before an `HpkeError` can be returned.
- Failure scenario: `getrandom` is blocked or fails in a sandbox, early boot, or constrained runtime.
- Timing/configuration: environment-specific, not input-specific. The `getrandom` path may block, retry, or fall back before returning or failing. Resolved `getrandom 0.4.3` supplies build-time `unsupported` and `custom` backends; `unsupported` deterministically forces failure, while `custom` can instrument draws.
- Existing evidence: none in repository tests; happy-path RNG use cannot exercise the branch.
- Instrumentation: build the test target with `getrandom_backend="unsupported"`; use accepted plaintext and a fixed valid non-low-order key so earlier guards pass; confirm the backend fired. A custom backend can observe draw calls. These backend flags are part of build identity.
- Investigation log: mechanism is confirmed; deployment reachability and consumer panic policy need human input.
