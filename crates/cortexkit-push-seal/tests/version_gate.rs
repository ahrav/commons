use std::process::Command;

use serde_json::{json, Value};

const FIXTURE_PATH: &str = "crates/cortexkit-push-seal/tests/golden/push-seal-wire-v1.json";
const MANIFEST_PATH: &str = "crates/cortexkit-push-seal/Cargo.toml";

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    for (index, byte) in line.bytes().enumerate() {
        match byte {
            b'"' | b'\'' => in_string = !in_string,
            b'#' if !in_string => return &line[..index],
            _ => {}
        }
    }
    line
}

fn package_version(manifest: &str) -> &str {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = strip_comment(line).trim();
        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            in_package = header.trim() == "package";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "version" {
                return value.trim().trim_matches(['"', '\'']);
            }
        }
    }
    panic!("package version missing");
}

// Numeric identifiers rank below alphanumeric ones in SemVer precedence, which the
// derived variant order reproduces.
// commentlint: allow(JUDGE)
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Identifier {
    Numeric(u64),
    Alphanumeric(String),
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
    let prerelease = prerelease
        .split('.')
        .filter(|identifier| !identifier.is_empty())
        .map(|identifier| match identifier.parse() {
            Ok(number) => Identifier::Numeric(number),
            Err(_) => Identifier::Alphanumeric(identifier.to_owned()),
        })
        .collect();
    Ok(Version { triple, prerelease })
}

// `provenance` and `build_identity` are recording metadata, not wire data or classification; changing them does not require a version bump.
fn represented_wire_surface(fixture: &str) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(fixture).map_err(|error| format!("unparseable fixture: {error}"))?;
    Ok(json!({
        "schema_version": value["schema_version"],
        "ciphersuite": value["ciphersuite"],
        "inputs": value["inputs"],
        "expected": value["expected"],
    }))
}

