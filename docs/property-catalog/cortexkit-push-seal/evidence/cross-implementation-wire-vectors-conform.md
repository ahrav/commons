# Evidence: `cross-implementation-wire-vectors-conform`

- Discovery lenses: architecture, protocol contracts, version compatibility.
- Trigger: crate docs state that a separate implementation opens these bytes, while every repository test uses this implementation on both sides.
- Code trail: the module docs state the separate-opener boundary; `VERSION`, `MAX_PLAINTEXT_BYTES`, and `ENC_LEN` fix the wire constants; `seal` and `seal_with_rng` emit the envelope; `open` and `OpenError::wire_code` parse it and classify failures.
- Failure scenario: both local functions change coherently, so self-roundtrip stays green while the external opener rejects the new bytes or classifies failures differently.
- Timing/configuration: independent release and dependency schedules across repositories.
- Existing evidence: no external opener, authoritative corpus, or cross-language vector exists in supplied evidence.
- Instrumentation: configure the ambient `SysRng` through a recorded `getrandom` custom backend that emits a fixed byte stream; record target/build-purpose identity; compare exact local seal bytes and local open results against vectors exercised by the external opener.
- Investigation log: the compatibility obligation is documented locally. Whether it holds is unresolved and tagged `(needs human input)` because the opener and corpus were not supplied.
