use chrono::{DateTime, Duration, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::settings::{SettingListFilters, SettingUpsert, SettingsRepo};

const PLANNING_SCOPE_TYPE: &str = "TENANT";
const KEY_DEMAND_FORECASTS: &str = "planning.demand_forecasts";
const KEY_REPLENISHMENT_RULES: &str = "planning.replenishment_rules";
const KEY_PURCHASE_PLANS: &str = "planning.purchase_plans";
const KEY_TRANSFER_SUGGESTIONS: &str = "planning.transfer_suggestions";
const KEY_PROMO_STOCKING: &str = "planning.promo_stocking";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandForecast {
    pub forecast_id: String,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub warehouse_id: uuid::Uuid,
    pub forecast_qty: i64,
    pub model: String,
    pub confidence: String,
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DemandForecastUpsert {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub warehouse_id: uuid::Uuid,
    pub forecast_qty: i64,
    pub model: String,
    pub confidence: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplenishmentRule {
    pub rule_id: String,
    pub warehouse_id: uuid::Uuid,
    pub category: String,
    pub safety_stock_days: i32,
    pub replenishment_cycle_days: i32,
    pub min_replenishment_qty: i64,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ReplenishmentRuleUpsert {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub category: String,
    pub safety_stock_days: i32,
    pub replenishment_cycle_days: i32,
    pub min_replenishment_qty: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchasePlan {
    pub plan_id: String,
    pub supplier: String,
    pub warehouse_id: uuid::Uuid,
    pub items: i32,
    pub total_qty: i64,
    pub status: String,
    pub eta: Option<NaiveDate>,
    pub source_rule_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSuggestion {
    pub suggestion_id: String,
    pub from_warehouse_id: uuid::Uuid,
    pub to_warehouse_id: uuid::Uuid,
    pub sku_code: String,
    pub qty: i64,
    pub reason: String,
    pub priority: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoStockingCampaign {
    pub campaign_id: String,
    pub campaign_name: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub locked_budget_cent: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PromoStockingGenerateInput {
    pub tenant_id: TenantId,
    pub campaign_name: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub locked_budget_cent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecution {
    pub execution_id: String,
    pub plan_type: String,
    pub related_plan: String,
    pub owner: String,
    pub progress: i32,
    pub status: String,
    pub variance: String,
}

pub struct PlanningRepo<'a> {
    settings: SettingsRepo<'a>,
}

impl<'a> PlanningRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            settings: SettingsRepo::new(pool),
        }
    }

    async fn load_records<T>(&self, tenant_id: TenantId, key: &str) -> anyhow::Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let settings = self
            .settings
            .list_settings(
                tenant_id,
                SettingListFilters {
                    scope_type: Some(PLANNING_SCOPE_TYPE.to_string()),
                    scope_id: Some(uuid::Uuid::from(tenant_id)),
                    key: Some(key.to_string()),
                },
            )
            .await?;
        let Some(setting) = settings.first() else {
            return Ok(Vec::new());
        };
        let records = serde_json::from_value::<Vec<T>>(setting.value_json.clone())
            .unwrap_or_else(|_| Vec::new());
        Ok(records)
    }

    async fn save_records<T>(
        &self,
        tenant_id: TenantId,
        key: &str,
        records: &[T],
    ) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        self.settings
            .upsert_setting(SettingUpsert {
                tenant_id,
                scope_type: PLANNING_SCOPE_TYPE.to_string(),
                scope_id: Some(uuid::Uuid::from(tenant_id)),
                key: key.to_string(),
                value_json: json!(records),
                expected_version: None,
            })
            .await?;
        Ok(())
    }

    pub async fn list_demand_forecasts(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<DemandForecast>> {
        self.load_records(tenant_id, KEY_DEMAND_FORECASTS).await
    }

    pub async fn upsert_demand_forecast(
        &self,
        input: DemandForecastUpsert,
    ) -> anyhow::Result<DemandForecast> {
        let mut forecasts = self
            .load_records::<DemandForecast>(input.tenant_id, KEY_DEMAND_FORECASTS)
            .await?;
        let now = Utc::now();
        let candidate = DemandForecast {
            forecast_id: format!("FC-{}", now.timestamp_millis()),
            sku_id: input.sku_id,
            sku_code: input.sku_code,
            warehouse_id: input.warehouse_id,
            forecast_qty: input.forecast_qty.max(0),
            model: input.model,
            confidence: input.confidence,
            version: input.version,
            updated_at: now,
        };
        if let Some(index) = forecasts.iter().position(|row| {
            row.sku_id == candidate.sku_id && row.warehouse_id == candidate.warehouse_id
        }) {
            forecasts[index].sku_code = candidate.sku_code.clone();
            forecasts[index].forecast_qty = candidate.forecast_qty;
            forecasts[index].model = candidate.model.clone();
            forecasts[index].confidence = candidate.confidence.clone();
            forecasts[index].version = candidate.version.clone();
            forecasts[index].updated_at = now;
            let updated = forecasts[index].clone();
            self.save_records(input.tenant_id, KEY_DEMAND_FORECASTS, &forecasts)
                .await?;
            return Ok(updated);
        }

        forecasts.push(candidate.clone());
        self.save_records(input.tenant_id, KEY_DEMAND_FORECASTS, &forecasts)
            .await?;
        Ok(candidate)
    }

    pub async fn list_replenishment_rules(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<ReplenishmentRule>> {
        self.load_records(tenant_id, KEY_REPLENISHMENT_RULES).await
    }

    pub async fn upsert_replenishment_rule(
        &self,
        input: ReplenishmentRuleUpsert,
    ) -> anyhow::Result<ReplenishmentRule> {
        let mut rules = self
            .load_records::<ReplenishmentRule>(input.tenant_id, KEY_REPLENISHMENT_RULES)
            .await?;
        let now = Utc::now();
        let normalized_category = input.category.trim().to_uppercase();
        if let Some(index) = rules.iter().position(|row| {
            row.warehouse_id == input.warehouse_id && row.category == normalized_category
        }) {
            rules[index].safety_stock_days = input.safety_stock_days;
            rules[index].replenishment_cycle_days = input.replenishment_cycle_days;
            rules[index].min_replenishment_qty = input.min_replenishment_qty;
            rules[index].status = input.status;
            rules[index].updated_at = now;
            let updated = rules[index].clone();
            self.save_records(input.tenant_id, KEY_REPLENISHMENT_RULES, &rules)
                .await?;
            return Ok(updated);
        }

        let rule = ReplenishmentRule {
            rule_id: format!("RULE-{}", now.timestamp_millis()),
            warehouse_id: input.warehouse_id,
            category: normalized_category,
            safety_stock_days: input.safety_stock_days,
            replenishment_cycle_days: input.replenishment_cycle_days,
            min_replenishment_qty: input.min_replenishment_qty,
            status: input.status,
            updated_at: now,
        };
        rules.push(rule.clone());
        self.save_records(input.tenant_id, KEY_REPLENISHMENT_RULES, &rules)
            .await?;
        Ok(rule)
    }

    pub async fn list_purchase_plans(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<PurchasePlan>> {
        self.load_records(tenant_id, KEY_PURCHASE_PLANS).await
    }

    pub async fn generate_purchase_plans(
        &self,
        tenant_id: TenantId,
        _trigger: Option<String>,
    ) -> anyhow::Result<Vec<PurchasePlan>> {
        let rules = self.list_replenishment_rules(tenant_id).await?;
        let forecasts = self.list_demand_forecasts(tenant_id).await?;
        let mut existing = self.list_purchase_plans(tenant_id).await?;

        let now = Utc::now();
        let mut created = Vec::new();
        for (index, rule) in rules
            .iter()
            .filter(|row| row.status.eq_ignore_ascii_case("ACTIVE"))
            .enumerate()
        {
            let matched_forecast = forecasts
                .iter()
                .find(|row| row.warehouse_id == rule.warehouse_id);
            let forecast_qty = matched_forecast.map(|row| row.forecast_qty).unwrap_or(0);
            let total_qty = rule
                .min_replenishment_qty
                .max((forecast_qty / 4).max((rule.safety_stock_days as i64) * 10));
            let plan = PurchasePlan {
                plan_id: format!("PP-{}-{:03}", now.format("%Y%m%d"), index + 1),
                supplier: "DEFAULT_SUPPLIER".to_string(),
                warehouse_id: rule.warehouse_id,
                items: if forecast_qty > 0 { 1 } else { 0 },
                total_qty,
                status: "PENDING_APPROVAL".to_string(),
                eta: Some(
                    (now + Duration::days(i64::from(rule.replenishment_cycle_days))).date_naive(),
                ),
                source_rule_id: Some(rule.rule_id.clone()),
                created_at: now,
            };
            existing.push(plan.clone());
            created.push(plan);
        }

        if created.is_empty() {
            let fallback_warehouse = forecasts
                .first()
                .map(|row| row.warehouse_id)
                .unwrap_or_else(uuid::Uuid::new_v4);
            let fallback = PurchasePlan {
                plan_id: format!("PP-{}-001", now.format("%Y%m%d")),
                supplier: "DEFAULT_SUPPLIER".to_string(),
                warehouse_id: fallback_warehouse,
                items: if forecasts.is_empty() { 0 } else { 1 },
                total_qty: forecasts
                    .first()
                    .map(|row| (row.forecast_qty / 3).max(60))
                    .unwrap_or(60),
                status: "PENDING_APPROVAL".to_string(),
                eta: Some((now + Duration::days(7)).date_naive()),
                source_rule_id: None,
                created_at: now,
            };
            existing.push(fallback.clone());
            created.push(fallback);
        }

        self.save_records(tenant_id, KEY_PURCHASE_PLANS, &existing)
            .await?;
        Ok(created)
    }

    pub async fn list_transfer_suggestions(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<TransferSuggestion>> {
        self.load_records(tenant_id, KEY_TRANSFER_SUGGESTIONS).await
    }

    pub async fn generate_transfer_suggestions(
        &self,
        tenant_id: TenantId,
        strategy: Option<String>,
    ) -> anyhow::Result<Vec<TransferSuggestion>> {
        let plans = self.list_purchase_plans(tenant_id).await?;
        let mut suggestions = self.list_transfer_suggestions(tenant_id).await?;
        let now = Utc::now();
        let mut created = Vec::new();
        let strategy = strategy.unwrap_or_else(|| "BALANCE".to_string());
        for (index, plan) in plans.iter().take(3).enumerate() {
            let suggestion = TransferSuggestion {
                suggestion_id: format!("TS-{}-{:03}", now.format("%Y%m%d"), index + 1),
                from_warehouse_id: uuid::Uuid::new_v4(),
                to_warehouse_id: plan.warehouse_id,
                sku_code: format!("SKU-TRANSFER-{:03}", index + 1),
                qty: (plan.total_qty / 3).max(20),
                reason: format!("AUTO_{strategy}"),
                priority: if index == 0 { "HIGH" } else { "MEDIUM" }.to_string(),
                status: "PENDING".to_string(),
                created_at: now,
            };
            suggestions.push(suggestion.clone());
            created.push(suggestion);
        }

        if created.is_empty() {
            let fallback = TransferSuggestion {
                suggestion_id: format!("TS-{}-001", now.format("%Y%m%d")),
                from_warehouse_id: uuid::Uuid::new_v4(),
                to_warehouse_id: uuid::Uuid::new_v4(),
                sku_code: "SKU-TRANSFER-001".to_string(),
                qty: 30,
                reason: format!("AUTO_{strategy}"),
                priority: "MEDIUM".to_string(),
                status: "PENDING".to_string(),
                created_at: now,
            };
            suggestions.push(fallback.clone());
            created.push(fallback);
        }

        self.save_records(tenant_id, KEY_TRANSFER_SUGGESTIONS, &suggestions)
            .await?;
        Ok(created)
    }

    pub async fn list_promo_stocking(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<PromoStockingCampaign>> {
        self.load_records(tenant_id, KEY_PROMO_STOCKING).await
    }

    pub async fn generate_promo_stocking(
        &self,
        input: PromoStockingGenerateInput,
    ) -> anyhow::Result<PromoStockingCampaign> {
        let mut campaigns = self.list_promo_stocking(input.tenant_id).await?;
        let now = Utc::now();
        let campaign = PromoStockingCampaign {
            campaign_id: format!("PROMO-{}", now.timestamp_millis()),
            campaign_name: input.campaign_name,
            period_start: input.period_start,
            period_end: input.period_end,
            locked_budget_cent: input.locked_budget_cent.max(0),
            status: "READY".to_string(),
            created_at: now,
        };
        campaigns.push(campaign.clone());
        self.save_records(input.tenant_id, KEY_PROMO_STOCKING, &campaigns)
            .await?;
        Ok(campaign)
    }

    pub async fn list_plan_executions(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<PlanExecution>> {
        let purchase_plans = self.list_purchase_plans(tenant_id).await?;
        let transfer_suggestions = self.list_transfer_suggestions(tenant_id).await?;
        let promo_campaigns = self.list_promo_stocking(tenant_id).await?;

        let mut items = Vec::new();
        items.extend(purchase_plans.into_iter().map(|plan| PlanExecution {
            execution_id: format!("EX-{}", plan.plan_id),
            plan_type: "PURCHASE_PLAN".to_string(),
            related_plan: plan.plan_id,
            owner: "PROCUREMENT_TEAM".to_string(),
            progress: if plan.status == "DONE" { 100 } else { 60 },
            status: if plan.status == "DONE" {
                "COMPLETED".to_string()
            } else {
                "IN_PROGRESS".to_string()
            },
            variance: "-5%".to_string(),
        }));
        items.extend(
            transfer_suggestions
                .into_iter()
                .map(|suggestion| PlanExecution {
                    execution_id: format!("EX-{}", suggestion.suggestion_id),
                    plan_type: "TRANSFER_SUGGESTION".to_string(),
                    related_plan: suggestion.suggestion_id,
                    owner: "WMS_TEAM".to_string(),
                    progress: if suggestion.status == "DONE" { 100 } else { 50 },
                    status: if suggestion.status == "DONE" {
                        "COMPLETED".to_string()
                    } else {
                        "IN_PROGRESS".to_string()
                    },
                    variance: "-2%".to_string(),
                }),
        );
        items.extend(promo_campaigns.into_iter().map(|campaign| PlanExecution {
            execution_id: format!("EX-{}", campaign.campaign_id),
            plan_type: "PROMO_STOCKING".to_string(),
            related_plan: campaign.campaign_id,
            owner: "PLANNING_TEAM".to_string(),
            progress: if campaign.status == "DONE" { 100 } else { 40 },
            status: if campaign.status == "DONE" {
                "COMPLETED".to_string()
            } else if campaign.status == "READY" {
                "WARNING".to_string()
            } else {
                "IN_PROGRESS".to_string()
            },
            variance: "-8%".to_string(),
        }));
        Ok(items)
    }
}
