# Evidence: `version-bump-accompanies-wire-change`

- Discovery lenses: lifecycle transitions, version compatibility, bug history.
- Trigger: docs call the crate version the only signal a path consumer receives.
- Code trail: claim at `src/lib.rs:13-20`; version at `Cargo.toml:3`; publication choice at `Cargo.toml:10-12`; workspace rule at `README.md:42-51`.
- Implemented mechanism: human process only. CI runs formatting, Clippy, and tests but no wire-corpus/version-diff gate.
- Failure scenario: source or dependency changes emitted bytes or acceptance behavior while self-roundtrip tests stay green and version remains `0.1.0`.
- Timing/configuration: commit and release boundary, not runtime.
- Existing evidence: tracked source diffs after introduction are prose/tests/examples, so source history has not exercised a qualifying wire change. Historical dependency closures are unavailable because the lockfile is untracked.
- Instrumentation: fixed sealed/open vectors or a recorded deterministic custom RNG backend, plus a revision diff that requires a manifest version change when bytes or expected classifications move. Record the vector producer's target and build identity.
- Investigation log: no corpus exists in this repository. Ownership and location of the external corpus need human input.
