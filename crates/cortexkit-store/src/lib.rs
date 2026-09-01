//! Backend mechanics for CortexKit module storage: open a database from a
//! [`StorageDescriptor`], guard it with the single-writer lease, and apply
//! versioned migrations once.
//!
//! Modules pass a resolved descriptor and ordered migrations here, then run domain
//! queries against the lease-guarded, migrated connection. Backends are
//! feature-gated, so module code does not branch on the descriptor's backend. Each
//! module owns its store trait, migrations, and queries.
//!
//! The single-writer lease ([`cortexkit_lease`]) is keyed by
//! `(module_id, backend, storage_namespace)`, preventing collisions between stores
//! that share a lease root. The persisted epoch serves as the fence token for
//! epoch-checked writes.

pub use cortexkit_store_types::{
    postgres_database_name, sqlite_store_path, Isolation, Migration, StorageBackend,
    StorageDescriptor,
};

use cortexkit_lease::LeaseError;
#[cfg(feature = "sqlite")]
use cortexkit_lease::LeaseKey;

#[derive(Debug)]
pub enum StoreError {
    /// A conflicting live holder prevented acquisition, or lease I/O failed.
    Lease(LeaseError),
    /// The descriptor asked for a backend this build was not compiled with.
    UnsupportedBackend(String),
    /// A migration or schema-version operation failed.
    Migration(String),
    /// A backend (database driver) operation failed.
    Backend(String),
    /// An io failure preparing the store location.
    Io(std::io::Error),
    /// A fenced (epoch-checked) write was rejected because the database has already
    /// been claimed by a newer writer. `db_epoch` (the epoch stamped in the
    /// database) is greater than `holder_epoch` (this store's lease epoch), so this
    /// writer has been superseded — for example a draining old instance attempting a
    /// late write after a replacement took the lease. The write was not applied.
    Fenced { holder_epoch: u64, db_epoch: u64 },
    /// An out-of-range database epoch prevents proving monotonic fencing. The store
    /// refuses to open until an operator resets `cortexkit_fence.epoch`.
    FenceCorrupt { db_epoch: i64 },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Lease(e) => write!(f, "storage lease: {e}"),
            StoreError::UnsupportedBackend(b) => write!(
                f,
                "storage backend '{b}' is not supported by this build (missing feature)"
            ),
            StoreError::Migration(m) => write!(f, "migration: {m}"),
            StoreError::Backend(m) => write!(f, "storage backend: {m}"),
            StoreError::Io(e) => write!(f, "storage io: {e}"),
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
                "database fence epoch {db_epoch} is outside the supported range; reset \
                 cortexkit_fence.epoch to at least the highest epoch a writer has used"
            ),
        }
    }
}

impl std::error::Error for StoreError {}

/// The lease key includes the module, backend, and storage namespace so stores
/// sharing a lease root cannot collide.
#[cfg(feature = "sqlite")]
fn lease_key(descriptor: &StorageDescriptor) -> LeaseKey {
    LeaseKey::new(
        &descriptor.module_id,
        descriptor.backend.label(),
        &descriptor.storage_namespace,
    )
}

#[cfg(feature = "sqlite")]
mod sqlite_backend {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        sync::Mutex,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use cortexkit_lease::{protect_file, FileLeaseStore, LeaseHandle};
    use rusqlite::{Connection, OpenFlags};

    /// A lease-guarded SQLite store. The lease remains held for the store's lifetime.
    /// A single mutexed connection preserves connection-local configuration and transaction scope.
    /// [`open_sqlite`] claims the database lease epoch before returning the store.
    pub struct SqliteStore {
        conn: Mutex<Connection>,
        epoch: u64,
        // The held lease releases on drop; kept alive for the store's lifetime.
        _lease: Box<dyn LeaseHandle>,
    }

    impl SqliteStore {
        pub fn epoch(&self) -> u64 {
            self.epoch
        }

        /// Construct a store over an open connection without acquiring a lease.
        ///
        /// Tests use this to model stale and replacement connections at different
        /// epochs, a state the OS lock prevents constructing through `open_sqlite`.
        #[cfg(test)]
        pub(crate) fn for_test(conn: Connection, epoch: u64) -> Self {
            #[derive(Debug)]
            struct NoLease(cortexkit_lease::LeaseKey);
            impl LeaseHandle for NoLease {
                fn epoch(&self) -> u64 {
                    0
                }
                fn key(&self) -> &cortexkit_lease::LeaseKey {
                    &self.0
                }
            }
            SqliteStore {
                conn: Mutex::new(conn),
                epoch,
                _lease: Box::new(NoLease(cortexkit_lease::LeaseKey::new(
                    "test", "sqlite", "test",
                ))),
            }
        }

        /// `with_conn` permits read-only queries and connection-local configuration.
        /// `PRAGMA query_only` makes database writes fail with `SQLITE_READONLY`,
        /// which keeps every durable write on the fenced path
        /// ([`Self::with_conn_fenced`]). An authorizer denies setting `query_only`,
        /// `synchronous`, `journal_mode`, `locking_mode`, and `writable_schema`, so the
        /// callback cannot lift the guard, weaken fence durability, or take a lock that
        /// blocks a replacement's fence-floor read. Matching ignores case, and reading
        /// those pragmas stays allowed.
        ///
        /// # Errors
        ///
        /// Returns [`StoreError::Backend`] if the callback returns an error,
        /// attempts a write, sets a denied pragma, or if installing or clearing the
        /// guard fails.
        pub fn with_conn<T>(
            &self,
            f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
        ) -> Result<T, StoreError> {
            let guard = self.conn.lock().unwrap_or_else(|p| p.into_inner());
            let read_only = QueryOnlyGuard::enable(&guard)?;
            let out = f(&guard);
            let restored = read_only.restore();
            let out = out.map_err(|e| StoreError::Backend(e.to_string()))?;
            restored?;
            Ok(out)
        }

