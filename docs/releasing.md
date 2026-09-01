# Releasing a crate

Each crate in this workspace versions and publishes independently. A release is
triggered by pushing a tag; there is no manual publish step.

## Procedure

1. Bump the crate's version in its `Cargo.toml` and merge to `main`.
2. Wait for CI to pass on that commit.
3. Tag it `<crate>-v<version>` — for example `cortexkit-paths-v0.1.1` — and push
   the tag.

If the unpublished lease and store crates are ever published at version 0.2,
publish `cortexkit-store-types` and `cortexkit-lease` first. Publish
`cortexkit-store` and `cortexkit-store-postgres` only after crates.io serves both
versions they depend on. The release workflow runs `cargo publish` without
`--no-verify`, so it builds the packaged crate, which resolves those dependencies
from the registry rather than through their workspace paths: a store crate
requiring `cortexkit-store-types = "0.1.1"` cannot publish while the registry
serves only 0.1.0.

The workflow re-runs the full test matrix (Linux, macOS, Windows) before
publishing, reusing the CI workflow rather than copying it, so a release cannot
ship code that would fail CI. It then parses the crate and version out of the tag,
resolves the package through `cargo metadata`, and refuses to publish if Cargo's
version disagrees with the tag. This catches unknown crates and stale versions
without scraping manifest text.

## Tag shape does not matter

Every tag in this repository's history happens to be lightweight, which reads like
a convention and is not one. The trigger matches on the tag's **ref name**
(`*-v*`); annotated and lightweight tags push the same ref, and the object type
behind it is invisible to the trigger. Either works.

This is written down because the inference is natural, the correction is not
discoverable without reading the workflow, and someone will otherwise spend
attention on it at the exact moment they have none — during an incident, deciding
whether a silent release is their tag's fault.

## When a tag produces no run

Check whether the release service is degraded before changing anything. A failed
publish and a pending one look identical: no run appears for the ref in either
case.

If the service is healthy and there is still no run, delete and re-push the tag.
Before doing so, **confirm no run is already queued for it** — a retag at the
*same* commit lands in the same concurrency group, so a stuck queued run would
swallow the retry. The group deliberately includes the commit SHA, which protects
a retag at a *different* commit and not this case.

Bind any wait to **the crates.io version changing**, not to a run appearing. The
published version is the fact; the workflow run is one mechanism for producing it.
A green run that publishes nothing cannot satisfy the first check, and a publish
that happens by another route still can.
