use std::process::Command;

use serde_json::{json, Value};

const FIXTURE_PATH: &str = "crates/cortexkit-push-seal/tests/golden/push-seal-wire-v1.json";
const MANIFEST_PATH: &str = "crates/cortexkit-push-seal/Cargo.toml";

// Cargo reads the manifest as TOML, so quoted keys, escape sequences, dotted keys, and
// either string style all name the same field. Approximating that grammar by hand read
// some of those spellings wrongly.
// commentlint: allow(JUDGE)
fn package_version(manifest: &str) -> Result<String, String> {
    let document: toml::Table = manifest
        .parse()
        .map_err(|error| format!("unparseable manifest: {error}"))?;
    let Some(version) = document
        .get("package")
        .and_then(|package| package.get("version"))
    else {
        return Err("manifest has no [package] version".to_owned());
    };
    // Resolving `version.workspace = true` needs the root manifest at the same revision,
    // which this gate does not read. Name the configuration instead of reporting a
    // missing version.
    // commentlint: allow(JUDGE)
    if version.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
        return Err(
            "[package] version is inherited from the workspace, which this gate does not \
             resolve; give the crate manifest its own version"
                .to_owned(),
        );
    }
    version
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("[package] version is not a string: {version:?}"))
}

// A numeric identifier is held as digits rather than an integer: SemVer bounds neither
// its width nor its value, and any fixed-width fallback to lexical comparison inverts
// the order of two identifiers whose digit counts differ.
// commentlint: allow(JUDGE)
#[derive(Debug, PartialEq, Eq)]
enum Identifier {
    Numeric(String),
    Alphanumeric(String),
}

impl Ord for Identifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            // Digits carry no leading zero, so more digits means a larger value and
            // equal digit counts order lexically.
            (Identifier::Numeric(left), Identifier::Numeric(right)) => {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            }
            (Identifier::Numeric(_), Identifier::Alphanumeric(_)) => Ordering::Less,
            (Identifier::Alphanumeric(_), Identifier::Numeric(_)) => Ordering::Greater,
            (Identifier::Alphanumeric(left), Identifier::Alphanumeric(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Version {
    triple: (u64, u64, u64),
    prerelease: Vec<Identifier>,
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        self.triple.cmp(&other.triple).then_with(|| {
            // A release outranks any prerelease sharing its triple.
            match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.prerelease.cmp(&other.prerelease),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Build metadata carries no precedence, so it is dropped before comparison. An
// unparseable version is an error: returning a pass here would let a wire change
// through whenever the version is spelled in a way this parser does not recognize.
// commentlint: allow(JUDGE)
fn parse_version(version: &str) -> Result<Version, String> {
    let without_build = version.split('+').next().unwrap_or(version);
    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, prerelease)) => (core, prerelease),
        None => (without_build, ""),
    };
    let mut parts = core.split('.');
    let mut number = |field: &str| -> Result<u64, String> {
        parts
            .next()
            .ok_or_else(|| format!("unparseable package version {version:?}: missing {field}"))?
            .parse()
            .map_err(|error| format!("unparseable package version {version:?}: {field}: {error}"))
    };
    let triple = (number("major")?, number("minor")?, number("patch")?);
    if parts.next().is_some() {
        return Err(format!(
            "unparseable package version {version:?}: more than three numeric fields"
        ));
    }
    let prerelease = match without_build.split_once('-') {
        None => Vec::new(),
        Some(_) => prerelease
            .split('.')
            .map(|identifier| identifier_of(version, identifier))
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(Version { triple, prerelease })
}

// An empty or leading-zero identifier is not a version Cargo accepts, so it is reported
// rather than ordered under a guess.
// commentlint: allow(JUDGE)
fn identifier_of(version: &str, identifier: &str) -> Result<Identifier, String> {
    if identifier.is_empty() {
        return Err(format!(
            "unparseable package version {version:?}: empty prerelease identifier"
        ));
    }
    if !identifier.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(Identifier::Alphanumeric(identifier.to_owned()));
    }
    if identifier.len() > 1 && identifier.starts_with('0') {
        return Err(format!(
            "unparseable package version {version:?}: leading zero in numeric prerelease \
             identifier {identifier:?}"
        ));
    }
    Ok(Identifier::Numeric(identifier.to_owned()))
}

// `provenance` and `build_identity` are recording metadata, not wire data or classification; changing them does not require a version bump.
//
// Every wire-surface section must be present and non-null: serde_json indexing
// maps an absent key to `Null`, so two fixtures that both lack a section would
// otherwise project equal and the gate would pass vacuously after a fixture
// shape change.
fn represented_wire_surface(fixture: &str) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(fixture).map_err(|error| format!("unparseable fixture: {error}"))?;
    for section in WIRE_SURFACE_SECTIONS {
        if value.get(section).is_none_or(Value::is_null) {
            return Err(format!("fixture is missing wire-surface section {section}"));
        }
    }
    Ok(json!({
        "schema_version": value["schema_version"],
        "ciphersuite": value["ciphersuite"],
        "inputs": value["inputs"],
        "expected": value["expected"],
    }))
}

const WIRE_SURFACE_SECTIONS: [&str; 4] = ["schema_version", "ciphersuite", "inputs", "expected"];

fn changed_sections(base: &Value, head: &Value) -> String {
    let sections: Vec<&str> = WIRE_SURFACE_SECTIONS
        .into_iter()
        .filter(|section| base[section] != head[section])
        .collect();
    sections.join(", ")
}

#[derive(Clone, Copy)]
struct Revision<'a> {
    fixture: Option<&'a str>,
    manifest: &'a str,
}

