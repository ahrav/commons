# Evidence: `open-is-total-over-bounded-input`

- Discovery lenses: failure recovery, resource boundaries, security boundaries.
- Trigger: `open` parses caller-provided byte slices and uses fixed offsets.
- Code trail: precondition gates at `src/lib.rs:162-171`; guarded slicing at `:176` and `:189`; result mapping at `:192`; sampled campaign at `:423-459`.
- Implemented mechanism: the 33-byte length guard precedes every index and slice requiring that minimum.
- Failure scenario: malformed lengths or key bytes within the caller-owned size bound panic, read out of bounds, or escape the documented error taxonomy. Arbitrarily large valid-shaped input can trigger proportional allocation and is outside this totality claim.
- Timing/configuration: no concurrency timing. Focused lengths are 0, 32, 33, 48, and 49, plus large values.
- Existing evidence: `sampled_malformed_bytes_are_total_through_the_local_envelope_bound` uses one deterministic malformed sample for every length `0..=2097`, checks the exact returned error under `catch_unwind`, and counts both sides of the length gate plus all four public error classes. A valid anchor opens first.
- Instrumentation: `catch_unwind` plus the returned variant is sufficient for local totality.
- Investigation log: the sampled campaign did not unwind. It does not cover every byte string, allocation failure, inputs above the largest locally emitted envelope, transport resource safety, or the missing caller-owned bound. The universal property remains unaudited. `handopen` has separate argv and hex-decoding panic paths; those are example behavior, not `open` behavior.
