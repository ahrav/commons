//! Postgres backend mechanics for CortexKit module storage: open a per-module
//! postgres database from a [`StorageDescriptor`], guard it with a native
//! single-writer lease, and apply versioned migrations once.
//!
//! Database and role provisioning occurs outside the crate.
//!
//! ## Single-writer lease
//!
//! PostgreSQL releases its session advisory lock when the connection closes.

use cortexkit_lease::{fnv1a, LeaseError, LeaseKey};
use cortexkit_store_types::{StorageBackend, StorageDescriptor};
use postgres::{Client, NoTls};

pub use cortexkit_store_types::{Isolation, Migration, StorageBackend as Backend};

#[derive(Debug)]
pub enum StoreError {
    Lease(LeaseError),
    UnsupportedBackend(String),
    Migration(String),
    Backend(String),
    /// The persisted database epoch belongs to a newer lease holder.
    Fenced {
        holder_epoch: i64,
        db_epoch: i64,
    },
    /// The persisted epoch cannot be reconciled with this holder's: it is negative, or
    /// it is below the epoch `open_postgres` stamped, which no later write lowers.
    /// Authorizing the write would also authorize a superseded writer whose epoch
    /// exceeds the regressed value, so the store refuses until an operator resets
    /// `cortexkit_lease.epoch`.
    FenceCorrupt {
        db_epoch: i64,
    },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Lease(e) => write!(f, "storage lease: {e}"),
            StoreError::UnsupportedBackend(b) => write!(
                f,
                "storage backend '{b}' is not a postgres descriptor for this backend"
            ),
            StoreError::Migration(m) => write!(f, "migration: {m}"),
            StoreError::Backend(m) => write!(f, "storage backend: {m}"),
            StoreError::Fenced {
                holder_epoch,
                db_epoch,
            } => write!(
                f,
                "fenced write rejected: this writer holds epoch {holder_epoch} but the \
                 database was claimed by a newer writer at epoch {db_epoch}"
            ),
            StoreError::FenceCorrupt { db_epoch } => write!(
                f,
                "database fence epoch {db_epoch} cannot be reconciled with this writer; \
                 reset cortexkit_lease.epoch to at least the highest epoch a writer has used"
            ),
        }
    }
}

impl std::error::Error for StoreError {}

/// A 64-bit advisory-lock key derived from the namespaced lease identity. postgres
/// `pg_advisory_lock` takes a bigint; we hash the `(module_id, backend, namespace)`
/// identity into one so distinct modules/namespaces map to distinct locks.
fn advisory_key(key: &LeaseKey) -> i64 {
    fnv1a(&key.identity()) as i64
}

fn lease_key(descriptor: &StorageDescriptor) -> LeaseKey {
    LeaseKey::new(
        &descriptor.module_id,
        descriptor.backend.label(),
        &descriptor.storage_namespace,
    )
}

/// A PostgreSQL store that holds a session advisory lock until its connection
/// closes.
pub struct PostgresStore {
    client: std::sync::Mutex<Client>,
    epoch: i64,
    lease_key: i64,
}

impl PostgresStore {
    /// The fence epoch of the held lease (strictly greater than any superseded
    /// writer's), available for a distributed write-path compare-and-set.
    pub fn epoch(&self) -> i64 {
        self.epoch
    }

