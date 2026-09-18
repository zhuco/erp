use super::*;
use sqlx::{
    Connection,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub(super) fn default_root() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("ErpGrid"));
    #[cfg(target_os = "macos")]
    let root = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join("Library/Application Support/ErpGrid"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|path| PathBuf::from(path).join(".local/share")))
        .map(|path| path.join("erp-grid"));
    root.filter(|path| path.is_absolute())
        .ok_or("Cannot locate local recovery data directory".into())
}
fn ordinary(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err("Recovery storage cannot use reparse points".into());
                }
            }
            if metadata.file_type().is_symlink() {
                return Err("Recovery storage cannot use symbolic links".into());
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Cannot inspect recovery storage path".into()),
    }
}
pub(super) fn prepare(root: PathBuf) -> Result<PathBuf, String> {
    if !root.is_absolute()
        || root
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("Recovery directory must be an absolute local path".into());
    }
    #[cfg(windows)]
    if matches!(root.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::UNC(..) | std::path::Prefix::VerbatimUNC(..)))
    {
        return Err("Recovery database must not be on a network share".into());
    }
    for path in root.ancestors() {
        ordinary(path)?;
    }
    std::fs::create_dir_all(&root).map_err(|_| "Cannot create recovery data directory")?;
    ordinary(&root)?;
    for name in [
        "actions-v1.sqlite3",
        "actions-v1.sqlite3-journal",
        "actions-v1.sqlite3-wal",
        "actions-v1.sqlite3-shm",
    ] {
        ordinary(&root.join(name))?;
    }
    Ok(root.join("actions-v1.sqlite3"))
}
pub(super) async fn connect(path: &Path) -> Result<sqlx::SqlitePool, String> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Delete)
        .synchronous(SqliteSynchronous::Extra)
        .busy_timeout(Duration::from_secs(3))
        .pragma("page_size", "4096")
        .pragma("max_page_count", "262144");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|_| "Cannot open recovery database")?;
    sqlx::query("create table if not exists action_checkpoints (scope text primary key, revision integer not null check(revision > 0 and revision <= 9007199254740991), body blob not null check(length(body) <= 16777216))")
        .execute(&pool).await.map_err(|_| "Cannot initialize recovery database")?;
    sqlx::query("create table if not exists draft_snapshots (scope text primary key, revision integer not null check(revision > 0 and revision <= 9007199254740991), expires_at integer not null check(expires_at > 0), body blob check(body is null or length(body) <= 1048576))")
        .execute(&pool).await.map_err(|_| "Cannot initialize draft storage")?;
    sqlx::query("create table if not exists draft_meta (singleton integer primary key check(singleton=1), epoch integer not null check(epoch >= 0 and epoch <= 9007199254740991))")
        .execute(&pool).await.map_err(|_| "Cannot initialize draft metadata")?;
    sqlx::query("insert or ignore into draft_meta(singleton,epoch) select 1,coalesce(max(revision),0) from draft_snapshots")
        .execute(&pool).await.map_err(|_| "Cannot initialize draft epoch")?;
    let (sync,): (i64,) = sqlx::query_as("pragma synchronous")
        .fetch_one(&pool)
        .await
        .map_err(|_| "Cannot verify storage durability")?;
    if sync != 3 {
        return Err("Recovery database requires synchronous EXTRA".into());
    }
    let (page_size,): (i64,) = sqlx::query_as("pragma page_size")
        .fetch_one(&pool)
        .await
        .map_err(|_| "Cannot verify recovery storage size")?;
    if page_size != 4096 {
        return Err("Unsupported recovery database page size".into());
    }
    Ok(pool)
}
pub(super) async fn load(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<(u64, Option<Vec<u8>>), String> {
    let row: Option<(i64, Vec<u8>)> =
        sqlx::query_as("select revision, body from action_checkpoints where scope=?")
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(|_| "Cannot read recovery checkpoint")?;
    match row {
        None => Ok((0, None)),
        Some((revision, bytes))
            if revision > 0 && revision as u64 <= MAX_REVISION && bytes.len() <= MAX_CHECKPOINT =>
        {
            Ok((revision as u64, Some(bytes)))
        }
        Some(_) => Err("Invalid recovery checkpoint envelope".into()),
    }
}
pub(super) async fn commit(
    store: &ActionStore,
    expected: u64,
    bytes: Vec<u8>,
) -> Result<u64, String> {
    let mut connection = store
        .pool
        .acquire()
        .await
        .map_err(|_| "Recovery database is busy")?;
    // Acquire a write lock before reading to serialize validation across processes.
    // TransactionManager owns rollback on every early return or dropped future.
    let mut tx = connection
        .begin()
        .await
        .map_err(|_| "Cannot start recovery transaction")?;
    sqlx::query("update action_checkpoints set revision=revision where scope=?")
        .bind(&store.key)
        .execute(&mut *tx)
        .await
        .map_err(|_| "Cannot lock recovery checkpoint")?;
    let old: Option<(i64, Vec<u8>)> =
        sqlx::query_as("select revision, body from action_checkpoints where scope=?")
            .bind(&store.key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| "Cannot read previous recovery checkpoint")?;
    let revision = old.as_ref().map_or(0, |row| row.0 as u64);
    if revision != expected {
        return Err("Recovery storage changed in another window; reopen it".into());
    }
    ActionRunner::validate_storage_change(
        &store.scope,
        old.as_ref().map(|row| row.1.as_slice()),
        &bytes,
    )?;
    if old.is_none() {
        let (count,): (i64,) = sqlx::query_as("select count(*) from action_checkpoints")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Cannot check recovery storage quota")?;
        if count >= MAX_SCOPES {
            return Err(
                "Recovery storage has reached 32 account scopes; retain existing recovery data"
                    .into(),
            );
        }
    }
    let next = expected + 1;
    sqlx::query("insert into action_checkpoints(scope,revision,body) values(?,?,?) on conflict(scope) do update set revision=excluded.revision, body=excluded.body")
        .bind(&store.key).bind(next as i64).bind(bytes).execute(&mut *tx).await.map_err(|_| "Cannot write recovery checkpoint; storage may be full")?;
    tx.commit()
        .await
        .map_err(|_| "Recovery commit result unknown; reopen storage before sending")?;
    Ok(next)
}

pub(crate) async fn load_draft(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<(u64, Option<Vec<u8>>), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "System clock is before the Unix epoch")?
        .as_secs();
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| "Cannot start draft storage transaction")?;
    sqlx::query("update draft_meta set epoch=epoch where singleton=1")
        .execute(&mut *tx)
        .await
        .map_err(|_| "Cannot lock draft metadata")?;
    let removed = sqlx::query("delete from draft_snapshots where body is null or expires_at<=?")
        .bind(now as i64)
        .execute(&mut *tx)
        .await
        .map_err(|_| "Cannot expire draft snapshots")?
        .rows_affected();
    if removed > 0 {
        let changed =
            sqlx::query("update draft_meta set epoch=epoch+1 where singleton=1 and epoch<?")
                .bind(MAX_REVISION as i64)
                .execute(&mut *tx)
                .await
                .map_err(|_| "Cannot advance draft epoch")?
                .rows_affected();
        if changed != 1 {
            return Err("Expired draft epoch cannot advance".into());
        }
    }
    let (epoch,): (i64,) = sqlx::query_as("select epoch from draft_meta where singleton=1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Cannot read draft epoch")?;
    let row: Option<(i64, Vec<u8>)> =
        sqlx::query_as("select expires_at, body from draft_snapshots where scope=?")
            .bind(key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| "Cannot read draft snapshot")?;
    tx.commit()
        .await
        .map_err(|_| "Cannot finish draft storage read")?;
    if epoch < 0 || epoch as u64 > MAX_REVISION {
        return Err("Invalid draft storage epoch".into());
    }
    match row {
        None => Ok((epoch as u64, None)),
        Some((expires_at, bytes))
            if expires_at > now as i64
                && !bytes.is_empty()
                && bytes.len() <= crate::draft_store::MAX_DRAFT =>
        {
            Ok((epoch as u64, Some(bytes)))
        }
        Some(_) => Err("Invalid draft storage envelope".into()),
    }
}

