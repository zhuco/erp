//! Bounded SKU fill preview from one server repeatable-read query snapshot.
use super::{Session, cross_page_fill::CrossPageFillRange, details};
use crate::{
    http::{HttpError, HttpRequest, HttpTask},
    remote_paste::{Preview, plan_indexed_fill},
};
use eframe::egui;
use erp_grid_core::{
    CellRect, Column, RowId,
    paged_query::PageQuery,
    remote::{PageAddress, RemoteQuery, RemoteRow},
};
use serde::{Deserialize, Serialize};

const MAX_TARGET_ROWS: usize = 1_000;
const MAX_CELLS: usize = 10_000;
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Serialize)]
struct Range {
    offset: u64,
    limit: u16,
}

#[derive(Serialize)]
struct Request {
    query: PageQuery,
    ranges: Vec<Range>,
}

#[derive(Deserialize)]
struct Response {
    total: u64,
    pages: Vec<ResponsePage>,
}

#[derive(Deserialize)]
struct ResponsePage {
    offset: u64,
    data: serde_json::Value,
}

pub(super) struct DetailsFill {
    parent: RowId,
    generation: u64,
    query: RemoteQuery,
    source: CellRect,
    target: CellRect,
    source_identity: Vec<(RowId, Option<String>)>,
    columns: Vec<Column>,
    task: Option<HttpTask>,
    preview: Option<Preview>,
    error: Option<String>,
    authentication_failed: bool,
}

impl DetailsFill {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start(
        session: &Session,
        parent: RowId,
        generation: u64,
        query: RemoteQuery,
        page_start: u64,
        source_local: CellRect,
        current_rows: &[RemoteRow],
        columns: &[Column],
        first: u64,
        last: u64,
        ctx: &egui::Context,
    ) -> Result<Self, String> {
        let target_len = validate_range(first, last)?;
        if source_local.top >= source_local.bottom
            || source_local.right <= source_local.left
            || source_local.bottom > current_rows.len()
            || source_local.right > columns.len()
        {
            return Err("填充源不在当前已加载 SKU 页内".into());
        }
        let width = source_local.right - source_local.left;
        if target_len
            .checked_mul(width)
            .is_none_or(|cells| cells > MAX_CELLS)
        {
            return Err("跨页填充最多包含 10000 个目标单元格".into());
        }
        let source_top = page_start
            .checked_add(source_local.top as u64)
            .ok_or("SKU 源行号溢出")?;
        let source_bottom = page_start
            .checked_add(source_local.bottom as u64)
            .ok_or("SKU 源行号溢出")?;
        let source = CellRect {
            top: usize::try_from(source_top).map_err(|_| "SKU 源行号溢出")?,
            left: source_local.left,
            bottom: usize::try_from(source_bottom).map_err(|_| "SKU 源行号溢出")?,
            right: source_local.right,
        };
        let target = CellRect {
            top: usize::try_from(first - 1).map_err(|_| "SKU 目标行号溢出")?,
            left: source.left,
            bottom: usize::try_from(last).map_err(|_| "SKU 目标行号溢出")?,
            right: source.right,
        };
        crate::remote_paste::fill_axis(source, target)?;
        let source_identity = current_rows[source_local.top..source_local.bottom]
            .iter()
            .map(|row| (row.id, row.version.clone()))
            .collect::<Vec<_>>();
        if source_identity
            .iter()
            .any(|(id, version)| id.0 == 0 || version.as_deref().is_none_or(str::is_empty))
        {
            return Err("SKU 填充源缺少稳定身份或版本".into());
        }
        let page_query = PageQuery {
            schema: query.scope.schema,
            offset: 0,
            limit: 1,
            filters: query.filters.clone(),
            sorts: query.sorts.clone(),
            search: query.search.clone(),
            expression: query.expression.as_deref().cloned(),
        };
        page_query.validate().map_err(str::to_owned)?;
        let ranges = vec![
            range(source.top, source.bottom)?,
            range(target.top, target.bottom)?,
        ];
        let body = serde_json::to_vec(&Request {
            query: page_query,
            ranges,
        })
        .map_err(|_| "无法编码 SKU 跨页填充请求")?;
        let task = session
            .client
            .start(
                HttpRequest {
                    path: format!(
                        "/v1/tenants/{}/products/{}/skus/query-ranges",
                        session.tenant.to_uuid(),
                        parent.to_uuid()
                    ),
                    query: vec![],
                    json: Some(body),
                    bearer: Some(session.token.clone()),
                },
                ctx,
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            parent,
            generation,
            query,
            source,
            target,
            source_identity,
            columns: columns.to_vec(),
            task: Some(task),
            preview: None,
            error: None,
            authentication_failed: false,
        })
    }

    pub(super) fn poll(
        &mut self,
        session: &Session,
        parent: RowId,
        generation: u64,
        query: &RemoteQuery,
    ) {
        if self.parent != parent || self.generation != generation || &self.query != query {
            self.task = None;
            self.error = Some("SKU 查询、排序或父商品已变化，请重新准备填充".into());
            return;
        }
        let Some(result) = self.task.as_mut().and_then(HttpTask::poll) else {
            return;
        };
        self.task = None;
        if matches!(result, Err(HttpError::Status(401 | 403))) {
            self.authentication_failed = true;
        }
        match result
            .map_err(|error| error.to_string())
            .and_then(|bytes| self.decode(session.tenant, &bytes))
        {
            Ok(preview) => self.preview = Some(preview),
            Err(error) => self.error = Some(error),
        }
    }

    fn decode(&self, tenant: RowId, bytes: &[u8]) -> Result<Preview, String> {
        if bytes.len() > MAX_BYTES {
            return Err("SKU 跨页填充响应超过 8 MiB".into());
        }
        let response: Response =
            serde_json::from_slice(bytes).map_err(|_| "SKU 跨页填充响应无效")?;
        if self.target.bottom as u64 > response.total || response.pages.len() != 2 {
            return Err("SKU 跨页填充响应范围不完整".into());
        }
        let source_rows = decode_range(
            tenant,
            self.parent,
            &response.pages[0],
            self.source.top,
            self.source.bottom,
            response.total,
        )?;
        let target_rows = decode_range(
            tenant,
            self.parent,
            &response.pages[1],
            self.target.top,
            self.target.bottom,
            response.total,
        )?;
        for (row, (id, version)) in source_rows.iter().zip(&self.source_identity) {
            if row.id != *id || !same_version(row.version.as_deref(), version.as_deref()) {
                return Err("SKU 填充源已变化，请重新框选".into());
            }
        }
        let indexed = target_rows
            .into_iter()
            .enumerate()
            .map(|(offset, row)| (self.target.top + offset, row))
            .collect::<Vec<_>>();
        plan_indexed_fill(
            "skus.update",
            self.source,
            &source_rows,
            self.target,
            &indexed,
            &self.columns,
        )
    }

    pub(super) fn busy(&self) -> bool {
        self.task.is_some()
    }
    pub(super) fn cancel(&mut self) {
        self.task = None;
        self.preview = None;
    }
    pub(super) fn take_preview(&mut self) -> Option<(Preview, Vec<Column>)> {
        self.preview
            .take()
            .map(|preview| (preview, self.columns.clone()))
    }
    pub(super) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub(super) fn authentication_failed(&self) -> bool {
        self.authentication_failed
    }
}

