//! Persistent non-secret action checkpoints. Storage completion, not scheduling,
//! is the acknowledgement boundary used by the HTTP action runner.
use crate::remote_actions::{ActionRunner, ActionScope, CheckpointWrite, MAX_CHECKPOINT};
use eframe::egui;
use futures::channel::oneshot;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) use crate::draft_store::{DraftStore, LoadedDraft};
pub(crate) mod order_recovery;

pub(crate) const MAX_REVISION: u64 = 9_007_199_254_740_991;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_SCOPES: i64 = 32;

pub struct StorageTask<T> {
    receiver: oneshot::Receiver<Result<T, String>>,
    complete: bool,
}
impl<T> StorageTask<T> {
    pub fn poll(&mut self) -> Option<Result<T, String>> {
        if self.complete {
            return None;
        }
        match self.receiver.try_recv() {
            Ok(Some(result)) => {
                self.complete = true;
                Some(result)
            }
            Ok(None) => None,
            Err(_) => {
                self.complete = true;
                Some(Err(
                    "Storage task interrupted; reopen recovery storage".into()
                ))
            }
        }
    }
}

pub struct LoadedStore {
    pub store: ActionStore,
    pub revision: u64,
    pub bytes: Option<Vec<u8>>,
}
#[derive(Clone)]
pub struct ActionStore {
    pub(crate) scope: ActionScope,
    key: String,
    busy: Arc<AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pool: sqlx::SqlitePool,
}
impl ActionStore {
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) async fn close(&self) {
        self.pool.close().await;
    }
    pub fn open(
        scope: ActionScope,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedStore>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::open_at(native::default_root()?, scope, ctx)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let key = scope.storage_key()?;
            Ok(spawn(ctx, async move {
                let (revision, bytes) = web::load(&key).await?;
                if let Some(bytes) = &bytes {
                    ActionRunner::restore(scope.clone(), bytes)?;
                }
                Ok(LoadedStore {
                    store: Self {
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_at(
        root: std::path::PathBuf,
        scope: ActionScope,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedStore>, String> {
        let key = scope.storage_key()?;
        spawn(ctx, async move {
            let path = tokio::task::spawn_blocking(move || native::prepare(root))
                .await
                .map_err(|_| "Recovery directory check failed")??;
            let pool = native::connect(&path).await?;
            let (revision, bytes) = native::load(&pool, &key).await?;
            if let Some(bytes) = &bytes
                && let Err(error) = ActionRunner::restore(scope.clone(), bytes)
            {
                pool.close().await;
                return Err(error);
            }
            Ok(LoadedStore {
                store: Self {
                    scope,
                    key,
                    pool,
                    busy: Arc::new(AtomicBool::new(false)),
                },
                revision,
                bytes,
            })
        })
    }
    /// Compare-and-swap protects against another window/process and stale callbacks.
    /// Dropping a task stops observing it; it does not roll back a committed write.
    pub fn commit(
        &self,
        expected: u64,
        write: &CheckpointWrite,
        ctx: &egui::Context,
    ) -> Result<StorageTask<u64>, String> {
        if expected >= MAX_REVISION || write.bytes.len() > MAX_CHECKPOINT {
            return Err("Recovery storage revision or size limit reached".into());
        }
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("A recovery write is already pending".into());
        }
        let permit = WritePermit(self.busy.clone());
        let store = self.clone();
        let bytes = write.bytes.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            spawn(ctx, async move {
                let _permit = permit;
                native::commit(&store, expected, bytes).await
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(spawn(ctx, async move {
                let _permit = permit;
                let (revision, before) = web::load(&store.key).await?;
                if revision != expected {
                    return Err("Recovery storage changed in another window; reopen it".into());
                }
                ActionRunner::validate_storage_change(&store.scope, before.as_deref(), &bytes)?;
                web::commit(&store.key, expected, &bytes).await
            }))
        }
    }
}

struct WritePermit(Arc<AtomicBool>);
impl Drop for WritePermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn<T: Send + 'static>(
    ctx: &egui::Context,
    future: impl std::future::Future<Output = Result<T, String>> + Send + 'static,
) -> Result<StorageTask<T>, String> {
    let (sender, receiver) = oneshot::channel();
    let ctx = ctx.clone();
    crate::http::runtime()
        .map_err(|error| error.to_string())?
        .spawn(async move {
            let _ = sender.send(future.await);
            ctx.request_repaint();
        });
    Ok(StorageTask {
        receiver,
        complete: false,
    })
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn<T: 'static>(
    ctx: &egui::Context,
    future: impl std::future::Future<Output = Result<T, String>> + 'static,
) -> StorageTask<T> {
    let (sender, receiver) = oneshot::channel();
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = sender.send(future.await);
        ctx.request_repaint();
    });
    StorageTask {
        receiver,
        complete: false,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;