fn at<'a>(fixture: Option<&'a str>, manifest: &'a str) -> Revision<'a> {
    Revision { fixture, manifest }
}

// One rule covers every revision the head could land beside: a prior constrains the
// version only when its represented wire surface differs from the head's. A prior with
// no fixture predates the surface, and a prior sharing the head's surface describes the
// same bytes, so neither is a version the head must clear. The merge base answers
// "did this branch change anything" and the base tip answers "is this version already
// taken", and both follow from that one rule.
// commentlint: allow(JUDGE)
fn check_version_gate(
    priors: &[Revision],
    head_fixture: &str,
    head_manifest: &str,
) -> Result<(), String> {
    let head_surface = represented_wire_surface(head_fixture)?;
    let mut constraints = Vec::new();
    for prior in priors {
        let Some(fixture) = prior.fixture else {
            continue;
        };
        let prior_surface = represented_wire_surface(fixture)?;
        if prior_surface == head_surface {
            continue;
        }
        constraints.push((
            package_version(prior.manifest)?,
            changed_sections(&prior_surface, &head_surface),
        ));
    }
    if constraints.is_empty() {
        return Ok(());
    }
    let head_version = package_version(head_manifest)?;
    let head = parse_version(&head_version)?;
    for (prior_version, changed) in constraints {
        let prior = parse_version(&prior_version)?;
        if head == prior {
            return Err(format!(
                "push-seal wire fixture changed without a package version bump \
                 ({head_version}); changed: {changed}"
            ));
        }
        if head < prior {
            return Err(format!(
                "push-seal wire fixture changed but the package version did not increase \
                 ({prior_version} -> {head_version}); changed: {changed}"
            ));
        }
    }
    Ok(())
}

fn fixture_with(version: &str, aad_hex: &str, provenance: &str) -> String {
    json!({
        "schema_version": 1,
        "provenance": { "material": provenance },
        "build_identity": { "package_version": version },
        "ciphersuite": { "kem": { "codepoint": 32 } },
        "inputs": { "aad_hex": aad_hex },
        "expected": { "envelope_hex": "01" },
    })
    .to_string()
}