// The head version must exceed every version in `prior_manifests`, not the merge base
// alone. Two branches that independently bump to the same version both satisfy a
// merge-base-only check, so one version ends up carrying two wire surfaces.
// commentlint: allow(JUDGE)
fn check_version_gate(
    base_fixture: Option<&str>,
    head_fixture: &str,
    prior_manifests: &[&str],
    head_manifest: &str,
) -> Result<(), String> {
    let Some(base_fixture) = base_fixture else {
        return Ok(());
    };
    if represented_wire_surface(base_fixture)? == represented_wire_surface(head_fixture)? {
        return Ok(());
    }
    let head_version = package_version(head_manifest);
    let head = parse_version(head_version)?;
    for manifest in prior_manifests {
        let prior_version = package_version(manifest);
        let prior = parse_version(prior_version)?;
        if head == prior {
            return Err(format!(
                "push-seal wire fixture changed without a package version bump ({head_version})"
            ));
        }
        if head < prior {
            return Err(format!(
                "push-seal wire fixture changed but the package version did not increase \
                 ({prior_version} -> {head_version})"
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
        check_version_gate(Some(&unchanged), &unchanged, &[manifest_v1], manifest_v1),
        Ok(())
    );
    assert!(
        check_version_gate(Some(&unchanged), &wire_change, &[manifest_v1], manifest_v1)
            .unwrap_err()
            .contains("without a package version bump")
    );
    assert_eq!(
        check_version_gate(Some(&unchanged), &wire_change, &[manifest_v1], manifest_v2),
        Ok(())
    );
    assert!(
        check_version_gate(Some(&unchanged), &wire_change, &[manifest_v1], manifest_v0)
            .unwrap_err()
            .contains("did not increase")
    );
    assert_eq!(
        check_version_gate(None, &wire_change, &[manifest_v1], manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(Some(&unchanged), &prose_only, &[manifest_v1], manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(Some(&unchanged), &reformatted, &[manifest_v1], manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(
            Some(&unchanged),
            &unchanged,
            &[manifest_v1],
            "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n# prose\n"
        ),
        Ok(())
    );
    assert!(
        check_version_gate(Some("not json"), &unchanged, &[manifest_v1], manifest_v1)
            .unwrap_err()
            .contains("unparseable fixture")
    );
}

#[test]
fn manifest_formatting_does_not_change_the_read_version() {
    let canonical = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n";
    for variant in [
        "[ package ]\nversion = \"0.1.0\"\n",
        "[package]\nversion=\"0.1.0\"\n",
        "[package]\nversion  =   \"0.1.0\"   # pinned\n",
        "[package] # metadata\nversion = \"0.1.0\"\n",
        "[dependencies]\nversion = \"9.9.9\"\n[package]\nversion = \"0.1.0\"\n",
        "[package]\nkeywords = [\n    \"push\",\n]\nversion = \"0.1.0\"\n",
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
        check_version_gate(Some(&unchanged), &wire_change, &[commented], commented)
            .unwrap_err()
            .contains("without a package version bump")
    );
    assert!(check_version_gate(
        Some(&unchanged),
        &wire_change,
        &[commented],
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
        check_version_gate(Some(&unchanged), &wire_change, &[double], literal)
            .unwrap_err()
            .contains("without a package version bump")
    );
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
            check_version_gate(Some(&unchanged), &wire_change, &[base], head)
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
                Some(&unchanged),
                &wire_change,
                &[&manifest(base)],
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
                Some(&unchanged),
                &wire_change,
                &[&manifest(base)],
                &manifest(head)
            )
            .unwrap_err()
            .contains("did not increase"),
            "{base} -> {head} was accepted"
        );
    }

    assert!(check_version_gate(
        Some(&unchanged),
        &wire_change,
        &[&manifest("0.2.0+build.1")],
        &manifest("0.2.0+build.2")
    )
    .unwrap_err()
    .contains("without a package version bump"));
}

#[test]
fn a_version_already_taken_on_the_base_tip_fails_the_gate() {
    let unchanged = fixture_with("0.1.0", "01", "synthetic");
    let wire_change = fixture_with("0.1.0", "02", "synthetic");
    let merge_base = "[package]\nversion = \"0.1.0\"\n";
    let base_tip = "[package]\nversion = \"0.2.0\"\n";
    let head = "[package]\nversion = \"0.2.0\"\n";

    assert_eq!(
        check_version_gate(Some(&unchanged), &wire_change, &[merge_base], head),
        Ok(())
    );
    assert!(check_version_gate(
        Some(&unchanged),
        &wire_change,
        &[merge_base, base_tip],
        head
    )
    .unwrap_err()
    .contains("without a package version bump"));
    assert_eq!(
        check_version_gate(
            Some(&unchanged),
            &wire_change,
            &[merge_base, base_tip],
            "[package]\nversion = \"0.3.0\"\n"
        ),
        Ok(())
    );
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

    // The merge base fixes what the head is compared against, so unrelated fixture
    // drift on the base branch tip does not register as this branch's change.
    let merge_base = git(&["merge-base", &base, &head])
        .unwrap_or_else(|error| panic!("no merge base between {base} and {head}: {error}"));
    let merge_base = merge_base.trim();

    // A merge base lacking the fixture may lack the manifest too, so reading the
    // manifest first would panic on the bootstrap case this gate accepts.
    let Some(base_fixture) = revision_file(merge_base, FIXTURE_PATH).unwrap() else {
        return;
    };
    let head_fixture = revision_file(&head, FIXTURE_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{FIXTURE_PATH} missing at {head}"));
    let merge_base_manifest = revision_file(merge_base, MANIFEST_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{MANIFEST_PATH} missing at {merge_base}"));
    let head_manifest = revision_file(&head, MANIFEST_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{MANIFEST_PATH} missing at {head}"));
    // The base tip is where a merge lands. A version already published there is taken,
    // whether or not this branch descends from the commit that took it.
    // commentlint: allow(JUDGE)
    let mut prior_manifests = vec![merge_base_manifest];
    if base != merge_base {
        if let Some(base_tip_manifest) = revision_file(&base, MANIFEST_PATH).unwrap() {
            prior_manifests.push(base_tip_manifest);
        }
    }
    let prior_manifests: Vec<&str> = prior_manifests.iter().map(String::as_str).collect();

    check_version_gate(
        Some(&base_fixture),
        &head_fixture,
        &prior_manifests,
        &head_manifest,
    )
    .unwrap();
}
