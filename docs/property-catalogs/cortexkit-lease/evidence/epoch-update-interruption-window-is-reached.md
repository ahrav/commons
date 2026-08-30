# `epoch-update-interruption-window-is-reached`

- **Discovery:** coverage/vacuity evaluation of crash atomicity.
- **Primary evidence:** the vulnerable point lies between `set_len(0)` and completed `write_all` (`crates/cortexkit-lease/src/lib.rs:328-338`, especially `:334-336`).
- **Coverage condition:** code reaches a named failpoint after truncation and process termination occurs before replacement bytes complete.
- **Why independent:** a correct implementation could reach the precondition and recover safely; the witness is not epoch regression itself.
- **Timing need:** deterministic failpoint; random kills are poor evidence for a two-syscall window.
- **Instrumentation:** no failpoint mechanism exists.
- **Open questions:** none.
