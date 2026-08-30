# `lease-file-creation-is-never-permissive`

- **Discovery:** targeted security refinement after portfolio evaluation.
- **Primary evidence:** `OpenOptions::create(true)` at `src/lib.rs:243-249,281-287` precedes hardening at `:254,288`; no create-time Unix mode is supplied.
- **Existing evidence:** T1 checks only post-acquisition mode (`:398-424`).
- **Failure scenario:** permissive umask creates `0644` or `0666`; another process opens during the window and retains access after chmod.
- **Timing window:** file creation through `protect_file` completion.
- **Instrumentation:** concurrent mode observer and open-success witness.
- **Open-question log:** deployment umasks were not supplied.
