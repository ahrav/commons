#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
RESOLVER="$ROOT/.github/scripts/resolve-release-tag.sh"
FIXTURE=$(mktemp -d)
trap 'rm -rf "$FIXTURE"' EXIT

mkdir -p \
  "$FIXTURE/crates/inherited/src" \
  "$FIXTURE/crates/formatted/src"
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace]
resolver = "2"
members = ["crates/inherited", "crates/formatted"]

[workspace.package]
version = "1.2.3"
EOF
cat > "$FIXTURE/crates/inherited/Cargo.toml" <<'EOF'
[package]
name = "inherited-package"
version.workspace = true
edition = "2021"
EOF
cat > "$FIXTURE/crates/formatted/Cargo.toml" <<'EOF'
package.name = 'formatted-package'
package.version = '2.3.4'
package.edition = '2021'
EOF
touch \
  "$FIXTURE/crates/inherited/src/lib.rs" \
  "$FIXTURE/crates/formatted/src/lib.rs"

OUTPUT="$FIXTURE/output"
LOG="$FIXTURE/log"

run_resolver() {
  local tag=$1
  (
    cd "$FIXTURE"
    GITHUB_REF_NAME="$tag" GITHUB_OUTPUT="$OUTPUT" bash "$RESOLVER"
  )
}

fail() {
  local tag=$1
  local expected=$2
  echo "resolver assertion failed: tag=$tag expected=$expected"
  echo "resolver log:"
  cat "$LOG"
  exit 1
}

assert_success() {
  local tag=$1
  local crate=$2
  : > "$OUTPUT"
  if ! run_resolver "$tag" > "$LOG" 2>&1; then
    fail "$tag" "success with crate=$crate"
  fi
  grep -Fxq "crate=$crate" "$OUTPUT" \
    || fail "$tag" "output containing crate=$crate"
}

assert_failure() {
  local tag=$1
  local expected=$2
  : > "$OUTPUT"
  if run_resolver "$tag" > "$LOG" 2>&1; then
    fail "$tag" "failure containing $expected"
  fi
  grep -Fq "$expected" "$LOG" \
    || fail "$tag" "failure containing $expected"
}

assert_failure "inherited-package-1.2.3" "must be '<crate>-v<version>'"
assert_failure "missing-package-v1.2.3" "No workspace package named 'missing-package'"
assert_failure "inherited-package-v1.2.4" "Tag wants 1.2.4 but Cargo reports 1.2.3"
assert_success "inherited-package-v1.2.3" "inherited-package"
assert_success "formatted-package-v2.3.4" "formatted-package"