#[test]
fn synthetic_version_gate_cases() {
    let manifest_v1 = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n";
    let manifest_v2 = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.2.0\"\n";
    let manifest_v0 = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.0.9\"\n";

    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let prose_only = fixture_with("0.1.0", "01", "rewritten provenance prose");
    let reformatted = format!("\n{}\n", unchanged.replace(",\"", ",\n\""));

    assert_eq!(
        check_version_gate(
            &[at(Some(&unchanged), manifest_v1)],
            &unchanged,
            manifest_v1
        ),
        Ok(())
    );
    assert!(check_version_gate(
        &[at(Some(&unchanged), manifest_v1)],
        &wire_change,
        manifest_v1
    )
    .unwrap_err()
    .contains("without a package version bump"));
    assert_eq!(
        check_version_gate(
            &[at(Some(&unchanged), manifest_v1)],
            &wire_change,
            manifest_v2
        ),
        Ok(())
    );
    assert!(check_version_gate(
        &[at(Some(&unchanged), manifest_v1)],
        &wire_change,
        manifest_v0
    )
    .unwrap_err()
    .contains("did not increase"));
    assert_eq!(
        check_version_gate(&[at(None, manifest_v1)], &wire_change, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(
            &[at(Some(&unchanged), manifest_v1)],
            &prose_only,
            manifest_v1
        ),
        Ok(())
    );
    assert_eq!(
        check_version_gate(
            &[at(Some(&unchanged), manifest_v1)],
            &reformatted,
            manifest_v1
        ),
        Ok(())
    );
    assert_eq!(
        check_version_gate(
            &[at(Some(&unchanged), manifest_v1)],
            &unchanged,
            "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n# prose\n"
        ),
        Ok(())
    );
    assert!(check_version_gate(
        &[at(Some("not json"), manifest_v1)],
        &unchanged,
        manifest_v1
    )
    .unwrap_err()
    .contains("unparseable fixture"));
}

// Valid JSON that lacks a wire-surface section must fail loudly on either
// side of the comparison; two hollow fixtures projecting `Null == Null`
// would otherwise pass the gate vacuously.
#[test]
fn a_fixture_missing_a_wire_surface_section_fails_the_gate() {
    let manifest = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n";
    let unchanged = fixture_with("0.1.0", "01", "synthetic");

    for section in WIRE_SURFACE_SECTIONS {
        let mut hollow: Value = serde_json::from_str(&unchanged).expect("fixture");
        hollow.as_object_mut().expect("object").remove(section);
        let hollow = hollow.to_string();

        for (base, head) in [
            (&unchanged, &hollow),
            (&hollow, &unchanged),
            (&hollow, &hollow),
        ] {
            let error =
                check_version_gate(&[at(Some(base), manifest)], head, manifest).unwrap_err();
            assert!(
                error.contains("missing wire-surface section"),
                "removing {section} passed the gate: {error}"
            );
        }

        let mut nulled: Value = serde_json::from_str(&unchanged).expect("fixture");
        nulled[section] = Value::Null;
        assert!(
            check_version_gate(
                &[at(Some(&unchanged), manifest)],
                &nulled.to_string(),
                manifest
            )
            .unwrap_err()
            .contains("missing wire-surface section"),
            "nulling {section} passed the gate"
        );
    }
}

#[test]
fn manifest_formatting_does_not_change_the_read_version() {
    let canonical = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n";
    for variant in [
        "[ package ]\nversion = \"0.1.0\"\n",
        "[package]\nversion=\"0.1.0\"\n",
        "[package]\nversion  =   \"0.1.0\"   # pinned\n",
        "[package] # metadata\nversion = \"0.1.0\"\n",
        "[dependencies]\nfoo = \"9.9.9\"\n[package]\nversion = \"0.1.0\"\n",
        "[package]\nkeywords = [\n    \"push\",\n]\nversion = \"0.1.0\"\n",
        "[package]\n\"version\" = \"0.1.0\"\n",
        "[package]\n'version' = \"0.1.0\"\n",
        "[package]\nversion = \"0.\\u0031.0\"\n",
        "package.version = \"0.1.0\"\n",
        "[package]\ndescription = \"\"\"\n[package]\nversion = \"9.9.9\"\n\"\"\"\nversion = \"0.1.0\"\n",
    ] {
        assert_eq!(
            package_version(variant),
            package_version(canonical),
            "variant read a different version: {variant:?}"
        );
    }
}

#[test]
fn a_commented_version_still_gates_a_wire_change() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let commented = "[package]\nversion = \"0.1.0\" # pinned until the wire settles\n";

    assert!(
        check_version_gate(&[at(Some(&unchanged), commented)], &wire_change, commented)
            .unwrap_err()
            .contains("without a package version bump")
    );
    assert!(check_version_gate(
        &[at(Some(&unchanged), commented)],
        &wire_change,
        "[package]\nversion = \"0.0.9\" # rolled back\n"
    )
    .unwrap_err()
    .contains("did not increase"));
}

