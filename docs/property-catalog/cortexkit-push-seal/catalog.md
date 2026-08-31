# `cortexkit-push-seal` property catalog

## Provenance and scope

- System: `crates/cortexkit-push-seal`
- Workspace revision observed: `34b0cae2d2ea7e181222c3d8b1957e394eb1dc2c`
- Target crate revision: unchanged since `292ed993c452456106124a398d0b40c392f68858`
- Catalog date: 2026-08-30
- Working tree at discovery: clean for this crate
- External evidence: none. The scope question was asked before analysis. No design docs, related repositories, issue trackers, incident reports, known failure modes, or earlier property artifacts were supplied.
- Repository evidence consulted: crate source, examples, manifest, workspace README and CI, Git history for the crate, and the source of the resolved `hpke 0.14.0` dependency where this crate delegates parsing and RNG behavior.

Documentation and commit messages are treated as claims or leads. Code resolves implemented behavior. The separate opener named in the module docs was not available, so cross-implementation agreement remains unverified.

## System model by discovery lens

| Lens | Model |
|---|---|
| Architecture and data flow | `seal` checks the plaintext and public key, calls HPKE base mode, and emits `version \|\| enc \|\| ciphertext`. `open` checks length and version, parses the private and encapsulated keys, then opens the ciphertext. `wire_code` maps local open failures to a two-string external vocabulary. |
| State and persistence | The crate source defines no retained state, persistence, cache, queue, or direct file I/O. `seal` enters the resolved OS entropy implementation, which may cache backend state and read `/dev/urandom` on Linux fallback paths. The envelope is the only artifact retained by this API. |
| Concurrency | The crate source defines no threads, async work, locks, atomics, or shared mutable state. The resolved entropy implementation uses internal atomic/cache state, so concurrency-specific crate invariants are not inferred from the absence of local synchronization. |
| Claimed safety | The docs claim a fixed suite, layout, version, AAD, empty `info`, inclusive plaintext cap, error vocabulary, recipient confidentiality, and version-bump discipline. |
| Claimed liveness | No explicit liveness guarantee. The library has no convergence or eventual-completion protocol. `seal` also depends on the OS entropy path, which may block, retry, or fall back before returning or panicking. |
| Bug history and density | Seven commits touch the crate. Three concentrate on selecting the correct operator-pasted key in `handseal`; no production incident or library regression is recorded. The library implementation has not changed since its initial commit. |
| Existing test strategy | Fourteen in-module unit tests cover round trips, suite IDs, layout and overhead, ephemeral freshness on both the ambient and RNG-injected paths, cap boundaries, open-error precedence, prefix and single-bit campaigns, sampled totality, the wire vocabulary, wrong recipient, exact AAD and `info`, key-deserialization reachability, and low-order encapsulation. There is no cross-language corpus, integration test, property test, fuzz target, fault injection, or example test. |
| Failure and degradation | The crate itself performs no retry or fallback. Its dependency may retry or fall back while obtaining entropy and panics if the ambient RNG ultimately fails. Crate documentation claims that a sealer/opener wire mismatch is silent locally and appears on the device as an undecryptable notification; the unavailable device path was not verified. |
| Dependencies | `hpke 0.14` supplies all cryptographic behavior; direct `getrandom 0.4` supplies the ambient `SysRng` that `seal` passes to it. hpke's `getrandom` feature and `hex` are dev-only. The tracked `Cargo.lock` records the resolved version-and-checksum closure, and CI uses `--locked`. Target-specific feature activation and entropy backends, alternate consumers, the external opener, and non-default build purposes remain unaudited. See [`byte-determining-dependency-closure-is-pinned`](evidence/byte-determining-dependency-closure-is-pinned.md). |
| Product context | This crate seals push-notification payloads. The actual opener is in another repository. The examples are operator tools for generating a keypair and hand-checking a round trip. |
| Unproven assumptions | The transport bounds input before `open`; the recipient key is dedicated to this protocol; the external opener agrees on suite, layout, gate order, and wire codes; every byte-affecting change includes a crate-version bump. |
| Wildcard | `open` hardcodes `ENC_LEN` while `seal` uses the serialized key length. The version value is not pinned to literal `0x01` by a test. The example label parser uses substring and first-match selection. Base-mode HPKE provides neither replay detection nor sender authentication. |

### Contract-versus-code leads

These disagreements stay visible because code may be the defect.

1. `BadRecipientKey` is documented as invalid X25519 point/scalar detection in the `SealError` and `OpenError` variant docs, but the resolved dependency checks only the 32-byte serialized length. A degenerate 32-byte public key can instead reach `SealError::Hpke`.
2. The docs say sealing failures preserve their cause in the `SealError` docs, but the dependency's ambient RNG wrapper panics on entropy failure.
3. The docs delegate `open`'s size bound to transport in the `open` docs, but no transport or bound exists in this repository.
4. The docs require version bumps for emitted-byte or behavior changes in the module docs, but no repository check enforces the rule.
5. The docs say the recipient key is dedicated to this purpose in the module docs, but this repository cannot inspect key use in the device or caller.

## Existing-check inventory

Audited checks are identified below. Other checks remain **unaudited**. Production-guard placement and failure behavior belong to `/low-level-systems:defensive-assertions-and-invariant-guards`.

### Production and example guards

| Location | Semantics and message | Status |
|---|---|---|
| `seal_with_rng` plaintext gate | Rejects plaintext over 2048 bytes with `PlaintextTooLarge { limit, observed }`. | unaudited runtime guard |
| `open` length gate | Rejects envelopes shorter than 33 bytes with `Malformed { observed }`. | unaudited runtime guard |
| `open` version gate | Rejects any leading byte other than `VERSION` with `UnknownVersion { observed }`. | unaudited runtime guard |
| `seal_with_rng` key and HPKE mapping | Maps public-key deserialization to `BadRecipientKey` and HPKE sealing failure to `Hpke`. | unaudited runtime validation/error mapping |
| `open` key and HPKE mapping | Maps private-key deserialization to `BadRecipientKey`, encapsulated-key parsing and HPKE open failures to `Aead`. | unaudited runtime validation/error mapping |
| `examples/handseal.rs:20-31` | Requires two arguments; exits 2 with usage or parser error. | unaudited example guard |
| `examples/handseal.rs:56-77` | Selects `push_seal_pubkey_hex`, accepts `:` or `=`, rejects a token-only labelled block. | unaudited example guard |
| `examples/handseal.rs:81-99` | Rejects empty, non-hex, and non-64-character keys with operator-facing messages. | unaudited example guard |
| `examples/handseal.rs:33-35` | Slices validated ASCII hex in two-character chunks and uses `expect("checked above")`; correctness depends on `validate` running first. | unaudited example invariant/panic site |
| `examples/handseal.rs:38` | `expect("seal")`; converts every library sealing error into a panic. | panic site, not an invariant guard |
| `examples/handopen.rs:2-9` | Positional indexing, slicing, and `unwrap`; malformed arguments can panic. | panic sites, not invariant guards |

No production `assert!`, `debug_assert!`, or equivalent invariant assertion was found.

### Claim-bearing tests

