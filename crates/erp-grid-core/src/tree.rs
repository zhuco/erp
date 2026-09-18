use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::RowId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub id: RowId,
    pub has_children: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub id: RowId,
    pub depth: u8,
    pub expanded: bool,
    pub has_children: bool,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadTicket {
    pub generation: u64,
    pub parent: RowId,
    pub request: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeError {
    ZeroId,
    DuplicateId(RowId),
    UnknownNode(RowId),
    NotAParent(RowId),
    AlreadyLoading(RowId),
    AlreadyLoaded(RowId),
    NotLoaded(RowId),
    NodeLimit,
    DepthLimit,
    RequestExhausted,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroId => f.write_str("tree node ID must be non-zero"),
            Self::DuplicateId(id) => write!(f, "duplicate tree node ID: {}", id.0),
            Self::UnknownNode(id) => write!(f, "unknown tree node ID: {}", id.0),
            Self::NotAParent(id) => write!(f, "tree node cannot have children: {}", id.0),
            Self::AlreadyLoading(id) => write!(f, "tree node is already loading: {}", id.0),
            Self::AlreadyLoaded(id) => write!(f, "tree node children are already loaded: {}", id.0),
            Self::NotLoaded(id) => write!(f, "tree node children are not loaded: {}", id.0),
            Self::NodeLimit => f.write_str("tree node limit exceeded"),
            Self::DepthLimit => f.write_str("tree depth limit exceeded"),
            Self::RequestExhausted => f.write_str("tree load request identity exhausted"),
        }
    }
}

impl Error for TreeError {}

#[derive(Clone, Debug)]
struct NodeState {
    node: TreeNode,
    depth: u8,
    expanded: bool,
    children: Option<Vec<RowId>>,
    loading: Option<PendingLoad>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingLoad {
    request: u64,
    append: bool,
}

#[derive(Clone, Debug)]
pub struct TreeModel {
    generation: u64,
    next_request: u64,
    roots: Vec<RowId>,
    nodes: BTreeMap<RowId, NodeState>,
}

impl Default for TreeModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeModel {
    pub const MAX_NODES: usize = 10_000;
    pub const MAX_DEPTH: u8 = 32;
    pub const MAX_ERROR_BYTES: usize = 4_096;