#[test]
fn literal_string_quoting_reads_the_same_version() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let double = "[package]\nversion = \"0.1.0\"\n";
    let literal = "[package]\nversion = '0.1.0'\n";

    assert_eq!(package_version(literal), package_version(double));
    assert!(
        check_version_gate(&[at(Some(&unchanged), double)], &wire_change, literal)
            .unwrap_err()
            .contains("without a package version bump")
    );
}

#[test]
fn an_unreadable_manifest_fails_the_gate() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let base = "[package]\nversion = \"0.1.0\"\n";

    assert!(
        check_version_gate(&[at(Some(&unchanged), base)], &wire_change, "[package")
            .unwrap_err()
            .contains("unparseable manifest")
    );
    assert!(
        check_version_gate(&[at(Some(&unchanged), base)], &wire_change, "[workspace]\n")
            .unwrap_err()
            .contains("no [package] version")
    );
}

#[test]
fn a_workspace_inherited_version_names_itself_in_the_failure() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let base = "[package]\nversion = \"0.1.0\"\n";

    let error = check_version_gate(
        &[at(Some(&unchanged), base)],
        &wire_change,
        "[package]\nversion.workspace = true\n",
    )
    .unwrap_err();
    assert!(error.contains("inherited from the workspace"), "{error}");
    assert!(!error.contains("has no [package] version"), "{error}");

    let error = check_version_gate(
        &[at(Some(&unchanged), base)],
        &wire_change,
        "[package]\nversion = 1\n",
    )
    .unwrap_err();
    assert!(error.contains("is not a string"), "{error}");
}

#[test]
fn the_failure_names_the_section_that_changed() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let manifest = "[package]\nversion = \"0.1.0\"\n";
    let mut inputs_only: Value = serde_json::from_str(&unchanged).expect("fixture");
    inputs_only["inputs"]["aad_hex"] = json!("02");
    let mut expected_only: Value = serde_json::from_str(&unchanged).expect("fixture");
    expected_only["expected"]["envelope_hex"] = json!("02");

    let error = check_version_gate(
        &[at(Some(&unchanged), manifest)],
        &inputs_only.to_string(),
        manifest,
    )
    .unwrap_err();
    assert!(error.contains("changed: inputs"), "{error}");

    let error = check_version_gate(
        &[at(Some(&unchanged), manifest)],
        &expected_only.to_string(),
        manifest,
    )
    .unwrap_err();
    assert!(error.contains("changed: expected"), "{error}");
}

#[test]
fn an_unparseable_version_fails_the_gate() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let base = "[package]\nversion = \"0.1.0\"\n";

    for head in [
        "[package]\nversion = \"0.2\"\n",
        "[package]\nversion = \"0.2.0.1\"\n",
        "[package]\nversion = \"zero.two.zero\"\n",
    ] {
        assert!(
            check_version_gate(&[at(Some(&unchanged), base)], &wire_change, head)
                .unwrap_err()
                .contains("unparseable package version"),
            "an unparseable version passed the gate: {head:?}"
        );
    }
}

