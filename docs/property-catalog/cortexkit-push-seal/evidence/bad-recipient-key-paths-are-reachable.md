# Evidence: `bad-recipient-key-paths-are-reachable`

- Discovery lenses: reachability, failure recovery, protocol contracts.
- Trigger: existing tests never construct either public `BadRecipientKey` result, and the open-side variant is part of `wire_code`.
- Code trail: public-key parse and mapping at `src/lib.rs:111-112`; private-key parse and mapping at `:173-174`; preceding size/version gates at `:104-109` and `:162-171`.
- Dependency trail: resolved `hpke 0.14.0` X25519 public and private `from_bytes` implementations reject serialized lengths other than 32 at `src/dhkex/x25519.rs:54-65` and `:80-100`.
- Failure scenario: a test supplies a wrong-length key but an earlier plaintext, envelope-length, or version gate wins, so the claimed branch remains unexercised.
- Timing/configuration: no timing dependence; precondition construction is the whole issue.
- Existing evidence: `key_deserialization_and_degenerate_public_key_paths_are_reachable` reaches both variants with 31-byte and 33-byte keys after the preceding gates succeed, with generated 32-byte controls.
- Instrumentation: direct variant result plus a branch counter if situation coverage needs to be explicit.
- Investigation log: both local paths are audited under the stated preceding gates. Whether semantic point/scalar validation should also map here remains an unresolved API contract question.
