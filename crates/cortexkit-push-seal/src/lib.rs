//! Seals push-notification payloads so that only the recipient device can read
//! them.
//!
//! # What breaks in consumers when this crate changes
//!
//! Sealed bytes are opened by a separate implementation in another repository,
//! which does not build this code.
//!
//! - Changing the ciphersuite, `info`, the associated data, or the envelope
//!   layout is a wire-format divergence. The opener fails with an authentication
//!   error, which renders on the device as an undecryptable notification — the
//!   same appearance as a locked phone. Nothing fails here.
//! - This crate is consumed by relative path, and a path dependency is recorded
//!   in `Cargo.lock` with a version string and no content hash. An unchanged
//!   version means new code compiles into a consuming repository with no lockfile
//!   diff anywhere. The version number is the only channel through which a
//!   consumer can learn that sealed output changed.
//!
//! Bump the version on any change to emitted bytes or behaviour. Do not bump it
//! for comments or tests, because prose-only bumps make the version unreliable.
//!
//! # The parameters, and why they are spelled out
//!
//! An HPKE ciphersuite is a triple. Naming two of its three parts leaves the
//! third to each implementation's default. RFC 9180's suite table opens with
//! AES-128-GCM, while the opener's platform offers exactly one X25519 suite.
//! Every such disagreement produces the same authentication failure, whose
//! diagnosis points at the transport.
//!
//! | parameter | RFC 9180 codepoint | value |
//! |---|---|---|
//! | KEM  | `0x0020` | `DHKEM(X25519, HKDF-SHA256)` |
//! | KDF  | `0x0001` | `HKDF-SHA256` |
//! | AEAD | `0x0003` | `ChaCha20Poly1305` |
//!
//! Codepoints are the wire facts both implementations pass to their libraries.
//! A platform-specific suite name is a symbol for this triple, not a wire fact.
//!
//! `info` is empty because the recipient key is dedicated to this purpose. It is
//! the key schedule's domain separator when one key serves several applications.
//! If this key is shared with another protocol, empty `info` is unsafe; use a
//! fixed non-empty domain string.

use hpke::{
    aead::ChaCha20Poly1305, kdf::HkdfSha256, kem::X25519HkdfSha256, Deserializable, OpModeR,
    OpModeS, Serializable,
};

/// The one envelope version this crate emits and accepts.
pub const VERSION: u8 = 0x01;

/// Normative plaintext cap, measured before sealing.
///
/// The composing party holds the plaintext and decides what to drop. A sealed
/// byte cap would require it to duplicate this crate's overhead calculation.
pub const MAX_PLAINTEXT_BYTES: usize = 2048;

/// Length of the encapsulated key for the pinned KEM.
const ENC_LEN: usize = 32;

/// Reported sealing failures preserve their cause so callers can diagnose input
/// and key errors separately.
#[derive(Debug, PartialEq, Eq)]
pub enum SealError {
    /// The plaintext exceeds [`MAX_PLAINTEXT_BYTES`]. Carries both numbers.
    ///
    /// Over-size is refused rather than truncated: the authentication tag covers
    /// the whole ciphertext, so a truncated blob does not decrypt to a fragment,
    /// it fails to decrypt entirely and renders as the generic placeholder —
    /// indistinguishable from a device that has not been unlocked.
    PlaintextTooLarge {
        limit: usize,
        observed: usize,
    },
    /// The recipient public key is not a valid X25519 point.
    BadRecipientKey,
    Hpke,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OpenError {
    /// The envelope is shorter than a version byte plus an encapsulated key.
    Malformed { observed: usize },
    /// The version byte is not one this build understands.
    ///
    /// Kept distinct from [`OpenError::Aead`] so callers can distinguish an
    /// unsupported wire format from payload corruption.
    UnknownVersion { observed: u8 },
    /// The recipient private key is not a valid X25519 scalar.
    BadRecipientKey,
    /// Authentication failed: wrong key, wrong suite, wrong `info`, wrong
    /// associated data, or altered bytes. These are indistinguishable here by
    /// construction — the tag covers all of them.
    Aead,
}

/// Seals `plaintext` to `recipient_public_key`.
///
/// Returns `version || enc || ciphertext`, where the version byte is also the
/// associated data, so it is authenticated. Left cleartext and unbound it would
/// not be covered by the tag, and flipping it would silently select a different
/// parse rather than failing.
///
/// # Panics
///
/// Panics if `getrandom::SysRng` fails to generate random bytes.
///
/// # Errors
///
/// - Returns [`SealError::PlaintextTooLarge`] when `plaintext` exceeds [`MAX_PLAINTEXT_BYTES`]; this check precedes key validation.
/// - Returns [`SealError::BadRecipientKey`] when `recipient_public_key` is not a 32-byte X25519 public key.
/// - Returns [`SealError::Hpke`] when HPKE rejects the sealing operation.
pub fn seal(recipient_public_key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
    seal_with_rng(
        recipient_public_key,
        plaintext,
        &mut hpke::rand_core::UnwrapErr(getrandom::SysRng),
    )
}

fn seal_with_rng(
    recipient_public_key: &[u8],
    plaintext: &[u8],
    rng: &mut impl hpke::rand_core::CryptoRng,
) -> Result<Vec<u8>, SealError> {
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(SealError::PlaintextTooLarge {
            limit: MAX_PLAINTEXT_BYTES,
            observed: plaintext.len(),
        });
    }

