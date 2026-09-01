# cortexkit/commons

Neutral home for cross-product [CortexKit](https://github.com/cortexkit) primitives — small, dependency-light building blocks shared across **subc**, **AFT**, and **Magic Context** that belong to no single product.

## Ownership

Maintained by the **subc** seat: direction, review, and releases.

Two crates carry a standing review obligation to the **claustrum** (vault) seat,
who must be routed any change to:

- `cortexkit-store` / `cortexkit-lease` — changes carry an external review and
  real-daemon coverage obligation. A claustrum review receipt remains a merge
  gate because no qualifying test exists in this repository or was supplied.
- **`cortexkit-paths` canonicalization** — see the warning below.

That is a duty carried, not a veto held.

## Publication is per-crate, and most of these are NOT published

Measured 2026-08-09 against crates.io rather than inferred from release tags:

| state | crates |
|---|---|
| published | `cortexkit-paths` (0.1.1), `cortexkit-provider-usage` (0.4.1) |
| unpublished | the other six — by omission, none sets `publish = false` |

Release tags are **not** the authority on what is published: `provider-usage` has
five versions on crates.io and four tags. Ask the registry.

**Publishing a crate here is close to irreversible** — a version can be yanked but
never removed — and it creates a SECOND DISTRIBUTION PATH. A crate consumed only
by sibling path-dependencies has exactly one; publishing it means a consumer can
resolve a registry version while another repo's sibling checkout floats
elsewhere, and both can end up in one binary. That is already live: `claustrum`
compiles two copies of `cortexkit-paths` at the same version, one path and one
registry, agreeing only because the published bytes currently match.

So: publish only when an external consumer genuinely cannot use a path
dependency, and set `publish = false` explicitly with the reason at the key when
the answer is no.

## Version bumps are the only signal a path-dependency consumer gets

`Cargo.lock` records a path dependency as a bare version string with **no source
and no checksum**, so changed code compiles into every consuming repo with no
lockfile diff and nothing for `--locked` to catch. The version number is the
entire channel.

Bump on any change to observable behaviour or emitted bytes. Not for comments or
tests — a version that moves for prose trains readers to bump reflexively, which
is how it stops meaning anything.

### `cortexkit-lease` 0.2 compatibility

Version 0.2 never publishes an empty final lease file: it initializes epoch zero
in a same-directory temporary file and publishes that inode without replacing an
existing path. Anything except 1-20 ASCII digits in `u64` range then fails with
`InvalidData` instead of resetting the counter.

Nothing on disk marks the state format. Both versions use the same lease file
name and the same decimal body, so the upgrade order here is enforced by the
operator rather than by code, and a violation is silent.

A 0.1.x reader resets the state that 0.2 rejects. Version 0.1.x parses the epoch
as `buf.trim().parse().unwrap_or(0)`, so an empty file, a legacy file caught
mid-update, or an over-width body reads as epoch 0, and the counter then
re-issues 1, 2, 3 — tokens a database or a peer still holds. One straggler on a
shared lease root is enough, and so is a rollback to 0.1.x. Upgrade every
consumer that shares a lease root before any consumer starts using 0.2, and do
not roll a single consumer back afterwards. A durable format discriminator, or a
0.1.2 whose reader fails closed, would make the ordering enforceable; neither
ships here.

Version 0.1.x also leaves empty lease files behind. Its shared acquisition
creates the file and never writes it, so any key first touched by a 0.1.x reader
has a 0-byte file, and a 0.1.x writer killed between truncate and write leaves
one too. Version 0.2 refuses those keys, nothing heals them, and deletion is
unsafe. Audit every lease root and repair each hit with the procedure below
before the first 0.2 consumer starts:

```sh
find "$lease_root" -name '*.lease' -size 0c -print
```

Do not recover an invalid epoch by deleting its lease file. Deletion resets the
counter and is unsafe when a database or another consumer retains a fence. Stop
every holder for that key, then choose a decimal `u64` greater than every
persisted consumer fence; epoch zero is correct only for a key no consumer has
ever written. Run the repair as a script, once per reported path, so a rejected
input cannot leave a truncated file behind:

```sh
#!/bin/sh
set -eu
printf '%s' "$epoch" | grep -Eq '^[0-9]{1,20}$' || {
  echo "epoch must be 1-20 decimal digits" >&2
  exit 1
}
printf '00000000000000000000%s' "$epoch" | tail -c 20 > "$lease_file.new"
chmod 600 "$lease_file.new"
mv "$lease_file.new" "$lease_file"
```

Every step of that shape is load-bearing. `printf '%020d'` converts through a
signed integer and silently truncates any epoch above `i64::MAX`. `printf '%020s'`
leaves the `0` flag undefined; coreutils `printf` rejects it and writes nothing,
which empties the target under direct redirection. An unvalidated `$epoch` writes
a valid epoch zero when it is unset, which is the rollback this procedure exists
to avoid. Writing to `$lease_file.new` and renaming keeps the live file intact
until a full 20 digits exist. Restart only upgraded consumers.

## Crates

| Crate | Description |
|-------|-------------|
| [`cortexkit-paths`](crates/cortexkit-paths) | Path canonicalization → canonical project-root identity (`ProjectRootId`). Dependency-free, `#![forbid(unsafe_code)]`, cross-platform (incl. Windows verbatim/UNC/drive-case normalization). **Its canonical form is a cryptographic identity input** — the vault hashes it to derive the keychain service name holding its master key and the vault id fencing admin MACs. A canonicalization change breaks those and presents as a locked vault over an intact store, never as a path mismatch. The name reads as a path helper; it is not only that. |

## License

MIT
