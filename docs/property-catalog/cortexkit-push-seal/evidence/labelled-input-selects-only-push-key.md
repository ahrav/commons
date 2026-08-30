# Evidence: `labelled-input-selects-only-push-key`

- Discovery lenses: bug history, security boundaries, wildcard.
- Trigger: three commits focus on wrong-key pastes, label selection, and `:` versus `=` parsing.
- Code trail: usage and hazard at `examples/handseal.rs:1-16`; selection at `:53-77`; validation at `:81-99`.
- Implemented mechanism: substring search, first matching line, selection of the text between the first and second `:` or `=`, token-only rejection, then exact 64-character hex validation of that selected segment.
- Failure scenario: duplicate key labels during rotation, prefixed/suffixed labels, extra separators with ignored suffix text, or bare token hex can select a value whose matching private key is unavailable while sealing still succeeds.
- Timing/configuration: operator paste and key-rotation windows; no process concurrency.
- Existing evidence: guards exist but examples contain no tests. The active property covers single-separator exact-label key-only and token-only cases plus documented bare-hex support (`examples/handseal.rs:3,76`). Substring, duplicate-label, and extra-separator/suffix semantics remain unresolved findings rather than required behavior.
- Instrumentation: make parser behavior callable from a test boundary and record exact selected label/value or refusal reason.
- Investigation log: commit history confirms this is a known hazard family. `examples/kp.rs:6-9` emits `PK ` and `SK ` prefixes that `handseal` cannot accept whole; the 66-character diagnostic at `handseal.rs:94-96` names only `SK `. Whether output should be directly pasteable, and whether duplicate or extra-separator labels occur during rotation, needs human input.