        /// [`Self::with_conn`]'s read-only guard rejects `VACUUM` as a write,
        /// and SQLite rejects it inside [`Self::with_conn_fenced`]'s
        /// transaction, so maintenance statements run here on the
        /// lease-holding connection. Fence-protected durable mutations belong
        /// in [`Self::with_conn_fenced`]; SQLite does not enforce that
        /// restriction here.
        ///
        /// # Errors
        ///
        /// Returns [`StoreError::Backend`] when the callback fails.
        pub fn with_conn_unfenced<T>(
            &self,
            f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
        ) -> Result<T, StoreError> {
            let guard = self.conn.lock().unwrap_or_else(|p| p.into_inner());
            f(&guard).map_err(|e| StoreError::Backend(e.to_string()))
        }

        /// Run a closure inside an epoch-fenced write transaction. The write is
        /// rejected ([`StoreError::Fenced`]) if a newer writer has taken over the
        /// database; otherwise it commits atomically.
        ///
        /// The persisted epoch rejects late writes from an instance that has released
        /// its lease.
        ///
        /// Mechanism: an IMMEDIATE transaction reads the database's stored fence
        /// epoch and, if it is greater than this store's lease epoch, rejects without
        /// applying `f` (a newer writer owns the database). Otherwise it claims the
        /// database for this epoch and runs `f`, committing atomically. Returning an
        /// error from `f` rolls the transaction back.
        ///
        /// Callbacks must not send transaction-control SQL. A `COMMIT` in the callback
        /// ends the fence-checked transaction; later store statements would commit
        /// without the fence, and the store operation fails instead. A callback that
        /// also opens a replacement transaction defeats that detection.
        ///
        /// # Errors
        ///
        /// Returns [`StoreError::Fenced`] if the persisted database epoch exceeds the store epoch.
        /// Returns [`StoreError::Backend`] if transaction setup, fence access, the callback, the durability pin, or commit fails, or if the callback ended the transaction.
        pub fn with_conn_fenced<T>(
            &self,
            f: impl FnOnce(&rusqlite::Transaction) -> rusqlite::Result<T>,
        ) -> Result<T, StoreError> {
            let mut guard = self.conn.lock().unwrap_or_else(|p| p.into_inner());
            pin_fence_durability(&guard)?;
            let tx = guard
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            claim_fence(&tx, self.epoch)?;

            let out = f(&tx).map_err(|e| StoreError::Backend(e.to_string()))?;
            require_open_transaction(&tx)?;
            tx.commit()
                .map_err(|e| StoreError::Backend(e.to_string()))?;
            Ok(out)
        }

        /// Applies a `namespace`'s migration chain using its recorded maximum as a
        /// watermark.
        ///
        /// Each namespace has an independent migration history. Versions at or
        /// below its watermark are silently skipped.
        /// Every transaction checks the persisted fence before executing schema
        /// changes.
        ///
        /// # Errors
        ///
        /// Returns [`StoreError::Fenced`] when a newer writer owns the database.
        /// Returns [`StoreError::Backend`] when the fence check fails.
        /// Returns [`StoreError::Migration`] if migration setup, SQL execution, recording, or commit fails.
        pub fn migrate(&self, namespace: &str, migrations: &[Migration]) -> Result<(), StoreError> {
            let mut guard = self.conn.lock().unwrap_or_else(|p| p.into_inner());
            run_migrations(&mut guard, self.epoch, namespace, migrations)
        }
    }

