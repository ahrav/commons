# Evidence: `cross-implementation-wire-vectors-conform`

- Discovery lenses: architecture, protocol contracts, version compatibility.
- Trigger: crate docs state that a separate implementation opens these bytes, while every repository test uses this implementation on both sides.
- Code trail: boundary claim at `src/lib.rs:6-20`; wire constants at `:30-59`; sealing at `:103-131`; opening and wire mapping at `:133-188`.
- Failure scenario: both local functions change coherently, so self-roundtrip stays green while the external opener rejects the new bytes or classifies failures differently.
- Timing/configuration: independent release and dependency schedules across repositories.
- Existing evidence: no external opener, authoritative corpus, or cross-language vector exists in supplied evidence.
- Instrumentation: configure the ambient `SysRng` through a recorded `getrandom` custom backend that emits a fixed byte stream; record target/build-purpose identity; compare exact local seal bytes and local open results against vectors exercised by the external opener.
- Investigation log: the compatibility obligation is documented locally. Whether it holds is unresolved and tagged `(needs human input)` because the opener and corpus were not supplied.
