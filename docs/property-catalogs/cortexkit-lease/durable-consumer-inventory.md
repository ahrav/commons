# Durable store consumer inventory

This inventory records default-branch source receipts captured on 2026-09-01.
Commit-pinned links keep the evidence stable as consumer branches move.

| Consumer | Receipt | Finding |
|---|---|---|
| `cortexkit/claustrum` | [`crates/credentials-core/src/store.rs:572-580`](https://github.com/cortexkit/claustrum/blob/67a6f22067ee36a44b8cc12ceac8360debc65093/crates/credentials-core/src/store.rs#L572-L580) | `fenced_write` delegates durable writes to `with_conn_fenced`. Unfenced `with_conn` remains in the file for reads and connection setup. |
| `cortexkit/synapse` | [`crates/synapse-module/src/store.rs:1768-1794`](https://github.com/cortexkit/synapse/blob/912467bb6d681f29f8c7d56ee2d8ccfadeba57c0/crates/synapse-module/src/store.rs#L1768-L1794) | Open and migration use the SQLite store; the first read-modify-write shown uses `with_conn_fenced`. The file contains additional fenced writes and unfenced reads. |
| `cortexkit/magic-context` | [`crates/mc-store/src/lib.rs`](https://github.com/cortexkit/magic-context/blob/44ac1982223d9464fa8fb23cf5ff872b1e6f3ac3/crates/mc-store/src/lib.rs) | The store has many uses of both callback APIs. Unfenced durable mutations include the delete at [6523-6530](https://github.com/cortexkit/magic-context/blob/44ac1982223d9464fa8fb23cf5ff872b1e6f3ac3/crates/mc-store/src/lib.rs#L6523-L6530) and inserts at [7479-7617](https://github.com/cortexkit/magic-context/blob/44ac1982223d9464fa8fb23cf5ff872b1e6f3ac3/crates/mc-store/src/lib.rs#L7479-L7617). Fenced paths include [6617](https://github.com/cortexkit/magic-context/blob/44ac1982223d9464fa8fb23cf5ff872b1e6f3ac3/crates/mc-store/src/lib.rs#L6617) and [7018](https://github.com/cortexkit/magic-context/blob/44ac1982223d9464fa8fb23cf5ff872b1e6f3ac3/crates/mc-store/src/lib.rs#L7018). Removing unrestricted SQLite access would break this default branch. |
| PostgreSQL consumers | [GitHub organization code search](https://github.com/search?q=org%3Acortexkit+PostgresStore&type=code) | No downstream `PostgresStore` or `with_client` use was found. Version 0.3.0 replaces the unrestricted callback with read-only and fenced transaction APIs without a known consumer migration. |
| `broca`, `broca-tagref` | Repository lookup returned HTTP 404 | Source was unavailable, so this inventory makes no claim about either repository. |

Two external blockers remain. Magic Context has durable mutations through
unfenced `with_conn`; removing that compatibility surface and defining the
complete protected write set require a companion consumer migration.
Claustrum has no supplied receipt for its real-daemon two-process review. The
backend-fencing PR remains draft until both blockers are resolved.
