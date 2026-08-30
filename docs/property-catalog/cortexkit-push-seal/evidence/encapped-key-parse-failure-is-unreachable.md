# Evidence: `encapped-key-parse-failure-is-unreachable`

- Discovery lenses: reachability, protocol contracts, version compatibility.
- Trigger: fresh portfolio review found no `unreachable` record despite a dedicated impossible branch at `src/lib.rs:176`.
- Code trail: minimum-length gate at `src/lib.rs:162`; exact slice at `:175`; error mapping at `:176`; `ENC_LEN = 32` at `:59`.
- Dependency trail: `hpke 0.14.0/src/kem/dhkem.rs:62-68` delegates to X25519 public-key deserialization at `src/dhkex/x25519.rs:54-65`, whose only current failure is serialized length mismatch.
- Competing explanation: the parser could reject non-canonical or low-order points. Dependency source says conversion is infallible once length is 32; low-order rejection occurs later during DH.
- Failure scenario: a future KEM or dependency changes its serialized size or adds same-size semantic validation while local assumptions remain unchanged, waking the branch and mapping the new failure to `Aead`.
- Timing/configuration: build-time suite identity; no runtime schedule.
- Existing evidence: no branch instrumentation. The total-length test catches the size relation only indirectly.
- Instrumentation: a branch counter or mutation that forces the mapping closure to run, paired with a build-time assertion tying local split assumptions to the resolved deserializer contract.
- Investigation log: bounded dependency-facing branch pass confirmed this call-site branch is unreachable under the pinned suite.
