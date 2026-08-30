# Evidence: `transport-bounds-open-envelope-size`

- Discovery lenses: resource boundaries, dependencies, unproven assumptions.
- Trigger: `open` explicitly omits a cap because transport is said to own it.
- Code trail: delegation at `src/lib.rs:156-160`; only minimum-length gate at `:162`; dependency allocates plaintext proportional to ciphertext length.
- Competing explanation: the cap may exist in a caller outside this repository. That cannot be confirmed or rejected with supplied evidence.
- Failure scenario: oversized untrusted envelope reaches `open` and causes large allocation and linear cryptographic work.
- Timing/configuration: memory pressure and caller admission policy matter.
- Existing evidence: no production caller or transport is present. `examples/handopen.rs:9` calls `open` without an explicit maximum and is not the delegated production boundary described by the library docs.
- Instrumentation: caller-boundary length counter, rejection result, and peak allocation watermark.
- Investigation log: repository search found no production caller. Question is tagged `(needs human input)`: identify the transport and exact maximum.