#[test]
fn prerelease_versions_compare_by_semver_precedence() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let manifest = |version: &str| format!("[package]\nversion = \"{version}\"\n");

    for (base, head) in [
        ("0.2.0-alpha.1", "0.2.0-alpha.2"),
        ("0.2.0-alpha.1", "0.2.0-alpha.1.1"),
        ("0.2.0-alpha.9", "0.2.0-beta"),
        ("0.2.0-alpha", "0.2.0"),
        ("0.2.0-alpha.1", "0.2.0+build.7"),
    ] {
        assert_eq!(
            check_version_gate(
                &[at(Some(&unchanged), &manifest(base))],
                &wire_change,
                &manifest(head)
            ),
            Ok(()),
            "{base} -> {head} was rejected"
        );
    }

    for (base, head) in [
        ("0.2.0-alpha.2", "0.2.0-alpha.1"),
        ("0.2.0-beta", "0.2.0-alpha.9"),
        ("0.2.0", "0.2.0-alpha"),
        ("0.2.0-alpha.1.1", "0.2.0-alpha.1"),
    ] {
        assert!(
            check_version_gate(
                &[at(Some(&unchanged), &manifest(base))],
                &wire_change,
                &manifest(head)
            )
            .unwrap_err()
            .contains("did not increase"),
            "{base} -> {head} was accepted"
        );
    }

    assert!(check_version_gate(
        &[at(Some(&unchanged), &manifest("0.2.0+build.1"))],
        &wire_change,
        &manifest("0.2.0+build.2")
    )
    .unwrap_err()
    .contains("without a package version bump"));
}

#[test]
fn numeric_prerelease_identifiers_order_by_value_at_any_width() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let manifest = |version: &str| format!("[package]\nversion = \"{version}\"\n");
    let wide = "100000000000000000000";
    let narrower = "99999999999999999999";

    assert_eq!(
        check_version_gate(
            &[at(
                Some(&unchanged),
                &manifest(&format!("0.2.0-alpha.{narrower}"))
            )],
            &wire_change,
            &manifest(&format!("0.2.0-alpha.{wide}"))
        ),
        Ok(())
    );
    assert!(
        check_version_gate(
            &[at(
                Some(&unchanged),
                &manifest(&format!("0.2.0-alpha.{wide}"))
            )],
            &wire_change,
            &manifest(&format!("0.2.0-alpha.{narrower}"))
        )
        .unwrap_err()
        .contains("did not increase"),
        "a decrease past u64 was accepted"
    );
    assert!(check_version_gate(
        &[at(
            Some(&unchanged),
            &manifest(&format!("0.2.0-alpha.{wide}"))
        )],
        &wire_change,
        &manifest(&format!("0.2.0-alpha.{wide}"))
    )
    .unwrap_err()
    .contains("without a package version bump"));
    assert_eq!(
        check_version_gate(
            &[at(
                Some(&unchanged),
                &manifest(&format!("0.2.0-alpha.{wide}"))
            )],
            &wire_change,
            &manifest("0.2.0-alpha.abc")
        ),
        Ok(())
    );
    assert!(check_version_gate(
        &[at(Some(&unchanged), &manifest("0.2.0-alpha.abc"))],
        &wire_change,
        &manifest(&format!("0.2.0-alpha.{wide}"))
    )
    .unwrap_err()
    .contains("did not increase"));
}

#[test]
fn a_malformed_prerelease_identifier_fails_the_gate() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let base = "[package]\nversion = \"0.1.0\"\n";

    for head in [
        "[package]\nversion = \"0.2.0-alpha.007\"\n",
        "[package]\nversion = \"0.2.0-alpha..1\"\n",
        "[package]\nversion = \"0.2.0-\"\n",
    ] {
        assert!(
            check_version_gate(&[at(Some(&unchanged), base)], &wire_change, head)
                .unwrap_err()
                .contains("unparseable package version"),
            "a malformed prerelease passed the gate: {head:?}"
        );
    }
}

#[test]
fn a_version_already_taken_on_the_base_tip_fails_the_gate() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let other_change = fixture_with("0.1.0", "03", "synthetic");
    let merge_base = "[package]\nversion = \"0.1.0\"\n";
    let base_tip = "[package]\nversion = \"0.2.0\"\n";
    let head = "[package]\nversion = \"0.2.0\"\n";

    assert_eq!(
        check_version_gate(&[at(Some(&unchanged), merge_base)], &wire_change, head),
        Ok(())
    );
    assert!(check_version_gate(
        &[
            at(Some(&unchanged), merge_base),
            at(Some(&other_change), base_tip)
        ],
        &wire_change,
        head
    )
    .unwrap_err()
    .contains("without a package version bump"));
    assert_eq!(
        check_version_gate(
            &[
                at(Some(&unchanged), merge_base),
                at(Some(&other_change), base_tip)
            ],
            &wire_change,
            "[package]\nversion = \"0.3.0\"\n"
        ),
        Ok(())
    );
}

