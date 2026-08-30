# `invalid-epoch-fails-closed`

- **Discovery:** data-integrity and protocol-format passes.
- **Primary evidence:** `read_epoch` at `src/lib.rs:319-324` and `bump_epoch` at `:328-339`.
- **Discriminating fact:** valid-UTF-8 parse errors become zero through `unwrap_or(0)`; invalid UTF-8 propagates `read_to_string` failure. One corruption class therefore fails open and another fails closed.
- **Existing evidence:** no malformed-content test. T1 pre-creates an empty new file but does not distinguish new from damaged state.
- **Failure scenario:** garbage, decimal overflow, foreign non-decimal text, or future-format content silently issues epoch 1. Valid decimal truncation belongs to monotonicity, not parse failure.
- **Instrumentation:** missing format discriminator and corruption event.
- **Open-question log:** comments only say “0 if new/empty” (`:316,326`); they do not declare malformed content equivalent to new.