    /// The transaction rejects writes but otherwise uses the server's configured isolation level.
    ///
    /// The enclosing `START TRANSACTION READ ONLY` and `COMMIT` cost two round trips
    /// beyond the callback's own statements, and the callback's locks and snapshot
    /// persist until it returns. Callbacks therefore keep non-database work out and
    /// send a single read through [`Self::with_client_unfenced`] instead.
    ///
    /// Callbacks must not send transaction-control SQL.
    /// Ending the read-only transaction inside the callback fails the store operation.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Backend`] when transaction setup, the callback, or commit fails, or when the callback ended the transaction.
    pub fn with_client_read<T>(
        &self,
        f: impl FnOnce(&mut postgres::Transaction<'_>) -> Result<T, postgres::Error>,
    ) -> Result<T, StoreError> {
        let mut guard = self.client.lock().unwrap_or_else(|p| p.into_inner());
        let mut tx = guard
            .build_transaction()
            .read_only(true)
            .start()
            .map_err(backend_error)?;
        let out = f(&mut tx).map_err(backend_error)?;
        require_read_only_transaction(&mut tx)?;
        tx.commit().map_err(backend_error)?;
        Ok(out)
    }

    /// A newer persisted epoch rejects the callback before domain effects begin.
    /// The epoch check and callback effects share one transaction.
    ///
    /// Callbacks must not send transaction-control SQL.
    /// Ending the fence-checked transaction inside the callback fails the store operation.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Fenced`] when a newer writer owns the database.
    /// Returns [`StoreError::FenceCorrupt`] when the persisted epoch is negative or below this holder's.
    /// Returns [`StoreError::Backend`] when transaction setup, the callback, commit, or a backend fence-check operation fails, or when the callback ended the transaction.
    pub fn with_client_fenced<T>(
        &self,
        f: impl FnOnce(&mut postgres::Transaction<'_>) -> Result<T, postgres::Error>,
    ) -> Result<T, StoreError> {
        let mut guard = self.client.lock().unwrap_or_else(|p| p.into_inner());
        let mut tx = guard.transaction().map_err(backend_error)?;
        let witness = check_fence(&mut tx, self.lease_key, self.epoch)?;
        let out = f(&mut tx).map_err(backend_error)?;
        require_same_transaction(&mut tx, &witness)?;
        tx.commit().map_err(backend_error)?;
        Ok(out)
    }

    /// Runs the callback in autocommit, with no transaction and no fence check.
    ///
    /// PostgreSQL forbids `VACUUM` and the `CONCURRENTLY` index forms inside a
    /// transaction block, which puts them out of reach of
    /// [`Self::with_client_read`] and [`Self::with_client_fenced`]. Maintenance
    /// statements reach the lease-holding connection through this method rather than
    /// a second connection outside the lease. Callers must not perform
    /// fence-protected durable mutations here; PostgreSQL does not enforce that
    /// restriction.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Backend`] when the callback fails.
    pub fn with_client_unfenced<T>(
        &self,
        f: impl FnOnce(&mut Client) -> Result<T, postgres::Error>,
    ) -> Result<T, StoreError> {
        let mut guard = self.client.lock().unwrap_or_else(|p| p.into_inner());
        f(&mut guard).map_err(backend_error)
    }

    /// Apply a `namespace`'s migration chain to this database, once. Applied
    /// migrations are tracked per `(namespace, version)`, so a multi-domain module
    /// registers an independent chain per domain.
    /// Migration SQL runs in the transaction that accepts the holder epoch through
    /// `check_fence`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Fenced`] when a newer writer owns the database.
    /// Returns [`StoreError::Backend`] when the fence check fails.
    /// Returns [`StoreError::Migration`] when migration setup, SQL, recording, or commit fails.
    pub fn migrate(&self, namespace: &str, migrations: &[Migration]) -> Result<(), StoreError> {
        let mut guard = self.client.lock().unwrap_or_else(|p| p.into_inner());
        run_migrations(
            &mut guard,
            self.lease_key,
            self.epoch,
            namespace,
            migrations,
        )
    }
}

/// Preserves PostgreSQL's server message instead of collapsing it to `db error`.
/// A server primary message can quote offending column values.
fn db_message(error: &postgres::Error) -> String {
    error.as_db_error().map_or_else(
        || error.to_string(),
        |db| format!("SQLSTATE {}: {}", db.code().code(), db.message()),
    )
}

fn backend_error(error: postgres::Error) -> StoreError {
    StoreError::Backend(db_message(&error))
}

fn migration_error(error: postgres::Error) -> StoreError {
    StoreError::Migration(db_message(&error))
}

/// Locks the current lease row so the fence check remains bound to transaction effects.
fn check_fence(
    tx: &mut postgres::Transaction<'_>,
    lease_key: i64,
    holder_epoch: i64,
) -> Result<TransactionWitness, StoreError> {
    let row = tx
        .query_one(
            "SELECT epoch, pg_current_xact_id()::text FROM cortexkit_lease \
             WHERE lease_key = $1 FOR UPDATE",
            &[&lease_key],
        )
        .map_err(backend_error)?;
    let db_epoch: i64 = row.get(0);
    require_nonnegative_epoch(db_epoch)?;
    require_nonnegative_epoch(holder_epoch)?;
    if db_epoch > holder_epoch {
        return Err(StoreError::Fenced {
            holder_epoch,
            db_epoch,
        });
    }
    // `bump_epoch` stamps the holder's epoch at open and no later write lowers it, so a
    // stored value below it means the row regressed. Accepting it would let a superseded
    // writer whose epoch is also above the regressed value commit after handover.
    if db_epoch < holder_epoch {
        return Err(StoreError::FenceCorrupt { db_epoch });
    }
    Ok(TransactionWitness {
        xact_id: row.get(1),
        lease_key,
        epoch: db_epoch,
    })
}

/// Negative epochs compare older than every positive holder epoch, bypassing fencing.
fn require_nonnegative_epoch(db_epoch: i64) -> Result<(), StoreError> {
    if db_epoch < 0 {
        return Err(StoreError::FenceCorrupt { db_epoch });
    }
    Ok(())
}

/// The transaction id assigned when the fence row was locked, and the row it locked.
struct TransactionWitness {
    xact_id: String,
    lease_key: i64,
    epoch: i64,
}

/// Transaction-control SQL sent by the callback ends the fence-checked transaction.
/// Later statements commit in autocommit without a fence, and `Transaction::commit`
/// still reports success because `COMMIT` outside a transaction block is a warning.
fn require_same_transaction(
    tx: &mut postgres::Transaction<'_>,
    witness: &TransactionWitness,
) -> Result<(), StoreError> {
    let row = tx
        .query_one(
            "SELECT pg_current_xact_id_if_assigned()::text, \
                    (SELECT epoch FROM cortexkit_lease WHERE lease_key = $1)",
            &[&witness.lease_key],
        )
        .map_err(backend_error)?;
    let current: Option<String> = row.get(0);
    if current.as_deref() != Some(witness.xact_id.as_str()) {
        return Err(StoreError::Backend(
            "the callback ended the fence-checked transaction; effects after that commit ran \
             unfenced"
                .to_string(),
        ));
    }
    // Nothing reserves cortexkit_lease against callback SQL, and a deleted or lowered
    // row would let a later open reissue an epoch a stale writer still holds.
    let epoch: Option<i64> = row.get(1);
    if epoch != Some(witness.epoch) {
        return Err(StoreError::Backend(format!(
            "the callback changed the lease row this write was fenced against: epoch was \
             {} and is now {}",
            witness.epoch,
            epoch.map_or_else(|| "absent".to_string(), |e| e.to_string())
        )));
    }
    Ok(())
}

/// `transaction_read_only` reverts to the session default once the started transaction
/// ends, so an implicit read-write transaction reports `off` here.
fn require_read_only_transaction(tx: &mut postgres::Transaction<'_>) -> Result<(), StoreError> {
    let read_only: String = tx
        .query_one("SHOW transaction_read_only", &[])
        .map_err(backend_error)?
        .get(0);
    if read_only == "on" {
        return Ok(());
    }
    Err(StoreError::Backend(
        "the read-only transaction is no longer read-only; the callback ended it or set \
         READ WRITE, so its statements ran unfenced"
            .to_string(),
    ))
}

impl Drop for PostgresStore {
    fn drop(&mut self) {
        // Best-effort explicit unlock; the server also releases the session lock
        // when the connection closes on drop.
        if let Ok(mut guard) = self.client.lock() {
            let _ = guard.execute("SELECT pg_advisory_unlock($1)", &[&self.lease_key]);
        }
    }
}

/// Open a module's postgres store from its descriptor: connect with the scoped
/// DSN, acquire the native single-writer advisory lock, and bump the persisted
/// epoch. Migrations are applied separately via [`PostgresStore::migrate`].
///
/// A second live writer is rejected (`StoreError::Lease`) because the advisory
/// lock is already held by the first connection.
///
/// # Errors
///
/// Returns [`StoreError::UnsupportedBackend`] for non-PostgreSQL descriptors.
/// Returns [`StoreError::Backend`] when connection or advisory-lock queries fail.
/// Returns [`StoreError::Lease`] when another writer holds the advisory lock.
/// Returns [`StoreError::Migration`] when infrastructure setup or epoch issuance fails.
pub fn open_postgres(descriptor: &StorageDescriptor) -> Result<PostgresStore, StoreError> {
    let dsn = match &descriptor.backend {
        StorageBackend::Postgres { dsn, .. } => dsn.clone(),
        other => return Err(StoreError::UnsupportedBackend(other.label().to_string())),
    };

    let mut client = Client::connect(&dsn, NoTls).map_err(backend_error)?;

    let lease_id = advisory_key(&lease_key(descriptor));

    let acquired: bool = client
        .query_one("SELECT pg_try_advisory_lock($1)", &[&lease_id])
        .map_err(backend_error)?
        .get(0);
    if !acquired {
        return Err(StoreError::Lease(LeaseError::Held {
            key: lease_key(descriptor),
        }));
    }

    let epoch =
        match ensure_infra_tables(&mut client).and_then(|()| bump_epoch(&mut client, lease_id)) {
            Ok(epoch) => epoch,
            Err(e) => {
                let _ = client.execute("SELECT pg_advisory_unlock($1)", &[&lease_id]);
                return Err(e);
            }
        };

    Ok(PostgresStore {
        client: std::sync::Mutex::new(client),
        epoch,
        lease_key: lease_id,
    })
}

/// A fixed advisory-lock key serializing creation of the SHARED infra tables (the
/// lease + schema-version tables), which every open touches regardless of module
/// or namespace. Postgres `CREATE TABLE IF NOT EXISTS` is NOT concurrency-safe:
/// two concurrent opens on a fresh database both pass the existence check and one
/// errors on the `pg_class` unique index. So the shared bootstrap DDL runs under
/// this one fixed transaction-scoped lock. It is distinct from the
/// per-(module, namespace) single-writer lock, which does not cover the shared
/// tables.
const INFRA_BOOTSTRAP_LOCK: i64 = 0x636b_5f69_6e66_7261;

/// Create the shared infra tables (lease + schema-version) race-safely. All
/// concurrent opens serialize on the fixed bootstrap lock so the
/// not-concurrency-safe `CREATE TABLE IF NOT EXISTS` runs one at a time; the lock
/// releases when this transaction commits.
fn ensure_infra_tables(client: &mut Client) -> Result<(), StoreError> {
    let mut tx = client.transaction().map_err(migration_error)?;
    tx.execute("SELECT pg_advisory_xact_lock($1)", &[&INFRA_BOOTSTRAP_LOCK])
        .map_err(migration_error)?;
    tx.batch_execute(
        "CREATE TABLE IF NOT EXISTS cortexkit_lease (\
             lease_key BIGINT PRIMARY KEY, \
             epoch BIGINT NOT NULL CONSTRAINT cortexkit_lease_epoch_nonnegative \
                 CHECK (epoch >= 0)\
         );\
         CREATE TABLE IF NOT EXISTS cortexkit_schema_version (\
             namespace TEXT NOT NULL, \
             version INTEGER NOT NULL, \
             applied_at_unix BIGINT NOT NULL, \
             PRIMARY KEY (namespace, version)\
         );",
    )
    .map_err(migration_error)?;
    // `CREATE TABLE IF NOT EXISTS` does not add constraints to existing lease tables.
    tx.batch_execute(
        "DO $$ BEGIN \
             ALTER TABLE cortexkit_lease ADD CONSTRAINT cortexkit_lease_epoch_nonnegative \
                 CHECK (epoch >= 0); \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$;",
    )
    .map_err(migration_error)?;
    tx.commit().map_err(migration_error)?;
    Ok(())
}

/// Persist + increment the monotonic epoch fence in the module's own database,
/// under the held advisory lock. The lease table is created by
/// [`ensure_infra_tables`] before this runs.
fn bump_epoch(client: &mut Client, lease_id: i64) -> Result<i64, StoreError> {
    let mut tx = client.transaction().map_err(migration_error)?;
    let previous: i64 = tx
        .query_one(
            "SELECT COALESCE((SELECT epoch FROM cortexkit_lease WHERE lease_key = $1 \
             FOR UPDATE), 0)",
            &[&lease_id],
        )
        .map_err(migration_error)?
        .get(0);
    let epoch: i64 = tx
        .query_one(
            "INSERT INTO cortexkit_lease (lease_key, epoch) VALUES ($1, 1) \
             ON CONFLICT (lease_key) DO UPDATE SET epoch = cortexkit_lease.epoch + 1 \
             RETURNING epoch",
            &[&lease_id],
        )
        .map_err(migration_error)?
        .get(0);
    require_nonnegative_epoch(epoch)?;
    require_epoch_advanced(previous, epoch)?;
    // `RETURNING` yields the tuple the statement produced, which an `AFTER UPDATE` trigger
    // can still overwrite. Re-reading the committed row is the only value a later
    // `check_fence` will compare against.
    let stored: i64 = tx
        .query_one(
            "SELECT epoch FROM cortexkit_lease WHERE lease_key = $1 FOR UPDATE",
            &[&lease_id],
        )
        .map_err(migration_error)?
        .get(0);
    if stored != epoch {
        return Err(StoreError::Migration(format!(
            "lease epoch {epoch} was issued but the row stores {stored}; a rule or trigger \
             on cortexkit_lease can rewrite the epoch after the increment"
        )));
    }
    tx.commit().map_err(migration_error)?;
    Ok(epoch)
}

/// A rule or `BEFORE UPDATE` trigger on the lease table can return the old row, so the
/// issuing statement reports success while handing out an epoch a live writer still holds.
/// Two holders would then share an epoch and both pass `check_fence`.
fn require_epoch_advanced(previous: i64, issued: i64) -> Result<(), StoreError> {
    if issued <= previous {
        return Err(StoreError::Migration(format!(
            "lease epoch did not advance: the row held {previous} and issued {issued}; \
             a rule or trigger on cortexkit_lease can suppress the increment"
        )));
    }
    Ok(())
}

/// Apply un-applied migrations for one `namespace` in ascending version order,
/// each in its own transaction with its version record, so a crash mid-migration
/// leaves it un-recorded and it re-runs cleanly. Keyed by `(namespace, version)`.
fn run_migrations(
    client: &mut Client,
    lease_key: i64,
    holder_epoch: i64,
    namespace: &str,
    migrations: &[Migration],
) -> Result<(), StoreError> {
    // The schema-version table is bootstrapped race-safely in ensure_infra_tables
    // at open, so migrate() does not (re-)create it here.
    let mut tx = client.transaction().map_err(migration_error)?;
    let _witness = check_fence(&mut tx, lease_key, holder_epoch)?;
    let current: i32 = tx
        .query_one(
            "SELECT COALESCE(MAX(version), 0) FROM cortexkit_schema_version WHERE namespace = $1",
            &[&namespace],
        )
        .map_err(migration_error)?
        .get(0);
    tx.commit().map_err(migration_error)?;
    let current = current as u32;

    let mut ordered: Vec<&Migration> = migrations.iter().collect();
    ordered.sort_by_key(|m| m.version);

    for m in ordered {
        if m.version <= current {
            continue;
        }
        let mut tx = client.transaction().map_err(migration_error)?;
        let witness = check_fence(&mut tx, lease_key, holder_epoch)?;
        tx.batch_execute(m.statements).map_err(|e| {
            StoreError::Migration(format!(
                "namespace '{namespace}' migration {}: {}",
                m.version,
                db_message(&e)
            ))
        })?;
        require_same_transaction(&mut tx, &witness).map_err(|e| {
            StoreError::Migration(format!(
                "namespace '{namespace}' migration {}: {e}",
                m.version
            ))
        })?;
        tx.execute(
            "INSERT INTO cortexkit_schema_version (namespace, version, applied_at_unix) \
             VALUES ($1, $2, $3)",
            &[&namespace, &(m.version as i32), &now_unix()],
        )
        .map_err(migration_error)?;
        tx.commit().map_err(migration_error)?;
    }
    Ok(())
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Live postgres tests. Require a reachable postgres and a DSN in
/// `CORTEXKIT_TEST_PG_DSN`; skipped (pass) when unset so the default `cargo test`
/// stays green without a database. CI provides a postgres service + the env var.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn test_dsn() -> Option<String> {
        match std::env::var("CORTEXKIT_TEST_PG_DSN") {
            Ok(dsn) => Some(dsn),
            // CORTEXKIT_REQUIRE_PG turns a missing DSN into a test failure;
            // when unset, a missing DSN skips the live tests.
            Err(_) => {
                assert!(
                    std::env::var("CORTEXKIT_REQUIRE_PG").is_err(),
                    "CORTEXKIT_REQUIRE_PG is set but CORTEXKIT_TEST_PG_DSN is missing: the live \
                     postgres tests must run in this job, not skip-pass"
                );
                None
            }
        }
    }

    fn descriptor(dsn: &str, namespace: &str) -> StorageDescriptor {
        StorageDescriptor {
            module_id: "test-module".into(),
            storage_namespace: namespace.into(),
            isolation: Isolation::Module,
            backend: StorageBackend::Postgres {
                dsn: dsn.into(),
                database: "test".into(),
            },
        }
    }

    // A unique namespace per test run so parallel tests + repeat runs against one
    // shared test database never collide on lease keys or migration tables.
    fn unique_ns(tag: &str) -> String {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{tag}_{}_{t}_{n}", std::process::id())
    }

    const M1: &[Migration] = &[Migration {
        version: 1,
        statements: "SELECT 1;",
    }];

    #[test]
    fn open_migrate_and_single_writer() {
        let Some(dsn) = test_dsn() else {
            eprintln!("CORTEXKIT_TEST_PG_DSN unset; skipping live postgres test");
            return;
        };
        let ns = unique_ns("sw");
        let d = descriptor(&dsn, &ns);

        let store = open_postgres(&d).expect("open");
        store.migrate(&ns, M1).expect("migrate");
        assert!(store.epoch() >= 1);

        match open_postgres(&d) {
            Err(StoreError::Lease(_)) => {}
            Err(e) => panic!("expected Lease(Held), got {e}"),
            Ok(_) => panic!("expected Lease(Held), got a second open"),
        }

        let applied: i64 = store
            .with_client_read(|c| {
                Ok(c.query_one(
                    "SELECT COUNT(*) FROM cortexkit_schema_version WHERE namespace = $1",
                    &[&ns],
                )?
                .get(0))
            })
            .expect("schema version query");
        assert_eq!(
            applied, 1,
            "exactly one migration recorded for the namespace"
        );
    }

    #[test]
    fn a_callback_that_ends_the_transaction_is_rejected() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("endtx");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        let table = format!("endtx_{ns}");
        store
            .with_client_unfenced(|c| c.batch_execute(&format!("CREATE TABLE {table} (v int)")))
            .expect("create");

        let escaped = store.with_client_fenced(|tx| {
            tx.batch_execute(&format!("COMMIT; INSERT INTO {table} (v) VALUES (1)"))?;
            Ok(())
        });
        assert!(
            matches!(&escaped, Err(StoreError::Backend(m)) if m.contains("ended the fence-checked transaction")),
            "ending the transaction is reported instead of a successful fenced write, got {escaped:?}"
        );

        let migration = &[Migration {
            version: 1,
            statements: "COMMIT; SELECT 1;",
        }];
        let migrated = store.migrate(&ns, migration);
        assert!(
            matches!(&migrated, Err(StoreError::Migration(m)) if m.contains("ended the fence-checked transaction")),
            "a migration that ends its transaction is rejected, got {migrated:?}"
        );
        let recorded: i64 = store
            .with_client_read(|c| {
                Ok(c.query_one(
                    "SELECT COUNT(*) FROM cortexkit_schema_version WHERE namespace = $1",
                    &[&ns],
                )?
                .get(0))
            })
            .expect("count");
        assert_eq!(recorded, 0, "the rejected migration recorded no version");

        let _ = store.with_client_unfenced(|c| c.batch_execute(&format!("DROP TABLE {table}")));
    }

