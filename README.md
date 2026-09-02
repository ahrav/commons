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

### `cortexkit-lease` 0.3 API and 0.2 state compatibility

Version 0.3 removes `LeaseStore` and `LeaseHandle`; `FileLeaseStore` methods return `HeldFileLease` directly, PostgreSQL keeps its native session lock, and the 0.2 lease path and epoch format remain unchanged.

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

Version 0.1.x leaves two kinds of damaged lease file behind, and only one of
them fails loudly. Audit every lease root for both and repair each hit with the
procedure below before the first 0.2 consumer starts.

Empty files are the visible class. Shared acquisition creates the file and never
writes it, so any key first touched by a 0.1.x reader has a 0-byte file. Version
0.2 refuses those keys, and nothing heals them:

```sh
find "$lease_root" -name '*.lease' -size 0c -print
```

Truncated files that still parse are the silent class, and they are the one that
loses fence tokens. Version 0.1.x `bump_epoch` truncates to zero length and then
writes a variable-width decimal, so a writer killed part-way through leaves a
prefix of the epoch it meant to publish. An interrupted write of `1000` can
leave `1`: nonempty, 1-20 ASCII digits, inside `u64` range, and below the `999`
already issued. Version 0.2 accepts that value and increments from it, reissuing
tokens a database or a peer still holds. A size filter cannot see this, because
a short file is exactly what a healthy 0.1.x epoch also looks like.

Nothing on disk separates a truncated epoch from a legitimately low one, so the
audit has to compare values against state 0.2 cannot read. List every lease file
0.2 has not yet rewritten — a 0.2 write is always exactly 20 bytes — and check
each value against the fence its consumer persisted:

```sh
find "$lease_root" -name '*.lease' ! -size 20c -exec sh -c \
  'for f; do printf "%s\t%s\n" "$(cat "$f")" "$f"; done' sh {} +
```

Repair every value that does not exceed its consumer's persisted fence, not only
the ones 0.2 rejects.

Do not recover an invalid epoch by deleting its lease file. Deletion resets the
counter and is unsafe when a database or another consumer retains a fence. Stop
every holder for that key, then choose a decimal `u64` greater than every
persisted consumer fence; epoch zero is correct only for a key no consumer has
ever written. Run the repair as a script, once per reported path, so a rejected
input cannot leave a truncated file behind:

```sh
#!/bin/sh
set -eu
case "${epoch:-}" in
  '' | *[!0-9]*)
    echo "epoch must be 1-20 decimal digits" >&2
    exit 1
    ;;
esac
[ "${#epoch}" -le 20 ] || {
  echo "epoch must be 1-20 decimal digits" >&2
  exit 1
}
padded=$(printf '00000000000000000000%s' "$epoch" | tail -c 20)
[ "$(printf '%s\n18446744073709551615\n' "$padded" | LC_ALL=C sort | tail -n 1)" \
  = 18446744073709551615 ] || {
  echo "epoch must not exceed u64::MAX (18446744073709551615)" >&2
  exit 1
}
printf '%s' "$padded" > "$lease_file.new"
chmod 600 "$lease_file.new"
mv "$lease_file.new" "$lease_file"
```

Every step of that shape is load-bearing. `printf '%020d'` converts through a
signed integer and silently truncates any epoch above `i64::MAX`. `printf '%020s'`
leaves the `0` flag undefined; coreutils `printf` rejects it and writes nothing,
which empties the target under direct redirection. An unvalidated `$epoch` writes
a valid epoch zero when it is unset, which is the rollback this procedure exists
to avoid. `case` matches the variable as one value; `grep` would accept a
multi-line `$epoch` whose individual lines are digits, and the padded result then
carries a newline that `read_epoch` rejects, leaving the store unusable until
another repair. The digit check alone admits 20-digit values above `u64::MAX`, which
`read_epoch` rejects, so the procedure would replace a lease no consumer can
acquire with another one; both operands are padded to 20 bytes, which makes the
`sort` comparison numeric without involving any shell integer type. Writing to
`$lease_file.new` and renaming keeps the live file intact until a full 20 digits
exist. Restart only upgraded consumers.

## Crates

| Crate | Description |
|-------|-------------|
| [`cortexkit-paths`](crates/cortexkit-paths) | Path canonicalization → canonical project-root identity (`ProjectRootId`). Dependency-free, `#![forbid(unsafe_code)]`, cross-platform (incl. Windows verbatim/UNC/drive-case normalization). **Its canonical form is a cryptographic identity input** — the vault hashes it to derive the keychain service name holding its master key and the vault id fencing admin MACs. A canonicalization change breaks those and presents as a locked vault over an intact store, never as a path mismatch. The name reads as a path helper; it is not only that. |

## License

MIT