    let pk = <X25519HkdfSha256 as hpke::Kem>::PublicKey::from_bytes(recipient_public_key)
        .map_err(|_| SealError::BadRecipientKey)?;

    let aad = [VERSION];
    let (enc, ciphertext) = hpke::single_shot_seal_with_rng::<
        ChaCha20Poly1305,
        HkdfSha256,
        X25519HkdfSha256,
    >(&OpModeS::Base, &pk, &[], plaintext, &aad, rng)
    .map_err(|_| SealError::Hpke)?;

    let enc = enc.to_bytes();
    let mut out = Vec::with_capacity(1 + enc.len() + ciphertext.len());
    out.push(VERSION);
    out.extend_from_slice(&enc);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

impl OpenError {
    /// The wire vocabulary a conformance vector reports.
    ///
    /// This mapping is defined here rather than in the corpus generator to avoid
    /// two independent mappings that can drift.
    ///
    /// `Aead`, `Malformed`, and `BadRecipientKey` map to `malformed`; the wire
    /// vocabulary distinguishes only an unsupported version from an unusable
    /// envelope. The authentication tag cannot distinguish a wrong key,
    /// ciphersuite, `info`, associated data, or altered bytes.
    ///
    /// An envelope with a valid version and encapsulated key but empty ciphertext
    /// passes the length gate and fails as `Aead`, then reports `malformed`.
    pub fn wire_code(&self) -> &'static str {
        match self {
            OpenError::UnknownVersion { .. } => "unsupported_version",
            OpenError::Malformed { .. } | OpenError::BadRecipientKey | OpenError::Aead => {
                "malformed"
            }
        }
    }
}

