# Evidence: `version-one-is-only-emitted-and-accepted-version`

- Discovery lenses: protocol contracts, lifecycle transitions, version compatibility.
- Trigger: `VERSION` is both an emitted field and the only accepted version.
- Code trail: `VERSION = 0x01` at `src/lib.rs:49-50`; emit at `:127`; gate at `:167-171`; tests at `:261-271` and `:291-296`.
- Implemented mechanism: equality against one compile-time constant; no negotiation or dual-read window.
- Failure scenario: a constant edit keeps symbolic self-tests green but diverges from the external opener.
- Timing/configuration: rollout order matters only when a second version is introduced; no such protocol exists today.
- Existing evidence: `0x02` and `0x7f` are rejected on full-length envelopes. No test pins the public constant itself to literal `0x01`.
- Instrumentation: complete locally; opener rollout evidence is missing.
- Investigation log: every current reference except the definition uses the symbol, so a one-line value change can preserve local round trips.
