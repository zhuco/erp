//! FEFO consumes only the caller's actual exact SKU effect in the same transaction.
use crate::inventory_precision::{InventoryPreciseEffect, InventoryPrecisionError};
use sqlx::{Postgres, Transaction};

pub(crate) async fn consume(
    tx: &mut Transaction<'_, Postgres>,
    effect: &InventoryPreciseEffect,
) -> anyhow::Result<()> {
    if effect.balance.on_hand >= effect.before.on_hand {
        return Err(InventoryPrecisionError::InvalidChange("batch consumption").into());
    }
    crate::inventory_batch_effects::apply(tx, effect, None).await?;
    Ok(())
}