/// Opens an envelope produced by [`seal`].
///
/// This function has no size cap. `seal` enforces the plaintext cap; transport
/// bounds the envelope before it reaches this code. A second opening cap could
/// disagree with the transport limit.
///
/// # Errors
///
/// - Returns [`OpenError::Malformed`] when `envelope` is shorter than 33 bytes; this check precedes version and key validation.
/// - Returns [`OpenError::UnknownVersion`] when the first envelope byte is not [`VERSION`].
/// - Returns [`OpenError::BadRecipientKey`] when `recipient_private_key` is not a valid X25519 scalar.
/// - Returns [`OpenError::Aead`] when the encapsulated key cannot be decoded or HPKE opening fails.
pub fn open(recipient_private_key: &[u8], envelope: &[u8]) -> Result<Vec<u8>, OpenError> {
    if envelope.len() < 1 + ENC_LEN {
        return Err(OpenError::Malformed {
            observed: envelope.len(),
        });
    }
    if envelope[0] != VERSION {
        return Err(OpenError::UnknownVersion {
            observed: envelope[0],
        });
    }

    let sk = <X25519HkdfSha256 as hpke::Kem>::PrivateKey::from_bytes(recipient_private_key)
        .map_err(|_| OpenError::BadRecipientKey)?;
    let enc = <X25519HkdfSha256 as hpke::Kem>::EncappedKey::from_bytes(&envelope[1..1 + ENC_LEN])
        .map_err(|_| OpenError::Aead)?;

    let aad = [VERSION];
    hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
        &OpModeR::Base,
        &sk,
        &enc,
        &[],
        &envelope[1 + ENC_LEN..],
        &aad,
    )
    .map_err(|_| OpenError::Aead)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hpke::rand_core::{Infallible, TryCryptoRng, TryRng};
    use hpke::{aead::Aead, kdf::Kdf, Kem as KemTrait};
    use serde_json::Value;

    const WIRE_V1_FIXTURE: &str = include_str!("../tests/golden/push-seal-wire-v1.json");

    fn keypair() -> (Vec<u8>, Vec<u8>) {
        let (sk, pk) = X25519HkdfSha256::gen_keypair();
        (sk.to_bytes().to_vec(), pk.to_bytes().to_vec())
    }

    /// Pins the suite by RFC 9180 codepoint rather than library type name.
    ///
    /// Type names can remain stable while their codepoints change. The external
    /// opener agrees on codepoints.
    #[test]
    fn the_pinned_suite_has_the_documented_codepoints() {
        assert_eq!(X25519HkdfSha256::KEM_ID, 0x0020, "KEM codepoint");
        assert_eq!(HkdfSha256::KDF_ID, 0x0001, "KDF codepoint");
        assert_eq!(ChaCha20Poly1305::AEAD_ID, 0x0003, "AEAD codepoint");
    }

    type PackageIdentity = (String, Option<String>);

    // Resolving a name against the whole lock file picks whichever `[[package]]` sorts
    // first, which is a different version than the root builds against as soon as any
    // workspace crate pulls in a second one. Walk the root's own dependency edges. A
    // package is identified by name, version, and source together, so two packages that
    // share a name and version across sources each contribute their own subtree.
    fn reachable_dependencies(
        lock: &str,
        root_manifest: &str,
        root: &str,
    ) -> std::collections::BTreeMap<String, std::collections::BTreeSet<PackageIdentity>> {
        let lock: toml::Table = lock.parse().expect("Cargo.lock parses as TOML");
        let packages = lock["package"].as_array().expect("lock packages");
        let identity = |package: &toml::Value| {
            (
                package["name"].as_str().expect("package name").to_owned(),
                package["version"]
                    .as_str()
                    .expect("package version")
                    .to_owned(),
                package
                    .get("source")
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned),
            )
        };
        let entry = |name: &str, version: Option<&str>, source: Option<&str>| {
            let mut matches = packages.iter().filter(|package| {
                package["name"].as_str() == Some(name)
                    && version.is_none_or(|version| package["version"].as_str() == Some(version))
                    && source.is_none_or(|source| {
                        package.get("source").and_then(toml::Value::as_str) == Some(source)
                    })
            });
            let found = matches
                .next()
                .unwrap_or_else(|| panic!("{name} {version:?} {source:?} missing from Cargo.lock"));
            assert!(
                matches.next().is_none(),
                "{name} {version:?} {source:?} is ambiguous in Cargo.lock"
            );
            found
        };
        // Cargo writes `name`, adds the version when the lock holds several of that
        // name, and adds `(source)` when it holds several of that name and version.
        fn split_id(dependency: &str) -> (&str, Option<&str>, Option<&str>) {
            let mut fields = dependency.splitn(3, ' ');
            let name = fields.next().expect("dependency name");
            let version = fields.next();
            let source = fields.next().map(|source| {
                source
                    .strip_prefix('(')
                    .and_then(|source| source.strip_suffix(')'))
                    .unwrap_or_else(|| panic!("unparseable dependency source in {dependency:?}"))
            });
            (name, version, source)
        }
        // The lock records one resolve covering every target and feature selection, so
        // this walk over-approximates the tested build. The root's dev-only edges are the
        // part that can be excluded exactly, since nothing behind them links into the
        // sealed bytes; dropping them takes this crate from 56 reachable packages to 39
        // and removes the only name the lock resolves at two versions.
        let manifest: toml::Table = root_manifest.parse().expect("root manifest parses");
        let names = |table: &str| -> std::collections::BTreeSet<String> {
            manifest
                .get(table)
                .and_then(toml::Value::as_table)
                .map(|table| table.keys().cloned().collect())
                .unwrap_or_default()
        };
        let linked = names("dependencies");
        let dev_only: std::collections::BTreeSet<String> = names("dev-dependencies")
            .difference(&linked)
            .cloned()
            .collect();

        let mut visited = std::collections::BTreeSet::new();
        let mut reachable: std::collections::BTreeMap<
            String,
            std::collections::BTreeSet<PackageIdentity>,
        > = std::collections::BTreeMap::new();
        let mut queue = vec![(entry(root, None, None), true)];
        while let Some((package, is_root)) = queue.pop() {
            let (name, version, source) = identity(package);
            if !visited.insert((name.clone(), version.clone(), source.clone())) {
                continue;
            }
            reachable.entry(name).or_default().insert((version, source));
            let Some(dependencies) = package.get("dependencies") else {
                continue;
            };
            for dependency in dependencies.as_array().expect("dependency list") {
                let (name, version, source) = split_id(dependency.as_str().expect("dependency"));
                if is_root && dev_only.contains(name) {
                    continue;
                }
                queue.push((entry(name, version, source), false));
            }
        }
        reachable
    }

    #[test]
    fn a_dependency_reached_through_two_sources_is_not_collapsed() {
        // `left` and `right` share a name and version across sources, so a visited check
        // keyed on name and version alone drops one subtree and hides its `hpke`.
        let lock = r#"
version = 4

[[package]]
name = "root"
version = "0.1.0"
dependencies = ["shared 1.0.0 (registry+https://example.com/r)", "shared 1.0.0 (git+https://example.com/s)"]

[[package]]
name = "shared"
version = "1.0.0"
source = "registry+https://example.com/r"
dependencies = ["hpke 0.14.0"]

[[package]]
name = "shared"
version = "1.0.0"
source = "git+https://example.com/s"
dependencies = ["hpke 0.15.0"]

[[package]]
name = "hpke"
version = "0.14.0"
source = "registry+https://example.com/r"

[[package]]
name = "hpke"
version = "0.15.0"
source = "registry+https://example.com/r"
"#;
        let reachable = reachable_dependencies(lock, "[package]\nname = \"root\"\n", "root");
        let shared = &reachable["shared"];
        assert_eq!(shared.len(), 2, "both sources must be visited: {shared:?}");
        let hpke: Vec<&str> = reachable["hpke"]
            .iter()
            .map(|(version, _)| version.as_str())
            .collect();
        assert_eq!(hpke, ["0.14.0", "0.15.0"], "a subtree was skipped");
    }

    #[test]
    fn root_dev_only_edges_are_not_traversed() {
        // `shared` is both a dependency and a dev-dependency, so it stays; `harness` is
        // dev-only, and the second `hpke` behind it must not appear.
        let manifest = r#"
[package]
name = "root"

[dependencies]
shared = "1"

[dev-dependencies]
shared = "1"
harness = "1"
"#;
        let lock = r#"
version = 4

[[package]]
name = "root"
version = "0.1.0"
dependencies = ["harness", "shared"]

[[package]]
name = "shared"
version = "1.0.0"
dependencies = ["hpke 0.14.0"]

[[package]]
name = "harness"
version = "1.0.0"
dependencies = ["hpke 0.15.0"]

[[package]]
name = "hpke"
version = "0.14.0"

[[package]]
name = "hpke"
version = "0.15.0"
"#;
        let reachable = reachable_dependencies(lock, manifest, "root");
        let hpke: Vec<&str> = reachable["hpke"]
            .iter()
            .map(|(version, _)| version.as_str())
            .collect();
        assert_eq!(hpke, ["0.14.0"], "a dev-only edge was traversed");
        assert!(
            reachable.contains_key("shared"),
            "a linked edge was dropped"
        );
        assert!(
            !reachable.contains_key("harness"),
            "a dev-only edge was traversed"
        );
    }

    #[test]
    fn wire_v1_fixture_matches_local_bytes_and_classifications() {
        let fixture: Value = serde_json::from_str(WIRE_V1_FIXTURE).expect("parse wire fixture");
        assert_eq!(fixture["schema_version"], 1);
        assert_eq!(fixture["build_identity"]["package"], "cortexkit-push-seal");
        assert_eq!(
            fixture["build_identity"]["package_version"],
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(fixture["ciphersuite"]["kem"]["codepoint"], 0x0020);
        assert_eq!(fixture["ciphersuite"]["kdf"]["codepoint"], 0x0001);
        assert_eq!(fixture["ciphersuite"]["aead"]["codepoint"], 0x0003);
        assert_eq!(fixture["ciphersuite"]["mode"], "Base");
        assert_eq!(fixture["inputs"]["info_hex"], "");
        assert_eq!(fixture["inputs"]["aad_hex"], "01");

        let input = |name: &str| {
            hex::decode(
                fixture["inputs"][name]
                    .as_str()
                    .unwrap_or_else(|| panic!("missing inputs.{name}")),
            )
            .unwrap_or_else(|_| panic!("invalid inputs.{name}"))
        };
        let ikm = input("recipient_ikm_hex");
        let (sk, pk) = X25519HkdfSha256::derive_keypair(&ikm);
        let sk = sk.to_bytes().to_vec();
        let pk = pk.to_bytes().to_vec();
        assert_eq!(sk, input("recipient_private_key_hex"));
        assert_eq!(pk, input("recipient_public_key_hex"));

        let ephemeral = input("ephemeral_randomness_hex");
        assert_eq!(ephemeral.len(), 32);
        assert!(ephemeral.iter().all(|byte| *byte == ephemeral[0]));
        let mut rng = RecordingRng {
            next: ephemeral[0],
            repeat: true,
            fills: Vec::new(),
        };
        let plaintext = input("plaintext_hex");
        let envelope = seal_with_rng(&pk, &plaintext, &mut rng).expect("deterministic seal");
        assert_eq!(rng.fills, [32]);
        assert_eq!(
            envelope,
            hex::decode(
                fixture["expected"]["envelope_hex"]
                    .as_str()
                    .expect("expected envelope"),
            )
            .expect("valid expected envelope")
        );
        assert_eq!(open(&sk, &envelope).expect("fixture opens"), plaintext);

        let reachable = reachable_dependencies(
            include_str!("../../../Cargo.lock"),
            include_str!("../Cargo.toml"),
            "cortexkit-push-seal",
        );
        let recorded_dependencies = fixture["build_identity"]["dependencies"]
            .as_object()
            .expect("recorded dependencies");
        assert!(!recorded_dependencies.is_empty());
        for (name, recorded) in recorded_dependencies {
            let resolved = reachable
                .get(name.as_str())
                .unwrap_or_else(|| panic!("{name} is not a dependency of this crate"));
            assert_eq!(
                resolved.len(),
                1,
                "this crate reaches several {name} packages: {resolved:?}; \
                 build_identity records one. The lock graph covers every target and \
                 feature selection, so confirm an active edge reaches each before \
                 changing the fixture"
            );
            let (version, _source) = resolved.iter().next().expect("resolved package");
            assert_eq!(
                recorded.as_str().expect("recorded dependency version"),
                version,
                "fixture build_identity.dependencies.{name} must match Cargo.lock"
            );
        }

        let cases = fixture["expected"]["classifications"]
            .as_array()
            .expect("classification cases");
        let expected_cases = [
            "short envelope",
            "unsupported version",
            "bad recipient key",
            "authentication failure",
        ];
        assert_eq!(cases.len(), expected_cases.len(), "classification count");
        let mut covered = std::collections::BTreeSet::new();
        for (case, expected_name) in cases.iter().zip(expected_cases) {
            let name = case["name"].as_str().expect("case name");
            assert_eq!(name, expected_name);
            let key = hex::decode(
                case["recipient_private_key_hex"]
                    .as_str()
                    .expect("case key"),
            )
            .expect("valid case key");
            let case_envelope = hex::decode(case["envelope_hex"].as_str().expect("case envelope"))
                .expect("valid case envelope");
            // Anchoring each case to the verified seal keeps a fixture
            // regeneration from detaching a case from the envelope it claims
            // to mutate.
            match name {
                "short envelope" => {
                    assert!(key.is_empty(), "case {name}");
                    assert!(case_envelope.is_empty(), "case {name}");
                }
                "unsupported version" => {
                    let mut expected_envelope = envelope.clone();
                    expected_envelope[0] = 0x02;
                    assert_eq!(case_envelope, expected_envelope, "case {name}");
                }
                "bad recipient key" => {
                    assert!(key.is_empty(), "case {name}");
                    assert_eq!(case_envelope, envelope, "case {name}");
                }
                "authentication failure" => {
                    assert_eq!(key, sk, "case {name}");
                    let mut tampered = envelope.clone();
                    *tampered.last_mut().expect("nonempty envelope") ^= 0x01;
                    assert_eq!(case_envelope, tampered, "case {name}");
                }
                other => panic!("unanchored classification case {other}"),
            }
            let error = open(&key, &case_envelope).expect_err("classification must fail");
            if let Some(expected) = case["observed"].as_u64() {
                match &error {
                    OpenError::Malformed { observed } => {
                        assert_eq!(*observed as u64, expected, "case {name}");
                    }
                    OpenError::UnknownVersion { observed } => {
                        assert_eq!(*observed as u64, expected, "case {name}");
                    }
                    OpenError::BadRecipientKey | OpenError::Aead => {
                        panic!("case {name} has an inapplicable observed field");
                    }
                }
            }
            let classification = match &error {
                OpenError::Malformed { .. } => "Malformed",
                OpenError::UnknownVersion { .. } => "UnknownVersion",
                OpenError::BadRecipientKey => "BadRecipientKey",
                OpenError::Aead => "Aead",
            };
            covered.insert(classification);
            assert_eq!(classification, case["error"], "case {name}");
            assert_eq!(
                error.wire_code(),
                case["wire_code"].as_str().expect("case wire code"),
                "case {name}",
            );
        }
        assert_eq!(
            covered,
            ["Aead", "BadRecipientKey", "Malformed", "UnknownVersion"]
                .into_iter()
                .collect(),
            "fixture must represent every public OpenError classification"
        );
    }

    #[test]
    fn a_sealed_payload_opens_to_the_same_plaintext() {
        let (sk, pk) = keypair();
        for len in [0, 2047, 2048] {
            let plaintext = vec![0xff; len];
            if !plaintext.is_empty() {
                assert!(
                    std::str::from_utf8(&plaintext).is_err(),
                    "fixture must be non-UTF-8"
                );
            }
            let sealed = seal(&pk, &plaintext).expect("seal");
            assert_eq!(open(&sk, &sealed).expect("open"), plaintext);
        }
    }

    #[test]
    fn the_envelope_has_version_one_and_fixed_overhead() {
        let (_, pk) = keypair();
        assert_eq!(VERSION, 0x01, "literal wire version");
        assert_eq!(ENC_LEN, 32, "literal encapsulated-key length");

        for plaintext_len in [0, 1, 2048] {
            let sealed = seal(&pk, &vec![0u8; plaintext_len]).expect("seal");
            assert_eq!(sealed[0], 0x01, "version byte leads");
            assert_eq!(sealed.len(), plaintext_len + 49, "literal overhead");
            assert!(sealed.len() <= 2097, "maximum local envelope length");
            if plaintext_len == 2048 {
                assert_eq!(sealed.len(), 2097, "maximum local envelope length");
            }
        }
    }

    struct RecordingRng {
        next: u8,
        repeat: bool,
        fills: Vec<usize>,
    }

    impl TryRng for RecordingRng {
        type Error = Infallible;

        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            hpke::rand_core::utils::next_word_via_fill(self)
        }

        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            hpke::rand_core::utils::next_word_via_fill(self)
        }

        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
            self.fills.push(dst.len());
            dst.fill(self.next);
            if !self.repeat {
                self.next = self.next.wrapping_add(1);
            }
            Ok(())
        }
    }

    impl TryCryptoRng for RecordingRng {}

    #[test]
    fn each_seal_uses_a_fresh_ephemeral() {
        let (_, pk) = keypair();

        // A `seal_with_rng` draw cannot observe which RNG `seal` selects.
        // Matching 32-byte encapsulated keys is negligibly likely.
        let ambient_a = seal(&pk, b"same").expect("first ambient seal");
        let ambient_b = seal(&pk, b"same").expect("second ambient seal");
        assert_ne!(
            ambient_a[1..1 + ENC_LEN],
            ambient_b[1..1 + ENC_LEN],
            "the ambient RNG must not repeat the encapsulated key"
        );

        let mut fresh = RecordingRng {
            next: 1,
            repeat: false,
            fills: Vec::new(),
        };
        let a = seal_with_rng(&pk, b"same", &mut fresh).expect("first seal");
        let b = seal_with_rng(&pk, b"same", &mut fresh).expect("second seal");
        assert_eq!(fresh.fills, [32, 32], "one fresh draw per seal");
        assert_ne!(
            a[1..1 + ENC_LEN],
            b[1..1 + ENC_LEN],
            "encapsulated key must not repeat across messages"
        );

        let mut repeated = RecordingRng {
            next: 1,
            repeat: true,
            fills: Vec::new(),
        };
        let a = seal_with_rng(&pk, b"same", &mut repeated).expect("first repeated seal");
        let b = seal_with_rng(&pk, b"same", &mut repeated).expect("second repeated seal");
        assert_eq!(repeated.fills, [32, 32], "negative-control draws");
        assert_eq!(
            a[1..1 + ENC_LEN],
            b[1..1 + ENC_LEN],
            "repeated entropy must trip the encapsulation canary"
        );
    }

    #[test]
    fn an_oversized_plaintext_is_refused_with_both_numbers() {
        let (sk, pk) = keypair();
        let too_big = vec![0u8; 2049];
        assert_eq!(
            seal(&[], &too_big),
            Err(SealError::PlaintextTooLarge {
                limit: 2048,
                observed: 2049
            }),
            "length must win before key parsing"
        );
        // Positive control: the boundary itself succeeds, so the refusal above
        // is not satisfied by an implementation that refuses everything.
        let at_limit = vec![0u8; 2048];
        let sealed = seal(&pk, &at_limit).expect("the cap itself must seal");
        assert_eq!(
            open(&sk, &sealed).expect("the cap itself must open"),
            at_limit
        );
    }

    #[test]
    fn open_error_precedence_is_stable() {
        let (sk, pk) = keypair();
        let sealed = seal(&pk, b"q").expect("seal");

        let mut short = sealed[..32].to_vec();
        short[0] = 0x02;
        assert_eq!(
            open(&[], &short),
            Err(OpenError::Malformed { observed: 32 }),
            "length must win before version and key parsing"
        );

        // 33 bytes is the smallest envelope with a valid version and encapsulated key but no ciphertext.
        let empty_ciphertext = &sealed[..33];
        assert_eq!(
            open(&sk, empty_ciphertext),
            Err(OpenError::Aead),
            "the minimum-length envelope must clear the length gate"
        );
        assert_eq!(
            open(&sk, empty_ciphertext).unwrap_err().wire_code(),
            "malformed",
            "an empty ciphertext reports as an unusable envelope"
        );

        for version in u8::MIN..=u8::MAX {
            if version == 0x01 {
                continue;
            }
            let mut wrong_version = sealed.clone();
            wrong_version[0] = version;
            assert_eq!(
                open(&[], &wrong_version),
                Err(OpenError::UnknownVersion { observed: version }),
                "version must win before key parsing"
            );
        }

        let mut corrupt = sealed.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert_eq!(
            open(&[], &corrupt),
            Err(OpenError::BadRecipientKey),
            "key parsing must precede authenticated opening"
        );
        assert_eq!(open(&sk, &corrupt), Err(OpenError::Aead));
        assert_eq!(open(&sk, &sealed).expect("valid neighboring control"), b"q");
    }

    #[test]
    fn every_proper_prefix_of_a_valid_envelope_is_rejected() {
        let (sk, pk) = keypair();
        let plaintext = b"prefix anchor";
        let sealed = seal(&pk, plaintext).expect("seal");
        assert_eq!(open(&sk, &sealed).expect("valid anchor"), plaintext);

        let mut malformed = 0;
        let mut aead = 0;
        for len in 0..sealed.len() {
            let expected = if len < 1 + ENC_LEN {
                malformed += 1;
                OpenError::Malformed { observed: len }
            } else {
                aead += 1;
                OpenError::Aead
            };
            assert_eq!(open(&sk, &sealed[..len]), Err(expected), "prefix {len}");
        }

        assert_eq!(malformed, 33, "all structural prefixes reached");
        assert_eq!(malformed + aead, sealed.len(), "all prefixes reached");
        assert!(aead > 0, "authenticated-open prefixes reached");
    }

    #[test]
    fn single_bit_mutations_have_field_specific_outcomes() {
        let (sk, pk) = keypair();
        let plaintext = b"mutation anchor";
        let sealed = seal(&pk, plaintext).expect("seal");
        assert_eq!(open(&sk, &sealed).expect("valid anchor"), plaintext);

        let mut version_mutations = 0;
        for bit in 0..8 {
            let mut mutated = sealed.clone();
            mutated[0] ^= 1 << bit;
            assert_eq!(
                open(&sk, &mutated),
                Err(OpenError::UnknownVersion {
                    observed: mutated[0]
                })
            );
            version_mutations += 1;
        }

        let mut encapsulation_rejections = 0;
        for byte in 1..1 + ENC_LEN {
            for bit in 0..8 {
                let mut mutated = sealed.clone();
                mutated[byte] ^= 1 << bit;
                assert_eq!(open(&sk, &mutated), Err(OpenError::Aead));
                encapsulation_rejections += 1;
            }
        }

        let tag_start = sealed.len() - 16;
        let mut ciphertext_mutations = 0;
        let mut tag_mutations = 0;
        for byte in 1 + ENC_LEN..sealed.len() {
            for bit in 0..8 {
                let mut mutated = sealed.clone();
                mutated[byte] ^= 1 << bit;
                assert_eq!(open(&sk, &mutated), Err(OpenError::Aead));
                if byte < tag_start {
                    ciphertext_mutations += 1;
                } else {
                    tag_mutations += 1;
                }
            }
        }

        assert_eq!(version_mutations, 8, "version bits reached");
        assert_eq!(
            encapsulation_rejections,
            ENC_LEN * 8,
            "encapsulated-key bits reached"
        );
        assert_eq!(
            ciphertext_mutations,
            plaintext.len() * 8,
            "ciphertext bits reached"
        );
        assert_eq!(tag_mutations, 16 * 8, "authentication-tag bits reached");
    }

    #[test]
    fn sampled_malformed_bytes_are_total_through_the_local_envelope_bound() {
        let (sk, pk) = keypair();
        let sealed = seal(&pk, b"bounded anchor").expect("seal");
        assert_eq!(open(&sk, &sealed).expect("valid anchor"), b"bounded anchor");

        let mut reached = [0usize; 4];
        for len in 0..=2097 {
            let mut envelope = vec![(len as u8).wrapping_mul(31); len];
            let (key, expected) = if len < 1 + ENC_LEN {
                reached[0] += 1;
                (&[][..], OpenError::Malformed { observed: len })
            } else if len == 1 + ENC_LEN {
                envelope[0] = VERSION;
                reached[3] += 1;
                (&sk[..], OpenError::Aead)
            } else if len % 2 == 0 {
                envelope[0] = VERSION.wrapping_add(1);
                reached[1] += 1;
                (
                    &[][..],
                    OpenError::UnknownVersion {
                        observed: VERSION.wrapping_add(1),
                    },
                )
            } else {
                envelope[0] = VERSION;
                reached[2] += 1;
                (&[][..], OpenError::BadRecipientKey)
            };

            let result = std::panic::catch_unwind(|| open(key, &envelope));
            assert!(result.is_ok(), "length {len} must not unwind");
            assert_eq!(result.unwrap(), Err(expected), "length {len}");
        }

        assert_eq!(reached.iter().sum::<usize>(), 2098, "all lengths reached");
        assert_eq!(reached[0], 33, "short-input gate reached");
        assert!(
            reached[1..].iter().all(|count| *count > 0),
            "every public error class reached: {reached:?}"
        );

        for len in [33, 34, 48, 49, 50, 256, 1024, 2097] {
            let mut envelope = sealed[..1 + ENC_LEN].to_vec();
            envelope.resize(len, 0xa5);
            let result = std::panic::catch_unwind(|| open(&sk, &envelope));
            assert!(result.is_ok(), "deep sample length {len} must not unwind");
            assert_eq!(
                result.unwrap(),
                Err(OpenError::Aead),
                "deep sample length {len}"
            );
        }
    }

    /// Pins the complete local error enum to the two-string wire vocabulary.
    #[test]
    fn every_open_failure_maps_to_the_wire_vocabulary() {
        let cases = [
            (
                OpenError::UnknownVersion { observed: 0x7f },
                "unsupported_version",
            ),
            (OpenError::Malformed { observed: 32 }, "malformed"),
            (OpenError::BadRecipientKey, "malformed"),
            (OpenError::Aead, "malformed"),
        ];
        for (error, expected) in cases {
            assert_eq!(error.wire_code(), expected);
        }
    }

    #[test]
    fn the_wrong_recipient_cannot_open() {
        let (_, pk) = keypair();
        let (other_sk, other_pk) = keypair();
        assert_ne!(pk, other_pk, "recipients must be distinct");
        let sealed = seal(&pk, b"q").expect("seal");
        assert_eq!(open(&other_sk, &sealed), Err(OpenError::Aead));
    }

    #[test]
    fn aad_and_info_are_exact() {
        let (sk, pk) = keypair();
        let sealed = seal(&pk, b"q").expect("seal");

        let recipient = <X25519HkdfSha256 as hpke::Kem>::PrivateKey::from_bytes(&sk).unwrap();
        let enc = <X25519HkdfSha256 as hpke::Kem>::EncappedKey::from_bytes(&sealed[1..1 + ENC_LEN])
            .unwrap();
        let direct_open = |info: &[u8], aad: &[u8]| {
            hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
                &OpModeR::Base,
                &recipient,
                &enc,
                info,
                &sealed[1 + ENC_LEN..],
                aad,
            )
        };

        assert_eq!(
            direct_open(&[], &[VERSION]).expect("direct correct open"),
            b"q"
        );
        assert_eq!(open(&sk, &sealed).expect("public correct open"), b"q");

        for aad in [&[][..], &[0][..], &[VERSION, 0][..]] {
            assert_eq!(
                direct_open(&[], aad),
                Err(hpke::HpkeError::OpenError),
                "AAD {aad:?}"
            );
        }

        assert_eq!(
            direct_open(b"push", &[VERSION]),
            Err(hpke::HpkeError::OpenError),
            "non-empty info"
        );
    }

    #[test]
    fn key_deserialization_and_degenerate_public_key_paths_are_reachable() {
        type PrivateKey = <X25519HkdfSha256 as hpke::Kem>::PrivateKey;
        type PublicKey = <X25519HkdfSha256 as hpke::Kem>::PublicKey;

        let (sk, pk) = keypair();
        assert!(
            PublicKey::from_bytes(&pk).is_ok(),
            "valid public key parses"
        );
        assert!(
            PrivateKey::from_bytes(&sk).is_ok(),
            "valid private key parses"
        );
        let sealed = seal(&pk, b"key control").expect("valid public key");
        assert_eq!(
            open(&sk, &sealed).expect("valid private key"),
            b"key control"
        );

        for len in [31, 33] {
            let key = vec![0; len];
            assert!(matches!(
                PublicKey::from_bytes(&key),
                Err(hpke::HpkeError::IncorrectInputLength(32, observed)) if observed == len
            ));
            assert!(matches!(
                PrivateKey::from_bytes(&key),
                Err(hpke::HpkeError::IncorrectInputLength(32, observed)) if observed == len
            ));
            assert_eq!(
                seal(&key, b"x"),
                Err(SealError::BadRecipientKey),
                "public key length {len}"
            );
            assert_eq!(
                open(&key, &sealed),
                Err(OpenError::BadRecipientKey),
                "private key length {len}"
            );
        }

        assert_eq!(seal(&[0; 32], b"x"), Err(SealError::Hpke));
    }

    #[test]
    fn low_order_encapsulation_reaches_decap_error() {
        type EncappedKey = <X25519HkdfSha256 as hpke::Kem>::EncappedKey;

        assert_eq!(
            <EncappedKey as Serializable>::size(),
            ENC_LEN,
            "local split must match the dependency serialization"
        );

        let (sk, pk) = keypair();
        let sealed = seal(&pk, b"low-order control").expect("seal");
        assert_eq!(
            open(&sk, &sealed).expect("valid neighboring control"),
            b"low-order control"
        );

        let recipient =
            <X25519HkdfSha256 as hpke::Kem>::PrivateKey::from_bytes(&sk).expect("private key");
        let low_order = EncappedKey::from_bytes(&[0; ENC_LEN]).expect("length-valid encapped key");
        assert_eq!(
            hpke::setup_receiver::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
                &OpModeR::Base,
                &recipient,
                &low_order,
                &[],
            )
            .err(),
            Some(hpke::HpkeError::DecapError),
            "dependency must reject during decapsulation"
        );

        let mut malformed = sealed;
        malformed[1..1 + ENC_LEN].fill(0);
        let error = open(&sk, &malformed).expect_err("low-order enc must fail");
        assert_eq!(error, OpenError::Aead);
        assert_eq!(error.wire_code(), "malformed");
    }
}
