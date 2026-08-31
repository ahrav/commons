# Evidence: `byte-determining-dependency-closure-is-pinned`

- Discovery lenses: dependencies, build configuration, lifecycle transitions.
- Trigger: fresh portfolio review found version-bump discipline cannot observe dependency drift when resolution is unrecorded.
- Code/config trail: `hpke = "=0.14.0"` and `getrandom = "=0.4.3"` now pin direct byte-affecting dependencies with explicit features. `Cargo.lock` is still ignored and CI Cargo commands still omit `--locked` at this stack layer.
- Resolution fact: direct HPKE and entropy versions no longer float, but their in-range transitive dependencies still determine KEM, KDF, AEAD, and RNG behavior without a repository-recorded closure.
- Build-identity scope: the check records an approved identity per `(supported target, build purpose)`: enabled features, `getrandom_backend` configuration, Cargo edge set, and the full enabled transitive graph from the tracked lockfile rather than a hand-maintained package subset. Purposes include default verification, deterministic-vector generation, and entropy-failure testing. The graph includes implementation crates below obvious wrappers, such as `curve25519-dalek`, `chacha20`, `poly1305`, and `hmac` in the current resolution.
- Competing explanation: suite-codepoint assertions might make resolution drift harmless. They cannot detect a same-codepoint serialization, key-schedule, clamping, or RNG-wrapper behavior change.
- Failure scenario: two fresh builds of one crate revision resolve different crypto closures and produce different bytes or classifications while local source and version remain unchanged.
- Timing/configuration: dependency publication between builds; workspace feature unification.
- Existing evidence: exact direct requirements and a recorded local dependency-source audit. There is still no tracked closure or locked CI build, so full resolution cannot be reconstructed from repository history alone.
- Instrumentation: compare `cargo metadata` plus the tracked lockfile, command edge set, enabled feature graph, target triple, build purpose, and `getrandom_backend` configuration; reject a build when any identity differs from the approved `(target, purpose)` contract.
- Investigation log: the unused direct `rand_core 0.9` requirement was removed. The `hex` dev dependency does not enter emitted bytes. Floating stable Rust was examined and excluded from this wire-identity property; absent a compiler defect, toolchain identity does not define HPKE wire bytes.