    pub fn new() -> Self {
        Self {
            generation: 0,
            next_request: 0,
            roots: Vec::new(),
            nodes: BTreeMap::new(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn reset(&mut self, generation: u64) {
        self.generation = generation;
        self.roots.clear();
        self.nodes.clear();
    }

    pub fn set_roots(&mut self, roots: Vec<TreeNode>) -> Result<(), TreeError> {
        validate_new_nodes(&roots, &BTreeSet::new(), Self::MAX_NODES)?;
        let root_ids = roots.iter().map(|node| node.id).collect();
        let nodes = roots
            .into_iter()
            .map(|node| {
                (
                    node.id,
                    NodeState {
                        node,
                        depth: 0,
                        expanded: false,
                        children: None,
                        loading: None,
                        error: None,
                    },
                )
            })
            .collect();
        self.roots = root_ids;
        self.nodes = nodes;
        Ok(())
    }

    pub fn append_roots(&mut self, roots: Vec<TreeNode>) -> Result<(), TreeError> {
        let existing = self.nodes.keys().copied().collect();
        let remaining = Self::MAX_NODES.saturating_sub(self.nodes.len());
        validate_new_nodes(&roots, &existing, remaining)?;

        self.roots.extend(roots.iter().map(|node| node.id));
        self.nodes.extend(roots.into_iter().map(|node| {
            (
                node.id,
                NodeState {
                    node,
                    depth: 0,
                    expanded: false,
                    children: None,
                    loading: None,
                    error: None,
                },
            )
        }));
        Ok(())
    }

    pub fn begin_load(&mut self, parent: RowId) -> Result<LoadTicket, TreeError> {
        let state = self
            .nodes
            .get(&parent)
            .ok_or(TreeError::UnknownNode(parent))?;
        if !state.node.has_children {
            return Err(TreeError::NotAParent(parent));
        }
        if state.loading.is_some() {
            return Err(TreeError::AlreadyLoading(parent));
        }
        if state.children.is_some() {
            return Err(TreeError::AlreadyLoaded(parent));
        }
        if state.depth >= Self::MAX_DEPTH {
            return Err(TreeError::DepthLimit);
        }
        let request = self
            .next_request
            .checked_add(1)
            .ok_or(TreeError::RequestExhausted)?;
        self.next_request = request;
        let state = self
            .nodes
            .get_mut(&parent)
            .ok_or(TreeError::UnknownNode(parent))?;
        state.loading = Some(PendingLoad {
            request,
            append: false,
        });
        state.error = None;
        Ok(LoadTicket {
            generation: self.generation,
            parent,
            request,
        })
    }

    pub fn begin_load_more(&mut self, parent: RowId) -> Result<LoadTicket, TreeError> {
        let state = self
            .nodes
            .get(&parent)
            .ok_or(TreeError::UnknownNode(parent))?;
        if !state.node.has_children {
            return Err(TreeError::NotAParent(parent));
        }
        if state.loading.is_some() {
            return Err(TreeError::AlreadyLoading(parent));
        }
        if state.children.is_none() {
            return Err(TreeError::NotLoaded(parent));
        }
        if state.depth >= Self::MAX_DEPTH {
            return Err(TreeError::DepthLimit);
        }
        let request = self
            .next_request
            .checked_add(1)
            .ok_or(TreeError::RequestExhausted)?;
        self.next_request = request;
        let state = self
            .nodes
            .get_mut(&parent)
            .ok_or(TreeError::UnknownNode(parent))?;
        state.loading = Some(PendingLoad {
            request,
            append: true,
        });
        state.error = None;
        Ok(LoadTicket {
            generation: self.generation,
            parent,
            request,
        })
    }

    pub fn complete(
        &mut self,
        ticket: LoadTicket,
        result: Result<Vec<TreeNode>, String>,
    ) -> Result<bool, TreeError> {
        let Some(pending) = self
            .nodes
            .get(&ticket.parent)
            .and_then(|state| state.loading)
        else {
            return Ok(false);
        };
        if ticket.generation != self.generation || pending.request != ticket.request {
            return Ok(false);
        }

        let children = match result {
            Ok(children) => children,
            Err(error) => {
                let state = self
                    .nodes
                    .get_mut(&ticket.parent)
                    .ok_or(TreeError::UnknownNode(ticket.parent))?;
                state.loading = None;
                state.error = Some(bounded_error(error));
                return Ok(true);
            }
        };

        let parent_depth = self
            .nodes
            .get(&ticket.parent)
            .ok_or(TreeError::UnknownNode(ticket.parent))?
            .depth;
        let child_depth = parent_depth
            .checked_add(1)
            .filter(|depth| *depth <= Self::MAX_DEPTH)
            .ok_or(TreeError::DepthLimit)?;
        let existing = self.nodes.keys().copied().collect();
        let remaining = Self::MAX_NODES.saturating_sub(self.nodes.len());
        if let Err(error) = validate_new_nodes(&children, &existing, remaining) {
            let state = self
                .nodes
                .get_mut(&ticket.parent)
                .ok_or(TreeError::UnknownNode(ticket.parent))?;
            state.loading = None;
            state.error = Some(bounded_error(error.to_string()));
            return Err(error);
        }

        let child_ids: Vec<_> = children.iter().map(|node| node.id).collect();
        for node in children {
            self.nodes.insert(
                node.id,
                NodeState {
                    node,
                    depth: child_depth,
                    expanded: false,
                    children: None,
                    loading: None,
                    error: None,
                },
            );
        }
        let parent = self
            .nodes
            .get_mut(&ticket.parent)
            .ok_or(TreeError::UnknownNode(ticket.parent))?;
        if pending.append {
            parent
                .children
                .as_mut()
                .ok_or(TreeError::NotLoaded(ticket.parent))?
                .extend(child_ids);
        } else {
            parent.children = Some(child_ids);
        }
        parent.loading = None;
        parent.error = None;
        Ok(true)
    }

    pub fn toggle(&mut self, id: RowId) -> Result<Option<LoadTicket>, TreeError> {
        let state = self.nodes.get(&id).ok_or(TreeError::UnknownNode(id))?;
        let (has_children, expanded, loaded, loading, failed) = (
            state.node.has_children,
            state.expanded,
            state.children.is_some(),
            state.loading.is_some(),
            state.error.is_some(),
        );
        if !has_children {
            return Ok(None);
        }
        if failed && !loaded && !loading {
            self.nodes
                .get_mut(&id)
                .ok_or(TreeError::UnknownNode(id))?
                .expanded = true;
            return self.begin_load(id).map(Some);
        }
        if expanded {
            self.nodes
                .get_mut(&id)
                .ok_or(TreeError::UnknownNode(id))?
                .expanded = false;
            return Ok(None);
        }
        self.nodes
            .get_mut(&id)
            .ok_or(TreeError::UnknownNode(id))?
            .expanded = true;
        if !loaded && !loading {
            self.begin_load(id).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn visible(&self) -> Vec<TreeRow> {
        let mut rows = Vec::with_capacity(self.nodes.len());
        let mut pending: Vec<_> = self.roots.iter().rev().copied().collect();
        while let Some(id) = pending.pop() {
            let Some(state) = self.nodes.get(&id) else {
                continue;
            };
            rows.push(TreeRow {
                id,
                depth: state.depth,
                expanded: state.expanded,
                has_children: state.node.has_children,
                loading: state.loading.is_some(),
                error: state.error.clone(),
            });
            if state.expanded
                && let Some(children) = &state.children
            {
                pending.extend(children.iter().rev().copied());
            }
        }
        rows
    }
}

fn bounded_error(mut error: String) -> String {
    if error.len() > TreeModel::MAX_ERROR_BYTES {
        let mut end = TreeModel::MAX_ERROR_BYTES;
        while !error.is_char_boundary(end) {
            end -= 1;
        }
        error.truncate(end);
    }
    error
}

fn validate_new_nodes(
    nodes: &[TreeNode],
    existing: &BTreeSet<RowId>,
    remaining: usize,
) -> Result<(), TreeError> {
    if nodes.len() > remaining {
        return Err(TreeError::NodeLimit);
    }
    let mut seen = BTreeSet::new();
    for node in nodes {
        if node.id.0 == 0 {
            return Err(TreeError::ZeroId);
        }
        if existing.contains(&node.id) || !seen.insert(node.id) {
            return Err(TreeError::DuplicateId(node.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u128, has_children: bool) -> TreeNode {
        TreeNode {
            id: RowId(id),
            has_children,
        }
    }

    #[test]
    fn lazy_load_preserves_order_and_does_not_reload_cached_children() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true), node(2, false)]).unwrap();
        let ticket = tree.toggle(RowId(1)).unwrap().unwrap();
        assert!(
            tree.complete(ticket, Ok(vec![node(3, false), node(4, true)]))
                .unwrap()
        );
        assert_eq!(
            tree.visible()
                .iter()
                .map(|row| (row.id.0, row.depth))
                .collect::<Vec<_>>(),
            [(1, 0), (3, 1), (4, 1), (2, 0)]
        );
        assert!(tree.toggle(RowId(1)).unwrap().is_none());
        assert!(tree.toggle(RowId(1)).unwrap().is_none());
    }

    #[test]
    fn collapsed_parent_caches_completion_without_reopening() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true)]).unwrap();
        let ticket = tree.toggle(RowId(1)).unwrap().unwrap();
        tree.toggle(RowId(1)).unwrap();
        assert!(tree.complete(ticket, Ok(vec![node(2, false)])).unwrap());
        assert_eq!(tree.visible().len(), 1);
        tree.toggle(RowId(1)).unwrap();
        assert_eq!(tree.visible().len(), 2);
    }