| Test and location | Existing semantics | Status |
|---|---|---|
| [`the_pinned_suite_has_the_documented_codepoints`](../../../crates/cortexkit-push-seal/src/lib.rs) | Three literal codepoint equalities; messages name KEM, KDF, and AEAD. | audited for local build-wide codepoints; external opener equality remains unaudited |
| [`wire_v1_fixture_matches_local_bytes_and_classifications`](../../../crates/cortexkit-push-seal/src/lib.rs) | Regenerates one exact deterministic envelope through the private RNG seam, opens it, and checks all represented local errors and wire codes. | audited as a local fixture oracle; not independent opener evidence |
| `synthetic_version_gate_cases` and `actual_git_diff_requires_version_bump`, `tests/version_gate.rs` | Exercise fixture/version policy in memory and compare explicitly named Git revisions only when both event SHAs are present. | audited for represented fixture changes; unrepresented behavior remains unaudited |
| [`a_sealed_payload_opens_to_the_same_plaintext`](../../../crates/cortexkit-push-seal/src/lib.rs) | Exact byte round trips at lengths 0, 2047, and 2048 with non-UTF-8 fixtures. | audited for these local boundaries |
| [`the_envelope_has_version_one_and_fixed_overhead`](../../../crates/cortexkit-push-seal/src/lib.rs) | Pins literal version 1, 32-byte encapsulation, 49-byte overhead, and 2097-byte maximum at lengths 0, 1, and 2048. | audited for the leading version and local size literals |
| [`each_seal_uses_a_fresh_ephemeral`](../../../crates/cortexkit-push-seal/src/lib.rs) | Records one 32-byte draw per seal; distinct draws produce distinct `enc`, and repeated draws repeat it. | audited for local draw/context behavior |
| [`an_oversized_plaintext_is_refused_with_both_numbers`](../../../crates/cortexkit-push-seal/src/lib.rs) | Invalid key plus length 2049 returns literal cap fields; a matching-key 2048-byte round trip is the positive control. | audited for the local cap boundary and guard order |
| [`open_error_precedence_is_stable`](../../../crates/cortexkit-push-seal/src/lib.rs) | Multi-defect inputs pin length, every unsupported version byte, private-key parsing, and authenticated-open order with exact variants and a valid control. | audited for local precedence |
| [`every_open_failure_maps_to_the_wire_vocabulary`](../../../crates/cortexkit-push-seal/src/lib.rs) | A finite table pins both literals for every `OpenError` variant, including `BadRecipientKey`. | audited for the local enum mapping |
| [`the_wrong_recipient_cannot_open`](../../../crates/cortexkit-push-seal/src/lib.rs) | Asserts generated public keys differ before the second private key returns `Aead`. | audited for the sampled local keypairs |
| [`every_proper_prefix_of_a_valid_envelope_is_rejected`](../../../crates/cortexkit-push-seal/src/lib.rs) | Every proper prefix returns exact `Malformed` or `Aead` with a valid anchor control. | audited for one generated anchor |
| [`single_bit_mutations_have_field_specific_outcomes`](../../../crates/cortexkit-push-seal/src/lib.rs) | Every bit in version, `enc`, ciphertext, and tag reaches its exact local rejection. | audited for one generated anchor |
| [`sampled_malformed_bytes_are_total_through_the_local_envelope_bound`](../../../crates/cortexkit-push-seal/src/lib.rs) | Samples every length through 2097 and exercises focused deep HPKE lengths without unwinding. | sampled evidence; universal totality unaudited |
| [`aad_and_info_are_exact`](../../../crates/cortexkit-push-seal/src/lib.rs) | Direct and public correct opens succeed; altered AAD or `info` fails. | audited for local HPKE semantics |
| [`key_deserialization_and_degenerate_public_key_paths_are_reachable`](../../../crates/cortexkit-push-seal/src/lib.rs) | Exercises sampled accepted/rejected key lengths and one degenerate public key. | audited for the sampled inputs |
| [`low_order_encapsulation_reaches_decap_error`](../../../crates/cortexkit-push-seal/src/lib.rs) | Observes direct `DecapError` before the public `Aead` collapse. | audited for one fixed low-order input |

## Property catalog