    /// Clearing `query_only` in `Drop` prevents a reused connection from rejecting
    /// later writes with `SQLITE_READONLY`.
    struct QueryOnlyGuard<'c> {
        /// `None` once cleared, so `Drop` never repeats a restore whose failure
        /// [`Self::restore`] already reported.
        conn: Option<&'c Connection>,
    }

    impl<'c> QueryOnlyGuard<'c> {
        fn enable(conn: &'c Connection) -> Result<Self, StoreError> {
            conn.pragma_update(None, "query_only", "ON")
                .map_err(|e| StoreError::Backend(e.to_string()))?;
            conn.authorizer(Some(deny_guard_pragmas))
                .map_err(|e| StoreError::Backend(e.to_string()))?;
            Ok(Self { conn: Some(conn) })
        }

        /// Reports a failed clear to the caller, which `Drop` cannot do.
        fn restore(mut self) -> Result<(), StoreError> {
            match self.conn.take() {
                Some(conn) => Self::clear(conn),
                None => Ok(()),
            }
        }

        fn clear(conn: &Connection) -> Result<(), StoreError> {
            conn.authorizer(NO_AUTHORIZER).map_err(|e| {
                StoreError::Backend(format!("failed to clear read-only guard: {e}"))
            })?;
            conn.pragma_update(None, "query_only", "OFF")
                .map_err(|e| StoreError::Backend(format!("failed to clear read-only guard: {e}")))
        }
    }

    /// SQLite pragma names are case-insensitive, and the authorizer receives the
    /// caller's spelling.
    const GUARDED_PRAGMAS: [&str; 5] = [
        "query_only",
        "synchronous",
        "journal_mode",
        "locking_mode",
        "writable_schema",
    ];

    /// Specifies callback type so `None` removes the authorizer.
    const NO_AUTHORIZER: Option<
        fn(rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization,
    > = None;

    /// `query_only` permits PRAGMA changes, including its own.
    /// A PRAGMA read has no value and remains allowed.
    fn deny_guard_pragmas(
        context: rusqlite::hooks::AuthContext<'_>,
    ) -> rusqlite::hooks::Authorization {
        use rusqlite::hooks::{AuthAction, Authorization};
        match context.action {
            AuthAction::Pragma {
                pragma_name,
                pragma_value: Some(_),
            } if GUARDED_PRAGMAS
                .iter()
                .any(|guarded| pragma_name.eq_ignore_ascii_case(guarded)) =>
            {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        }
    }

    impl Drop for QueryOnlyGuard<'_> {
        fn drop(&mut self) {
            if let Some(conn) = self.conn.take() {
                // Drop ignores cleanup errors because it cannot return them.
                let _ = Self::clear(conn);
            }
        }
    }

    /// `query_only` permits lowering `synchronous`, which changes no database content.
    /// With WAL and `synchronous=NORMAL`, power loss can roll back committed
    /// transactions, so a protected transaction cannot trust the mode set at open.
    fn pin_fence_durability(conn: &Connection) -> Result<(), StoreError> {
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let mode: String = conn
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(StoreError::Backend(format!(
                "fenced writes require a crash-safe journal, but journal_mode is {mode}"
            )));
        }
        Ok(())
    }

    /// Transaction-control SQL sent by the callback ends the fence-checked transaction.
    /// Later statements commit in autocommit without a fence.
    fn require_open_transaction(tx: &rusqlite::Transaction<'_>) -> Result<(), StoreError> {
        if tx.is_autocommit() {
            return Err(StoreError::Backend(
                "the callback ended the fence-checked transaction; effects after that \
                 commit ran unfenced"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Open a module's SQLite store from its descriptor.
    ///
    /// The returned store has already claimed its lease epoch in the database.
    /// Call [`SqliteStore::migrate`] separately for each domain migration chain.
    ///
    /// The stored database fence becomes the lease floor. Deleting or restoring an
    /// old lease sidecar cannot reissue an epoch represented in the database.
    /// Databases created by older versions without a fence table use floor zero.
    ///
    /// The lease lives next to the database file (its parent directory), derived
    /// from the descriptor's path rather than passed in. This makes the
    /// one-lease-per-database invariant structural: two distinct database paths get
    /// distinct leases (correct isolation), and the same database path gets one
    /// lease (the single-writer guarantee). A caller cannot accidentally point a
    /// shared lease directory at several distinct databases (which would falsely
    /// make them contend) or split one database across lease directories (which
    /// would break single-writer).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::UnsupportedBackend`] for non-SQLite descriptors.
    /// Returns [`StoreError::Io`] when the parent directory cannot be created.
    /// Returns [`StoreError::Lease`] when lease acquisition fails.
    /// Returns [`StoreError::Fenced`] if the database advances during open.
    /// Returns [`StoreError::FenceCorrupt`] if the stored fence epoch is out of range.
    /// Returns [`StoreError::Backend`] when SQLite inspection, setup, or fence claim fails.
    pub fn open_sqlite(descriptor: &StorageDescriptor) -> Result<SqliteStore, StoreError> {
        let path = match &descriptor.backend {
            StorageBackend::Sqlite { path } => path.clone(),
            other => return Err(StoreError::UnsupportedBackend(other.label().to_string())),
        };

        let parent = Path::new(&path)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(&parent).map_err(StoreError::Io)?;

        let epoch_floor = read_fence_epoch(Path::new(&path))?;
        let lease = FileLeaseStore::new(&parent)
            .acquire_above(&lease_key(descriptor), epoch_floor)
            .map_err(StoreError::Lease)?;
        let epoch = lease.epoch();

        let mut conn = Connection::open(&path).map_err(|e| StoreError::Backend(e.to_string()))?;
        // WAL permits concurrent readers. The busy timeout makes transient locks
        // wait rather than fail, and foreign-key enforcement is enabled.
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        // In WAL mode, `synchronous=NORMAL` may lose the most recent commits
        // after power loss, which would roll the persisted fence epoch backward.
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        ensure_fence_table(&tx)?;
        claim_fence_strict(&tx, epoch)?;
        tx.commit()
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        // Apply owner-only permissions after enabling WAL, which may create sibling
        // files. WAL can hold recently committed rows before checkpointing.
        for suffix in ["", "-wal", "-shm"] {
            protect_file(Path::new(&format!("{path}{suffix}")))
                .map_err(|e| StoreError::Backend(e.to_string()))?;
        }

        Ok(SqliteStore {
            conn: Mutex::new(conn),
            epoch,
            _lease: lease,
        })
    }

    fn read_fence_epoch(path: &Path) -> Result<u64, StoreError> {
        if !path.try_exists().map_err(StoreError::Io)? {
            return Ok(0);
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let has_fence: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'cortexkit_fence')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        if !has_fence {
            return Ok(0);
        }
        read_fence_epoch_in(&conn)
    }

    const FENCE_EPOCH_SQL: &str =
        "SELECT COALESCE((SELECT epoch FROM cortexkit_fence WHERE id = 0), 0)";

    /// The caller guarantees that `cortexkit_fence` exists.
    fn read_fence_epoch_in(conn: &Connection) -> Result<u64, StoreError> {
        let epoch: i64 = conn
            .query_row(FENCE_EPOCH_SQL, [], |row| row.get(0))
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        decode_fence_epoch(epoch)
    }

    /// Initializes fence storage before `SqliteStore` is exposed.
    fn ensure_fence_table(tx: &rusqlite::Transaction<'_>) -> Result<(), StoreError> {
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS cortexkit_fence (\
                 id INTEGER PRIMARY KEY CHECK (id = 0), \
                 epoch INTEGER NOT NULL CHECK (epoch >= 0))",
        )
        .map_err(|e| StoreError::Backend(e.to_string()))
    }

    /// Binds fence comparison and claim to the caller's protected transaction.
    ///
    /// An epoch equal to the stored epoch permits repeated writes.
    pub(crate) fn claim_fence(
        tx: &rusqlite::Transaction<'_>,
        holder_epoch: u64,
    ) -> Result<(), StoreError> {
        let holder_epoch_sql = fence_epoch_sql_value(holder_epoch)?;
        let db_epoch = read_fence_epoch_in(tx)?;

        if db_epoch > holder_epoch {
            return Err(StoreError::Fenced {
                holder_epoch,
                db_epoch,
            });
        }
        if holder_epoch > db_epoch {
            write_fence(tx, holder_epoch_sql)?;
        }
        Ok(())
    }

    /// A stale externally derived floor can otherwise reissue the stored epoch.
    pub(crate) fn claim_fence_strict(
        tx: &rusqlite::Transaction<'_>,
        holder_epoch: u64,
    ) -> Result<(), StoreError> {
        let holder_epoch_sql = fence_epoch_sql_value(holder_epoch)?;
        let db_epoch = read_fence_epoch_in(tx)?;

        if holder_epoch <= db_epoch {
            return Err(StoreError::Fenced {
                holder_epoch,
                db_epoch,
            });
        }
        write_fence(tx, holder_epoch_sql)
    }

    /// `i64::try_from` rejects unrepresentable epochs before any database access.
    fn fence_epoch_sql_value(holder_epoch: u64) -> Result<i64, StoreError> {
        i64::try_from(holder_epoch).map_err(|_| {
            StoreError::Backend(format!(
                "lease epoch {holder_epoch} exceeds SQLite INTEGER maximum"
            ))
        })
    }

    fn write_fence(
        tx: &rusqlite::Transaction<'_>,
        holder_epoch_sql: i64,
    ) -> Result<(), StoreError> {
        tx.execute(
            "INSERT INTO cortexkit_fence (id, epoch) VALUES (0, ?1) \
             ON CONFLICT(id) DO UPDATE SET epoch = excluded.epoch",
            rusqlite::params![holder_epoch_sql],
        )
        .map(|_| ())
        .map_err(|e| StoreError::Backend(e.to_string()))
    }

    /// Rejects negative SQLite integers instead of wrapping them into writer epochs.
    fn decode_fence_epoch(epoch: i64) -> Result<u64, StoreError> {
        u64::try_from(epoch).map_err(|_| StoreError::FenceCorrupt { db_epoch: epoch })
    }

    /// Apply un-applied migrations for one `namespace` in ascending version order,
    /// each in its own transaction together with its version record, so a migration
    /// and the record that it ran commit atomically (a crash mid-migration leaves
    /// it un-recorded and it re-runs cleanly next open).
    ///
    /// Applied migrations are keyed by `(namespace, version)`, so independent
    /// domain chains in one database never collide or re-run each other.
    fn run_migrations(
        conn: &mut Connection,
        holder_epoch: u64,
        namespace: &str,
        migrations: &[Migration],
    ) -> Result<(), StoreError> {
        pin_fence_durability(conn)?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| StoreError::Migration(e.to_string()))?;
        claim_fence(&tx, holder_epoch)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS cortexkit_schema_version (\
                 namespace TEXT NOT NULL, \
                 version INTEGER NOT NULL, \
                 applied_at_unix INTEGER NOT NULL, \
                 PRIMARY KEY (namespace, version)\
             )",
        )
        .map_err(|e| StoreError::Migration(e.to_string()))?;

        let current: u32 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM cortexkit_schema_version WHERE namespace = ?1",
                rusqlite::params![namespace],
                |r| r.get(0),
            )
            .map_err(|e| StoreError::Migration(e.to_string()))?;
        tx.commit()
            .map_err(|e| StoreError::Migration(e.to_string()))?;

        let mut ordered: Vec<&Migration> = migrations.iter().collect();
        ordered.sort_by_key(|m| m.version);

        for m in ordered {
            if m.version <= current {
                continue;
            }
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| StoreError::Migration(e.to_string()))?;
            claim_fence(&tx, holder_epoch)?;
            tx.execute_batch(m.statements).map_err(|e| {
                StoreError::Migration(format!(
                    "namespace '{namespace}' migration {}: {e}",
                    m.version
                ))
            })?;
            require_open_transaction(&tx).map_err(|e| {
                StoreError::Migration(format!(
                    "namespace '{namespace}' migration {}: {e}",
                    m.version
                ))
            })?;
            tx.execute(
                "INSERT INTO cortexkit_schema_version (namespace, version, applied_at_unix) \
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![namespace, m.version, now_unix()],
            )
            .map_err(|e| StoreError::Migration(e.to_string()))?;
            tx.commit()
                .map_err(|e| StoreError::Migration(e.to_string()))?;
        }
        Ok(())
    }

    fn now_unix() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

