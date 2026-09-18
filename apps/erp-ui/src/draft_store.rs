//! Durable, account-scoped UI drafts. Drafts never share the action journal type.
use crate::{
    action_store::{ActionStore, MAX_REVISION, StorageTask, spawn},
    edit_drafts::{DraftScope, decode},
};
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) const MAX_DRAFT: usize = 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_DRAFTS: i64 = 32;
pub(crate) const DRAFT_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;

pub(crate) struct LoadedDraft {
    pub store: DraftStore,
    pub revision: u64,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Clone)]
pub(crate) struct DraftStore {
    pub(crate) scope: DraftScope,
    pub(crate) key: String,
    busy: Arc<AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pool: sqlx::SqlitePool,
}
impl DraftStore {
    pub(crate) fn scope(&self) -> &DraftScope {
        &self.scope
    }

    /// Every write advances the shared draft epoch; clear removes this scope's slot.
    pub(crate) fn commit(
        &self,
        expected: u64,
        bytes: Option<&[u8]>,
        now: u64,
        ctx: &egui::Context,
    ) -> Result<StorageTask<u64>, String> {
        if expected >= MAX_REVISION
            || now == 0
            || now.checked_add(DRAFT_TTL_SECONDS).is_none()
            || bytes.is_some_and(|bytes| bytes.is_empty() || bytes.len() > MAX_DRAFT)
        {
            return Err("Draft storage revision, timestamp or size limit reached".into());
        }
        if let Some(bytes) = bytes {
            decode(&self.scope, now, bytes)?;
        }
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("A draft write is already pending".into());
        }
        let permit = DraftWritePermit(self.busy.clone());
        let store = self.clone();
        let bytes = bytes.map(ToOwned::to_owned);
        #[cfg(not(target_arch = "wasm32"))]
        {
            spawn(ctx, async move {
                let _permit = permit;
                crate::action_store::native::commit_draft(&store, expected, bytes, now).await
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(spawn(ctx, async move {
                let _permit = permit;
                crate::action_store::web::commit_draft(&store.key, expected, bytes.as_deref(), now)
                    .await
            }))
        }
    }
}

struct DraftWritePermit(Arc<AtomicBool>);
impl Drop for DraftWritePermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl ActionStore {
    pub(crate) fn open_drafts(
        &self,
        grid: &str,
        schema: u16,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedDraft>, String> {
        if grid.is_empty()
            || grid.len() > 64
            || !grid
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            || schema == 0
        {
            return Err("Invalid draft grid scope".into());
        }
        let scope = DraftScope {
            server: self.scope.server().into(),
            tenant: self.scope.tenant(),
            actor: self.scope.actor(),
            grid: grid.into(),
            schema,
        };
        let key = serde_json::to_string(&scope)
            .map(|value| format!("draft-v1:{value}"))
            .map_err(|_| "Cannot encode draft scope")?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pool = self.pool.clone();
            spawn(ctx, async move {
                let (revision, bytes) =
                    crate::action_store::native::load_draft(&pool, &key).await?;
                Ok(LoadedDraft {
                    store: DraftStore {
                        scope,
                        key,
                        busy: Arc::new(AtomicBool::new(false)),
                        pool,
                    },
                    revision,
                    bytes,
                })
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(spawn(ctx, async move {
                let (revision, bytes) = crate::action_store::web::load_draft(&key).await?;
                Ok(LoadedDraft {
                    store: DraftStore {
                        scope,
                        key,
                        busy: Arc::new(AtomicBool::new(false)),
                    },
                    revision,
                    bytes,
                })
            }))
        }
    }
}
