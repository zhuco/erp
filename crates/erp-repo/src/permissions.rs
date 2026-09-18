use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::HashSet;

fn normalize_non_blank(field: &str, value: &str) -> anyhow::Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(anyhow::anyhow!("{field} must not be blank"));
    }
    Ok(normalized.to_string())
}

fn normalize_permission_items(
    items: &[PermissionDefinition],
) -> anyhow::Result<Vec<PermissionDefinition>> {
    let mut seen_codes = HashSet::new();
    let mut normalized_items = Vec::with_capacity(items.len());
    for item in items {
        let code = normalize_non_blank("permission code", &item.code)?;
        let name = normalize_non_blank("permission name", &item.name)?;
        let module = normalize_non_blank("permission module", &item.module)?;
        let description = item.description.trim().to_string();
        if seen_codes.insert(code.clone()) {
            normalized_items.push(PermissionDefinition {
                code,
                name,
                module,
                description,
            });
        }
    }
    Ok(normalized_items)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PermissionDefinition {
    pub code: String,
    pub name: String,
    pub module: String,
    pub description: String,
}

pub struct PermissionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PermissionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_permissions(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<PermissionDefinition>> {
        let rows: Vec<PermissionDefinition> = sqlx::query_as(
            r#"
            select code, name, module, description
            from permissions
            where tenant_id = $1
            order by module, code
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn seed_permissions(
        &self,
        tenant_id: TenantId,
        items: &[PermissionDefinition],
    ) -> anyhow::Result<()> {
        let normalized_items = normalize_permission_items(items)?;
        if normalized_items.is_empty() {
            return Ok(());
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            "insert into permissions (tenant_id, code, name, module, description) ",
        );
        qb.push_values(normalized_items, |mut builder, item| {
            builder
                .push_bind(uuid::Uuid::from(tenant_id))
                .push_bind(item.code)
                .push_bind(item.name)
                .push_bind(item.module)
                .push_bind(item.description);
        });
        qb.push(" on conflict (tenant_id, code) do nothing");
        qb.build().execute(self.pool).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{PermissionDefinition, normalize_permission_items};

    #[test]
    fn normalize_permission_items_rejects_blank_code() {
        let items = vec![PermissionDefinition {
            code: " ".to_string(),
            name: "name".to_string(),
            module: "module".to_string(),
            description: "".to_string(),
        }];
        assert!(normalize_permission_items(&items).is_err());
    }

    #[test]
    fn normalize_permission_items_deduplicates_and_trims() {
        let items = vec![
            PermissionDefinition {
                code: " orders.read ".to_string(),
                name: " 读取 ".to_string(),
                module: " 订单 ".to_string(),
                description: " 描述 ".to_string(),
            },
            PermissionDefinition {
                code: "orders.read".to_string(),
                name: "忽略重复".to_string(),
                module: "订单".to_string(),
                description: "".to_string(),
            },
        ];
        let normalized = normalize_permission_items(&items).expect("should normalize");
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].code, "orders.read");
        assert_eq!(normalized[0].name, "读取");
        assert_eq!(normalized[0].module, "订单");
        assert_eq!(normalized[0].description, "描述");
    }
}
