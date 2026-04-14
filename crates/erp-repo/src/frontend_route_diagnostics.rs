use chrono::{DateTime, Utc};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::operation_logs::{
    OperationLogCreate, OperationLogsRepo, OP_ACTION_SYSTEM_FRONTEND_ROUTE_DIAGNOSTIC,
};

pub const FRONTEND_ROUTE_DIAGNOSTIC_REF_TYPE: &str = "FRONTEND_ROUTE";
pub const FRONTEND_ROUTE_SLOW_THRESHOLD_MS: i64 = 1200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendRouteDiagnosticEvent {
    pub session_id: String,
    pub path: String,
    pub route_label: Option<String>,
    pub event_type: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub prefetch_status: Option<String>,
    pub prefetch_source: Option<String>,
    pub prefetch_duration_ms: Option<i64>,
    pub error_summary: Option<String>,
    pub detail: Option<String>,
    pub event_at: i64,
    pub created_at: DateTime<Utc>,
    pub page_url: String,
    pub build_version: String,
    pub user_agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrontendRouteDiagnosticEventCreate {
    pub session_id: String,
    pub path: String,
    pub route_label: Option<String>,
    pub event_type: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub prefetch_status: Option<String>,
    pub prefetch_source: Option<String>,
    pub prefetch_duration_ms: Option<i64>,
    pub error_summary: Option<String>,
    pub detail: Option<String>,
    pub event_at: i64,
    pub page_url: String,
    pub build_version: String,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct FrontendRouteDiagnosticsFilters {
    pub path_keyword: Option<String>,
    pub status: Option<String>,
    pub event_type: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendRouteDiagnosticsSummary {
    pub total_events: i64,
    pub unique_routes: i64,
    pub ready_events: i64,
    pub timeout_events: i64,
    pub error_events: i64,
    pub slow_events: i64,
    pub prefetch_hits: i64,
    pub affected_sessions: i64,
    pub measured_first_loads: i64,
    pub successful_first_loads: i64,
    pub first_load_avg_duration_ms: Option<i64>,
    pub prefetch_short_circuit_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendRouteDiagnosticsPathSummary {
    pub path: String,
    pub route_label: Option<String>,
    pub total_events: i64,
    pub ready_events: i64,
    pub timeout_events: i64,
    pub error_events: i64,
    pub slow_events: i64,
    pub prefetch_hits: i64,
    pub avg_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub measured_first_loads: i64,
    pub successful_first_loads: i64,
    pub first_load_avg_duration_ms: Option<i64>,
    pub prefetch_short_circuit_count: i64,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrontendRouteDiagnosticsResult {
    pub summary: FrontendRouteDiagnosticsSummary,
    pub paths: Vec<FrontendRouteDiagnosticsPathSummary>,
    pub events: Vec<FrontendRouteDiagnosticEvent>,
}

pub struct FrontendRouteDiagnosticsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> FrontendRouteDiagnosticsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn ingest_events(
        &self,
        tenant_id: TenantId,
        actor_user_id: Option<UserId>,
        events: &[FrontendRouteDiagnosticEventCreate],
    ) -> anyhow::Result<usize> {
        if events.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        for event in events {
            let path = event.path.trim();
            if path.is_empty() {
                continue;
            }

            let metadata = serde_json::json!({
                "session_id": event.session_id.trim(),
                "path": path,
                "route_label": event.route_label.as_deref().map(str::trim).filter(|value| !value.is_empty()),
                "event_type": event.event_type.trim(),
                "status": event.status.trim(),
                "duration_ms": event.duration_ms,
                "prefetch_status": event.prefetch_status.as_deref().map(str::trim).filter(|value| !value.is_empty()),
                "prefetch_source": event.prefetch_source.as_deref().map(str::trim).filter(|value| !value.is_empty()),
                "prefetch_duration_ms": event.prefetch_duration_ms,
                "error_summary": event.error_summary.as_deref().map(str::trim).filter(|value| !value.is_empty()),
                "detail": event.detail.as_deref().map(str::trim).filter(|value| !value.is_empty()),
                "event_at": event.event_at,
                "page_url": event.page_url.trim(),
                "build_version": event.build_version.trim(),
                "user_agent": event.user_agent.trim(),
            });

            OperationLogsRepo::insert_with_tx(
                tx.as_mut(),
                OperationLogCreate {
                    tenant_id,
                    actor_type: if actor_user_id.is_some() {
                        "USER".to_string()
                    } else {
                        "SYSTEM".to_string()
                    },
                    actor_user_id,
                    action: OP_ACTION_SYSTEM_FRONTEND_ROUTE_DIAGNOSTIC.to_string(),
                    ref_type: Some(FRONTEND_ROUTE_DIAGNOSTIC_REF_TYPE.to_string()),
                    ref_id: Some(path.to_string()),
                    metadata: Some(metadata),
                },
            )
            .await?;
        }
        tx.commit().await?;
        Ok(events.len())
    }

    pub async fn list(
        &self,
        tenant_id: TenantId,
        filters: &FrontendRouteDiagnosticsFilters,
    ) -> anyhow::Result<FrontendRouteDiagnosticsResult> {
        let summary = self.fetch_summary(tenant_id, filters).await?;
        let paths = self.fetch_paths(tenant_id, filters).await?;
        let events = self.fetch_events(tenant_id, filters).await?;
        Ok(FrontendRouteDiagnosticsResult {
            summary,
            paths,
            events,
        })
    }

    async fn fetch_summary(
        &self,
        tenant_id: TenantId,
        filters: &FrontendRouteDiagnosticsFilters,
    ) -> anyhow::Result<FrontendRouteDiagnosticsSummary> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            with filtered as (
                select metadata, created_at, log_id
                from operation_logs
            "#,
        );
        self.push_filtered_where_clause(&mut qb, tenant_id, filters);
        qb.push(
            r#"
            ),
            first_terminal as (
                select
                    metadata->>'path' as path,
                    metadata->>'status' as status,
                    nullif(metadata->>'duration_ms', '')::bigint as duration_ms,
                    row_number() over (
                        partition by nullif(metadata->>'session_id', ''), metadata->>'path'
                        order by coalesce(nullif(metadata->>'event_at', '')::bigint, 0), created_at, log_id
                    ) as rn
                from filtered
                where metadata->>'event_type' in ('loading-ready', 'loading-timeout', 'loading-error')
                    and nullif(metadata->>'session_id', '') is not null
                    and nullif(metadata->>'path', '') is not null
            )
            select
                (select count(*)::bigint from filtered) as total_events,
                (select count(distinct metadata->>'path')::bigint from filtered) as unique_routes,
                (select count(*) filter (where metadata->>'status' = 'ready')::bigint from filtered) as ready_events,
                (select count(*) filter (where metadata->>'status' = 'timeout')::bigint from filtered) as timeout_events,
                (select count(*) filter (where metadata->>'status' = 'error')::bigint from filtered) as error_events,
                (select count(*) filter (
                    where metadata->>'status' = 'timeout'
                        or coalesce(nullif(metadata->>'duration_ms', '')::bigint, 0) >=
            "#,
        );
        qb.push_bind(FRONTEND_ROUTE_SLOW_THRESHOLD_MS);
        qb.push(
            r#"
                )::bigint from filtered) as slow_events,
                (select count(*) filter (where metadata->>'prefetch_status' = 'ready')::bigint from filtered) as prefetch_hits,
                (select count(distinct nullif(metadata->>'session_id', ''))::bigint from filtered) as affected_sessions,
                (select count(*)::bigint from first_terminal where rn = 1) as measured_first_loads,
                (select count(*) filter (where status = 'ready')::bigint from first_terminal where rn = 1) as successful_first_loads,
                (select round(avg(duration_ms)::numeric)::bigint from first_terminal where rn = 1 and status = 'ready') as first_load_avg_duration_ms,
                (select count(*)::bigint from filtered where metadata->>'event_type' = 'prefetch-skipped'
                    and coalesce(metadata->>'detail', '') in ('already-ready', 'inflight')) as prefetch_short_circuit_count
            "#,
        );

        let row: SummaryRow = qb.build_query_as().fetch_one(self.pool).await?;
        Ok(row.into_summary())
    }

    async fn fetch_paths(
        &self,
        tenant_id: TenantId,
        filters: &FrontendRouteDiagnosticsFilters,
    ) -> anyhow::Result<Vec<FrontendRouteDiagnosticsPathSummary>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            with filtered as (
                select metadata, created_at, log_id
                from operation_logs
            "#,
        );
        self.push_filtered_where_clause(&mut qb, tenant_id, filters);
        qb.push(
            r#"
            ),
            first_terminal as (
                select
                    metadata->>'path' as path,
                    metadata->>'status' as status,
                    nullif(metadata->>'duration_ms', '')::bigint as duration_ms,
                    row_number() over (
                        partition by nullif(metadata->>'session_id', ''), metadata->>'path'
                        order by coalesce(nullif(metadata->>'event_at', '')::bigint, 0), created_at, log_id
                    ) as rn
                from filtered
                where metadata->>'event_type' in ('loading-ready', 'loading-timeout', 'loading-error')
                    and nullif(metadata->>'session_id', '') is not null
                    and nullif(metadata->>'path', '') is not null
            ),
            path_aggregates as (
                select
                    metadata->>'path' as path,
                    max(nullif(metadata->>'route_label', '')) as route_label,
                    count(*)::bigint as total_events,
                    count(*) filter (where metadata->>'status' = 'ready')::bigint as ready_events,
                    count(*) filter (where metadata->>'status' = 'timeout')::bigint as timeout_events,
                    count(*) filter (where metadata->>'status' = 'error')::bigint as error_events,
                    count(*) filter (
                        where metadata->>'status' = 'timeout'
                            or coalesce(nullif(metadata->>'duration_ms', '')::bigint, 0) >=
            "#,
        );
        qb.push_bind(FRONTEND_ROUTE_SLOW_THRESHOLD_MS);
        qb.push(
            r#"
                    )::bigint as slow_events,
                    count(*) filter (where metadata->>'prefetch_status' = 'ready')::bigint as prefetch_hits,
                    round(avg(nullif(metadata->>'duration_ms', '')::numeric))::bigint as avg_duration_ms,
                    max(nullif(metadata->>'duration_ms', '')::bigint) as max_duration_ms,
                    count(*) filter (
                        where metadata->>'event_type' = 'prefetch-skipped'
                            and coalesce(metadata->>'detail', '') in ('already-ready', 'inflight')
                    )::bigint as prefetch_short_circuit_count,
                    max(created_at) as last_seen_at
                from filtered
                group by metadata->>'path'
            ),
            first_load_aggregates as (
                select
                    path,
                    count(*)::bigint as measured_first_loads,
                    count(*) filter (where status = 'ready')::bigint as successful_first_loads,
                    round((avg(duration_ms) filter (where status = 'ready'))::numeric)::bigint as first_load_avg_duration_ms
                from first_terminal
                where rn = 1
                group by path
            )
            select
                path_aggregates.path,
                path_aggregates.route_label,
                path_aggregates.total_events,
                path_aggregates.ready_events,
                path_aggregates.timeout_events,
                path_aggregates.error_events,
                path_aggregates.slow_events,
                path_aggregates.prefetch_hits,
                path_aggregates.avg_duration_ms,
                path_aggregates.max_duration_ms,
                coalesce(first_load_aggregates.measured_first_loads, 0) as measured_first_loads,
                coalesce(first_load_aggregates.successful_first_loads, 0) as successful_first_loads,
                first_load_aggregates.first_load_avg_duration_ms,
                path_aggregates.prefetch_short_circuit_count,
                path_aggregates.last_seen_at
            from path_aggregates
            left join first_load_aggregates on first_load_aggregates.path = path_aggregates.path
            order by
                case
                    when path_aggregates.error_events > 0 then 0
                    when path_aggregates.timeout_events > 0 then 1
                    when path_aggregates.slow_events > 0 then 2
                    when path_aggregates.ready_events > 0 then 3
                    else 4
                end,
                path_aggregates.last_seen_at desc,
                path_aggregates.path asc
            limit
            "#,
        );
        qb.push_bind(filters.limit);

        let rows: Vec<PathSummaryRow> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(PathSummaryRow::into_summary).collect())
    }

    async fn fetch_events(
        &self,
        tenant_id: TenantId,
        filters: &FrontendRouteDiagnosticsFilters,
    ) -> anyhow::Result<Vec<FrontendRouteDiagnosticEvent>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                coalesce(metadata->>'session_id', '') as session_id,
                metadata->>'path' as path,
                nullif(metadata->>'route_label', '') as route_label,
                metadata->>'event_type' as event_type,
                metadata->>'status' as status,
                nullif(metadata->>'duration_ms', '')::bigint as duration_ms,
                nullif(metadata->>'prefetch_status', '') as prefetch_status,
                nullif(metadata->>'prefetch_source', '') as prefetch_source,
                nullif(metadata->>'prefetch_duration_ms', '')::bigint as prefetch_duration_ms,
                nullif(metadata->>'error_summary', '') as error_summary,
                nullif(metadata->>'detail', '') as detail,
                coalesce(nullif(metadata->>'event_at', '')::bigint, 0) as event_at,
                created_at,
                coalesce(metadata->>'page_url', '') as page_url,
                coalesce(metadata->>'build_version', '') as build_version,
                coalesce(metadata->>'user_agent', '') as user_agent
            from operation_logs
            "#,
        );
        self.push_filtered_where_clause(&mut qb, tenant_id, filters);
        qb.push(
            r#"
            order by created_at desc, log_id desc
            limit
            "#,
        );
        qb.push_bind(filters.limit);

        let rows: Vec<EventRow> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(EventRow::into_event).collect())
    }

    fn push_filtered_where_clause(
        &self,
        qb: &mut QueryBuilder<'_, Postgres>,
        tenant_id: TenantId,
        filters: &FrontendRouteDiagnosticsFilters,
    ) {
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and action = ");
        qb.push_bind(OP_ACTION_SYSTEM_FRONTEND_ROUTE_DIAGNOSTIC);

        if let Some(created_from) = filters.created_from {
            qb.push(" and created_at >= ");
            qb.push_bind(created_from);
        }
        if let Some(path_keyword) = filters.path_keyword.clone() {
            qb.push(" and metadata->>'path' ilike ");
            qb.push_bind(format!("%{path_keyword}%"));
        }
        if let Some(status) = filters.status.clone() {
            qb.push(" and metadata->>'status' = ");
            qb.push_bind(status);
        }
        if let Some(event_type) = filters.event_type.clone() {
            qb.push(" and metadata->>'event_type' = ");
            qb.push_bind(event_type);
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SummaryRow {
    total_events: i64,
    unique_routes: i64,
    ready_events: i64,
    timeout_events: i64,
    error_events: i64,
    slow_events: i64,
    prefetch_hits: i64,
    affected_sessions: i64,
    measured_first_loads: i64,
    successful_first_loads: i64,
    first_load_avg_duration_ms: Option<i64>,
    prefetch_short_circuit_count: i64,
}

impl SummaryRow {
    fn into_summary(self) -> FrontendRouteDiagnosticsSummary {
        FrontendRouteDiagnosticsSummary {
            total_events: self.total_events,
            unique_routes: self.unique_routes,
            ready_events: self.ready_events,
            timeout_events: self.timeout_events,
            error_events: self.error_events,
            slow_events: self.slow_events,
            prefetch_hits: self.prefetch_hits,
            affected_sessions: self.affected_sessions,
            measured_first_loads: self.measured_first_loads,
            successful_first_loads: self.successful_first_loads,
            first_load_avg_duration_ms: self.first_load_avg_duration_ms,
            prefetch_short_circuit_count: self.prefetch_short_circuit_count,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PathSummaryRow {
    path: String,
    route_label: Option<String>,
    total_events: i64,
    ready_events: i64,
    timeout_events: i64,
    error_events: i64,
    slow_events: i64,
    prefetch_hits: i64,
    avg_duration_ms: Option<i64>,
    max_duration_ms: Option<i64>,
    measured_first_loads: i64,
    successful_first_loads: i64,
    first_load_avg_duration_ms: Option<i64>,
    prefetch_short_circuit_count: i64,
    last_seen_at: DateTime<Utc>,
}

impl PathSummaryRow {
    fn into_summary(self) -> FrontendRouteDiagnosticsPathSummary {
        FrontendRouteDiagnosticsPathSummary {
            path: self.path,
            route_label: self.route_label,
            total_events: self.total_events,
            ready_events: self.ready_events,
            timeout_events: self.timeout_events,
            error_events: self.error_events,
            slow_events: self.slow_events,
            prefetch_hits: self.prefetch_hits,
            avg_duration_ms: self.avg_duration_ms,
            max_duration_ms: self.max_duration_ms,
            measured_first_loads: self.measured_first_loads,
            successful_first_loads: self.successful_first_loads,
            first_load_avg_duration_ms: self.first_load_avg_duration_ms,
            prefetch_short_circuit_count: self.prefetch_short_circuit_count,
            last_seen_at: self.last_seen_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EventRow {
    session_id: String,
    path: String,
    route_label: Option<String>,
    event_type: String,
    status: String,
    duration_ms: Option<i64>,
    prefetch_status: Option<String>,
    prefetch_source: Option<String>,
    prefetch_duration_ms: Option<i64>,
    error_summary: Option<String>,
    detail: Option<String>,
    event_at: i64,
    created_at: DateTime<Utc>,
    page_url: String,
    build_version: String,
    user_agent: String,
}

impl EventRow {
    fn into_event(self) -> FrontendRouteDiagnosticEvent {
        FrontendRouteDiagnosticEvent {
            session_id: self.session_id,
            path: self.path,
            route_label: self.route_label,
            event_type: self.event_type,
            status: self.status,
            duration_ms: self.duration_ms,
            prefetch_status: self.prefetch_status,
            prefetch_source: self.prefetch_source,
            prefetch_duration_ms: self.prefetch_duration_ms,
            error_summary: self.error_summary,
            detail: self.detail,
            event_at: self.event_at,
            created_at: self.created_at,
            page_url: self.page_url,
            build_version: self.build_version,
            user_agent: self.user_agent,
        }
    }
}
