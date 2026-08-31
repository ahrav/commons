use std::process::Command;

use serde_json::{json, Value};

const FIXTURE_PATH: &str = "crates/cortexkit-push-seal/tests/golden/push-seal-wire-v1.json";
const MANIFEST_PATH: &str = "crates/cortexkit-push-seal/Cargo.toml";

fn package_version(manifest: &str) -> &str {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line == "[package]" {
            in_package = true;
        } else if line.starts_with('[') {
            in_package = false;
        } else if in_package {
            if let Some(version) = line.strip_prefix("version = ") {
                return version.trim_matches('"');
            }
        }
    }
    panic!("package version missing");
}

fn check_version_gate(
    base_fixture: Option<&str>,
    head_fixture: &str,
    base_manifest: &str,
    head_manifest: &str,
) -> Result<(), &'static str> {
    let Some(base_fixture) = base_fixture else {
        return Ok(());
    };
    if represented_contract(base_fixture)? != represented_contract(head_fixture)?
        && package_version(base_manifest) == package_version(head_manifest)
    {
        return Err("push-seal fixture changed without a package version bump");
    }
    Ok(())
}

fn represented_contract(fixture: &str) -> Result<Value, &'static str> {
    let fixture: Value =
        serde_json::from_str(fixture).map_err(|_| "push-seal fixture is not valid JSON")?;
    Ok(json!({
        "ciphersuite": {
            "mode": required(&fixture, &["ciphersuite", "mode"])?,
            "kem": required(&fixture, &["ciphersuite", "kem", "codepoint"])?,
            "kdf": required(&fixture, &["ciphersuite", "kdf", "codepoint"])?,
            "aead": required(&fixture, &["ciphersuite", "aead", "codepoint"])?,
        },
        "inputs": required(&fixture, &["inputs"])?,
        "expected": required(&fixture, &["expected"])?,
    }))
}

/// A structurally hollow fixture must fail loudly, not project `Null` and
/// compare equal to another hollow fixture: absent or `Null` contract fields
/// would otherwise make the version gate silently vacuous after a fixture
/// shape change.
fn required<'a>(fixture: &'a Value, path: &[&str]) -> Result<&'a Value, &'static str> {
    let mut current = fixture;
    for key in path {
        current = current
            .get(key)
            .ok_or("push-seal fixture is missing a represented contract field")?;
    }
    if current.is_null() {
        return Err("push-seal fixture is missing a represented contract field");
    }
    Ok(current)
}

#[test]
fn synthetic_version_gate_cases() {
    let manifest_v1 = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n";
    let manifest_v2 = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.2.0\"\n";
    let contract_v1 = r#"{
        "ciphersuite":{"mode":"Base","kem":{"codepoint":32},"kdf":{"codepoint":1},"aead":{"codepoint":3}},
        "inputs":{"aad":"01"},
        "expected":{"wire":"aa"}
    }"#;
    let contract_v1_with_new_prose = r#"{
        "provenance": {"note": "updated test prose"},
        "build_identity": {"source_revision": "different"},
        "ciphersuite":{"mode":"Base","kem":{"codepoint":32},"kdf":{"codepoint":1},"aead":{"codepoint":3}},
        "inputs": {"aad": "01"},
        "expected": {"wire": "aa"}
    }"#;
    let contract_v2 = r#"{
        "ciphersuite":{"mode":"Base","kem":{"codepoint":32},"kdf":{"codepoint":1},"aead":{"codepoint":3}},
        "inputs":{"aad":"01"},
        "expected":{"wire":"bb"}
    }"#;
    let hollow_contract = r#"{"ciphersuite":{"mode":"Base"},"inputs":{"aad":"01"}}"#;

    assert_eq!(
        check_version_gate(Some(contract_v1), contract_v1, manifest_v1, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(Some(contract_v1), contract_v2, manifest_v1, manifest_v1),
        Err("push-seal fixture changed without a package version bump")
    );
    assert_eq!(
        check_version_gate(Some(contract_v1), contract_v2, manifest_v1, manifest_v2),
        Ok(())
    );
    assert_eq!(
        check_version_gate(None, contract_v1, manifest_v1, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(
            Some(contract_v1),
            contract_v1_with_new_prose,
            manifest_v1,
            "unrelated prose or tests"
        ),
        Ok(())
    );
    assert_eq!(
        check_version_gate(Some(contract_v1), hollow_contract, manifest_v1, manifest_v1),
        Err("push-seal fixture is missing a represented contract field")
    );
    assert_eq!(
        check_version_gate(
            Some(hollow_contract),
            hollow_contract,
            manifest_v1,
            manifest_v1
        ),
        Err("push-seal fixture is missing a represented contract field")
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

fn revision_file(revision: &str, path: &str) -> Result<Option<String>, String> {
    git(&["cat-file", "-e", &format!("{revision}^{{commit}}")])
        .map_err(|error| format!("unreadable revision {revision}: {error}"))?;
    match git(&["show", &format!("{revision}:{path}")]) {
        Ok(contents) => Ok(Some(contents)),
        Err(_) => Ok(None),
    }
}

/// True when the revision resolves to a readable commit in this clone. Push
/// events can name a base that is not fetchable (ref creation or a
/// force-push); such a base identifies nothing to compare against, so the
/// gate treats it like a missing base fixture instead of failing the run.
fn revision_exists(revision: &str) -> bool {
    git(&["cat-file", "-e", &format!("{revision}^{{commit}}")]).is_ok()
}

#[test]
#[ignore = "requires PUSH_SEAL_BASE_SHA and PUSH_SEAL_HEAD_SHA"]
fn actual_git_diff_requires_version_bump() {
    let base = std::env::var("PUSH_SEAL_BASE_SHA").expect("PUSH_SEAL_BASE_SHA must be set");
    let head = std::env::var("PUSH_SEAL_HEAD_SHA").expect("PUSH_SEAL_HEAD_SHA must be set");

    if !revision_exists(&base) {
        eprintln!("skipping version gate: base revision {base} is not a readable commit");
        return;
    }

    let base_fixture = revision_file(&base, FIXTURE_PATH).unwrap();
    let head_fixture = revision_file(&head, FIXTURE_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{FIXTURE_PATH} missing at {head}"));
    let base_manifest = revision_file(&base, MANIFEST_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{MANIFEST_PATH} missing at {base}"));
    let head_manifest = revision_file(&head, MANIFEST_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{MANIFEST_PATH} missing at {head}"));

    check_version_gate(
        base_fixture.as_deref(),
        &head_fixture,
        &base_manifest,
        &head_manifest,
    )
    .unwrap();
}
