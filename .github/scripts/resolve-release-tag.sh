#!/usr/bin/env bash
set -euo pipefail

TAG="${GITHUB_REF_NAME}"
if [[ ! "$TAG" =~ ^(.+)-v([0-9].*)$ ]]; then
  echo "::error::Tag '$TAG' must be '<crate>-v<version>', e.g. cortexkit-paths-v0.1.1"
  exit 1
fi

CRATE="${BASH_REMATCH[1]}"
VERSION="${BASH_REMATCH[2]}"
CARGO_VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r --arg crate "$CRATE" \
  '[.packages[] | select(.name == $crate)] | if length == 1 then .[0].version else empty end')
if [ -z "$CARGO_VERSION" ]; then
  echo "::error::No workspace package named '${CRATE}'"
  exit 1
fi

if [ "$VERSION" != "$CARGO_VERSION" ]; then
  echo "::error::Tag wants ${VERSION} but Cargo reports ${CARGO_VERSION} for ${CRATE}"
  exit 1
fi

echo "crate=$CRATE" >> "$GITHUB_OUTPUT"
echo "Publishing ${CRATE} v${VERSION}"