pub(crate) async fn commit_draft(
    store: &crate::draft_store::DraftStore,
    expected: u64,
    bytes: Option<Vec<u8>>,
    now: u64,
) -> Result<u64, String> {
    let expires = bytes
        .as_deref()
        .map(|bytes| crate::edit_drafts::decode(&store.scope, now, bytes))
        .transpose()?
        .map(|snapshot| snapshot.expires_at);
    let mut connection = store
        .pool
        .acquire()
        .await
        .map_err(|_| "Draft database is busy")?;
    let mut tx = connection
        .begin()
        .await
        .map_err(|_| "Cannot start draft storage transaction")?;
    sqlx::query("update draft_meta set epoch=epoch where singleton=1")
        .execute(&mut *tx)
        .await
        .map_err(|_| "Cannot lock draft metadata")?;
    let (epoch,): (i64,) = sqlx::query_as("select epoch from draft_meta where singleton=1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Cannot read draft epoch")?;
    if epoch < 0 || epoch as u64 != expected {
        return Err("Draft storage changed in another window; reopen it".into());
    }
    sqlx::query("delete from draft_snapshots where body is null or expires_at<=?")
        .bind(now as i64)
        .execute(&mut *tx)
        .await
        .map_err(|_| "Cannot expire draft snapshots")?;
    let exists: (i64,) = sqlx::query_as("select count(*) from draft_snapshots where scope=?")
        .bind(&store.key)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Cannot inspect draft snapshot")?;
    if bytes.is_some() && exists.0 == 0 {
        let (count,): (i64,) = sqlx::query_as("select count(*) from draft_snapshots")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Cannot check draft storage quota")?;
        if count >= crate::draft_store::MAX_DRAFTS {
            return Err("Draft storage has reached 32 scopes".into());
        }
    }
    let next = expected
        .checked_add(1)
        .filter(|value| *value <= MAX_REVISION)
        .ok_or("Draft storage epoch cannot advance")?;
    match (bytes, expires) {
        (Some(bytes), Some(expires)) => {
            sqlx::query("insert into draft_snapshots(scope,revision,expires_at,body) values(?,?,?,?) on conflict(scope) do update set revision=excluded.revision, expires_at=excluded.expires_at, body=excluded.body")
                .bind(&store.key).bind(next as i64).bind(expires as i64).bind(bytes)
                .execute(&mut *tx).await
                .map_err(|_| "Cannot write draft snapshot; storage may be full")?;
        }
        (None, None) => {
            sqlx::query("delete from draft_snapshots where scope=?")
                .bind(&store.key)
                .execute(&mut *tx)
                .await
                .map_err(|_| "Cannot clear draft snapshot")?;
        }
        _ => return Err("Invalid draft expiry envelope".into()),
    }
    sqlx::query("update draft_meta set epoch=? where singleton=1")
        .bind(next as i64)
        .execute(&mut *tx)
        .await
        .map_err(|_| "Cannot write draft epoch")?;
    tx.commit()
        .await
        .map_err(|_| "Draft commit result unknown; reopen storage")?;
    Ok(next)
}
