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

fn version_triple(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
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

fn check_version_gate(
    base_fixture: Option<&str>,
    head_fixture: &str,
    base_manifest: &str,
    head_manifest: &str,
) -> Result<(), String> {
    let Some(base_fixture) = base_fixture else {
        return Ok(());
    };
    if represented_wire_surface(base_fixture)? == represented_wire_surface(head_fixture)? {
        return Ok(());
    }
    let base_version = package_version(base_manifest);
    let head_version = package_version(head_manifest);
    if base_version == head_version {
        return Err(format!(
            "push-seal wire fixture changed without a package version bump ({head_version})"
        ));
    }
    match (version_triple(base_version), version_triple(head_version)) {
        (Some(base), Some(head)) if head <= base => Err(format!(
            "push-seal wire fixture changed but the package version did not increase \
             ({base_version} -> {head_version})"
        )),
        _ => Ok(()),
    }
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
        check_version_gate(Some(&unchanged), &unchanged, manifest_v1, manifest_v1),
        Ok(())
    );
    assert!(
        check_version_gate(Some(&unchanged), &wire_change, manifest_v1, manifest_v1)
            .unwrap_err()
            .contains("without a package version bump")
    );
    assert_eq!(
        check_version_gate(Some(&unchanged), &wire_change, manifest_v1, manifest_v2),
        Ok(())
    );
    assert!(
        check_version_gate(Some(&unchanged), &wire_change, manifest_v1, manifest_v0)
            .unwrap_err()
            .contains("did not increase")
    );
    assert_eq!(
        check_version_gate(None, &wire_change, manifest_v1, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(Some(&unchanged), &prose_only, manifest_v1, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(Some(&unchanged), &reformatted, manifest_v1, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(
            Some(&unchanged),
            &unchanged,
            manifest_v1,
            "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n# prose\n"
        ),
        Ok(())
    );
    assert!(
        check_version_gate(Some("not json"), &unchanged, manifest_v1, manifest_v1)
            .unwrap_err()
            .contains("unparseable fixture")
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

    // The merge base excludes unrelated version drift on the base branch tip.
    let merge_base = git(&["merge-base", &base, &head])
        .unwrap_or_else(|error| panic!("no merge base between {base} and {head}: {error}"));
    let merge_base = merge_base.trim();

    let base_fixture = revision_file(merge_base, FIXTURE_PATH).unwrap();
    let head_fixture = revision_file(&head, FIXTURE_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{FIXTURE_PATH} missing at {head}"));
    let base_manifest = revision_file(merge_base, MANIFEST_PATH)
        .unwrap()
        .unwrap_or_else(|| panic!("{MANIFEST_PATH} missing at {merge_base}"));
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
