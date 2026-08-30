# `epoch-input-size-is-bounded`

- **Discovery:** resource-boundary pass.
- **Primary evidence:** shared acquisition and exclusive epoch bump both call `read_to_string` with no pre-check or `take` (`crates/cortexkit-lease/src/lib.rs:319-323,328-332`).
- **Existing evidence:** none; all generated files contain at most a few decimal bytes.
- **Failure scenario:** oversized restored or hostile file allocates proportional memory during module startup.
- **Timing window:** none; file contents alone enable it.
- **Instrumentation:** missing read-byte counter or bounded format parser.
- **Open-question log:** allocator failure behavior and intended future-format size are not specified.
