# `replacement-fence-is-claimed-before-old-writer-writes`

- **Discovery:** targeted SQLite handover pass after portfolio evaluation.
- **Primary evidence:** `open_sqlite` acquires epoch but does not claim the database fence (`cortexkit-store/src/lib.rs:245-284`); claim is lazy inside `with_conn_fenced` (`:162-205`).
- **Existing evidence:** synthetic handover test claims epoch 2 before stale epoch 1 attempts a write (`:629-667`); equal epoch is permitted (`:670-689`).
- **Failure scenario:** on a declared fence-protected path, old epoch 1 connection survives; replacement acquires epoch 2 but does not write; an old transaction commits after replacement acquisition while the database still stores epoch 1.
- **Timing window:** replacement acquisition through first replacement fenced write.
- **Instrumentation:** lease acquisition event, database fence value, and old-writer effect event.
- **Open-question log:** no design text accepts this window; owner must decide whether claim belongs at open.
