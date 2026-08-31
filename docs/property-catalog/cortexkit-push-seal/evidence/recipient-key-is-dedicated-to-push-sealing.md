# Evidence: `recipient-key-is-dedicated-to-push-sealing`

- Discovery lenses: security boundaries, dependencies, unproven assumptions.
- Trigger: empty `info` is justified only by key dedication.
- Code trail: the module docs carry the dedication rationale; `seal_with_rng` and `open` both pass an empty `info`.
- Competing explanation: the same device key may be reused by another protocol in the unavailable caller or opener repository.
- Failure scenario: cross-protocol key reuse removes the intended key-schedule domain separation.
- Timing/configuration: provisioning changes, migrations, or later sender-authentication work are relevant transitions.
- Existing evidence: local claim only; no key identifier, provisioning code, or device code exists here.
- Instrumentation: organization-wide key-use inventory keyed by stable key purpose or identifier.
- Investigation log: repository search exhausted local evidence. Question is tagged `(needs human input)`.