    #[test]
    fn a_read_callback_cannot_escape_read_only_mode() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("endread");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        let table = format!("endread_{ns}");
        store
            .with_client_unfenced(|c| c.batch_execute(&format!("CREATE TABLE {table} (v int)")))
            .expect("create");

        let escaped = store.with_client_read(|tx| {
            tx.batch_execute(&format!("COMMIT; INSERT INTO {table} (v) VALUES (1)"))?;
            Ok(())
        });
        assert!(
            matches!(&escaped, Err(StoreError::Backend(m)) if m.contains("no longer read-only")),
            "the read API reports the escape instead of succeeding, got {escaped:?}"
        );

        let switched = store.with_client_read(|tx| {
            tx.batch_execute("SET TRANSACTION READ WRITE")?;
            tx.batch_execute(&format!("INSERT INTO {table} (v) VALUES (2)"))?;
            Ok(())
        });
        assert!(
            matches!(&switched, Err(StoreError::Backend(m)) if m.contains("no longer read-only")),
            "switching the access mode without ending the transaction is rejected, got {switched:?}"
        );

        let escaped_rows: i64 = store
            .with_client_unfenced(|c| {
                Ok(
                    c.query_one(&format!("SELECT COUNT(*) FROM {table} WHERE v = 1"), &[])?
                        .get(0),
                )
            })
            .expect("count");
        assert_eq!(
            escaped_rows, 1,
            "ending the transaction autocommits the write before the check runs, so \
             rejection reports it rather than undoing it"
        );
        let switched_rows: i64 = store
            .with_client_unfenced(|c| {
                Ok(
                    c.query_one(&format!("SELECT COUNT(*) FROM {table} WHERE v = 2"), &[])?
                        .get(0),
                )
            })
            .expect("count");
        assert_eq!(
            switched_rows, 0,
            "switching the access mode leaves the transaction open, so refusing to commit \
             rolls the write back"
        );

