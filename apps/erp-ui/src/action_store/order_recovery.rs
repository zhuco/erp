//! Non-expiring, versioned order recovery; acknowledgement means committed storage.
use super::{MAX_REVISION, StorageTask, spawn};
use crate::remote_actions::ActionScope;
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const MAX_BYTES: usize = 1024 * 1024;
const EMPTY: &[u8] = b"null";

pub(crate) struct LoadedOrderRecovery {
    pub store: OrderRecoveryStore,
    pub revision: u64,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Clone)]
pub(crate) struct OrderRecoveryStore {
    key: String,
    busy: Arc<AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    pool: sqlx::SqlitePool,
}

impl OrderRecoveryStore {
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) async fn open_trade_test(
        scope: ActionScope,
        root: &std::path::Path,
    ) -> Result<LoadedOrderRecovery, String> {
        Self::open_test(scope, root, "trade-sales-v1").await
    }
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) async fn open_material_test(
        scope: ActionScope,
        root: &std::path::Path,
    ) -> Result<LoadedOrderRecovery, String> {
        Self::open_test(scope, root, "material-documents-v1").await
    }
    #[cfg(all(test, not(target_arch = "wasm32")))]
    async fn open_test(
        scope: ActionScope,
        root: &std::path::Path,
        namespace: &str,
    ) -> Result<LoadedOrderRecovery, String> {
        let key = format!("{namespace}:{}", scope.storage_key()?);
        let path = super::native::prepare(root.to_path_buf())?;
        let pool = super::native::connect(&path).await?;
        let (revision, bytes) = super::native::load(&pool, &key).await?;
        Ok(LoadedOrderRecovery {
            store: Self {
                key,
                busy: Arc::new(AtomicBool::new(false)),
                pool,
            },
            revision,
            bytes: unpack(bytes)?,
        })
    }
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) async fn close_test(&self) {
        self.pool.close().await;
    }
    pub(crate) fn open_trade(
        scope: ActionScope,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedOrderRecovery>, String> {
        Self::open_named(scope, "trade-sales-v1", ctx)
    }
    pub(crate) fn open_material(
        scope: ActionScope,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedOrderRecovery>, String> {
        Self::open_named(scope, "material-documents-v1", ctx)
    }
    pub(crate) fn open(
        scope: ActionScope,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedOrderRecovery>, String> {
        Self::open_named(scope, "order-recovery-v1", ctx)
    }

    pub(crate) fn open_inspector(
        scope: ActionScope,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedOrderRecovery>, String> {
        Self::open_named(scope, "order-inspector-v1", ctx)
    }

    fn open_named(
        scope: ActionScope,
        namespace: &'static str,
        ctx: &egui::Context,
    ) -> Result<StorageTask<LoadedOrderRecovery>, String> {
        let key = format!("{namespace}:{}", scope.storage_key()?);
        #[cfg(not(target_arch = "wasm32"))]
        {
            spawn(ctx, async move {
                let path = tokio::task::spawn_blocking(|| {
                    super::native::prepare(super::native::default_root()?)
                })
                .await
                .map_err(|_| "Order recovery directory check failed")??;
                let pool = super::native::connect(&path).await?;
                let (revision, bytes) = super::native::load(&pool, &key).await?;
                Ok(LoadedOrderRecovery {
                    store: Self {
                        key,
                        busy: Arc::new(AtomicBool::new(false)),
                        pool,
                    },
                    revision,
                    bytes: unpack(bytes)?,
                })
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(spawn(ctx, async move {
                let (revision, bytes) = super::web::load(&key).await?;
                Ok(LoadedOrderRecovery {
                    store: Self {
                        key,
                        busy: Arc::new(AtomicBool::new(false)),
                    },
                    revision,
                    bytes: unpack(bytes)?,
                })
            }))
        }
    }

    pub(crate) fn commit(
        &self,
        expected: u64,
        bytes: Option<&[u8]>,
        ctx: &egui::Context,
    ) -> Result<StorageTask<u64>, String> {
        if expected >= MAX_REVISION
            || bytes
                .is_some_and(|bytes| bytes.is_empty() || bytes.len() > MAX_BYTES || bytes == EMPTY)
        {
            return Err("Order recovery size or revision limit reached".into());
        }
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("Order recovery write is already pending".into());
        }
        let permit = Permit(self.busy.clone());
        let store = self.clone();
        let bytes = bytes.unwrap_or(EMPTY).to_vec();
        #[cfg(not(target_arch = "wasm32"))]
        {
            spawn(ctx, async move {
                let _permit = permit;
                store.commit_native(expected, bytes).await
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(spawn(ctx, async move {
                let _permit = permit;
                super::web::commit(&store.key, expected, &bytes).await
            }))
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn commit_native(&self, expected: u64, bytes: Vec<u8>) -> Result<u64, String> {
        use sqlx::Connection;
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|_| "Order recovery database is busy")?;
        let mut tx = connection
            .begin()
            .await
            .map_err(|_| "Cannot start order recovery transaction")?;
        sqlx::query("update action_checkpoints set revision=revision where scope=?")
            .bind(&self.key)
            .execute(&mut *tx)
            .await
            .map_err(|_| "Cannot lock order recovery")?;
        let old: Option<(i64, Vec<u8>)> =
            sqlx::query_as("select revision,body from action_checkpoints where scope=?")
                .bind(&self.key)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| "Cannot read order recovery")?;
        if old.as_ref().is_some_and(|(revision, bytes)| {
            *revision <= 0
                || *revision as u64 > MAX_REVISION
                || bytes.is_empty()
                || bytes.len() > MAX_BYTES
        }) || old.as_ref().map_or(0, |(revision, _)| *revision as u64) != expected
        {
            return Err("Order recovery changed or is invalid; reopen before sending".into());
        }
        if old.is_none() {
            let count: i64 = sqlx::query_scalar("select count(*) from action_checkpoints")
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "Cannot check recovery quota")?;
            if count >= super::MAX_SCOPES {
                return Err("Recovery account limit reached; existing data retained".into());
            }
        }
        let next = expected + 1;
        sqlx::query("insert into action_checkpoints(scope,revision,body) values(?,?,?) on conflict(scope) do update set revision=excluded.revision,body=excluded.body")
            .bind(&self.key).bind(next as i64).bind(bytes).execute(&mut *tx).await.map_err(|_| "Cannot write order recovery")?;
        tx.commit()
            .await
            .map_err(|_| "Order recovery commit uncertain; reopen before sending")?;
        Ok(next)
    }
}

fn unpack(bytes: Option<Vec<u8>>) -> Result<Option<Vec<u8>>, String> {
    match bytes {
        Some(bytes) if bytes.is_empty() || bytes.len() > MAX_BYTES => {
            Err("Invalid order recovery envelope".into())
        }
        Some(bytes) if bytes == EMPTY => Ok(None),
        other => Ok(other),
    }
}

struct Permit(Arc<AtomicBool>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn committed_order_survives_reopen_and_clear_rejects_stale_writers() {
        let dir = tempfile::tempdir().unwrap();
        crate::http::runtime().unwrap().block_on(async {
            let path = super::super::native::prepare(dir.path().to_owned()).unwrap();
            let pool = super::super::native::connect(&path).await.unwrap();
            let store = OrderRecoveryStore {
                key: "order-recovery-v1:test".into(),
                busy: Arc::new(AtomicBool::new(false)),
                pool,
            };
            let bytes = br#"{"state":"unknown","original":"retained"}"#.to_vec();
            assert_eq!(store.commit_native(0, bytes.clone()).await.unwrap(), 1);
            store.pool.close().await;
            let pool = super::super::native::connect(&path).await.unwrap();
            let (revision, stored) = super::super::native::load(&pool, &store.key).await.unwrap();
            assert_eq!(revision, 1);
            assert_eq!(stored, Some(bytes));
            let store = OrderRecoveryStore { pool, ..store };
            assert!(store.commit_native(0, b"stale".to_vec()).await.is_err());
            assert_eq!(store.commit_native(1, EMPTY.to_vec()).await.unwrap(), 2);
            let (revision, stored) = super::super::native::load(&store.pool, &store.key)
                .await
                .unwrap();
            assert_eq!(revision, 2);
            assert_eq!(unpack(stored).unwrap(), None);
            assert!(
                store
                    .commit_native(0, b"stale-after-clear".to_vec())
                    .await
                    .is_err()
            );
            store.pool.close().await;
        });
    }
}