#[test]
fn a_base_tip_that_already_carries_this_surface_demands_no_further_bump() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let merge_base = "[package]\nversion = \"0.1.0\"\n";
    let bumped = "[package]\nversion = \"0.2.0\"\n";

    // The base tip already carries this surface at this version, so merging introduces
    // no change relative to it.
    assert_eq!(
        check_version_gate(
            &[
                at(Some(&unchanged), merge_base),
                at(Some(&wire_change), bumped)
            ],
            &wire_change,
            bumped
        ),
        Ok(())
    );
    // A prior that predates the fixture constrains nothing either.
    assert_eq!(
        check_version_gate(
            &[at(None, merge_base), at(Some(&wire_change), bumped)],
            &wire_change,
            bumped
        ),
        Ok(())
    );
    // Same surface on the base tip, but the head walked the version backwards.
    assert!(check_version_gate(
        &[
            at(Some(&unchanged), merge_base),
            at(Some(&wire_change), bumped)
        ],
        &wire_change,
        "[package]\nversion = \"0.1.0\"\n"
    )
    .unwrap_err()
    .contains("without a package version bump"));
}

fn git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    String::from_utf8(output.stdout).map_err(|error| format!("git returned non-UTF-8: {error}"))
}

// `Ok(None)` means Git reports that the path is absent at the requested revision; any git failure propagates as `Err` so a transient failure cannot pass as fixture bootstrap.
fn revision_file(revision: &str, path: &str) -> Result<Option<String>, String> {
    git(&["cat-file", "-e", &format!("{revision}^{{commit}}")])
        .map_err(|error| format!("unreadable revision {revision}: {error}"))?;
    // `--full-tree` resolves the path from the repository root; Cargo runs the test from the crate directory.
    let listing = git(&["ls-tree", "--full-tree", revision, "--", path])
        .map_err(|error| format!("cannot list {path} at {revision}: {error}"))?;
    if listing.trim().is_empty() {
        return Ok(None);
    }
    git(&["show", &format!("{revision}:{path}")])
        .map(Some)
        .map_err(|error| format!("cannot read {path} at {revision}: {error}"))
}

#[test]
#[ignore = "requires PUSH_SEAL_BASE_SHA and PUSH_SEAL_HEAD_SHA"]
fn actual_git_diff_requires_version_bump() {
    let base = std::env::var("PUSH_SEAL_BASE_SHA").expect("PUSH_SEAL_BASE_SHA must be set");
    let head = std::env::var("PUSH_SEAL_HEAD_SHA").expect("PUSH_SEAL_HEAD_SHA must be set");

    // Two revisions the head could land beside: the merge base answers whether this
    // branch changed the surface at all, and the base tip answers whether the version is
    // already taken there. A revision without the fixture predates it and constrains
    // nothing, so its manifest is never read.
    // commentlint: allow(JUDGE)
    let merge_base = git(&["merge-base", &base, &head])
        .unwrap_or_else(|error| panic!("no merge base between {base} and {head}: {error}"));
    let merge_base = merge_base.trim();
    let head_fixture = revision_file(&head, FIXTURE_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{FIXTURE_PATH} missing at {head}"));
    let head_manifest = revision_file(&head, MANIFEST_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{MANIFEST_PATH} missing at {head}"));

    let mut revisions = vec![merge_base.to_owned()];
    if base != merge_base {
        revisions.push(base.clone());
    }
    let priors: Vec<(Option<String>, String)> = revisions
        .iter()
        .filter_map(|revision| {
            let fixture = revision_file(revision, FIXTURE_PATH).unwrap()?;
            let manifest = revision_file(revision, MANIFEST_PATH)
                .unwrap()
                .unwrap_or_else(|| panic!("{MANIFEST_PATH} missing at {revision}"));
            Some((Some(fixture), manifest))
        })
        .collect();
    let priors: Vec<Revision> = priors
        .iter()
        .map(|(fixture, manifest)| at(fixture.as_deref(), manifest))
        .collect();

    check_version_gate(&priors, &head_fixture, &head_manifest).unwrap();
}
