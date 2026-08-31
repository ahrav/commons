use std::process::Command;

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
    if base_fixture != head_fixture
        && package_version(base_manifest) == package_version(head_manifest)
    {
        return Err("push-seal fixture changed without a package version bump");
    }
    Ok(())
}

#[test]
fn synthetic_version_gate_cases() {
    let manifest_v1 = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.1.0\"\n";
    let manifest_v2 = "[package]\nname = \"cortexkit-push-seal\"\nversion = \"0.2.0\"\n";

    assert_eq!(
        check_version_gate(Some("same"), "same", manifest_v1, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(Some("old"), "new", manifest_v1, manifest_v1),
        Err("push-seal fixture changed without a package version bump")
    );
    assert_eq!(
        check_version_gate(Some("old"), "new", manifest_v1, manifest_v2),
        Ok(())
    );
    assert_eq!(
        check_version_gate(None, "new", manifest_v1, manifest_v1),
        Ok(())
    );
    assert_eq!(
        check_version_gate(
            Some("same"),
            "same",
            manifest_v1,
            "unrelated prose or tests"
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

fn revision_file(revision: &str, path: &str) -> Result<Option<String>, String> {
    git(&["cat-file", "-e", &format!("{revision}^{{commit}}")])
        .map_err(|error| format!("unreadable revision {revision}: {error}"))?;
    match git(&["show", &format!("{revision}:{path}")]) {
        Ok(contents) => Ok(Some(contents)),
        Err(_) => Ok(None),
    }
}

#[test]
#[ignore = "requires PUSH_SEAL_BASE_SHA and PUSH_SEAL_HEAD_SHA"]
fn actual_git_diff_requires_version_bump() {
    let base = std::env::var("PUSH_SEAL_BASE_SHA").expect("PUSH_SEAL_BASE_SHA must be set");
    let head = std::env::var("PUSH_SEAL_HEAD_SHA").expect("PUSH_SEAL_HEAD_SHA must be set");

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
