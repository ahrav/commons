# `replacement-fence-is-claimed-before-old-writer-writes`

- **Discovery:** targeted SQLite handover pass after portfolio evaluation.
- **Primary evidence:** `open_sqlite` reads the existing database fence, acquires a file epoch above that floor, and claims the epoch in an `IMMEDIATE` transaction before constructing the returned store (`cortexkit-store/src/lib.rs:225-273,275-299,312-343`). `with_conn_fenced` reuses the same `claim_fence` helper (`:164-179,312-343`).
- **Existing evidence:** open observes its claimed epoch before domain setup (`:533-545`); repeated sidecar loss issues greater epochs (`:572-598`); the synthetic handover rejects stale epoch 1 after epoch 2 claims (`:833-871`); equal epoch is permitted (`:907-927`).
- **Failure scenario:** an old epoch 1 connection survives and races a transaction after replacement file-lease acquisition but before replacement obtains the `IMMEDIATE` database transaction. No replacement store is exposed during this interval.
- **Timing window:** internal file-lease acquisition through committed database claim.
- **Instrumentation:** lease acquisition event, database fence value, and old-writer effect event.
- **Open-question log:** owner must define whether authority transfers at internal file-lease acquisition or successful `open_sqlite` return.
