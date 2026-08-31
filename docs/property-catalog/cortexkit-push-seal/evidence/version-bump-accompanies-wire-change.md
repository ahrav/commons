# Evidence: `version-bump-accompanies-wire-change`

- Discovery lenses: lifecycle transitions, version compatibility, bug history.
- Trigger: docs call the crate version the only signal a path consumer receives.
- Code trail: claim at `src/lib.rs:13-20`; version at `Cargo.toml:3`; publication choice at `Cargo.toml:10-12`; workspace rule at `README.md:42-51`.
- Implemented mechanism: `tests/golden/push-seal-wire-v1.json` freezes one deterministic local envelope and all four local `OpenError` classifications. `tests/version_gate.rs` compares the fixture's represented `ciphersuite`, `inputs`, and `expected` projection plus package versions at explicit base/head revisions; formatting and provenance prose are excluded.
- Failure scenario: source or dependency changes emitted bytes or acceptance behavior while self-roundtrip tests stay green and version remains `0.1.0`.
- Timing/configuration: pull requests and pushes run the actual revision comparison on Ubuntu with explicit event SHAs. Local and `workflow_call` runs execute synthetic policy cases without claiming an actual revision comparison.
- Existing evidence: synthetic tests reject a represented fixture change with an unchanged version, accept the same change with a version bump, accept initial fixture bootstrap, and ignore formatting/provenance-only or unrelated changes. The actual gate fails on unreadable named revisions.
- Audit: audited only for changes represented by the committed fixture. Source changes that preserve the fixture, unrepresented behavior, and external opener compatibility remain unaudited.
- Instrumentation: the fixture records schema version, synthetic test-only provenance, verified package/version identity, suite and mode, empty `info`, version AAD, plaintext, exact envelope, and represented classifications. The tracked lockfile owns dependency identity.
- Investigation log: no independent opener corpus exists in this repository. The frozen bytes are a local regression oracle only.
