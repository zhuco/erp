use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuSerial {
    pub serial_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub serial_no: String,
    pub status: String,
    pub ref_doc_type: Option<String>,
    pub ref_doc_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SkuSerialCursor {
    pub created_at: DateTime<Utc>,
    pub serial_id: uuid::Uuid,
}

impl SkuSerialCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.serial_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let serial_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            serial_id,
        })
    }
}

pub struct SkuSerialListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub serial_no: Option<String>,
    pub cursor: Option<SkuSerialCursor>,
    pub limit: i64,
}

pub struct SkuSerialListResult {
    pub items: Vec<SkuSerial>,
    pub next_cursor: Option<SkuSerialCursor>,
}

pub struct SkuSerialGenerateInput {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub prefix: String,
    pub count: i64,
    pub seq_length: i32,
    pub increment_step: i32,
}

pub struct ShipmentSerialBindInput {
    pub tenant_id: TenantId,
    pub shipment_id: uuid::Uuid,
    pub order_id: uuid::Uuid,
    pub order_item_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub serial_nos: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ShipmentSerialCount {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

pub struct SkuSerialsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkuSerialsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_serials(
        &self,
        tenant_id: TenantId,
        filters: SkuSerialListFilters,
    ) -> anyhow::Result<SkuSerialListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                serial_id, sku_id, serial_no, status,
                ref_doc_type, ref_doc_id, created_at, updated_at
            from sku_serials
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(sku_id) = filters.sku_id {
            qb.push(" and sku_id = ");
            qb.push_bind(sku_id);
        }
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(serial_no) = filters.serial_no {
            qb.push(" and serial_no = ");
            qb.push_bind(serial_no);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, serial_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.serial_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, serial_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            serial_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            serial_no: String,
            status: String,
            ref_doc_type: Option<String>,
            ref_doc_id: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| SkuSerialCursor {
            created_at: row.created_at,
            serial_id: row.serial_id,
        });

        let items = rows
            .into_iter()
            .map(|row| SkuSerial {
                serial_id: row.serial_id,
                tenant_id,
                sku_id: row.sku_id,
                serial_no: row.serial_no,
                status: row.status,
                ref_doc_type: row.ref_doc_type,
                ref_doc_id: row.ref_doc_id,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SkuSerialListResult { items, next_cursor })
    }

    pub async fn generate_serials(
        &self,
        input: SkuSerialGenerateInput,
    ) -> anyhow::Result<Vec<SkuSerial>> {
        if input.count <= 0 {
            return Err(anyhow!("count must be positive"));
        }
        if input.seq_length <= 0 {
            return Err(anyhow!("seq_length must be positive"));
        }
        if input.increment_step <= 0 {
            return Err(anyhow!("increment_step must be positive"));
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let now = Utc::now();
        let date_key = now.format("%Y%m%d%H%M%S").to_string();

        let last_seq: i64 = sqlx::query_scalar(
            r#"
            insert into sku_serial_sequences(
                tenant_id, sku_id, prefix, date_key, last_seq
            ) values ($1,$2,$3,$4,$5)
            on conflict (tenant_id, sku_id, prefix, date_key)
            do update set
                last_seq = sku_serial_sequences.last_seq + $5,
                updated_at = now()
            returning last_seq
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.prefix)
        .bind(&date_key)
        .bind(input.count * i64::from(input.increment_step))
        .fetch_one(&mut *tx)
        .await?;

        let start_seq = last_seq - (input.count * i64::from(input.increment_step)) + 1;
        let mut serial_nos = Vec::with_capacity(input.count as usize);
        for offset in 0..input.count {
            let seq = start_seq + offset * i64::from(input.increment_step);
            serial_nos.push(format!(
                "{}{}{:0width$}",
                input.prefix,
                date_key,
                seq,
                width = input.seq_length as usize
            ));
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            "insert into sku_serials(tenant_id, sku_id, serial_no, status) ",
        );
        qb.push_values(serial_nos.iter(), |mut row, serial_no| {
            row.push_bind(uuid::Uuid::from(input.tenant_id));
            row.push_bind(input.sku_id);
            row.push_bind(serial_no);
            row.push_bind("IN_STOCK");
        });
        qb.push(
            " returning serial_id, sku_id, serial_no, status, ref_doc_type, ref_doc_id, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            serial_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            serial_no: String,
            status: String,
            ref_doc_type: Option<String>,
            ref_doc_id: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(&mut *tx).await?;
        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|row| SkuSerial {
                serial_id: row.serial_id,
                tenant_id: input.tenant_id,
                sku_id: row.sku_id,
                serial_no: row.serial_no,
                status: row.status,
                ref_doc_type: row.ref_doc_type,
                ref_doc_id: row.ref_doc_id,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn bind_serials_to_shipment(
        &self,
        input: ShipmentSerialBindInput,
    ) -> anyhow::Result<Vec<SkuSerial>> {
        if input.serial_nos.is_empty() {
            return Ok(Vec::new());
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let mut bound = Vec::with_capacity(input.serial_nos.len());

        #[derive(sqlx::FromRow)]
        struct Row {
            serial_id: uuid::Uuid,
            serial_no: String,
            status: String,
            ref_doc_id: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        for serial_no in &input.serial_nos {
            let row: Row = sqlx::query_as(
                r#"
                select serial_id, serial_no, status, ref_doc_id, created_at, updated_at
                from sku_serials
                where tenant_id = $1 and sku_id = $2 and serial_no = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.sku_id)
            .bind(serial_no)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("serial not found"))?;

            if row.status != "IN_STOCK"
                && !(row.status == "ALLOCATED"
                    && row.ref_doc_id.as_deref() == Some(&input.shipment_id.to_string()))
            {
                return Err(anyhow!("serial not available"));
            }

            sqlx::query(
                r#"
                update sku_serials
                set status = 'ALLOCATED',
                    ref_doc_type = 'SHIPMENT',
                    ref_doc_id = $4,
                    updated_at = now()
                where tenant_id = $1 and serial_id = $2 and sku_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(row.serial_id)
            .bind(input.sku_id)
            .bind(input.shipment_id.to_string())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                insert into shipment_item_serials(
                    tenant_id, shipment_id, order_id, order_item_id, sku_id, serial_id, serial_no
                ) values ($1,$2,$3,$4,$5,$6,$7)
                on conflict do nothing
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.shipment_id)
            .bind(input.order_id)
            .bind(input.order_item_id)
            .bind(input.sku_id)
            .bind(row.serial_id)
            .bind(&row.serial_no)
            .execute(&mut *tx)
            .await?;

            bound.push(SkuSerial {
                serial_id: row.serial_id,
                tenant_id: input.tenant_id,
                sku_id: input.sku_id,
                serial_no: row.serial_no,
                status: "ALLOCATED".to_string(),
                ref_doc_type: Some("SHIPMENT".to_string()),
                ref_doc_id: Some(input.shipment_id.to_string()),
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        tx.commit().await?;
        Ok(bound)
    }

    pub async fn count_shipment_serials(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<ShipmentSerialCount>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            qty: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select sku_id, count(*) as qty
            from shipment_item_serials
            where tenant_id = $1 and shipment_id = $2
            group by sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ShipmentSerialCount {
                sku_id: row.sku_id,
                qty: row.qty,
            })
            .collect())
    }

    pub async fn mark_shipment_serials_shipped(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update sku_serials
            set status = 'SHIPPED',
                updated_at = now()
            where tenant_id = $1
              and serial_id in (
                select serial_id
                from shipment_item_serials
                where tenant_id = $1 and shipment_id = $2
              )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