fn range(start: usize, end: usize) -> Result<Range, String> {
    Ok(Range {
        offset: u64::try_from(start).map_err(|_| "SKU 填充范围溢出")?,
        limit: u16::try_from(end - start).map_err(|_| "SKU 填充范围过大")?,
    })
}

fn validate_range(first: u64, last: u64) -> Result<usize, String> {
    if first == 0 {
        return Err("起始行必须是正整数".into());
    }
    last.checked_sub(first)
        .and_then(|v| v.checked_add(1))
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| (1..=MAX_TARGET_ROWS).contains(v))
        .ok_or_else(|| "跨页填充目标必须为连续 1 至 1000 行".into())
}

fn decode_range(
    tenant: RowId,
    parent: RowId,
    page: &ResponsePage,
    start: usize,
    end: usize,
    total: u64,
) -> Result<Vec<RemoteRow>, String> {
    if page.offset != start as u64 {
        return Err("SKU 跨页填充响应偏移不匹配".into());
    }
    let data = page.data.as_array().ok_or("SKU 跨页填充页数据无效")?;
    if data.len() != end - start {
        return Err("SKU 跨页填充页行数不完整".into());
    }
    let body = serde_json::to_vec(&serde_json::json!({"total": total, "data": data}))
        .map_err(|_| "SKU 跨页填充页无法解码")?;
    Ok(details::decode(
        tenant,
        parent,
        &body,
        &PageAddress::Offset {
            start: start as u64,
            limit: u16::try_from(end - start).map_err(|_| "SKU 填充范围过大")?,
        },
    )?
    .rows)
}

fn same_version(left: Option<&str>, right: Option<&str>) -> bool {
    fn utc(value: &str) -> Option<(&str, &str)> {
        let value = value
            .strip_suffix('Z')
            .or_else(|| value.strip_suffix("+00:00"))?;
        let (seconds, fraction) = value.split_once('.').unwrap_or((value, ""));
        Some((seconds, fraction.trim_end_matches('0')))
    }
    matches!((left, right), (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() && (a == b || utc(a).is_some_and(|parts| Some(parts) == utc(b))))
}

pub(super) fn range_ui() -> CrossPageFillRange {
    CrossPageFillRange::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_and_versions_are_strict() {
        assert_eq!(validate_range(1, 1000), Ok(1000));
        assert!(validate_range(0, 1).is_err());
        assert!(validate_range(1, 1001).is_err());
        assert!(same_version(
            Some("2026-09-10T00:00:00.120Z"),
            Some("2026-09-10T00:00:00.12+00:00")
        ));
        assert!(!same_version(
            Some("2026-09-10T00:00:00.120001Z"),
            Some("2026-09-10T00:00:00.120002Z")
        ));
    }
}