        let _ = store.with_client_unfenced(|c| c.batch_execute(&format!("DROP TABLE {table}")));
    }

    #[test]
    fn a_negative_epoch_fails_closed() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("negepoch");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        let key = store.lease_key;

        let persisted = store.with_client_unfenced(|c| {
            c.execute(
                "UPDATE cortexkit_lease SET epoch = -5 WHERE lease_key = $1",
                &[&key],
            )
        });
        assert!(
            matches!(&persisted, Err(StoreError::Backend(m)) if m.contains("cortexkit_lease_epoch_nonnegative")),
            "the table constraint rejects a negative epoch, got {persisted:?}"
        );

        let corrupt = PostgresStore {
            client: std::sync::Mutex::new(Client::connect(&dsn, NoTls).expect("client")),
            epoch: -5,
            lease_key: key,
        };
        let fenced = corrupt.with_client_fenced(|_| Ok(()));
        assert!(
            matches!(fenced, Err(StoreError::FenceCorrupt { db_epoch: -5 })),
            "a negative epoch fails closed rather than authorizing the write, got {fenced:?}"
        );
    }

    #[test]
    fn a_regressed_positive_epoch_fails_closed() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("regress");
        let d = descriptor(&dsn, &ns);
        // Reopen twice so the stamped epoch is above 1 and a lower positive value exists.
        drop(open_postgres(&d).expect("open"));
        drop(open_postgres(&d).expect("reopen"));
        let store = open_postgres(&d).expect("reopen again");
        let key = store.lease_key;
        let stamped = store.epoch();
        assert!(
            stamped >= 3,
            "expected a stamped epoch above 1, got {stamped}"
        );

        store
            .with_client_unfenced(|c| {
                c.execute(
                    "UPDATE cortexkit_lease SET epoch = 1 WHERE lease_key = $1",
                    &[&key],
                )
            })
            .expect("regress the row");

        let fenced = store.with_client_fenced(|_| Ok(()));
        assert!(
            matches!(fenced, Err(StoreError::FenceCorrupt { db_epoch: 1 })),
            "a positive epoch below the one stamped at open fails closed, got {fenced:?}"
        );

        let stale = PostgresStore {
            client: std::sync::Mutex::new(Client::connect(&dsn, NoTls).expect("client")),
            epoch: stamped - 1,
            lease_key: key,
        };
        let stale_write = stale.with_client_fenced(|_| Ok(()));
        assert!(
            matches!(stale_write, Err(StoreError::FenceCorrupt { db_epoch: 1 })),
            "a superseded writer above the regressed value is rejected too, got {stale_write:?}"
        );
    }

    #[test]
    fn a_callback_cannot_damage_the_lease_row_it_is_fenced_against() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("leaserow");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        let key = store.lease_key;
        let epoch = store.epoch();

        let deleted = store.with_client_fenced(|tx| {
            tx.execute("DELETE FROM cortexkit_lease WHERE lease_key = $1", &[&key])?;
            Ok(())
        });
        assert!(
            matches!(&deleted, Err(StoreError::Backend(m)) if m.contains("changed the lease row")),
            "deleting the fence row is rejected, got {deleted:?}"
        );

        let lowered = store.with_client_fenced(|tx| {
            tx.execute(
                "UPDATE cortexkit_lease SET epoch = 0 WHERE lease_key = $1",
                &[&key],
            )?;
            Ok(())
        });
        assert!(
            matches!(&lowered, Err(StoreError::Backend(m)) if m.contains("changed the lease row")),
            "lowering the fence row is rejected, got {lowered:?}"
        );

        let intact: i64 = store
            .with_client_read(|c| {
                Ok(c.query_one(
                    "SELECT epoch FROM cortexkit_lease WHERE lease_key = $1",
                    &[&key],
                )?
                .get(0))
            })
            .expect("read the lease row");
        assert_eq!(
            intact, epoch,
            "both rejected callbacks rolled back, so the fence row still holds this epoch"
        );
    }

    #[test]
    fn a_suppressed_epoch_increment_is_rejected() {
        // A rule or `BEFORE UPDATE` trigger on cortexkit_lease can return the old row, so
        // the issuing statement reports success while handing out an epoch a live writer
        // still holds. Installing one would break every concurrent test on the shared
        // table, so the rule is exercised directly.
        assert!(require_epoch_advanced(0, 1).is_ok(), "a fresh row issues 1");
        assert!(
            require_epoch_advanced(4, 5).is_ok(),
            "an increment advances"
        );
        for (previous, issued) in [(5_i64, 5_i64), (5, 4), (5, 0)] {
            let r = require_epoch_advanced(previous, issued);
            assert!(
                matches!(&r, Err(StoreError::Migration(m)) if m.contains("did not advance")),
                "issuing {issued} while the row held {previous} is rejected, got {r:?}"
            );
        }
    }

    #[test]
    fn open_verifies_the_stored_epoch_matches_the_issued_one() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("stored");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        let key = store.lease_key;
        let issued = store.epoch();

        let stored: i64 = store
            .with_client_read(|c| {
                Ok(c.query_one(
                    "SELECT epoch FROM cortexkit_lease WHERE lease_key = $1",
                    &[&key],
                )?
                .get(0))
            })
            .expect("read the row");
        assert_eq!(
            stored, issued,
            "open re-reads the committed row, so an AFTER trigger rewriting the epoch \
             cannot leave the issued value unbacked"
        );
    }

    #[test]
    fn unfenced_callback_runs_statements_a_transaction_forbids() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("maintenance");
        let store = open_postgres(&descriptor(&dsn, &ns)).expect("open");
        store.migrate(&ns, M1).expect("migrate");

        let blocked = store
            .with_client_fenced(|tx| tx.batch_execute("VACUUM cortexkit_schema_version"))
            .expect_err("a transaction block must reject VACUUM");
        assert!(
            matches!(blocked, StoreError::Backend(ref message) if message.starts_with("SQLSTATE 25001:")),
            "expected SQLSTATE 25001 inside a transaction, got {blocked:?}"
        );

        store
            .with_client_unfenced(|client| client.batch_execute("VACUUM cortexkit_schema_version"))
            .expect("autocommit maintenance reaches the lease-holding connection");
    }

    #[test]
    fn read_only_callback_rejects_mutation_without_rows() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("read_only");
        let store = open_postgres(&descriptor(&dsn, &ns)).expect("open");
        store.migrate(&ns, M1).expect("migrate");
        let read_only_error = store
            .with_client_read(|tx| {
                tx.execute(
                    "INSERT INTO cortexkit_schema_version \
                     (namespace, version, applied_at_unix) VALUES ($1, 1, 0)",
                    &[&format!("{ns}_probe")],
                )?;
                Ok(())
            })
            .expect_err("read-only callback mutated the database");
        assert!(
            matches!(read_only_error, StoreError::Backend(message) if message.starts_with("SQLSTATE 25006:")),
            "read-only mutation must preserve SQLSTATE 25006"
        );
        let rows: i64 = store
            .with_client_read(|tx| {
                Ok(tx
                    .query_one(
                        "SELECT COUNT(*) FROM cortexkit_schema_version WHERE namespace = $1",
                        &[&format!("{ns}_probe")],
                    )?
                    .get(0))
            })
            .expect("read unchanged rows");
        assert_eq!(rows, 0);
    }

    #[test]
    fn fenced_callback_error_rolls_back_rows() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("rollback");
        let store = open_postgres(&descriptor(&dsn, &ns)).expect("open");
        store.migrate(&ns, M1).expect("migrate");
        let callback_error = store
            .with_client_fenced(|tx| {
                tx.execute(
                    "INSERT INTO cortexkit_schema_version \
                     (namespace, version, applied_at_unix) VALUES ($1, 1, 0)",
                    &[&format!("{ns}_probe")],
                )?;
                tx.batch_execute("SELECT * FROM cortexkit_missing_table")?;
                Ok(())
            })
            .expect_err("callback error must abort the transaction");
        assert!(matches!(callback_error, StoreError::Backend(_)));
        let rows_after_error: i64 = store
            .with_client_read(|tx| {
                Ok(tx
                    .query_one(
                        "SELECT COUNT(*) FROM cortexkit_schema_version WHERE namespace = $1",
                        &[&format!("{ns}_probe")],
                    )?
                    .get(0))
            })
            .expect("rollback state");
        assert_eq!(
            rows_after_error, 0,
            "failed callback did not roll back its domain write"
        );
    }

    #[test]
    fn repeated_fenced_writes_at_current_epoch_succeed() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("repeat");
        let store = open_postgres(&descriptor(&dsn, &ns)).expect("open");
        store.migrate(&ns, M1).expect("migrate");
        for version in [1, 2] {
            store
                .with_client_fenced(|tx| {
                    tx.execute(
                        "INSERT INTO cortexkit_schema_version \
                         (namespace, version, applied_at_unix) VALUES ($1, $2, 0)",
                        &[&format!("{ns}_probe"), &version],
                    )?;
                    Ok(())
                })
                .expect("equal epoch write");
        }
    }

    #[test]
    fn superseded_writer_is_rejected_after_reopen() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("stale_write");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        let old_epoch = store.epoch();
        let lease_key = store.lease_key;
        drop(store);
        let reopened = open_postgres(&d).expect("reopen after release");
        assert!(
            reopened.epoch() > old_epoch,
            "epoch is monotonic across opens"
        );

        let stale = PostgresStore {
            client: std::sync::Mutex::new(Client::connect(&dsn, NoTls).expect("stale client")),
            epoch: old_epoch,
            lease_key,
        };
        match stale.with_client_fenced(|_| Ok(())) {
            Err(StoreError::Fenced {
                holder_epoch,
                db_epoch,
            }) => {
                assert_eq!(holder_epoch, old_epoch);
                assert_eq!(db_epoch, reopened.epoch());
            }
            other => panic!("expected stale writer rejection, got {other:?}"),
        }
    }

    #[test]
    fn superseded_writer_cannot_migrate() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("stale_migrate");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        let old_epoch = store.epoch();
        let lease_key = store.lease_key;
        drop(store);
        let reopened = open_postgres(&d).expect("reopen");
        let stale = PostgresStore {
            client: std::sync::Mutex::new(Client::connect(&dsn, NoTls).expect("stale client")),
            epoch: old_epoch,
            lease_key,
        };
        let table = format!("stale_schema_{ns}");
        let statements: &'static str =
            Box::leak(format!("CREATE TABLE {table} (id INT PRIMARY KEY);").into_boxed_str());
        let migrations = [Migration {
            version: 1,
            statements,
        }];
        assert!(matches!(
            stale.migrate(&format!("{ns}_migration"), &migrations),
            Err(StoreError::Fenced { .. })
        ));
        let exists: bool = reopened
            .with_client_read(|tx| {
                Ok(tx
                    .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])?
                    .get(0))
            })
            .expect("schema state");
        assert!(!exists);
    }

    #[test]
    fn independent_namespace_chains() {
        let Some(dsn) = test_dsn() else {
            return;
        };
        let ns = unique_ns("ns");
        let d = descriptor(&dsn, &ns);
        let store = open_postgres(&d).expect("open");
        const A: &[Migration] = &[Migration {
            version: 1,
            statements: "CREATE TABLE IF NOT EXISTS dom_a (id INT PRIMARY KEY);",
        }];
        const B: &[Migration] = &[Migration {
            version: 1,
            statements: "CREATE TABLE IF NOT EXISTS dom_b (id INT PRIMARY KEY);",
        }];
        store.migrate(&format!("{ns}_a"), A).expect("a");
        store
            .migrate(&format!("{ns}_b"), B)
            .expect("b - same version, distinct namespace");
        let count: i64 = store
            .with_client_read(|c| {
                Ok(c.query_one(
                    "SELECT COUNT(*) FROM cortexkit_schema_version WHERE namespace IN ($1, $2)",
                    &[&format!("{ns}_a"), &format!("{ns}_b")],
                )?
                .get(0))
            })
            .expect("count");
        assert_eq!(count, 2, "both namespace chains recorded independently");
    }

    /// The advisory key must be stable across versions: two builds deriving
    /// different keys for one store would take different locks and both write.
    #[test]
    fn advisory_key_derivation_is_stable() {
        let k = LeaseKey::new("test-module", "postgres", "main");
        assert_eq!(advisory_key(&k), -3_153_521_753_806_872_150_i64);
    }

    #[test]
    fn sqlite_descriptor_is_rejected() {
        let d = StorageDescriptor {
            module_id: "m".into(),
            storage_namespace: "n".into(),
            isolation: Isolation::Module,
            backend: StorageBackend::Sqlite {
                path: "/tmp/x.db".into(),
            },
        };
        match open_postgres(&d) {
            Err(StoreError::UnsupportedBackend(b)) => assert_eq!(b, "sqlite"),
            Err(e) => panic!("expected UnsupportedBackend, got {e}"),
            Ok(_) => panic!("expected UnsupportedBackend"),
        }
    }
}
