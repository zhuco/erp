use super::Session;
use crate::http::{HttpError, HttpRequest, HttpTask, MAX_BODY};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, LoadTicket, RowId, TreeModel, TreeNode};
use erp_ui_egui::{DataGrid, GridState, TreeRowVisual};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use uuid::Uuid;

const PAGE_LIMIT: u16 = 100;
const MAX_PENDING: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct Cursor {
    code: String,
    id: Uuid,
}

#[derive(serde::Serialize)]
struct Request<'a> {
    schema: u8,
    parent: Option<Uuid>,
    after: Option<&'a Cursor>,
    limit: u16,
    search: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct Response {
    items: Vec<WireItem>,
    total: u64,
    next: Option<Cursor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EntityKind {
    Warehouse,
    Location,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum WireKind {
    Warehouse,
    Location,
}

#[derive(serde::Deserialize)]
struct WireItem {
    id: Uuid,
    parent_id: Option<Uuid>,
    kind: WireKind,
    code: String,
    name: Option<String>,
    status: String,
    has_children: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EntityKey {
    kind: EntityKind,
    id: Uuid,
}

#[derive(Clone)]
struct Record {
    key: EntityKey,
    name: String,
    code: String,
    status: String,
}

#[derive(Clone, Default)]
struct Identities {
    ids: BTreeMap<EntityKey, RowId>,
    next: u128,
}

impl Identities {
    fn id(&mut self, key: EntityKey) -> Result<RowId, String> {
        if let Some(id) = self.ids.get(&key) {
            return Ok(*id);
        }
        self.next = self.next.checked_add(1).ok_or("树节点身份已耗尽")?;
        let id = RowId(self.next);
        self.ids.insert(key, id);
        Ok(id)
    }
}

#[derive(Clone)]
struct PageState {
    loaded: u64,
    total: u64,
    next: Option<Cursor>,
}

#[derive(Default)]
struct PageProgress {
    loaded: u64,
    total: Option<u64>,
}

enum Target {
    Roots {
        generation: u64,
        after: Option<Cursor>,
    },
    Children {
        ticket: LoadTicket,
        parent: Uuid,
        after: Option<Cursor>,
    },
}

struct Job {
    session_number: u64,
    target: Target,
    task: HttpTask,
}

struct Decoded {
    nodes: Vec<TreeNode>,
    records: Vec<(RowId, Record)>,
    identities: Identities,
    page: PageState,
}

pub(super) struct WarehouseTree {
    open: bool,
    bound_session: Option<u64>,
    model: TreeModel,
    identities: Identities,
    records: BTreeMap<RowId, Record>,
    roots: PageState,
    children: BTreeMap<RowId, PageState>,
    jobs: Vec<Job>,
    search: String,
    composing: bool,
    deadline: Option<f64>,
    generation: u64,
    view_generation: u64,
    visible: Vec<erp_grid_core::TreeRow>,
    columns: Vec<Column>,
    state: GridState,
    error: Option<String>,
}

impl Default for WarehouseTree {
    fn default() -> Self {
        let mut state = GridState::default();
        state.read_only = true;
        state.sorting_enabled = false;
        state.filtering_enabled = false;
        state.quick_search_open = false;
        Self {
            open: false,
            bound_session: None,
            model: TreeModel::new(),
            identities: Identities::default(),
            records: BTreeMap::new(),
            roots: PageState {
                loaded: 0,
                total: 0,
                next: None,
            },
            children: BTreeMap::new(),
            jobs: Vec::new(),
            search: String::new(),
            composing: false,
            deadline: None,
            generation: 0,
            view_generation: 0,
            visible: Vec::new(),
            columns: vec![
                Column::new(ColumnId(0), "名称", 280.0),
                Column::new(ColumnId(1), "类型", 90.0),
                Column::new(ColumnId(2), "编码", 180.0),
                Column::new(ColumnId(3), "状态", 110.0),
            ],
            state,
            error: None,
        }
    }
}

impl WarehouseTree {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn open_button(&mut self, ui: &mut egui::Ui, session_number: u64) {
        if self.bound_session != Some(session_number) {
            self.clear();
            self.bound_session = Some(session_number);
        }
        if ui.button("仓库库位").clicked() {
            self.open = true;
            self.invalidate(ui.input(|input| input.time), true);
        }
    }

    fn invalidate(&mut self, now: f64, immediate: bool) {
        self.jobs.clear();
        self.generation = self.generation.wrapping_add(1);
        self.model.reset(self.generation);
        self.identities = Identities::default();
        self.records.clear();
        self.children.clear();
        self.roots = PageState {
            loaded: 0,
            total: 0,
            next: None,
        };
        self.visible.clear();
        self.state.tree_rows.clear();
        self.view_generation = self.view_generation.wrapping_add(1);
        self.state.bind_view(self.view_generation);
        self.error = None;
        self.deadline = Some(if immediate { now } else { now + 0.25 });
    }

    pub(super) fn ui(
        &mut self,
        ctx: &egui::Context,
        session: &Session,
        session_number: u64,
    ) -> bool {
        if self.bound_session != Some(session_number) {
            self.clear();
            self.bound_session = Some(session_number);
            return false;
        }
        if !self.open {
            return false;
        }
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
        });
        let auth_failed = self.poll(session_number);
        let now = ctx.input(|input| input.time);
        if self.deadline.is_some_and(|at| at <= now) && !self.composing {
            self.deadline = None;
            if let Err(error) = self.start_roots(session, ctx, None, session_number) {
                self.error = Some(error);
            }
        }
        if !self.jobs.is_empty() || self.deadline.is_some() {
            ctx.request_repaint_after(Duration::from_millis(25));
        }
        let mut open = self.open;
        egui::Window::new("仓库与库位")
            .id(egui::Id::new("remote-warehouse-tree"))
            .open(&mut open)
            .default_size(egui::vec2(760.0, 560.0))
            .show(ctx, |ui| self.contents(ui, session, session_number));
        self.open = open;
        if !open {
            self.jobs.clear();
            self.deadline = None;
            self.generation = self.generation.wrapping_add(1);
            self.model.reset(self.generation);
            self.visible.clear();
            self.state.tree_rows.clear();
        }
        auth_failed
    }

    fn contents(&mut self, ui: &mut egui::Ui, session: &Session, session_number: u64) {
        ui.horizontal(|ui| {
            ui.label("搜索");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.search)
                        .desired_width(230.0)
                        .char_limit(256),
                )
                .changed()
            {
                self.invalidate(ui.input(|input| input.time), false);
            }
            if ui.button("刷新").clicked() {
                self.invalidate(ui.input(|input| input.time), true);
            }
            if ui
                .add_enabled(
                    self.jobs.len() < MAX_PENDING
                        && !self.has_root_job()
                        && self.roots.next.is_some(),
                    egui::Button::new("更多仓库"),
                )
                .clicked()
            {
                let after = self.roots.next.clone();
                if let Err(error) = self.start_roots(session, ui.ctx(), after, session_number) {
                    self.error = Some(error);
                }
            }
            ui.label(format!("仓库 {}/{}", self.roots.loaded, self.roots.total));
        });
        if !self.jobs.is_empty() || self.deadline.is_some() {
            ui.spinner();
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        let selected = self.state.row_selection.active;
        let selected_page = selected
            .and_then(|id| self.records.get(&id).map(|record| (id, record.key.clone())))
            .filter(|(_, key)| key.kind == EntityKind::Warehouse);
        if let Some((id, key)) = selected_page {
            let page = self.children.get(&id).cloned();
            ui.horizontal(|ui| {
                ui.label(format!(
                    "所选仓库库位 {}/{}",
                    page.as_ref().map_or(0, |page| page.loaded),
                    page.as_ref().map_or(0, |page| page.total)
                ));
                if ui
                    .add_enabled(
                        self.jobs.len() < MAX_PENDING
                            && !self.has_child_job(id)
                            && page.as_ref().is_some_and(|page| page.next.is_some()),
                        egui::Button::new("更多库位"),
                    )
                    .clicked()
                {
                    let after = page.as_ref().and_then(|page| page.next.clone());
                    match self.model.begin_load_more(id) {
                        Ok(ticket) => {
                            self.rebuild_visible();
                            if let Err(error) = self.start_children(
                                session,
                                ui.ctx(),
                                ticket,
                                key.id,
                                after,
                                session_number,
                            ) {
                                let _ = self.model.complete(ticket, Err(error.clone()));
                                self.error = Some(error);
                            }
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            });
        }

        let visible = &self.visible;
        let records = &self.records;
        let output = DataGrid {
            id: ui.id().with("warehouse-location-tree"),
            columns: &mut self.columns,
            rows: visible.len(),
            state: &mut self.state,
            row_id: Some(&|index| visible[index].id),
        }
        .show(ui, |row, column| {
            let Some(record) = visible.get(row).and_then(|row| records.get(&row.id)) else {
                return String::new();
            };
            match column.0 {
                0 => record.name.clone(),
                1 => match record.key.kind {
                    EntityKind::Warehouse => "仓库",
                    EntityKind::Location => "库位",
                }
                .into(),
                2 => record.code.clone(),
                3 => record.status.clone(),
                _ => String::new(),
            }
        });
        if let Some(id) = output.toggle_tree {
            match self.model.toggle(id) {
                Ok(Some(ticket)) => {
                    let parent = self.records.get(&id).map(|record| record.key.id);
                    if let Some(parent) = parent {
                        if self.jobs.len() >= MAX_PENDING {
                            let _ = self
                                .model
                                .complete(ticket, Err("并发请求已满，请稍后重试".into()));
                        } else if let Err(error) = self.start_children(
                            session,
                            ui.ctx(),
                            ticket,
                            parent,
                            None,
                            session_number,
                        ) {
                            let _ = self.model.complete(ticket, Err(error.clone()));
                            self.error = Some(error);
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => self.error = Some(error.to_string()),
            }
            self.rebuild_visible();
        }
    }

    fn start_roots(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
        after: Option<Cursor>,
        session_number: u64,
    ) -> Result<(), String> {
        if self.jobs.len() >= MAX_PENDING {
            return Err("并发请求已满，请稍后重试".into());
        }
        if self.has_root_job() {
            return Err("仓库页正在加载".into());
        }
        let generation = self.generation;
        let task = start(session, ctx, None, after.as_ref(), self.search.trim())?;
        self.jobs.push(Job {
            session_number,
            target: Target::Roots { generation, after },
            task,
        });
        Ok(())
    }

    fn start_children(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
        ticket: LoadTicket,
        parent: Uuid,
        after: Option<Cursor>,
        session_number: u64,
    ) -> Result<(), String> {
        if self.jobs.len() >= MAX_PENDING {
            return Err("并发请求已满，请稍后重试".into());
        }
        let task = start(
            session,
            ctx,
            Some(parent),
            after.as_ref(),
            self.search.trim(),
        )?;
        self.jobs.push(Job {
            session_number,
            target: Target::Children {
                ticket,
                parent,
                after,
            },
            task,
        });
        Ok(())
    }

    fn poll(&mut self, session_number: u64) -> bool {
        let mut auth_failed = false;
        let mut index = 0;
        while index < self.jobs.len() {
            let Some(result) = self.jobs[index].task.poll() else {
                index += 1;
                continue;
            };
            let job = self.jobs.remove(index);
            if job.session_number != session_number {
                continue;
            }
            match result {
                Err(HttpError::Status(401)) => {
                    self.jobs.clear();
                    auth_failed = true;
                }
                Err(HttpError::Status(403)) => {
                    self.fail_target(job.target, "无权查看仓库与库位".into())
                }
                Err(error) => self.fail_target(job.target, error.to_string()),
                Ok(bytes) => self.accept(job.target, &bytes),
            }
        }
        auth_failed
    }

    fn fail_target(&mut self, target: Target, error: String) {
        match target {
            Target::Roots { generation, .. } if generation == self.generation => {
                self.error = Some(error)
            }
            Target::Children { ticket, .. } => {
                let _ = self.model.complete(ticket, Err(error));
            }
            _ => {}
        }
        self.rebuild_visible();
    }

    fn accept(&mut self, target: Target, bytes: &[u8]) {
        let (expected_kind, parent, after) = match &target {
            Target::Roots { generation, after } if *generation == self.generation => {
                (EntityKind::Warehouse, None, after.as_ref())
            }
            Target::Children {
                ticket,
                parent,
                after,
            } if ticket.generation == self.generation => {
                (EntityKind::Location, Some(*parent), after.as_ref())
            }
            _ => return,
        };
        let previously_loaded = match &target {
            Target::Roots { .. } => self.roots.loaded,
            Target::Children { ticket, .. } => self
                .children
                .get(&ticket.parent)
                .map_or(0, |page| page.loaded),
        };
        let previous_total = match &target {
            Target::Roots { after, .. } => after.as_ref().map(|_| self.roots.total),
            Target::Children { ticket, after, .. } => after
                .as_ref()
                .and_then(|_| self.children.get(&ticket.parent).map(|page| page.total)),
        };
        let decoded = decode(
            bytes,
            expected_kind,
            parent,
            after,
            PageProgress {
                loaded: previously_loaded,
                total: previous_total,
            },
            &self.identities,
            &self.records,
        );
        let decoded = match decoded {
            Ok(decoded) => decoded,
            Err(error) => {
                self.fail_target(target, error);
                return;
            }
        };
        let accepted = match &target {
            Target::Roots { after, .. } => {
                let result = if after.is_none() {
                    self.model.set_roots(decoded.nodes.clone())
                } else {
                    self.model.append_roots(decoded.nodes.clone())
                };
                result.map(|_| true)
            }
            Target::Children { ticket, .. } => {
                self.model.complete(*ticket, Ok(decoded.nodes.clone()))
            }
        };
        match accepted {
            Ok(true) => {
                self.identities = decoded.identities;
                self.records.extend(decoded.records);
                match target {
                    Target::Roots { .. } => self.roots = decoded.page,
                    Target::Children { ticket, .. } => {
                        self.children.insert(ticket.parent, decoded.page);
                    }
                }
                self.error = None;
            }
            Ok(false) => {}
            Err(error) => self.error = Some(error.to_string()),
        }
        self.rebuild_visible();
    }

    fn rebuild_visible(&mut self) {
        let active = self.state.row_selection.active;
        let next = self.model.visible();
        let ids: Vec<_> = next.iter().map(|row| row.id).collect();
        let old: Vec<_> = self.visible.iter().map(|row| row.id).collect();
        self.visible = next;
        self.state.tree_rows = self
            .visible
            .iter()
            .map(|row| {
                (
                    row.id,
                    TreeRowVisual {
                        depth: row.depth,
                        expanded: row.expanded,
                        has_children: row.has_children,
                        loading: row.loading,
                        error: row.error.clone(),
                    },
                )
            })
            .collect();
        if ids != old {
            self.view_generation = self.view_generation.wrapping_add(1);
            self.state.rebind_page(self.view_generation, false);
            if active.is_some_and(|active| ids.contains(&active)) {
                self.state.row_selection.active = active;
            }
        }
    }

    fn has_root_job(&self) -> bool {
        self.jobs
            .iter()
            .any(|job| matches!(&job.target, Target::Roots { .. }))
    }

    fn has_child_job(&self, parent: RowId) -> bool {
        self.jobs.iter().any(
            |job| matches!(&job.target, Target::Children { ticket, .. } if ticket.parent == parent),
        )
    }
}

fn start(
    session: &Session,
    ctx: &egui::Context,
    parent: Option<Uuid>,
    after: Option<&Cursor>,
    search: &str,
) -> Result<HttpTask, String> {
    let body = serde_json::to_vec(&Request {
        schema: 1,
        parent,
        after,
        limit: PAGE_LIMIT,
        search: (!search.is_empty()).then_some(search),
    })
    .map_err(|_| "无法编码仓库树请求".to_owned())?;
    session
        .client
        .start(
            HttpRequest {
                path: format!("/v1/tenants/{}/warehouses/tree", session.tenant.to_uuid()),
                query: vec![],
                json: Some(body),
                bearer: Some(session.token.clone()),
            },
            ctx,
        )
        .map_err(|error| error.to_string())
}

fn decode(
    bytes: &[u8],
    expected_kind: EntityKind,
    parent: Option<Uuid>,
    after: Option<&Cursor>,
    progress: PageProgress,
    identities: &Identities,
    existing: &BTreeMap<RowId, Record>,
) -> Result<Decoded, String> {
    let PageProgress {
        loaded: previously_loaded,
        total: previous_total,
    } = progress;
    if bytes.len() > MAX_BODY {
        return Err("仓库树响应超过 8 MiB".into());
    }
    let response: Response =
        serde_json::from_slice(bytes).map_err(|_| "仓库树响应无效".to_owned())?;
    if response.items.len() > usize::from(PAGE_LIMIT) {
        return Err("仓库树响应超过分页上限".into());
    }
    if after.is_some() && response.items.is_empty() {
        return Err("仓库树下一页没有前进，请刷新后重试".into());
    }
    if previous_total.is_some_and(|total| total != response.total) {
        return Err("仓库树数据已变化，请刷新后重试".into());
    }
    if identities.ids.len().saturating_add(response.items.len()) > TreeModel::MAX_NODES {
        return Err("仓库树节点超过 10000 项上限".into());
    }
    let mut staged = identities.clone();
    let mut seen = BTreeSet::new();
    let existing_keys: BTreeSet<_> = existing.values().map(|record| record.key.clone()).collect();
    let mut text_bytes = existing.values().fold(0usize, |total, record| {
        total
            .saturating_add(record.name.len())
            .saturating_add(record.code.len())
            .saturating_add(record.status.len())
    });
    let mut nodes = Vec::with_capacity(response.items.len());
    let mut records = Vec::with_capacity(response.items.len());
    let mut previous = after.cloned();
    for item in response.items {
        let kind = match item.kind {
            WireKind::Warehouse => EntityKind::Warehouse,
            WireKind::Location => EntityKind::Location,
        };
        if kind != expected_kind
            || item.parent_id != parent
            || (kind == EntityKind::Location && item.has_children)
        {
            return Err("仓库树节点类型或父级无效".into());
        }
        if item.id.is_nil()
            || item.code.is_empty()
            || item.code.len() > 4096
            || item.status.len() > 4096
            || item.name.as_ref().is_some_and(|name| name.len() > 4096)
            || item.code.contains('\0')
            || item.status.contains('\0')
            || item.name.as_ref().is_some_and(|name| name.contains('\0'))
        {
            return Err("仓库树节点文本无效".into());
        }
        text_bytes = text_bytes
            .saturating_add(item.code.len())
            .saturating_add(item.status.len())
            .saturating_add(item.name.as_ref().map_or(item.code.len(), String::len));
        if text_bytes > MAX_BODY {
            return Err("仓库树节点文本超过 8 MiB".into());
        }
        if previous.as_ref().is_some_and(|previous| {
            (item.code.as_str(), item.id) <= (previous.code.as_str(), previous.id)
        }) {
            return Err("仓库树节点顺序或游标无效".into());
        }
        previous = Some(Cursor {
            code: item.code.clone(),
            id: item.id,
        });
        let key = EntityKey { kind, id: item.id };
        if existing_keys.contains(&key) || !seen.insert(key.clone()) {
            return Err("仓库树节点重复".into());
        }
        let id = staged.id(key.clone())?;
        nodes.push(TreeNode {
            id,
            has_children: item.has_children,
        });
        records.push((
            id,
            Record {
                key,
                name: item.name.unwrap_or_else(|| item.code.clone()),
                code: item.code,
                status: item.status,
            },
        ));
    }
    let loaded = previously_loaded.saturating_add(nodes.len() as u64);
    if response.total < loaded || response.next.is_some() != (loaded < response.total) {
        return Err("仓库树数据已变化，请刷新后重试".into());
    }
    if let Some(next) = &response.next {
        if next.id.is_nil()
            || next.code.is_empty()
            || next.code.len() > 4096
            || next.code.contains('\0')
        {
            return Err("仓库树下一页游标无效".into());
        }
        let Some(previous) = previous else {
            return Err("空页不能包含下一页游标".into());
        };
        if next != &previous {
            return Err("仓库树下一页游标与末项不一致".into());
        }
    }
    Ok(Decoded {
        nodes,
        records,
        identities: staged,
        page: PageState {
            loaded,
            total: response.total,
            next: response.next,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_identity_separates_equal_uuid_and_cursor_must_match_last_item() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let roots = decode(format!(r#"{{"items":[{{"id":"{id}","parent_id":null,"kind":"warehouse","code":"W","name":null,"status":"active","has_children":true}}],"total":1,"next":null}}"#).as_bytes(), EntityKind::Warehouse, None, None, PageProgress::default(), &Identities::default(), &BTreeMap::new()).unwrap();
        let parent_row = roots.nodes[0].id;
        let existing = roots.records.into_iter().collect();
        let child = decode(format!(r#"{{"items":[{{"id":"{id}","parent_id":"{id}","kind":"location","code":"L","name":"库位","status":"active","has_children":false}}],"total":1,"next":null}}"#).as_bytes(), EntityKind::Location, Some(id), None, PageProgress::default(), &roots.identities, &existing).unwrap();
        assert_ne!(parent_row, child.nodes[0].id);
        let invalid = format!(
            r#"{{"items":[{{"id":"{id}","parent_id":null,"kind":"warehouse","code":"W","name":null,"status":"active","has_children":true}}],"total":2,"next":{{"code":"X","id":"{id}"}}}}"#
        );
        assert!(
            decode(
                invalid.as_bytes(),
                EntityKind::Warehouse,
                None,
                None,
                PageProgress::default(),
                &Identities::default(),
                &BTreeMap::new()
            )
            .is_err()
        );
        let cursor = Cursor {
            code: "W".into(),
            id,
        };
        assert!(
            decode(
                br#"{"items":[],"total":2,"next":{"code":"W","id":"11111111-1111-1111-1111-111111111111"}}"#,
                EntityKind::Warehouse,
                None,
                Some(&cursor),
                PageProgress { loaded: 1, total: Some(2) },
                &Identities::default(),
                &BTreeMap::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn stale_response_cannot_publish_after_query_reset() {
        let mut view = WarehouseTree::default();
        let old = view.generation;
        view.invalidate(1.0, false);
        assert_ne!(old, view.generation);
        let target = Target::Roots {
            generation: old,
            after: None,
        };
        view.accept(target, br#"{"items":[],"total":0,"next":null}"#);
        assert!(view.records.is_empty());
        assert!(view.visible.is_empty());
    }
}