    #[test]
    fn late_generation_and_superseded_results_are_ignored() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true)]).unwrap();
        let old = tree.toggle(RowId(1)).unwrap().unwrap();
        tree.reset(0);
        tree.set_roots(vec![node(1, true)]).unwrap();
        let current = tree.toggle(RowId(1)).unwrap().unwrap();
        assert_eq!(old.generation, current.generation);
        assert!(current.request > old.request);
        assert!(!tree.complete(old, Ok(vec![node(2, false)])).unwrap());
        assert!(tree.complete(current, Ok(vec![])).unwrap());
    }

    #[test]
    fn failed_load_is_visible_and_toggle_retries() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true)]).unwrap();
        let failed = tree.toggle(RowId(1)).unwrap().unwrap();
        tree.toggle(RowId(1)).unwrap();
        tree.complete(failed, Err("界".repeat(2_000))).unwrap();
        assert!(tree.visible()[0].error.as_ref().unwrap().len() <= TreeModel::MAX_ERROR_BYTES);
        let retry = tree.toggle(RowId(1)).unwrap().unwrap();
        assert!(retry.request > failed.request);
        assert!(tree.visible()[0].loading && tree.visible()[0].expanded);
    }

    #[test]
    fn invalid_child_batch_is_atomic() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true), node(9, false)]).unwrap();
        let ticket = tree.toggle(RowId(1)).unwrap().unwrap();
        assert_eq!(
            tree.complete(ticket, Ok(vec![node(2, false), node(9, false)])),
            Err(TreeError::DuplicateId(RowId(9)))
        );
        let row = tree.visible().remove(0);
        assert!(!row.loading);
        assert!(row.error.is_some());
        assert_eq!(tree.nodes.len(), 2);
        let retry = tree.toggle(RowId(1)).unwrap().unwrap();
        assert_eq!(
            tree.complete(retry, Ok(vec![node(1, false)])),
            Err(TreeError::DuplicateId(RowId(1)))
        );

        let mut full_tree = TreeModel::new();
        let mut roots = vec![node(1, true)];
        roots.extend((2..=TreeModel::MAX_NODES as u128).map(|id| node(id, false)));
        full_tree.set_roots(roots).unwrap();
        let ticket = full_tree.toggle(RowId(1)).unwrap().unwrap();
        assert_eq!(
            full_tree.complete(ticket, Ok(vec![node(10_001, false)])),
            Err(TreeError::NodeLimit)
        );
        assert_eq!(full_tree.nodes.len(), TreeModel::MAX_NODES);

        let mut deep_tree = TreeModel::new();
        deep_tree.set_roots(vec![node(1, true)]).unwrap();
        for id in 1..=TreeModel::MAX_DEPTH as u128 {
            let ticket = deep_tree.toggle(RowId(id)).unwrap().unwrap();
            deep_tree
                .complete(ticket, Ok(vec![node(id + 1, true)]))
                .unwrap();
        }
        assert_eq!(
            deep_tree.toggle(RowId(TreeModel::MAX_DEPTH as u128 + 1)),
            Err(TreeError::DepthLimit)
        );
    }

    #[test]
    fn load_more_appends_in_order_and_preserves_expansion_state() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true)]).unwrap();
        let initial = tree.toggle(RowId(1)).unwrap().unwrap();
        tree.complete(initial, Ok(vec![node(2, true), node(3, false)]))
            .unwrap();
        let nested = tree.toggle(RowId(2)).unwrap().unwrap();
        tree.complete(nested, Ok(vec![node(4, false)])).unwrap();
        tree.toggle(RowId(2)).unwrap();

        let append = tree.begin_load_more(RowId(1)).unwrap();
        assert!(append.request > initial.request);
        tree.complete(append, Ok(vec![node(5, false), node(6, false)]))
            .unwrap();

        assert_eq!(
            tree.visible()
                .iter()
                .map(|row| (row.id.0, row.expanded))
                .collect::<Vec<_>>(),
            [(1, true), (2, false), (3, false), (5, false), (6, false)]
        );
        tree.toggle(RowId(2)).unwrap();
        assert_eq!(
            tree.visible()
                .iter()
                .map(|row| row.id.0)
                .collect::<Vec<_>>(),
            [1, 2, 4, 3, 5, 6]
        );
    }

    #[test]
    fn failed_or_duplicate_load_more_is_atomic_and_retryable() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true), node(9, false)]).unwrap();
        let initial = tree.toggle(RowId(1)).unwrap().unwrap();
        tree.complete(initial, Ok(vec![node(2, false)])).unwrap();

        let failed = tree.begin_load_more(RowId(1)).unwrap();
        tree.complete(failed, Err("remote page failed".into()))
            .unwrap();
        assert!(tree.visible()[0].error.is_some());
        let duplicate = tree.begin_load_more(RowId(1)).unwrap();
        assert_eq!(
            tree.complete(duplicate, Ok(vec![node(3, false), node(9, false)])),
            Err(TreeError::DuplicateId(RowId(9)))
        );
        assert_eq!(
            tree.visible()
                .iter()
                .map(|row| row.id.0)
                .collect::<Vec<_>>(),
            [1, 2, 9]
        );

        let retry = tree.begin_load_more(RowId(1)).unwrap();
        assert!(retry.request > duplicate.request);
        tree.complete(retry, Ok(vec![node(3, false)])).unwrap();
        assert_eq!(
            tree.visible()
                .iter()
                .map(|row| row.id.0)
                .collect::<Vec<_>>(),
            [1, 2, 3, 9]
        );
    }

    #[test]
    fn stale_load_more_after_reset_does_not_mutate_replacement_tree() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true)]).unwrap();
        let initial = tree.toggle(RowId(1)).unwrap().unwrap();
        tree.complete(initial, Ok(vec![])).unwrap();
        let stale = tree.begin_load_more(RowId(1)).unwrap();

        tree.reset(7);
        tree.set_roots(vec![node(8, false)]).unwrap();
        assert!(!tree.complete(stale, Ok(vec![node(2, false)])).unwrap());
        assert_eq!(
            tree.visible()
                .iter()
                .map(|row| row.id.0)
                .collect::<Vec<_>>(),
            [8]
        );
    }

    #[test]
    fn append_roots_preserves_loaded_nodes_and_enforces_atomic_bounds() {
        let mut tree = TreeModel::new();
        tree.set_roots(vec![node(1, true)]).unwrap();
        let initial = tree.toggle(RowId(1)).unwrap().unwrap();
        tree.complete(initial, Ok(vec![node(2, false)])).unwrap();
        tree.toggle(RowId(1)).unwrap();

        assert_eq!(
            tree.append_roots(vec![node(0, false)]),
            Err(TreeError::ZeroId)
        );
        assert_eq!(
            tree.append_roots(vec![node(3, false), node(2, false)]),
            Err(TreeError::DuplicateId(RowId(2)))
        );
        tree.append_roots(
            (3..=TreeModel::MAX_NODES as u128)
                .map(|id| node(id, false))
                .collect(),
        )
        .unwrap();
        assert_eq!(
            tree.append_roots(vec![node(10_001, false)]),
            Err(TreeError::NodeLimit)
        );
        assert_eq!(tree.nodes.len(), TreeModel::MAX_NODES);
        assert_eq!(tree.visible().len(), TreeModel::MAX_NODES - 1);
        tree.toggle(RowId(1)).unwrap();
        assert_eq!(tree.visible()[1].id, RowId(2));
    }
}
