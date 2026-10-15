//! Platform storage for non-expiring form recovery. A receipt is minted after independent readback.
use super::detail::{DetailPanel, DetailSaveRequest};
use crate::action_store::{MAX_REVISION, StorageTask, spawn};
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const EMPTY: &[u8] = b"null";

pub struct DetailPersistWrite {
    pub(super) scope: String,
    pub(super) bytes: Option<Vec<u8>>,
    pub(super) resolved: Option<DetailSaveRequest>,
}

pub struct DetailPersistReceipt {
    scope: String,
    primary: bool,
    bytes: Option<Vec<u8>>,
    revision: u64,
}
impl DetailPersistReceipt {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub(super) fn matches(&self, scope: &str, bytes: Option<&[u8]>) -> bool {
        self.primary && self.scope == scope && self.bytes.as_deref() == bytes
    }
}

#[derive(Clone)]
pub struct DetailStore {
    scope: String,
    key: String,
    primary: bool,
    busy: Arc<AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    pool: sqlx::SqlitePool,
}
pub struct LoadedDetailStore {
    pub store: DetailStore,
    pub revision: u64,
    pub bytes: Option<Vec<u8>>,
}

impl DetailStore {
    pub fn scope(&self) -> &str {
        &self.scope
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(
        scope: String,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDetailStore>, String> {
        Self::open_at(crate::action_store::native::default_root()?, scope, ctx)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_at(
        root: std::path::PathBuf,
        scope: String,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDetailStore>, String> {
        Self::open_named_at(root, scope, true, ctx)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_backup_at(
        root: std::path::PathBuf,
        scope: String,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDetailStore>, String> {
        Self::open_named_at(root, scope, false, ctx)
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn open_named_at(
        root: std::path::PathBuf,
        scope: String,
        primary: bool,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDetailStore>, String> {
        validate_scope(&scope)?;
        let key = store_key(&scope, primary)?;
        spawn(ctx, async move {
            let path =
                tokio::task::spawn_blocking(move || crate::action_store::native::prepare(root))
                    .await
                    .map_err(|_| "Cannot inspect form storage")??;
            let pool = crate::action_store::native::connect(&path).await?;
            sqlx::query("create table if not exists detail_checkpoints (scope text primary key, revision integer not null check(revision>0 and revision<=9007199254740991), body blob not null check(length(body)<=8388608))").execute(&pool).await.map_err(|_|"Cannot initialize form storage")?;
            let store = Self {
                scope,
                key,
                primary,
                busy: Arc::new(AtomicBool::new(false)),
                pool,
            };
            let (revision, bytes) = store.load_native().await?;
            Ok(LoadedDetailStore {
                store,
                revision,
                bytes,
            })
        })
    }
    #[cfg(target_arch = "wasm32")]
    pub fn open(
        scope: String,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDetailStore>, String> {
        Self::open_named(scope, true, ctx)
    }
    #[cfg(target_arch = "wasm32")]
    fn open_named(
        scope: String,
        primary: bool,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDetailStore>, String> {
        validate_scope(&scope)?;
        let key = store_key(&scope, primary)?;
        Ok(spawn(ctx, async move {
            let (revision, bytes) = web::load(&key).await?;
            validate_bytes(&scope, bytes.as_deref())?;
            Ok(LoadedDetailStore {
                store: Self {
                    scope,
                    key,
                    primary,
                    busy: Arc::new(AtomicBool::new(false)),
                },
                revision,
                bytes,
            })
        }))
    }
    pub fn open_backup(
        scope: String,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDetailStore>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::open_named_at(
                crate::action_store::native::default_root()?,
                scope,
                false,
                ctx,
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::open_named(scope, false, ctx)
        }
    }
    pub fn commit(
        &self,
        expected: u64,
        write: DetailPersistWrite,
        ctx: &egui::Context,
    ) -> Result<StorageTask<DetailPersistReceipt>, String> {
        if expected >= MAX_REVISION || write.scope != self.scope {
            return Err("Form storage scope/revision mismatch".into());
        }
        validate_bytes(&self.scope, write.bytes.as_deref())?;
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("A form checkpoint write is pending".into());
        }
        let permit = Permit(self.busy.clone());
        let store = self.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            spawn(ctx, async move {
                let _permit = permit;
                let revision = store.commit_native(expected, &write).await?;
                let (read_revision, bytes) = store.load_native().await?;
                if read_revision != revision || bytes != write.bytes {
                    return Err("Form checkpoint readback changed; reopen before sending".into());
                }
                Ok(DetailPersistReceipt {
                    scope: store.scope,
                    primary: store.primary,
                    bytes,
                    revision,
                })
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(spawn(ctx, async move {
                let _permit = permit;
                let (old_revision, old) = web::load(&store.key).await?;
                if old_revision != expected {
                    return Err("Form checkpoint changed in another window".into());
                }
                validate_transition(&store.scope, old.as_deref(), &write)?;
                let revision = web::commit(
                    &store.key,
                    expected,
                    write.bytes.as_deref().unwrap_or(EMPTY),
                )
                .await?;
                let (read_revision, bytes) = web::load(&store.key).await?;
                if read_revision != revision || bytes != write.bytes {
                    return Err("Form checkpoint readback changed; reopen before sending".into());
                }
                Ok(DetailPersistReceipt {
                    scope: store.scope,
                    primary: store.primary,
                    bytes,
                    revision,
                })
            }))
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn load_native(&self) -> Result<(u64, Option<Vec<u8>>), String> {
        let row: Option<(i64, Vec<u8>)> =
            sqlx::query_as("select revision,body from detail_checkpoints where scope=?")
                .bind(&self.key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| "Cannot load form checkpoint")?;
        unpack(&self.scope, row)
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn commit_native(
        &self,
        expected: u64,
        write: &DetailPersistWrite,
    ) -> Result<u64, String> {
        use sqlx::Connection;
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|_| "Form storage is busy")?;
        let mut tx = connection
            .begin()
            .await
            .map_err(|_| "Cannot start form storage transaction")?;
        sqlx::query("update detail_checkpoints set revision=revision where scope=?")
            .bind(&self.key)
            .execute(&mut *tx)
            .await
            .map_err(|_| "Cannot lock form storage")?;
        let old: Option<(i64, Vec<u8>)> =
            sqlx::query_as("select revision,body from detail_checkpoints where scope=?")
                .bind(&self.key)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| "Cannot read previous form checkpoint")?;
        let (revision, bytes) = unpack(&self.scope, old.clone())?;
        if revision != expected {
            return Err("Form checkpoint changed in another window".into());
        }
        validate_transition(&self.scope, bytes.as_deref(), write)?;
        if old.is_none() {
            let count: i64 = sqlx::query_scalar("select count(*) from detail_checkpoints")
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "Cannot inspect form quota")?;
            if count >= 32 {
                return Err(
                    "Form recovery has reached32 scopes; existing recovery retained".into(),
                );
            }
        }
        let next = expected + 1;
        sqlx::query("insert into detail_checkpoints(scope,revision,body) values(?,?,?) on conflict(scope) do update set revision=excluded.revision,body=excluded.body")
            .bind(&self.key).bind(next as i64).bind(write.bytes.as_deref().unwrap_or(EMPTY)).execute(&mut *tx).await.map_err(|_|"Cannot write form checkpoint")?;
        tx.commit()
            .await
            .map_err(|_| "Form checkpoint commit result unknown; reopen storage")?;
        Ok(next)
    }
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) async fn close(&self) {
        self.pool.close().await;
    }
}
struct Permit(Arc<AtomicBool>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
fn store_key(scope: &str, primary: bool) -> Result<String, String> {
    if primary {
        Ok(scope.to_owned())
    } else {
        serde_json::to_string(&("detail-local-backup-v1", scope))
            .map_err(|_| "Cannot encode backup scope".into())
    }
}
fn validate_scope(scope: &str) -> Result<(), String> {
    if scope.trim().is_empty() || scope.len() > 1024 {
        return Err("Invalid authenticated form scope".into());
    }
    Ok(())
}
fn validate_bytes(scope: &str, bytes: Option<&[u8]>) -> Result<(), String> {
    if let Some(bytes) = bytes {
        if bytes.is_empty() || bytes.len() > MAX_BYTES || bytes == EMPTY {
            return Err("Invalid form checkpoint capacity".into());
        }
        DetailPanel::restore(scope, bytes)?;
    }
    Ok(())
}
fn validate_transition(
    scope: &str,
    old: Option<&[u8]>,
    write: &DetailPersistWrite,
) -> Result<(), String> {
    if let Some(old) = old {
        let before = DetailPanel::restore(scope, old)?;
        if let Some(request) = before.pending_request() {
            let next = write
                .bytes
                .as_deref()
                .map(|bytes| DetailPanel::restore(scope, bytes))
                .transpose()?;
            let retained = next.as_ref().and_then(DetailPanel::pending_request) == Some(request);
            if !retained && write.resolved.as_ref() != Some(request) {
                return Err(
                    "Unresolved original form checkpoint cannot be replaced or cleared".into(),
                );
            }
        }
    }
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
fn unpack(scope: &str, row: Option<(i64, Vec<u8>)>) -> Result<(u64, Option<Vec<u8>>), String> {
    match row {
        None => Ok((0, None)),
        Some((revision, bytes)) => {
            if revision <= 0 || revision as u64 > MAX_REVISION || bytes.len() > MAX_BYTES {
                return Err("Invalid form storage envelope".into());
            }
            let bytes = (bytes != EMPTY).then_some(bytes);
            validate_bytes(scope, bytes.as_deref())?;
            Ok((revision as u64, bytes))
        }
    }
}
#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(module = "/src/workbench/detail_db.js")]
    extern "C" {
        #[wasm_bindgen(js_name=loadFormCheckpoint)]
        fn load_js(scope: &str) -> js_sys::Promise;
        #[wasm_bindgen(js_name=commitFormCheckpoint)]
        fn commit_js(scope: &str, expected: f64, bytes: &[u8]) -> js_sys::Promise;
    }
    pub(super) async fn load(scope: &str) -> Result<(u64, Option<Vec<u8>>), String> {
        let value = wasm_bindgen_futures::JsFuture::from(load_js(scope))
            .await
            .map_err(|_| "Cannot load browser form storage")?;
        let array = js_sys::Array::from(&value);
        let revision = array
            .get(0)
            .as_f64()
            .ok_or("Invalid form storage revision")?;
        if !revision.is_finite()
            || revision < 0.0
            || revision.fract() != 0.0
            || revision > MAX_REVISION as f64
        {
            return Err("Invalid form storage revision".into());
        }
        let body = array.get(1);
        if revision == 0.0 && body.is_null() {
            return Ok((0, None));
        }
        if revision == 0.0 || !body.is_instance_of::<js_sys::Uint8Array>() {
            return Err("Invalid form storage body".into());
        }
        let bytes = js_sys::Uint8Array::new(&body).to_vec();
        let bytes = (bytes != EMPTY).then_some(bytes);
        Ok((revision as u64, bytes))
    }
    pub(super) async fn commit(scope: &str, expected: u64, bytes: &[u8]) -> Result<u64, String> {
        let value = wasm_bindgen_futures::JsFuture::from(commit_js(scope, expected as f64, bytes))
            .await
            .map_err(|_| "Browser form checkpoint write failed or conflicted")?;
        if value.as_f64() != Some((expected + 1) as f64) {
            return Err("Unexpected form storage acknowledgement".into());
        }
        Ok(expected + 1)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn wait_storage<T>(mut task: StorageTask<T>) -> Result<T, String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(result) = task.poll() {
            return result;
        }
        if std::time::Instant::now() >= deadline {
            return Err("Storage QA timed out".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn authorize_fixture(
    panel: &mut DetailPanel,
    request: &DetailSaveRequest,
) -> Result<(), String> {
    let root = std::env::current_dir()
        .map_err(|_| "QA directory unavailable")?
        .join("qa/acceptance/detail-durable-native")
        .join(uuid::Uuid::new_v4().to_string());
    let ctx = egui::Context::default();
    let loaded = wait_storage(DetailStore::open_at(root, request.scope.clone(), &ctx)?)?;
    let receipt = wait_storage(loaded.store.commit(
        loaded.revision,
        panel.persistence_write()?,
        &ctx,
    )?)?;
    panel.authorize_send(request, &receipt)
}
