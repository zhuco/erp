use anyhow::{anyhow, bail};
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone)]
pub struct StockMoveDoc {
    pub move_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub from_location_id: Option<uuid::Uuid>,
    pub to_location_id: Option<uuid::Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockMoveItem {
    pub move_item_id: uuid::Uuid,
    pub move_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockMoveDetail {
    pub doc: StockMoveDoc,
    pub items: Vec<StockMoveItem>,
}

#[derive(Debug, Clone)]
pub struct StockMoveItemInput {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct StockMoveCreateInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub from_location_id: Option<uuid::Uuid>,
    pub to_location_id: Option<uuid::Uuid>,
    pub items: Vec<StockMoveItemInput>,
}

#[derive(Debug, Clone)]
pub struct StockMoveListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct StockMoveListResult {
    pub items: Vec<StockMoveDoc>,
    pub total: i64,
}

pub struct StockMovesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockMovesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_move(&self, input: StockMoveCreateInput) -> anyhow::Result<StockMoveDoc> {
        if input.items.is_empty() {
            bail!("move items required");
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockMoveDocRow = sqlx::query_as(
            r#"
            insert into stock_move_docs(
                tenant_id, warehouse_id, from_location_id, to_location_id, status
            ) values ($1,$2,$3,$4,'DRAFT')
            returning move_id, tenant_id, warehouse_id, from_location_id, to_location_id,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(input.from_location_id)
        .bind(input.to_location_id)
        .fetch_one(&mut *tx)
        .await?;

        for item in input.items {
            if item.qty <= 0 {
                bail!("qty must be positive");
            }
            sqlx::query(
                r#"
                insert into stock_move_items(
                    tenant_id, move_id, sku_id, qty
                ) values ($1,$2,$3,$4)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(row.move_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(row.into_doc())
    }

    pub async fn list_moves(
        &self,
        tenant_id: TenantId,
        filters: StockMoveListFilters,
    ) -> anyhow::Result<StockMoveListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from stock_move_docs
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_move_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select move_id, tenant_id, warehouse_id, from_location_id, to_location_id,
                   status, created_at, updated_at
            from stock_move_docs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_move_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, move_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<StockMoveDocRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows.into_iter().map(StockMoveDocRow::into_doc).collect();

        Ok(StockMoveListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        move_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockMoveDetail>> {
        let row: Option<StockMoveDocRow> = sqlx::query_as(
            r#"
            select move_id, tenant_id, warehouse_id, from_location_id, to_location_id,
                   status, created_at, updated_at
            from stock_move_docs
            where tenant_id = $1 and move_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(move_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let items = self.list_items(tenant_id, move_id).await?;
        Ok(Some(StockMoveDetail {
            doc: row.into_doc(),
            items,
        }))
    }

    pub async fn list_items(
        &self,
        tenant_id: TenantId,
        move_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<StockMoveItem>> {
        let rows: Vec<StockMoveItemRow> = sqlx::query_as(
            r#"
            select move_item_id, move_id, sku_id, qty, created_at, updated_at
            from stock_move_items
            where tenant_id = $1 and move_id = $2
            order by move_item_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(move_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(StockMoveItemRow::into_item).collect())
    }

    pub async fn apply_move(
        &self,
        tenant_id: TenantId,
        move_id: uuid::Uuid,
    ) -> anyhow::Result<StockMoveDoc> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockMoveDocRow = sqlx::query_as(
            r#"
            select move_id, tenant_id, warehouse_id, from_location_id, to_location_id,
                   status, created_at, updated_at
            from stock_move_docs
            where tenant_id = $1 and move_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(move_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("move doc not found"))?;

        if row.status == "APPLIED" {
            return Ok(row.into_doc());
        }
        if row.status == "CANCELLED" {
            return Err(anyhow!("move doc cancelled"));
        }

        let from_location_id = row
            .from_location_id
            .ok_or_else(|| anyhow!("missing from_location_id"))?;
        let to_location_id = row
            .to_location_id
            .ok_or_else(|| anyhow!("missing to_location_id"))?;

        let items = self.list_items(tenant_id, move_id).await?;
        if items.is_empty() {
            return Err(anyhow!("move items missing"));
        }

        let occurred_at = Utc::now();
        for item in &items {
            let balance: LocationBalanceRow = sqlx::query_as(
                r#"
                select on_hand
                from inventory_location_balances
                where tenant_id = $1 and warehouse_id = $2
                  and location_id = $3 and sku_id = $4
                for update
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.warehouse_id)
            .bind(from_location_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("missing source location balance"))?;

            if balance.on_hand < item.qty {
                return Err(anyhow!("location balance insufficient"));
            }

            sqlx::query(
                r#"
                update inventory_location_balances
                set on_hand = on_hand - $5,
                    updated_at = now()
                where tenant_id = $1 and warehouse_id = $2
                  and location_id = $3 and sku_id = $4
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.warehouse_id)
            .bind(from_location_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                insert into inventory_location_balances(
                    tenant_id, warehouse_id, location_id, sku_id, on_hand
                ) values ($1,$2,$3,$4,$5)
                on conflict (tenant_id, warehouse_id, location_id, sku_id)
                do update set
                    on_hand = inventory_location_balances.on_hand + excluded.on_hand,
                    updated_at = now()
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.warehouse_id)
            .bind(to_location_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                insert into inventory_ledgers(
                    tenant_id, warehouse_id, sku_id,
                    delta_on_hand, delta_reserved, delta_in_transit, delta_frozen,
                    reason_type, ref_type, ref_id, occurred_at
                ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.warehouse_id)
            .bind(item.sku_id)
            .bind(0_i64)
            .bind(0_i64)
            .bind(0_i64)
            .bind(0_i64)
            .bind("STOCK_MOVE_APPLY")
            .bind(Some("STOCK_MOVE".to_string()))
            .bind(Some(move_id.to_string()))
            .bind(occurred_at)
            .execute(&mut *tx)
            .await?;
        }

        let updated: StockMoveDocRow = sqlx::query_as(
            r#"
            update stock_move_docs
            set status = 'APPLIED',
                updated_at = now()
            where tenant_id = $1 and move_id = $2
            returning move_id, tenant_id, warehouse_id, from_location_id, to_location_id,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(move_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.into_doc())
    }

    pub async fn cancel_move(
        &self,
        tenant_id: TenantId,
        move_id: uuid::Uuid,
    ) -> anyhow::Result<StockMoveDoc> {
        let row: StockMoveDocRow = sqlx::query_as(
            r#"
            update stock_move_docs
            set status = 'CANCELLED',
                updated_at = now()
            where tenant_id = $1 and move_id = $2
            returning move_id, tenant_id, warehouse_id, from_location_id, to_location_id,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(move_id)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_doc())
    }
}

fn apply_move_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a StockMoveListFilters) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
}

#[derive(sqlx::FromRow)]
struct StockMoveDocRow {
    move_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    from_location_id: Option<uuid::Uuid>,
    to_location_id: Option<uuid::Uuid>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockMoveDocRow {
    fn into_doc(self) -> StockMoveDoc {
        StockMoveDoc {
            move_id: self.move_id,
            tenant_id: TenantId(self.tenant_id),
            warehouse_id: self.warehouse_id,
            from_location_id: self.from_location_id,
            to_location_id: self.to_location_id,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StockMoveItemRow {
    move_item_id: uuid::Uuid,
    move_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockMoveItemRow {
    fn into_item(self) -> StockMoveItem {
        StockMoveItem {
            move_item_id: self.move_item_id,
            move_id: self.move_id,
            sku_id: self.sku_id,
            qty: self.qty,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct LocationBalanceRow {
    on_hand: i64,
}
