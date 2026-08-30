# Evidence: `open-is-total-over-bounded-input`

- Discovery lenses: failure recovery, resource boundaries, security boundaries.
- Trigger: `open` parses caller-provided byte slices and uses fixed offsets.
- Code trail: precondition gates at `src/lib.rs:162-171`; slicing at `:175` and `:184`; result mapping at `:187`.
- Implemented mechanism: the 33-byte length guard precedes every index and slice requiring that minimum.
- Failure scenario: malformed lengths or key bytes within the caller-owned size bound panic, read out of bounds, or escape the documented error taxonomy. Arbitrarily large valid-shaped input can trigger proportional allocation and is outside this totality claim.
- Timing/configuration: no concurrency timing. Focused lengths are 0, 32, 33, 48, and 49, plus large values.
- Existing evidence: fixed cases at lengths 32 and 33. No arbitrary-input campaign or fuzz target exists, and the transport bound is unknown.
- Instrumentation: `catch_unwind` plus the returned variant is sufficient for local totality.
- Investigation log: code inspection finds no unguarded library slice. `handopen` has separate argv and hex-decoding panic paths; those are example behavior, not `open` behavior.