#[cfg(feature = "sqlite")]
pub use sqlite_backend::{open_sqlite, SqliteStore};

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::sqlite_backend::{claim_fence, claim_fence_strict};
    use super::*;

    /// Reopening covers pre-existing permissive files. A first open cannot test
    /// permissive WAL repair because a fresh WAL inherits the restricted database
    /// mode.
    #[cfg(unix)]
    #[test]
    fn reopening_a_permissive_store_protects_the_database_and_its_wal() {
        use std::os::unix::fs::PermissionsExt;

        let (root, descriptor) = tmp();
        let StorageBackend::Sqlite { path } = &descriptor.backend else {
            panic!("sqlite descriptor");
        };
        let path = std::path::PathBuf::from(path);
        let wal = std::path::PathBuf::from(format!("{}-wal", path.display()));

        {
            let store = open_sqlite(&descriptor).expect("first open");
            store
                .migrate(
                    "perm",
                    &[Migration {
                        version: 1,
                        statements: "CREATE TABLE t (k TEXT);",
                    }],
                )
                .expect("migrate");
        }

        // A clean close removes the WAL. Recreate it to model a file left by an
        // unclean shutdown.
        std::fs::write(&wal, b"").expect("leave a WAL behind");

        for file in [&path, &wal] {
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o644))
                .expect("set permissive mode");
        }

        let store = open_sqlite(&descriptor).expect("reopen");

        let mode = |p: &std::path::Path| {
            std::fs::metadata(p)
                .unwrap_or_else(|error| panic!("stat {}: {error}", p.display()))
                .permissions()
                .mode()
                & 0o777
        };
        assert_eq!(
            mode(&path),
            0o600,
            "the database stayed group/world readable on reopen"
        );
        assert_eq!(
            mode(&wal),
            0o600,
            "the WAL stayed group/world readable while the database looked correct"
        );

        drop(store);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn tmp() -> (std::path::PathBuf, StorageDescriptor) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        // Per-call atomic counter (not a clock) guarantees a unique dir even when
        // tests run in parallel and the clock resolution is coarse.
        let root = std::env::temp_dir().join(format!(
            "cortexkit-store-{}-{}-{}",
            std::process::id(),
            now_nanos(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let db = root.join("store.db");
        let descriptor = StorageDescriptor {
            module_id: "test-module".into(),
            storage_namespace: "main".into(),
            isolation: Isolation::Module,
            backend: StorageBackend::Sqlite {
                path: db.to_string_lossy().into_owned(),
            },
        };
        (root, descriptor)
    }

    fn now_nanos() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    const M1: &[Migration] = &[Migration {
        version: 1,
        statements: "CREATE TABLE facts (id INTEGER PRIMARY KEY, name TEXT NOT NULL); \
                     INSERT INTO facts (id, name) VALUES (1, 'seed-a'), (2, 'seed-b');",
    }];

    #[test]
    fn open_claims_fence_before_return() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        let claimed: i64 = store
            .with_conn(|c| {
                c.query_row("SELECT epoch FROM cortexkit_fence WHERE id = 0", [], |r| {
                    r.get(0)
                })
            })
            .expect("open claimed fence");
        assert_eq!(claimed as u64, store.epoch());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Models the interleaving where a floor read before lease acquisition goes stale:
    /// an opener issues the epoch the database already stores. `claim_fence` authorizes
    /// that equal epoch, which would place two holders on one epoch, so open uses
    /// `claim_fence_strict` instead.
    #[test]
    fn open_claim_rejects_an_epoch_the_database_already_stores() {
        let (root, d) = tmp();
        let path = sqlite_path(&d);
        open_sqlite(&d).expect("seed database");

        let mut conn = rusqlite::Connection::open(&path).expect("reopen database");
        let stored: u64 = conn
            .query_row(
                "SELECT epoch FROM cortexkit_fence WHERE id = 0",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|epoch| epoch as u64)
            .expect("stored fence");

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .expect("claim transaction");
        match claim_fence_strict(&tx, stored) {
            Err(StoreError::Fenced {
                holder_epoch,
                db_epoch,
            }) => {
                assert_eq!(holder_epoch, stored);
                assert_eq!(db_epoch, stored);
            }
            other => panic!("expected an equal epoch to be rejected, got {other:?}"),
        }
        assert!(
            claim_fence(&tx, stored).is_ok(),
            "an equal epoch stays authorized for a holder that already claimed it"
        );
        claim_fence_strict(&tx, stored + 1).expect("a strictly greater epoch claims");
        drop(tx);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn migrations_seed_once_across_reopen() {
        let (root, d) = tmp();
        {
            let store = open_sqlite(&d).expect("open");
            store.migrate("facts", M1).expect("migrate");
            let n: i64 = store
                .with_conn(|c| c.query_row("SELECT COUNT(*) FROM facts", [], |r| r.get(0)))
                .expect("count");
            assert_eq!(n, 2, "seed rows inserted");
            assert_eq!(store.epoch(), 1);
        }
        {
            let store = open_sqlite(&d).expect("reopen");
            store.migrate("facts", M1).expect("migrate again");
            let n: i64 = store
                .with_conn(|c| c.query_row("SELECT COUNT(*) FROM facts", [], |r| r.get(0)))
                .expect("count");
            assert_eq!(n, 2, "seed not re-inserted on reopen (run-once)");
            assert_eq!(store.epoch(), 2, "lease epoch is monotonic across opens");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn database_epoch_survives_repeated_lease_sidecar_loss() {
        let (root, d) = tmp();
        let first = open_sqlite(&d).expect("first open");
        let first_epoch = first.epoch();
        drop(first);

        remove_lease_sidecar(&root);
        let second = open_sqlite(&d).expect("open after first sidecar loss");
        assert!(second.epoch() > first_epoch);
        let second_epoch = second.epoch();
        drop(second);

        remove_lease_sidecar(&root);
        let third = open_sqlite(&d).expect("open after second sidecar loss");
        assert!(third.epoch() > second_epoch);
        let db_epoch: i64 = third
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT epoch FROM cortexkit_fence WHERE id = 0",
                    [],
                    |row| row.get(0),
                )
            })
            .expect("database fence");
        assert_eq!(db_epoch as u64, third.epoch());
        let _ = std::fs::remove_dir_all(&root);
    }

    fn remove_lease_sidecar(root: &std::path::Path) {
        let lease = std::fs::read_dir(root)
            .expect("read store directory")
            .map(|entry| entry.expect("directory entry").path())
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "lease")
            })
            .expect("lease sidecar");
        std::fs::remove_file(lease).expect("remove lease sidecar");
    }

    #[test]
    fn second_live_writer_is_rejected() {
        let (root, d) = tmp();
        let _held = open_sqlite(&d).expect("first open");
        match open_sqlite(&d) {
            Err(StoreError::Lease(_)) => {}
            Err(e) => panic!("expected Lease(Held), got {e}"),
            Ok(_) => panic!("expected Lease(Held), got a second open"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn distinct_databases_do_not_falsely_contend() {
        let (root_a, a) = tmp();
        let (root_b, b) = tmp();
        let held_a = open_sqlite(&a).expect("open a");
        let held_b = open_sqlite(&b).expect("open b - distinct db, must not contend with a");
        drop((held_a, held_b));
        let _ = std::fs::remove_dir_all(&root_a);
        let _ = std::fs::remove_dir_all(&root_b);
    }

    #[test]
    fn later_migration_applies_on_top_of_earlier() {
        let (root, d) = tmp();
        {
            let s = open_sqlite(&d).expect("v1");
            s.migrate("facts", M1).expect("v1 migrate");
        }
        const M2: &[Migration] = &[
            Migration {
                version: 1,
                statements: "CREATE TABLE facts (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
            },
            Migration {
                version: 2,
                statements: "ALTER TABLE facts ADD COLUMN weight REAL NOT NULL DEFAULT 0;",
            },
        ];
        let store = open_sqlite(&d).expect("v2");
        store.migrate("facts", M2).expect("v2 migrate");
        let ok: i64 = store
            .with_conn(|c| {
                c.query_row("SELECT COUNT(*) FROM facts WHERE weight = 0", [], |r| {
                    r.get(0)
                })
            })
            .expect("weight column queryable");
        assert_eq!(ok, 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn independent_namespace_chains_in_one_database() {
        let (root, d) = tmp();
        const WORK_GRAPH: &[Migration] = &[Migration {
            version: 1,
            statements: "CREATE TABLE wg_nodes (id INTEGER PRIMARY KEY);",
        }];
        const HIRES: &[Migration] = &[Migration {
            version: 1,
            statements: "CREATE TABLE hires (id INTEGER PRIMARY KEY);",
        }];
        let store = open_sqlite(&d).expect("open");
        store.migrate("work_graph", WORK_GRAPH).expect("work_graph");
        store.migrate("hires", HIRES).expect("hires");
        store
            .migrate("work_graph", WORK_GRAPH)
            .expect("work_graph again");
        let tables: i64 = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('wg_nodes','hires')",
                    [],
                    |r| r.get(0),
                )
            })
            .expect("count tables");
        assert_eq!(
            tables, 2,
            "both domains' tables exist; version 1 did not collide across namespaces"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unsupported_backend_is_rejected() {
        let d = StorageDescriptor {
            module_id: "m".into(),
            storage_namespace: "n".into(),
            isolation: Isolation::Module,
            backend: StorageBackend::Postgres {
                dsn: "postgres://x".into(),
                database: "y".into(),
            },
        };
        match open_sqlite(&d) {
            Err(StoreError::UnsupportedBackend(b)) => assert_eq!(b, "postgres"),
            Err(e) => panic!("expected UnsupportedBackend, got {e}"),
            Ok(_) => panic!("expected UnsupportedBackend, got an open store"),
        }
    }

    const FENCE_SCHEMA: &[Migration] = &[Migration {
        version: 1,
        statements: "CREATE TABLE kv (k TEXT PRIMARY KEY, v TEXT NOT NULL);",
    }];

    #[test]
    fn fenced_write_commits_and_persists() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        store.migrate("kv", FENCE_SCHEMA).expect("migrate");
        store
            .with_conn_fenced(|tx| {
                tx.execute("INSERT INTO kv (k, v) VALUES ('a', '1')", [])?;
                Ok(())
            })
            .expect("fenced write");
        let v: String = store
            .with_conn(|c| c.query_row("SELECT v FROM kv WHERE k = 'a'", [], |r| r.get(0)))
            .expect("read back");
        assert_eq!(v, "1");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unfenced_connection_rejects_writes() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        store.migrate("kv", FENCE_SCHEMA).expect("migrate");
        let r = store.with_conn(|c| {
            c.execute("INSERT INTO kv (k, v) VALUES ('sneak', '1')", [])
                .map(|_| ())
        });
        assert!(
            matches!(&r, Err(StoreError::Backend(m)) if m.contains("readonly")),
            "unfenced write must fail with SQLITE_READONLY, got {r:?}"
        );
        let n: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM kv", [], |r| r.get(0)))
            .expect("count");
        assert_eq!(n, 0, "the rejected write left no row");
        store
            .with_conn_fenced(|tx| {
                tx.execute("INSERT INTO kv (k, v) VALUES ('a', '1')", [])
                    .map(|_| ())
            })
            .expect("fenced writes still work after the read-only guard clears");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn open_pins_full_synchronous() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        let sync: i64 = store
            .with_conn(|c| c.query_row("PRAGMA synchronous", [], |r| r.get(0)))
            .expect("read synchronous");
        assert_eq!(sync, 2, "fence durability requires synchronous=FULL");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_panicking_read_does_not_strand_the_connection_read_only() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        store.migrate("kv", FENCE_SCHEMA).expect("migrate");

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            store.with_conn(|_| -> rusqlite::Result<()> { panic!("callback panics") })
        }));
        assert!(panicked.is_err(), "the callback's panic propagates");

        store
            .with_conn_fenced(|tx| {
                tx.execute("INSERT INTO kv (k, v) VALUES ('after-panic', '1')", [])
                    .map(|_| ())
            })
            .expect("a fenced write after a panicking read is still authorized");
        store
            .with_conn_unfenced(|c| c.execute_batch("VACUUM"))
            .expect("maintenance after a panicking read still reaches the database");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_read_callback_cannot_lower_fence_durability() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        store.migrate("kv", FENCE_SCHEMA).expect("migrate");

        let lowered = store.with_conn(|c| c.pragma_update(None, "synchronous", "OFF"));
        assert!(
            matches!(&lowered, Err(StoreError::Backend(m)) if m.contains("not authorized")),
            "the read guard denies lowering synchronous, got {lowered:?}"
        );
        let unchanged: i64 = store
            .with_conn(|c| c.query_row("PRAGMA synchronous", [], |r| r.get(0)))
            .expect("reading a pragma stays allowed");
        assert_eq!(unchanged, 2, "the denied pragma left synchronous=FULL");

        // The fenced write restores `synchronous=FULL` after unrestricted maintenance
        // changes it.
        store
            .with_conn_unfenced(|c| c.pragma_update(None, "synchronous", "OFF"))
            .expect("maintenance may lower it");
        store
            .with_conn_fenced(|tx| {
                tx.execute("INSERT INTO kv (k, v) VALUES ('durable', '1')", [])
                    .map(|_| ())
            })
            .expect("fenced write");
        let after: i64 = store
            .with_conn(|c| c.query_row("PRAGMA synchronous", [], |r| r.get(0)))
            .expect("read synchronous");
        assert_eq!(
            after, 2,
            "the fenced write re-pinned synchronous=FULL, so the committed epoch is crash-durable"
        );

        store
            .with_conn_unfenced(|c| c.pragma_update(None, "synchronous", "NORMAL"))
            .expect("lower again");
        let second = &[Migration {
            version: 1,
            statements: "CREATE TABLE kv2 (k TEXT PRIMARY KEY);",
        }];
        store.migrate("kv2", second).expect("migrate");
        let after_migrate: i64 = store
            .with_conn(|c| c.query_row("PRAGMA synchronous", [], |r| r.get(0)))
            .expect("read synchronous");
        assert_eq!(after_migrate, 2, "migration re-pinned synchronous=FULL");

        store
            .with_conn_unfenced(|c| c.pragma_update(None, "journal_mode", "MEMORY"))
            .expect("maintenance may drop the journal");
        store
            .with_conn_fenced(|tx| {
                tx.execute("INSERT INTO kv (k, v) VALUES ('journal', '1')", [])
                    .map(|_| ())
            })
            .expect("fenced write");
        let journal: String = store
            .with_conn(|c| c.query_row("PRAGMA journal_mode", [], |r| r.get(0)))
            .expect("read journal_mode");
        assert!(
            journal.eq_ignore_ascii_case("wal"),
            "the fenced write restored a crash-safe journal, got {journal}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_read_callback_cannot_clear_the_read_only_guard() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        store.migrate("kv", FENCE_SCHEMA).expect("migrate");

        let bypass = store.with_conn(|c| {
            c.pragma_update(None, "query_only", "OFF")?;
            c.execute("INSERT INTO kv (k, v) VALUES ('bypass', '1')", [])
                .map(|_| ())
        });
        assert!(
            matches!(&bypass, Err(StoreError::Backend(m)) if m.contains("not authorized")),
            "clearing the guard is denied before any write runs, got {bypass:?}"
        );
        for pragma in ["journal_mode", "locking_mode", "writable_schema"] {
            let denied = store.with_conn(|c| c.pragma_update(None, pragma, "EXCLUSIVE"));
            assert!(
                matches!(&denied, Err(StoreError::Backend(m)) if m.contains("not authorized")),
                "setting {pragma} from a read callback is denied, got {denied:?}"
            );
        }
        // SQLite pragma names are case-insensitive.
        for spelling in ["QUERY_ONLY", "Query_Only", "qUeRy_OnLy"] {
            let denied = store.with_conn(|c| {
                c.execute_batch(&format!("PRAGMA {spelling}=OFF"))?;
                c.execute("INSERT INTO kv (k, v) VALUES ('cased', '1')", [])
                    .map(|_| ())
            });
            assert!(
                matches!(&denied, Err(StoreError::Backend(m)) if m.contains("not authorized")),
                "`PRAGMA {spelling}` is denied like the lowercase spelling, got {denied:?}"
            );
        }
        let rows: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM kv", [], |r| r.get(0)))
            .expect("count");
        assert_eq!(
            rows, 0,
            "no spelling of the guard pragma let a write through"
        );
        let n: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM kv", [], |r| r.get(0)))
            .expect("count");
        assert_eq!(n, 0, "the denied callback wrote nothing");

        store
            .with_conn_fenced(|tx| {
                tx.execute("INSERT INTO kv (k, v) VALUES ('fenced', '1')", [])
                    .map(|_| ())
            })
            .expect("the fenced path still works after a denied callback");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_callback_that_ends_the_transaction_is_rejected() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        store.migrate("kv", FENCE_SCHEMA).expect("migrate");

        let r = store.with_conn_fenced(|tx| {
            tx.execute_batch("COMMIT; INSERT INTO kv (k, v) VALUES ('escaped', '1')")?;
            Ok(())
        });
        assert!(
            matches!(&r, Err(StoreError::Backend(m)) if m.contains("ended the fence-checked transaction")),
            "ending the transaction is reported as such, not as a failed commit, got {r:?}"
        );

        let migration = &[Migration {
            version: 9,
            statements: "COMMIT; CREATE TABLE escaped_ddl (v TEXT);",
        }];
        let m = store.migrate("escape", migration);
        assert!(
            matches!(&m, Err(StoreError::Migration(msg)) if msg.contains("ended the fence-checked transaction")),
            "a migration that ends its transaction is rejected, got {m:?}"
        );
        let recorded: i64 = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT COUNT(*) FROM cortexkit_schema_version WHERE namespace = 'escape'",
                    [],
                    |r| r.get(0),
                )
            })
            .expect("count");
        assert_eq!(recorded, 0, "the rejected migration recorded no version");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn maintenance_runs_through_the_unfenced_path() {
        let (root, d) = tmp();
        let store = open_sqlite(&d).expect("open");
        store.migrate("kv", FENCE_SCHEMA).expect("migrate");
        let r = store.with_conn(|c| c.execute_batch("VACUUM"));
        assert!(
            matches!(&r, Err(StoreError::Backend(m)) if m.contains("readonly")),
            "VACUUM must not pass the read-only guard, got {r:?}"
        );
        store
            .with_conn_unfenced(|c| c.execute_batch("VACUUM"))
            .expect("VACUUM through the maintenance path");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn fenced_write_rolls_back_on_error() {
        let (root, d) = tmp();
        let path = sqlite_path(&d);
        open_sqlite(&d)
            .expect("open")
            .migrate("kv", FENCE_SCHEMA)
            .expect("migrate");
        let store = SqliteStore::for_test(rusqlite::Connection::open(path).unwrap(), 2);
        let r: Result<(), StoreError> = store.with_conn_fenced(|tx| {
            tx.execute("INSERT INTO kv (k, v) VALUES ('a', '1')", [])?;
            tx.query_row("SELECT * FROM does_not_exist", [], |_| Ok(()))?;
            Ok(())
        });
        assert!(
            matches!(r, Err(StoreError::Backend(_))),
            "closure error surfaces"
        );
        let n: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM kv", [], |r| r.get(0)))
            .expect("count");
        assert_eq!(n, 0, "the failed fenced write rolled back");
        let claimed: i64 = store
            .with_conn(|c| {
                c.query_row("SELECT epoch FROM cortexkit_fence WHERE id = 0", [], |r| {
                    r.get(0)
                })
            })
            .expect("read fence");
        assert_eq!(
            claimed, 1,
            "the failed callback did not roll back its fence claim"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_database_without_fence_table_uses_zero_floor() {
        let (root, d) = tmp();
        let path = sqlite_path(&d);
        std::fs::create_dir_all(&root).expect("create store directory");
        let conn = rusqlite::Connection::open(&path).expect("create legacy database");
        conn.execute_batch("CREATE TABLE legacy_data (id INTEGER PRIMARY KEY);")
            .expect("create legacy schema");
        drop(conn);

        let store = open_sqlite(&d).expect("open legacy database");
        assert_eq!(store.epoch(), 1, "missing fence table must use floor zero");
        let claimed: i64 = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT epoch FROM cortexkit_fence WHERE id = 0",
                    [],
                    |row| row.get(0),
                )
            })
            .expect("read claimed fence");
        assert_eq!(claimed, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_negative_database_fence_fails_closed() {
        let (root, d) = tmp();
        let path = sqlite_path(&d);
        std::fs::create_dir_all(&root).expect("create store directory");
        let conn = rusqlite::Connection::open(&path).expect("create legacy database");
        conn.execute_batch(
            "CREATE TABLE cortexkit_fence (id INTEGER PRIMARY KEY, epoch INTEGER NOT NULL); \
             INSERT INTO cortexkit_fence (id, epoch) VALUES (0, -1);",
        )
        .expect("seed pre-fence-validation database");
        drop(conn);

        let error = match open_sqlite(&d) {
            Err(error) => error,
            Ok(_) => panic!("negative fence must fail closed"),
        };
        assert!(matches!(error, StoreError::FenceCorrupt { db_epoch } if db_epoch == -1));
        let persisted: i64 = rusqlite::Connection::open(&path)
            .expect("reopen legacy database")
            .query_row(
                "SELECT epoch FROM cortexkit_fence WHERE id = 0",
                [],
                |row| row.get(0),
            )
            .expect("read unchanged negative fence");
        assert_eq!(persisted, -1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn superseded_writer_is_fenced_out_after_handover() {
        // Model the post-handover state directly: the OS lock prevents two live
        // lease holders, but a stale connection can persist after releasing its lease.
        let (root, d) = tmp();
        let path = sqlite_path(&d);

        open_sqlite(&d)
            .expect("seed schema")
            .migrate("kv", FENCE_SCHEMA)
            .expect("migrate");

        let new = SqliteStore::for_test(rusqlite::Connection::open(&path).unwrap(), 2);
        new.with_conn_fenced(|tx| {
            tx.execute("INSERT INTO kv (k, v) VALUES ('owner', 'new')", [])
                .map(|_| ())
        })
        .expect("replacement claims the db at epoch 2");

        let stale = SqliteStore::for_test(rusqlite::Connection::open(&path).unwrap(), 1);
        match stale.with_conn_fenced(|tx| {
            tx.execute("UPDATE kv SET v = 'clobbered' WHERE k = 'owner'", [])
                .map(|_| ())
        }) {
            Err(StoreError::Fenced {
                holder_epoch,
                db_epoch,
            }) => {
                assert_eq!(holder_epoch, 1);
                assert_eq!(db_epoch, 2);
            }
            other => panic!("expected Fenced, got {other:?}"),
        }

        let v: String = new
            .with_conn(|c| c.query_row("SELECT v FROM kv WHERE k = 'owner'", [], |r| r.get(0)))
            .expect("read");
        assert_eq!(v, "new", "stale writer was fenced out, no clobber");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn superseded_writer_cannot_migrate() {
        let (root, d) = tmp();
        let path = sqlite_path(&d);
        open_sqlite(&d).expect("seed database");

        let replacement = SqliteStore::for_test(rusqlite::Connection::open(&path).unwrap(), 2);
        replacement
            .with_conn_fenced(|_| Ok(()))
            .expect("replacement claim");
        let stale = SqliteStore::for_test(rusqlite::Connection::open(&path).unwrap(), 1);
        let migration = [Migration {
            version: 1,
            statements: "CREATE TABLE stale_schema (id INTEGER PRIMARY KEY);",
        }];
        assert!(matches!(
            stale.migrate("stale", &migration),
            Err(StoreError::Fenced { .. })
        ));

        let tables: i64 = replacement
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'stale_schema'",
                    [],
                    |row| row.get(0),
                )
            })
            .expect("schema state");
        assert_eq!(tables, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn equal_epoch_writer_is_not_fenced() {
        let (root, d) = tmp();
        let path = sqlite_path(&d);
        open_sqlite(&d)
            .expect("seed")
            .migrate("kv", FENCE_SCHEMA)
            .expect("migrate");
        let s = SqliteStore::for_test(rusqlite::Connection::open(&path).unwrap(), 5);
        s.with_conn_fenced(|tx| {
            tx.execute("INSERT INTO kv (k, v) VALUES ('a', '1')", [])
                .map(|_| ())
        })
        .expect("claims at 5");
        s.with_conn_fenced(|tx| {
            tx.execute("INSERT INTO kv (k, v) VALUES ('b', '2')", [])
                .map(|_| ())
        })
        .expect("same epoch 5 still writes");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn epoch_above_sqlite_integer_range_fails() {
        let (root, d) = tmp();
        let path = sqlite_path(&d);
        open_sqlite(&d).expect("seed database");
        let too_large = SqliteStore::for_test(
            rusqlite::Connection::open(&path).unwrap(),
            (i64::MAX as u64) + 1,
        );
        let error = too_large
            .with_conn_fenced(|_| Ok(()))
            .expect_err("epochs above SQLite INTEGER range must fail");
        assert!(
            matches!(error, StoreError::Backend(message) if message.contains("exceeds SQLite INTEGER maximum"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    fn sqlite_path(d: &StorageDescriptor) -> String {
        match &d.backend {
            StorageBackend::Sqlite { path } => path.clone(),
            _ => unreachable!(),
        }
    }
}
