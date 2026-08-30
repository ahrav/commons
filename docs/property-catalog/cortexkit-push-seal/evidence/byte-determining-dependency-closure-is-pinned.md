# Evidence: `byte-determining-dependency-closure-is-pinned`

- Discovery lenses: dependencies, build configuration, lifecycle transitions.
- Trigger: fresh portfolio review found version-bump discipline cannot observe dependency drift when resolution is unrecorded.
- Code/config trail: `hpke = "0.14"` and four explicit features at `crates/cortexkit-push-seal/Cargo.toml:15`; `Cargo.lock` ignored by `.gitignore:3`; CI Cargo commands omit `--locked` in `.github/workflows/ci.yml`.
- Resolution fact: `"0.14"` permits future `0.14.x` releases. `hpke` itself has in-range transitive dependencies that determine KEM, KDF, AEAD, and RNG behavior.
- Build-identity scope: the check records an approved identity per `(supported target, build purpose)`: enabled features, `getrandom_backend` configuration, Cargo edge set, and the full enabled transitive graph from the tracked lockfile rather than a hand-maintained package subset. Purposes include default verification, deterministic-vector generation, and entropy-failure testing. The graph includes implementation crates below obvious wrappers, such as `curve25519-dalek`, `chacha20`, `poly1305`, and `hmac` in the current resolution.
- Competing explanation: suite-codepoint assertions might make resolution drift harmless. They cannot detect a same-codepoint serialization, key-schedule, clamping, or RNG-wrapper behavior change.
- Failure scenario: two fresh builds of one crate revision resolve different crypto closures and produce different bytes or classifications while local source and version remain unchanged.
- Timing/configuration: dependency publication between builds; workspace feature unification.
- Existing evidence: no tracked closure and no locked CI build. Current and historical determinism cannot be reconstructed from repository history alone; it depends on registry state and any local lockfile present at each build.
- Instrumentation: compare `cargo metadata` plus the tracked lockfile, command edge set, enabled feature graph, target triple, build purpose, and `getrandom_backend` configuration; reject a build when any identity differs from the approved `(target, purpose)` contract.
- Investigation log: floating stable Rust toolchain was examined and excluded from this wire-identity property; absent a compiler defect, toolchain identity does not define HPKE wire bytes. Unused direct `rand_core 0.9` and `hex` do not enter emitted bytes.