### matching-key-roundtrip-preserves-plaintext

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited at lengths 0, 2047, and 2048 with non-UTF-8 bytes
- **Guarantee:** For every plaintext of at most 2048 bytes and every generated matching keypair, opening the sealed envelope returns exactly the original bytes.
- **Check:** `always(open(sk, seal(pk, plaintext)?)? == plaintext)` for lengths `0..=MAX_PLAINTEXT_BYTES`, including non-UTF-8 bytes. `always` fits because every accepted seal/open pair must preserve bytes.
- **Fault/timing angle:** Boundary lengths 0 and 2048; binary payloads; no injected fault.
- **Required faults and enabling state:** Matching keypair and accepted plaintext. The workload must reach both length boundaries.
- **Confidence:** high; [evidence](evidence/matching-key-roundtrip-preserves-plaintext.md)
- **Existing check:** [`a_sealed_payload_opens_to_the_same_plaintext`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for the named finite boundaries, not a universal or external compatibility claim.
- **Impact:** Failure destroys the crate's core function and, according to crate documentation, renders notifications undecryptable.
- **Open questions:** None.

### pinned-ciphersuite-codepoints

- **Type:** safety
- **Status:** active
- **Exercised:** yes; all three codepoints are asserted as literals and repeated in the schema-versioned local wire fixture
- **Guarantee:** The suite remains KEM `0x0020`, KDF `0x0001`, and AEAD `0x0003`.
- **Check:** `always(KEM_ID == 0x0020 && KDF_ID == 0x0001 && AEAD_ID == 0x0003)`. `always` fits because these are build-wide wire constants.
- **Fault/timing angle:** Dependency upgrade or type substitution behind a stable name.
- **Required faults and enabling state:** None; evaluate on every build.
- **Confidence:** high; [evidence](evidence/pinned-ciphersuite-codepoints.md)
- **Existing check:** [`the_pinned_suite_has_the_documented_codepoints`](../../../crates/cortexkit-push-seal/src/lib.rs) and [`wire_v1_fixture_matches_local_bytes_and_classifications`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for local build-wide codepoints. External opener equality remains unaudited.
- **Impact:** Mismatch causes cross-repository authentication failure with no local sealer error.
- **Open questions:** Whether the unavailable opener asserts the same literals.

### cross-implementation-wire-vectors-conform

- **Type:** safety
- **Status:** active
- **Exercised:** not yet; the external opener and shared vector corpus were not supplied
- **Guarantee:** The local sealer and external opener agree byte-for-byte on suite, version, layout, `info`, AAD, plaintext, and failure classification in both directions.
- **Check:** Under a recorded deterministic `getrandom` custom backend, local `seal` of each fixed `(recipient public key, plaintext, RNG byte stream)` equals the expected envelope that the external opener accepts; fixed external envelopes open locally to expected plaintext or exact wire failure. Record target and build-purpose identity with every vector. `always` fits because every vector is a normative cross-implementation contract.
- **Fault/timing angle:** Independent implementation drift, release skew, or dependency drift that keeps local self-roundtrip tests green.
- **Required faults and enabling state:** External opener or authoritative corpus, deterministic custom entropy backend, matching key material, positive vectors, and multi-defect negative vectors.
- **Confidence:** high that the contract is required, low that it currently holds; [evidence](evidence/cross-implementation-wire-vectors-conform.md)
- **Existing check:** none; all current cryptographic tests seal and open with this implementation.
- **Impact:** This is the direct oracle for the documented cross-repository compatibility boundary.
- **Open questions:** Corpus location, ownership, vocabulary, opener build identity, and vector-production process. `(needs human input)`

### envelope-layout-and-overhead-stay-fixed

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited at plaintext lengths 0, 1, and 2048
- **Guarantee:** Every envelope is `0x01 || 32-byte enc || ciphertext`, and its length is `plaintext.len() + 49`.
- **Check:** `always(envelope[0] == 0x01 && enc.len() == 32 && envelope.len() == plaintext.len() + 49 && envelope.len() <= 2097)`, plus a manual split that opens successfully. Literal sizes pin the wire rather than restating local constants; 2097 is the maximum local envelope size.
- **Fault/timing angle:** KEM or AEAD change, field insertion/reordering, stale `ENC_LEN`.
- **Required faults and enabling state:** Accepted plaintexts at lengths 0, 1, 2048 and a matching keypair.
- **Confidence:** high; [evidence](evidence/envelope-layout-and-overhead-stay-fixed.md)
- **Existing check:** [`the_envelope_has_version_one_and_fixed_overhead`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for the leading version and local size literals. Field ordering beyond the version, external opener compatibility, and transport compatibility remain unaudited.
- **Impact:** Wrong offsets or overhead break the external opener and downstream transport sizing.
- **Open questions:** Transport encoding and byte limit are unavailable.

### version-one-is-only-emitted-and-accepted-version

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited; literal `VERSION == 0x01` is pinned and every other byte is rejected after the length gate
- **Guarantee:** `seal` emits literal version `0x01`, and `open` accepts no other version after the minimum-length gate passes.
- **Check:** `always(VERSION == 0x01 && seal(...)?[0] == 0x01)` and, for all `v != 0x01` on envelopes of at least 33 bytes, `open(...) == UnknownVersion { observed: v }`.
- **Fault/timing angle:** One-byte constant edit or rollout of a second format.
- **Required faults and enabling state:** Full-length envelope with each non-`0x01` leading byte.
- **Confidence:** high; [evidence](evidence/version-one-is-only-emitted-and-accepted-version.md)
- **Existing check:** [`the_envelope_has_version_one_and_fixed_overhead`](../../../crates/cortexkit-push-seal/src/lib.rs) and [`open_error_precedence_is_stable`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for local emission, acceptance, and precedence. Rollout and external opener agreement remain unaudited.
- **Impact:** A version drift is a silent cross-repository wire break.
- **Open questions:** Whether future rollout requires dual-version acceptance.

### version-byte-is-exact-aad

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited with a direct HPKE differential oracle
- **Guarantee:** The associated data used by both sides is exactly the envelope's one-byte version field.
- **Check:** After the version gate establishes `envelope[0] == VERSION`, `always([VERSION] == [envelope[0]])`; direct HPKE open succeeds with computed AAD `[VERSION]` and fails with `[]`, `[0x00]`, and `[0x01, 0x00]`. `always` fits every accepted envelope and reflects what the code computes.
- **Fault/timing angle:** AAD refactor or multi-version acceptance while `open` continues using the build constant.
- **Required faults and enabling state:** Valid envelope plus exact and altered AAD values.
- **Confidence:** high for current code; [evidence](evidence/version-byte-is-exact-aad.md)
- **Existing check:** [`aad_and_info_are_exact`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for direct correct open and empty, wrong, and extra AAD. External opener compatibility remains unaudited.
- **Impact:** Unbound or mismatched version bytes permit parse confusion or cause opaque authentication failures.
- **Open questions:** None at version 1; multi-version rollout remains unspecified.

### hpke-info-remains-empty

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited with a direct HPKE differential oracle
- **Guarantee:** Both sealer and opener use an empty HPKE `info` value.
- **Check:** Pin both call sites to `&[]`, then differentially open one sealed envelope with empty and fixed non-empty `info`; only empty succeeds. `always` fits because `info` is a build-wide wire constant. The durable compatibility oracle belongs in the external conformance corpus.
- **Fault/timing angle:** Adding domain separation on only one implementation.
- **Required faults and enabling state:** Direct dependency-level open using empty and non-empty `info`.
- **Confidence:** high for this implementation; [evidence](evidence/hpke-info-remains-empty.md)
- **Existing check:** [`aad_and_info_are_exact`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for empty `info` success and fixed non-empty `info` failure against the same envelope. External opener compatibility and key-purpose isolation remain unaudited.
- **Impact:** Any one-sided change breaks every envelope.
- **Open questions:** Whether the recipient key is actually dedicated is tracked separately.

### plaintext-cap-is-inclusive-and-nontruncating

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited at 2047, 2048, and 2049 bytes
- **Guarantee:** Plaintexts over 2048 bytes return both the limit and observed size before key parsing, RNG use, or envelope allocation; accepted lengths are never rejected as `PlaintextTooLarge`.
- **Check:** `always(len > 2048 => PlaintextTooLarge { limit: 2048, observed: len })` and `always(len <= 2048 => result != PlaintextTooLarge)`, with source or instrumentation confirming guard order. For a generated valid key and working entropy source, a 2048-byte plaintext must round-trip unchanged.
- **Fault/timing angle:** Off-by-one errors and accidental truncation.
- **Required faults and enabling state:** Plaintext lengths 2047, 2048, 2049, and a much larger value.
- **Confidence:** high; [evidence](evidence/plaintext-cap-is-inclusive-and-nontruncating.md)
- **Existing check:** [`a_sealed_payload_opens_to_the_same_plaintext`](../../../crates/cortexkit-push-seal/src/lib.rs) and [`an_oversized_plaintext_is_refused_with_both_numbers`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for neighboring accepted lengths, exact 2049 error fields, guard order before key parsing, and an opened at-limit control.
- **Impact:** Truncation produces an authenticated blob that does not represent caller intent; rejecting everything would block notifications.
- **Open questions:** Whether the composing caller also preflights the cap.

### tampered-or-truncated-envelope-never-opens

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited for every proper prefix and every single-bit field mutation of one valid anchor, plus every unsupported version byte and the 33-byte minimum-length envelope
- **Guarantee:** For the finite anchor campaign, every proper prefix and every single-bit version, encapsulated-key, ciphertext, and tag mutation is rejected.
- **Check:** For one valid anchor, require exact `Malformed` or `Aead` errors at every proper-prefix length and exact `UnknownVersion` or `Aead` errors for each field mutation. The finite campaign is empirical evidence, not universal mutation coverage or proof of zero forgery probability.
- **Fault/timing angle:** Short read, partial write, transport corruption, or active tampering in the version, `enc`, ciphertext, or tag.
- **Required faults and enabling state:** A valid envelope that first opens successfully, all proper-prefix lengths, and every single-bit position across every field.
- **Confidence:** high for the finite local campaign; [evidence](evidence/tampered-or-truncated-envelope-never-opens.md)
- **Existing check:** [`every_proper_prefix_of_a_valid_envelope_is_rejected`](../../../crates/cortexkit-push-seal/src/lib.rs) and [`single_bit_mutations_have_field_specific_outcomes`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for one generated anchor with field reach counters. [`open_error_precedence_is_stable`](../../../crates/cortexkit-push-seal/src/lib.rs) additionally rejects every non-`0x01` version byte and the 33-byte gate boundary. This is not cross-implementation evidence.
- **Impact:** Acceptance would expose attacker-controlled or partial notification content.
- **Open questions:** None.

### wrong-recipient-never-opens

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited for one asserted-distinct generated recipient pair
- **Guarantee:** Under HPKE and AEAD security assumptions, an envelope sealed to one recipient opens under a distinct recipient key only with negligible forgery probability.
- **Check:** After asserting `other_pk != pk`, require `always(open(other_sk, seal(pk, plaintext)?) == Err(OpenError::Aead))` for the sampled keypairs; any acceptance is a failure requiring cryptographic investigation.
- **Fault/timing angle:** Key selection or environment mix-up.
- **Required faults and enabling state:** Two generated keypairs with asserted-distinct public keys and a non-empty plaintext.
- **Confidence:** high; [evidence](evidence/wrong-recipient-never-opens.md)
- **Existing check:** [`the_wrong_recipient_cannot_open`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for the sampled local pair after asserting distinct public keys. The cryptographic guarantee and external key-selection paths are not universally audited.
- **Impact:** Violation breaks recipient confidentiality.
- **Open questions:** None.

### open-error-precedence-is-stable

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited with multi-defect inputs
- **Guarantee:** Open failures are selected in the order: short envelope, unsupported version, bad private-key length, then dependency decapsulation or authenticated-open failure.
- **Check:** `always(result == first_applicable_error)` over a table combining short length, unknown version, wrong-length private key, and corrupt ciphertext. `always` fits because precedence is part of deterministic wire classification.
- **Fault/timing angle:** Multi-defect envelope, especially a truncated future-version envelope.
- **Required faults and enabling state:** Inputs with at least two defects at once; single-defect tests are vacuous for precedence.
- **Confidence:** high for this implementation; cross-implementation agreement is unknown; [evidence](evidence/open-error-precedence-is-stable.md)
- **Existing check:** [`open_error_precedence_is_stable`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for local length, exhaustive version-byte, private-key parsing, and authenticated-open precedence with exact errors and a valid control. External opener precedence remains unknown.
- **Impact:** If the unavailable opener uses different precedence, the implementations produce different corpus results and diagnosis.
- **Open questions:** The unavailable opener's gate order.

### wire-error-vocabulary-is-stable

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited for every current `OpenError` variant
- **Guarantee:** `UnknownVersion` maps to `unsupported_version`; `Malformed`, `BadRecipientKey`, and `Aead` map to `malformed`; no other string is emitted.
- **Check:** `always(wire_code(error) == expected_literal)` for every enum variant. `always` fits because the mapping is a total wire function.
- **Fault/timing angle:** New error variant, string rename, or reclassification.
- **Required faults and enabling state:** Construct all variants, including a wrong-length private key.
- **Confidence:** high; [evidence](evidence/wire-error-vocabulary-is-stable.md)
- **Existing check:** [`every_open_failure_maps_to_the_wire_vocabulary`](../../../crates/cortexkit-push-seal/src/lib.rs), with `BadRecipientKey` reached through [`open_error_precedence_is_stable`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for the local enum and exact literals. External vocabulary agreement remains unaudited.
- **Impact:** Drift breaks the cross-language conformance vocabulary.
- **Open questions:** Whether opener-side key misconfiguration should intentionally collapse to `malformed`.

### each-seal-uses-fresh-ephemeral

- **Type:** safety
- **Status:** active
- **Exercised:** yes and locally audited through both the RNG-injected sealing path and two calls to the public `seal`
- **Guarantee:** Every successful `seal` after the plaintext and key gates uses a newly generated ephemeral and a newly constructed sender context.
- **Check:** A test-only RNG records exactly one fresh draw per successful call and supplies distinct fixed draw bytes; each call constructs a new sender context. A negative-control RNG repeats draw bytes and must produce repeated `enc`, proving the no-repeat canary detects degraded entropy. Production no-repeat campaigns remain statistical evidence, not proof. `always` applies to the per-successful-call draw/context obligation.
- **Fault/timing angle:** Degraded or accidental deterministic custom RNG, or cached sender context. The resolved default `SysRng` obtains OS bytes per call; fork duplication is not asserted for that backend.
- **Required faults and enabling state:** Two successful seals with identical valid recipient and accepted plaintext under distinct deterministic draws, plus two seals under a repeated-draw negative control.
- **Confidence:** high for intended behavior; [evidence](evidence/each-seal-uses-fresh-ephemeral.md)
- **Existing check:** [`each_seal_uses_a_fresh_ephemeral`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for one 32-byte draw per successful `seal_with_rng` call, distinct deterministic draws producing distinct encapsulations, and repeated draws producing repeated encapsulations. The same test also calls the public `seal` twice and requires distinct encapsulations, so an ambient RNG replaced by a fixed or seeded source fails the suite. Production entropy quality remains outside this check.
- **Impact:** Reuse can repeat the AEAD key/nonce pair and break confidentiality.
- **Open questions:** Whether production build flags can select a custom entropy backend and how that configuration is controlled.

### open-is-total-over-bounded-input

- **Type:** safety
- **Status:** active
- **Exercised:** yes with one deterministic malformed sample at every length `0..=2097`; the universal property remains unaudited
- **Guarantee:** Within the caller-owned envelope bound and available-memory contract, every private-key and envelope byte string returns `Ok` or a documented `OpenError` without indexing or parsing panic.
- **Check:** For `envelope.len() <= TRANSPORT_MAX_ENVELOPE_BYTES`, `always(catch_unwind(|| open(key, envelope)).is_ok())` over generated bytes, with focused lengths 0, 32, 33, 48, and 49. Allocation failure is outside this claim.
- **Fault/timing angle:** Boundary slicing and malformed authenticated data.
- **Required faults and enabling state:** Arbitrary key and envelope bytes within the caller-owned bound, including very short inputs. The exact bound is unresolved.
- **Confidence:** medium by code inspection because the caller bound is unavailable; [evidence](evidence/open-is-total-over-bounded-input.md)
- **Existing check:** [`sampled_malformed_bytes_are_total_through_the_local_envelope_bound`](../../../crates/cortexkit-push-seal/src/lib.rs); sampled evidence only. It reaches every length through the largest locally emitted envelope, every public error class, and focused authenticated-open lengths through 2097, but not every byte string, transport resource safety, or a caller-owned bound. Status remains unaudited.
- **Impact:** Panic would turn malformed input into denial of service for any caller exposing `open`.
- **Open questions:** Whether untrusted input reaches this helper outside corpus generation.

### entropy-failure-does-not-unwind

- **Type:** safety
- **Status:** active — dependency source predicts this claim is violated if the ambient RNG fails
- **Exercised:** no; ambient entropy failure remains an untested known panic
- **Guarantee:** Under sufficient memory, an entropy-source error is returned through the sealing error surface instead of unwinding through `seal`.
- **Check:** Build with the resolved `getrandom` `unsupported` backend; call `seal` with accepted plaintext and a fixed valid, non-low-order 32-byte public key; prove the backend was invoked; then require `always(catch_unwind(seal) == Ok(Err(_)))`. The input preconditions and backend witness prevent earlier guards from producing a vacuous green result.
- **Fault/timing angle:** OS entropy-source failure before encapsulation.
- **Required faults and enabling state:** Sufficient memory, accepted plaintext, fixed valid non-low-order public key, and deterministic entropy failure through `getrandom_backend="unsupported"` or a failing custom backend, with an invocation witness.
- **Confidence:** high that the resolved dependency panics; medium that the crate promises otherwise; [evidence](evidence/entropy-failure-does-not-unwind.md)
- **Existing check:** none by design in this wave. [`seal`](../../../crates/cortexkit-push-seal/src/lib.rs) documents the current panic; consumer policy and the desired no-unwind property remain unaudited.
- **Impact:** Depending on panic policy, one entropy failure can abort a request task or process instead of returning a diagnosable error.
- **Open questions:** Consumer panic policy and deployment conditions; both need human input.

### transport-bounds-open-envelope-size

- **Type:** safety
- **Status:** active
- **Exercised:** not yet; no production transport caller exists in this repository
- **Guarantee:** Every call to `open` receives an envelope no larger than the transport-owned maximum.
- **Check:** `always(envelope.len() <= TRANSPORT_MAX_ENVELOPE_BYTES)` at the caller boundary, with an allocation watermark confirming bounded memory. `always` fits every call.
- **Fault/timing angle:** Oversized untrusted envelope under memory pressure.
- **Required faults and enabling state:** Caller integration plus maximum-size and over-limit inputs.
- **Confidence:** low for the system, high that this crate delegates the obligation; [evidence](evidence/transport-bounds-open-envelope-size.md)
- **Existing check:** none.
- **Impact:** Without the delegated bound, `open` allocates and performs work in proportion to attacker-controlled input.
- **Open questions:** Which transport owns the bound and its exact value. `(needs human input)`

### recipient-key-is-dedicated-to-push-sealing

- **Type:** safety
- **Status:** active
- **Exercised:** not yet; key use is outside this repository
- **Guarantee:** A recipient key used with empty HPKE `info` is never reused by another protocol or purpose.
- **Check:** `always(key_id used by push sealing is absent from every non-push protocol key-use site)`. `always` fits the domain-separation assumption.
- **Fault/timing angle:** Key reuse introduced by provisioning, migration, or sender-authentication work.
- **Required faults and enabling state:** Complete key-provisioning and device-use inventory across repositories.
- **Confidence:** low; only the local documentation claims dedication; [evidence](evidence/recipient-key-is-dedicated-to-push-sealing.md)
- **Existing check:** none.
- **Impact:** If the key is shared, empty `info` loses protocol-level domain separation.
- **Open questions:** Actual device and provisioning key-use graph. `(needs human input)`

### labelled-input-selects-only-push-key

- **Type:** safety
- **Status:** active
- **Exercised:** not yet; examples have no tests
- **Guarantee:** A single-separator block with one exact `push_seal_pubkey_hex` label selects that value with either `:` or `=`; a block with only `apns_device_token_hex` is rejected; bare 64-character hex remains supported; empty, non-hex, and wrong-length selected values are rejected rather than repaired.
- **Check:** `always(select_key(input) == expected)` over single-separator exact-label key-only and token-only blocks, both separators, valid bare hex, empty values, non-hex, and wrong lengths. Duplicate labels, substring labels, and extra separators/suffixes are excluded until their contract is decided.
- **Fault/timing angle:** Operator paste of a single-separator key-only or token-only labelled block.
- **Required faults and enabling state:** Single-separator exact-label blocks and bare input representing each listed case; the token-only situation must occur at least once.
- **Confidence:** high for the narrow exact-label contract; [evidence](evidence/labelled-input-selects-only-push-key.md)
- **Existing check:** guards at `examples/handseal.rs:53-99`; no test; status unaudited.
- **Impact:** Wrong selection can seal successfully to a value whose matching private key is unavailable; crate documentation says this failure may appear only on the device.
- **Open questions:** Whether matching must use a whole label and whether duplicate or extra-separator labels must be rejected during key rotation.

### bad-recipient-key-paths-are-reachable

- **Type:** reachability
- **Status:** active
- **Exercised:** yes and locally audited for 31-byte and 33-byte public and private keys
- **Guarantee:** Both public `BadRecipientKey` return paths can be reached under their preceding successful gates.
- **Check:** `reachable(seal(short_public_key, accepted_plaintext) == Err(SealError::BadRecipientKey))` and `reachable(open(short_private_key, current_version_full_length_envelope) == Err(OpenError::BadRecipientKey))`.
- **Fault/timing angle:** Truncated or prefixed local key material.
- **Required faults and enabling state:** Accepted-size plaintext for `seal`; at least 33-byte current-version envelope for `open`; public/private key lengths other than 32.
- **Confidence:** high from control flow and resolved dependency source; [evidence](evidence/bad-recipient-key-paths-are-reachable.md)
- **Existing check:** [`key_deserialization_and_degenerate_public_key_paths_are_reachable`](../../../crates/cortexkit-push-seal/src/lib.rs); audited with accepted plaintext, a valid full envelope, exact errors, and valid 32-byte controls.
- **Impact:** Without these enabling states, public variants and the `BadRecipientKey -> malformed` wire mapping remain unexercised.
- **Open questions:** The separate contract disagreement over whether `BadRecipientKey` should mean length-only or semantic validation remains unresolved.

### bad-recipient-key-follows-resolved-deserializer

- **Type:** safety
- **Status:** active
- **Exercised:** sampled at rejected lengths 31 and 33 plus generated accepted 32-byte keys
- **Guarantee:** After earlier gates pass, each API returns `BadRecipientKey` exactly when the resolved HPKE public/private-key deserializer rejects the supplied bytes.
- **Check:** With accepted plaintext for `seal` and a current-version full-length envelope for `open`, `always((from_bytes(key).is_err()) == (result == BadRecipientKey))` for the respective resolved key type.
- **Fault/timing angle:** Dependency deserializer changes from length-only behavior to same-size semantic validation, or local error mapping drifts.
- **Required faults and enabling state:** Inputs accepted and rejected by each resolved deserializer while all earlier API gates pass.
- **Confidence:** high for current control flow; [evidence](evidence/bad-recipient-key-follows-resolved-deserializer.md)
- **Existing check:** [`key_deserialization_and_degenerate_public_key_paths_are_reachable`](../../../crates/cortexkit-push-seal/src/lib.rs); sampled evidence for rejected lengths 31 and 33 and generated accepted 32-byte controls. The universal mapping remains source-backed and unaudited.
- **Impact:** Preserves the documented error classification without pretending this repository has independently defined X25519 point/scalar validity.
- **Open questions:** Whether public docs should describe the resolved length-only behavior or the API should impose a stricter independent validity contract.

### degenerate-public-key-hpke-error-is-reachable

- **Type:** reachability
- **Status:** active
- **Exercised:** yes and locally audited with an all-zero 32-byte public value
- **Guarantee:** The `SealError::Hpke` branch is reachable with a 32-byte public value whose X25519 shared secret is all zero.
- **Check:** `reachable(seal(degenerate_key, b"x") == Err(SealError::Hpke))`. `reachable` fits because this records a specific branch that existing happy-path keys never enter.
- **Fault/timing angle:** Degenerate or low-order recipient public value.
- **Required faults and enabling state:** Known degenerate 32-byte input and a working entropy source; ordinary generated keys cannot witness the branch.
- **Confidence:** high from resolved dependency source; [evidence](evidence/degenerate-public-key-hpke-error-is-reachable.md)
- **Existing check:** [`key_deserialization_and_degenerate_public_key_paths_are_reachable`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for exact `SealError::Hpke` under working ambient entropy.
- **Impact:** Without this reachability condition, `SealError::Hpke` and its caller handling can remain untested forever.
- **Open questions:** Which degenerate vectors should be normative for the pinned dependency.

### encapped-key-parse-failure-is-unreachable

- **Type:** reachability
- **Status:** active
- **Exercised:** partially; the dependency serialized size is pinned to the local split, but the error branch is not instrumented
- **Guarantee:** Under the resolved X25519 deserializer contract, the `EncappedKey::from_bytes` error mapping in `open` is never entered.
- **Check:** `unreachable(encapped_key_parse_error_branch)`. `unreachable` fits because this is a dedicated code point whose execution would mean `ENC_LEN` no longer matches the dependency's serialized key size.
- **Fault/timing angle:** KEM or dependency change that alters serialized size or adds same-size semantic validation while local splitting remains unchanged.
- **Required faults and enabling state:** None under the resolved dependency behavior; a future size or deserializer-semantics change can wake the branch.
- **Confidence:** high from two-hop dependency source analysis; [evidence](evidence/encapped-key-parse-failure-is-unreachable.md)
- **Existing check:** [`low_order_encapsulation_reaches_decap_error`](../../../crates/cortexkit-push-seal/src/lib.rs) directly asserts the dependency encapsulated-key serialized size equals `ENC_LEN`. The unreachable-branch claim still rests on pinned dependency source analysis, not branch proof.
- **Impact:** If this branch becomes reachable, the dependency deserializer contract no longer matches local assumptions and the new failure is mapped to `Aead` and then `malformed`.
- **Open questions:** None.

### low-order-encapsulation-aead-path-is-reachable

- **Type:** reachability
- **Status:** active
- **Exercised:** yes and locally audited with a fixed all-zero low-order encapsulation
- **Guarantee:** A low-order 32-byte encapsulated key can reach the dependency's decapsulation rejection and is collapsed to `OpenError::Aead` and wire code `malformed`.
- **Check:** Use direct dependency decapsulation or branch instrumentation to prove `DecapError` was reached for a low-order `enc`; then require the public result to be `OpenError::Aead` with wire code `malformed`. `reachable` fits the witnessed dependency-facing path; the public result alone is non-discriminating.
- **Fault/timing angle:** Attacker-controlled low-order `enc` field with an otherwise valid envelope shape.
- **Required faults and enabling state:** Valid recipient private key and a dependency-approved low-order X25519 point in bytes 1..33.
- **Confidence:** high from dependency decapsulation source; [evidence](evidence/low-order-encapsulation-aead-path-is-reachable.md)
- **Existing check:** [`low_order_encapsulation_reaches_decap_error`](../../../crates/cortexkit-push-seal/src/lib.rs); audited for direct `HpkeError::DecapError`, public `OpenError::Aead`, wire code `malformed`, and a valid neighboring envelope control.
- **Impact:** Without this situation, the KEM-level rejection and its caller classification can remain untested while ordinary random fuzzing almost never reaches it.
- **Open questions:** Which low-order vector should be normative for the pinned dependency.

### byte-determining-dependency-closure-is-pinned

- **Type:** safety
- **Status:** active
- **Exercised:** partially; default CI records direct version requirements, manifest features, and the resolved version-and-checksum closure, and rejects manifest/lock mismatch
- **Guarantee:** Every supported target and build purpose that produces or verifies sealed bytes uses its approved enabled-feature set, entropy-backend configuration, command edge set, and version-and-checksum identity for the full transitive dependency graph.
- **Check:** `always((enabled_features, getrandom_backend, cargo_edges, resolved_graph) == approved_build_identity[target, purpose])` for default verification, deterministic-vector generation, and entropy-failure testing on each supported target. `always` fits because one target can intentionally have several backend/configuration identities.
- **Fault/timing angle:** A new in-range dependency release appears between developer, CI, consumer, or release builds.
- **Required faults and enabling state:** The tracked lockfile plus a deliberate manifest/lock mismatch prove default-CI resolution drift is rejected. Deliberate target-feature, backend, consumer, opener, and build-purpose drift remain untested.
- **Confidence:** high for the default-CI resolution and manifest feature set; low for the broader property; [evidence](evidence/byte-determining-dependency-closure-is-pinned.md)
- **Existing check:** Direct dependency requirements and explicit features in `crates/cortexkit-push-seal/Cargo.toml`, the tracked workspace `Cargo.lock`, and `--locked` on every CI Cargo build/lint/test command. The broad property remains unaudited.
- **Impact:** Dependency drift can change bytes or error behavior without a crate-source diff or version signal, and it makes revision-to-revision wire comparisons ambiguous.
- **Open questions:** Target-specific activated features and entropy backends; identities for deterministic-vector and entropy-failure builds; alternate path consumers; whether the external opener pins its closure and where conformance vectors record build identity. `(needs human input)`

### version-bump-accompanies-wire-change

- **Type:** safety
- **Status:** active
- **Exercised:** yes for synthetic represented-fixture changes; no committed historical wire change exists
- **Guarantee:** Every change to emitted bytes, accepted bytes, error classification, or wire-code strings includes a crate-version bump. Provenance prose, build-identity metadata, and fixture formatting do not. Replacing the vector material itself -- keys, entropy, or plaintext -- does require a bump: once the inputs move, the recorded envelope no longer witnesses the same computation, so the comparison cannot separate a deliberate refresh from drift the refresh would hide, and the gate holds the conservative side.
- **Check:** One rule over the revisions the head could land beside, the merge base and the base-branch tip: a revision constrains the version only when its represented wire surface (`schema_version`, `ciphersuite`, `inputs`, `expected`) differs from the head's, and the head package version must then exceed that revision's by SemVer precedence, including prerelease identifiers of any width. A revision without the fixture predates the surface and constrains nothing, and a revision already carrying the head's surface describes the same bytes, so neither demands a bump. Provenance prose, build-identity metadata, and JSON formatting are excluded from the comparison; a fixture missing or nulling any represented wire-surface section fails the gate loudly instead of projecting an empty section. The manifest is deserialized as TOML, and an unreadable manifest or a version that is not SemVer fails the gate. A version inherited through `version.workspace = true` is unsupported and reported as such, because resolving it needs the root manifest at the same revision, which this gate does not read. The failure names which of the four sections moved, so a deliberate vector refresh is distinguishable from unexplained drift. Source-only and unrepresented behavior changes remain outside this gate.
- **Fault/timing angle:** Source or dependency change that keeps self-roundtrip tests green while breaking the external opener.
- **Required faults and enabling state:** Synthetic unchanged, changed-without-bump, changed-with-bump, decreased-version, bootstrap, prose-only, reformatting, unrelated-change, unparseable-fixture, unparseable-version, manifest-formatting, prerelease-precedence, wide-numeric-prerelease, malformed-prerelease, unreadable-manifest, workspace-inherited-version, changed-section-reporting, version-taken-on-the-base-tip, and matching-base-tip-surface cases; readable pull-request base and head revisions for the actual Git comparison. Git failures other than proven path absence fail the gate rather than passing as bootstrap.
- **Confidence:** high for represented fixture changes; low for unrepresented behavior and external compatibility; [evidence](evidence/version-bump-accompanies-wire-change.md)
- **Existing check:** [`synthetic_version_gate_cases`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`manifest_formatting_does_not_change_the_read_version`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`a_commented_version_still_gates_a_wire_change`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`literal_string_quoting_reads_the_same_version`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`an_unparseable_version_fails_the_gate`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`prerelease_versions_compare_by_semver_precedence`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`numeric_prerelease_identifiers_order_by_value_at_any_width`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`a_malformed_prerelease_identifier_fails_the_gate`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`an_unreadable_manifest_fails_the_gate`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`a_workspace_inherited_version_names_itself_in_the_failure`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`the_failure_names_the_section_that_changed`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`a_base_tip_that_already_carries_this_surface_demands_no_further_bump`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), [`a_version_already_taken_on_the_base_tip_fails_the_gate`](../../../crates/cortexkit-push-seal/tests/version_gate.rs), and [`actual_git_diff_requires_version_bump`](../../../crates/cortexkit-push-seal/tests/version_gate.rs); audited for represented fixture cases only.
- **Impact:** The docs call the version the only notification channel for path consumers.
- **Open questions:** Ownership and location of the cross-language wire corpus. `(needs human input)`

## Limitation, delegated-obligation, and unresolved-contract register

These findings are not active safety properties. A future sender-authentication or replay-protection mechanism would invalidate the current behavior for good reasons, so checks that require the limitation to persist would be inverted security oracles.

| Finding | Evidence | Required human decision |
|---|---|---|
| The crate neither adds nor enforces replay identifiers, counters, timestamps, or expiry. Encrypted plaintext may carry such metadata, but this crate does not inspect it. | [replayed-envelope-opens-identically](evidence/replayed-envelope-opens-identically.md) | Decide whether payload or device semantics provide replay protection and whether replay belongs in the threat model. |
| HPKE base mode authenticates envelope bytes but not sender identity; any holder of the public key can seal. | [base-mode-does-not-authenticate-sender](evidence/base-mode-does-not-authenticate-sender.md) | Identify the external sender-authentication layer or decide whether the protocol needs one. |
| `BadRecipientKey` is documented as semantic X25519 validation, while the resolved dependency performs length-only deserialization and reports a degenerate public value through `Hpke`. | [key-error-classification-matches-key-shape](evidence/key-error-classification-matches-key-shape.md) | Decide whether the docs should describe length-only behavior or the API should add stricter validation. |
| The label parser uses substring and first-match selection, and ignores text after a second separator; duplicate, prefixed, suffixed, and extra-separator semantics are unspecified. Bare valid hex is separately documented and supported. | [labelled-input-selects-only-push-key](evidence/labelled-input-selects-only-push-key.md) | Define ambiguous labelled-input behavior before adding an oracle for those cases. |
| `kp` prints `PK ` and `SK ` prefixes, but `handseal` rejects either whole line as non-hex or wrong length; its 66-character diagnostic mentions only `SK `. | [labelled-input-selects-only-push-key](evidence/labelled-input-selects-only-push-key.md) | Decide whether operator output should be directly pasteable or explicitly require stripping the prefix, and make the diagnostic symmetric. |
| `seal` can wait inside the OS entropy path, which may retry or fall back, and no completion bound is documented. | [entropy-failure-does-not-unwind](evidence/entropy-failure-does-not-unwind.md) | Decide whether the caller needs a deadline or a separately supervised sealing boundary. |

## Fault-to-property map

| Fault or enabling state | Properties that become non-vacuous | Available in this repository? |
|---|---|---|
| Lengths 0, 2048, and 2049 | `matching-key-roundtrip-preserves-plaintext`, `plaintext-cap-is-inclusive-and-nontruncating`, `envelope-layout-and-overhead-stay-fixed` | yes |
| Suite, KEM-size, AAD, `info`, or version edit | `pinned-ciphersuite-codepoints`, `envelope-layout-and-overhead-stay-fixed`, `version-one-is-only-emitted-and-accepted-version`, `version-byte-is-exact-aad`, `hpke-info-remains-empty` | yes at build/test time |
| External opener or authoritative two-direction vector corpus | `cross-implementation-wire-vectors-conform` | no; requires human-supplied external evidence |
| Proper-prefix truncation or bit mutation | `tampered-or-truncated-envelope-never-opens` | yes |
| Different valid recipient key | `wrong-recipient-never-opens` | yes |
| Two simultaneous input defects | `open-error-precedence-is-stable` | yes |
| Key bytes accepted/rejected by the resolved deserializer under successful preceding gates | `wire-error-vocabulary-is-stable`, `bad-recipient-key-paths-are-reachable`, `bad-recipient-key-follows-resolved-deserializer` | yes |
| Repeated seals or degraded RNG | `each-seal-uses-fresh-ephemeral` | repetition yes; RNG degradation no |
| Bounded arbitrary malformed bytes | `open-is-total-over-bounded-input` | yes once the external bound is named |
| OS entropy failure | `entropy-failure-does-not-unwind` | yes through resolved `getrandom` `unsupported` or custom backend configuration |
| Oversized envelope at production transport boundary | `transport-bounds-open-envelope-size` | no production transport caller present; the `handopen` example is uncapped |
| Cross-protocol key reuse | `recipient-key-is-dedicated-to-push-sealing` | no; requires external inventory |
| Exact-label token-only paste | `labelled-input-selects-only-push-key` | yes, but example helper is private |
| Degenerate or low-order public value | `degenerate-public-key-hpke-error-is-reachable` | yes |
| KEM serialized-size or deserializer-semantics drift | `encapped-key-parse-failure-is-unreachable`, `envelope-layout-and-overhead-stay-fixed` | build-time only |
| Low-order attacker-controlled `enc` | `low-order-encapsulation-aead-path-is-reachable` | yes with a fixed dependency-approved vector |
| New in-range crypto dependency release | `byte-determining-dependency-closure-is-pinned` | yes for default CI through the tracked `Cargo.lock` and `--locked`; broader identities remain unaudited |
| Represented fixture change without version bump | `version-bump-accompanies-wire-change` | yes through synthetic policy cases and merge-base pull-request CI; no qualifying historical commit yet |

## Relationship map

- **Wire agreement group:** `cross-implementation-wire-vectors-conform`, `pinned-ciphersuite-codepoints`, `envelope-layout-and-overhead-stay-fixed`, `version-one-is-only-emitted-and-accepted-version`, `version-byte-is-exact-aad`, `hpke-info-remains-empty`, `open-error-precedence-is-stable`, `wire-error-vocabulary-is-stable`, and `byte-determining-dependency-closure-is-pinned` share the out-of-repository opener boundary.
- **Cryptographic acceptance group:** `matching-key-roundtrip-preserves-plaintext`, `tampered-or-truncated-envelope-never-opens`, `wrong-recipient-never-opens`, and `each-seal-uses-fresh-ephemeral` share the HPKE call sites.
- **Entropy and build-identity group:** `each-seal-uses-fresh-ephemeral`, `entropy-failure-does-not-unwind`, and `byte-determining-dependency-closure-is-pinned` share the resolved entropy backend and build configuration.
- **Input-classification group:** `plaintext-cap-is-inclusive-and-nontruncating`, `open-is-total-over-bounded-input`, `bad-recipient-key-paths-are-reachable`, `bad-recipient-key-follows-resolved-deserializer`, `degenerate-public-key-hpke-error-is-reachable`, `encapped-key-parse-failure-is-unreachable`, and `low-order-encapsulation-aead-path-is-reachable` share public API guards and dependency parsers.
- **External-assumption group:** `transport-bounds-open-envelope-size`, `recipient-key-is-dedicated-to-push-sealing`, and `version-bump-accompanies-wire-change` require evidence outside this crate before system-level conclusions can be drawn.
- **Operator-paste group:** `labelled-input-selects-only-push-key` and `wrong-recipient-never-opens` share the wrong-64-hex-value hazard documented in three commits. A wrong 64-hex value decodes to 32 bytes and does not exercise `BadRecipientKey`.
- **Suspected dominance:** If exact cross-language vectors record dependency identity and cover suite, layout, version, AAD, `info`, error precedence, and wire strings, they dominate most individual wire-agreement checks for compatibility, but not `each-seal-uses-fresh-ephemeral`, key dedication, or size delegation.
- **Suspected dominance:** `matching-key-roundtrip-preserves-plaintext` over the full accepted domain subsumes happy-path self-acceptance and empty-plaintext behavior, but it does not prove external compatibility or tamper rejection.

## Portfolio evaluation synthesis

Fresh-context evaluation ran after the first catalog draft.

### Gaps

1. The first draft had no `unreachable` record for the encapsulated-key parse branch and no reachability record for the real low-order decapsulation path. A bounded dependency-facing branch pass confirmed both. `encapped-key-parse-failure-is-unreachable` and `low-order-encapsulation-aead-path-is-reachable` were added.
2. Dependency-resolution drift appeared in the system model but not the catalog. A bounded build-identity pass added `byte-determining-dependency-closure-is-pinned` and excluded floating compiler identity from the wire-byte claim.
3. Local self-roundtrip did not directly test the documented external-opener boundary. `cross-implementation-wire-vectors-conform` now records the missing two-direction deterministic oracle and remains blocked on the unavailable opener/corpus.
4. Dual-version rollout is a real future risk, but no such system exists to mine. It is not a catalog property. If planned, construct its invariant and rollout contract first with `/software-design:invariant-driven-domain-modeling` and schema/version-compatibility review.

### Refinements

- Fresh-ephemeral checking now asserts a fresh RNG request and sender context per call; observed no-repeat checks are explicitly statistical canaries.
- The AAD property now checks the computed `[VERSION]` value under the explicit version-gate precondition.
- Label-parser claims now cover exact-label base cases and documented bare-hex support; substring, duplicate-label, and extra-separator semantics remain open contract questions.
- The cap property now checks guard ordering instead of claiming that no temporary bytes can exist.
- The layout property now records the locally derivable maximum envelope size of 2097 bytes.
- RNG failure is marked as a source-confirmed contradiction to the error-return claim and uses the resolved `getrandom` build-time failure seam; entropy blocking/retry is retained as a liveness limitation.
- Empty `info` now names its dependency-level observation method and the external corpus as the durable oracle.
- Default-CI identity now records manifest features and the resolved version-and-checksum closure. Target-specific activation, backend configuration, consumers, the opener, and other build purposes remain unaudited.

### Biases and dispositions

- The first draft treated replayability and absent sender authentication as safety properties. They are current security limitations; hardening would correctly invalidate them. They moved to the limitation register and are excluded from test handoff.
- `transport-bounds-open-envelope-size` and `recipient-key-is-dedicated-to-push-sealing` remain active at system scope because local docs explicitly delegate correctness to those external obligations. They are not local crate-test claims and remain blocked on human evidence.
- The portfolio has no `sometimes` record. This is deliberate for this small stateless library: finite boundary tables and mutation loops can record situation counts within their owning checks without adding standalone campaign properties. Required enabling states remain explicit. Revisit this exemption if a seeded, long-running, or distributed harness is introduced.

### Harness fit and balance

- Build constants and dependency identity belong at build or repository boundaries.
- Pure byte-domain properties fit local API or dependency-level boundaries.
- RNG failure uses the resolved `getrandom` backend-selection seam; a custom backend can also observe draw calls.
- Transport bounds and key dedication belong in unavailable caller/provisioning systems.
- Safety dominance is expected: the crate has no persistent state, scheduler, or recovery protocol. Four reachability-type records, including one `unreachable` semantic, cover rare dependency-facing branches.

## Handoff list

Every active record goes to `/testing:test-strategy` for test form, oracle, and boundary selection. Tests not reviewed above stay `unaudited` until `/testing:invariant-test-review` reviews them. The limitation register is excluded until a human turns a limitation into a required security contract.

| Property | Additional route |
|---|---|
| `matching-key-roundtrip-preserves-plaintext` | `/testing:test-strategy`; local byte-domain boundary |
| `pinned-ciphersuite-codepoints` | `/testing:test-strategy`; build-constant boundary |
| `cross-implementation-wire-vectors-conform` | Human opener/corpus evidence request, then `/testing:test-strategy` for deterministic custom-backend vector production and two-direction conformance |
| `envelope-layout-and-overhead-stay-fixed` | `/testing:test-strategy`; local byte-layout boundary |
| `version-one-is-only-emitted-and-accepted-version` | `/testing:test-strategy`; build constant plus parser boundary |
| `version-byte-is-exact-aad` | `/testing:test-strategy`; dependency-level differential oracle |
| `hpke-info-remains-empty` | `/testing:test-strategy`; dependency-level differential oracle and external corpus |
| `plaintext-cap-is-inclusive-and-nontruncating` | `/testing:test-strategy`; local boundary table |
| `tampered-or-truncated-envelope-never-opens` | `/testing:test-strategy`; local mutation and prefix domain |
| `wrong-recipient-never-opens` | `/testing:test-strategy`; local multi-key boundary |
| `open-error-precedence-is-stable` | `/testing:test-strategy`; local multi-defect classification table and external corpus |
| `wire-error-vocabulary-is-stable` | `/testing:test-strategy`; local enum table and external corpus |
| `each-seal-uses-fresh-ephemeral` | `/testing:test-strategy`; RNG-call/context observation plus a statistical repeated-call canary |
| `open-is-total-over-bounded-input` | `/testing:test-strategy`; arbitrary-byte parser boundary under the caller-owned size/resource contract |
| `entropy-failure-does-not-unwind` | `/testing:test-strategy` for resolved `getrandom` backend configuration and panic/error oracle; not a deterministic-simulation problem in the current architecture |
| `transport-bounds-open-envelope-size` | `/testing:test-strategy` at the eventual caller/transport boundary |
| `recipient-key-is-dedicated-to-push-sealing` | Human evidence request, then `/testing:test-strategy` only if the provisioning boundary is observable |
| `labelled-input-selects-only-push-key` | `/testing:test-strategy` for example/helper boundary and parser oracle |
| `bad-recipient-key-paths-are-reachable`, `bad-recipient-key-follows-resolved-deserializer`, `degenerate-public-key-hpke-error-is-reachable` | `/testing:test-strategy`; route current docs/guard semantics to `/low-level-systems:defensive-assertions-and-invariant-guards` only if production validation is proposed |
| `encapped-key-parse-failure-is-unreachable` | `/testing:test-strategy`; branch reachability plus build-size assertion |
| `low-order-encapsulation-aead-path-is-reachable` | `/testing:test-strategy`; dependency-approved fixed semantic vector |
| `byte-determining-dependency-closure-is-pinned` | `/testing:test-strategy`; repository/build identity boundary |
| `version-bump-accompanies-wire-change` | `/testing:test-strategy` for repository-history and conformance-artifact gates |

No property currently requires `/testing:deterministic-simulation-testing`: the crate has no scheduler, clock, network, persistence, or multi-node state. If later work adds replay state, key rotation state, or a multi-version rollout protocol, reassess that routing.
