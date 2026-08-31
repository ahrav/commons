# Evidence: `version-one-is-only-emitted-and-accepted-version`

- Discovery lenses: protocol contracts, lifecycle transitions, version compatibility.
- Trigger: `VERSION` is both an emitted field and the only accepted version.
- Code trail: version definition, emission, and gate in [`src/lib.rs`](../../../../crates/cortexkit-push-seal/src/lib.rs); [`the_envelope_has_version_one_and_fixed_overhead`](../../../../crates/cortexkit-push-seal/src/lib.rs) and [`open_error_precedence_is_stable`](../../../../crates/cortexkit-push-seal/src/lib.rs).
- Implemented mechanism: equality against one compile-time constant; no negotiation or dual-read window.
- Failure scenario: a constant edit keeps symbolic self-tests green but diverges from the external opener.
- Timing/configuration: rollout order matters only when a second version is introduced; no such protocol exists today.
- Existing evidence: the public constant and emitted byte are pinned to literal `0x01`. A finite loop checks every other byte on a full-length envelope and proves version rejection precedes private-key parsing.
- Audit status: audited for local emission, exhaustive one-byte rejection, and local gate order. External opener behavior and future rollout remain unaudited.
- Instrumentation: complete locally; opener rollout evidence is missing.
- Investigation log: every current reference except the definition uses the symbol, so a one-line value change can preserve local round trips.
