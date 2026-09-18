use crate::{Column, ColumnId};
use std::collections::{BTreeMap, BTreeSet};

/// Header groups add exactly one compact row above the ordinary column header.
pub const HEADER_GROUP_ROW_COUNT: usize = 1;
pub const MAX_HEADER_GROUPS: usize = 64;
pub const MAX_COLUMNS_PER_HEADER_GROUP: usize = 256;
pub const MAX_HEADER_GROUP_TITLE_CHARS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderGroup {
    title: String,
    columns: Vec<ColumnId>,
}

impl HeaderGroup {
    pub fn new(title: impl Into<String>, columns: impl IntoIterator<Item = ColumnId>) -> Self {
        let mut seen = BTreeSet::new();
        let columns = columns
            .into_iter()
            .filter(|id| seen.insert(*id))
            .take(MAX_COLUMNS_PER_HEADER_GROUP)
            .collect();
        Self {
            title: bounded_title(title.into()),
            columns,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn columns(&self) -> &[ColumnId] {
        &self.columns
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderGroupRun {
    pub group_index: usize,
    pub title: String,
    pub pin_band: u8,
    pub first_column: ColumnId,
    pub last_column: ColumnId,
}

/// Resolve group spans from the final visible column order.
///
/// A span ends at an ungrouped column, another group, or a pin-band boundary.
/// If metadata assigns a column more than once, the first bounded group wins.
pub fn header_group_runs(
    groups: &[HeaderGroup],
    visible_columns: &[Column],
) -> Vec<HeaderGroupRun> {
    header_group_runs_by(
        groups,
        visible_columns
            .iter()
            .map(|column| (column.id, column.pin_rank())),
    )
}

fn header_group_runs_by(
    groups: &[HeaderGroup],
    visible_columns: impl IntoIterator<Item = (ColumnId, u8)>,
) -> Vec<HeaderGroupRun> {
    let groups = &groups[..groups.len().min(MAX_HEADER_GROUPS)];
    let mut membership = BTreeMap::new();
    for (group_index, group) in groups.iter().enumerate() {
        for id in group.columns.iter().take(MAX_COLUMNS_PER_HEADER_GROUP) {
            membership.entry(*id).or_insert(group_index);
        }
    }

    let mut runs: Vec<HeaderGroupRun> = Vec::new();
    let mut previous_visible: Option<(ColumnId, Option<usize>, u8)> = None;
    for (column_id, pin_band) in visible_columns {
        let group_index = membership.get(&column_id).copied();
        let Some(group_index) = group_index else {
            previous_visible = Some((column_id, None, pin_band));
            continue;
        };
        if let Some(run) = runs.last_mut()
            && run.group_index == group_index
            && run.pin_band == pin_band
            && previous_visible == Some((run.last_column, Some(group_index), pin_band))
        {
            run.last_column = column_id;
            previous_visible = Some((column_id, Some(group_index), pin_band));
            continue;
        }
        runs.push(HeaderGroupRun {
            group_index,
            title: bounded_title(groups[group_index].title.clone()),
            pin_band,
            first_column: column_id,
            last_column: column_id,
        });
        previous_visible = Some((column_id, Some(group_index), pin_band));
    }
    runs
}

fn bounded_title(title: String) -> String {
    title.chars().take(MAX_HEADER_GROUP_TITLE_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_reordered_and_ungrouped_columns_split_runs() {
        let groups = [HeaderGroup::new(
            "Amounts",
            [ColumnId(1), ColumnId(2), ColumnId(3)],
        )];
        let runs = header_group_runs_by(
            &groups,
            [(ColumnId(3), 1), (ColumnId(9), 1), (ColumnId(1), 1)],
        );
        assert_eq!(runs.len(), 2);
        assert_eq!(
            (runs[0].first_column, runs[0].last_column),
            (ColumnId(3), ColumnId(3))
        );
        assert_eq!(
            (runs[1].first_column, runs[1].last_column),
            (ColumnId(1), ColumnId(1))
        );
    }

    #[test]
    fn pin_boundaries_split_an_otherwise_contiguous_group() {
        let groups = [HeaderGroup::new(
            "Product",
            [ColumnId(1), ColumnId(2), ColumnId(3)],
        )];
        let runs = header_group_runs_by(
            &groups,
            [(ColumnId(1), 0), (ColumnId(2), 1), (ColumnId(3), 2)],
        );
        assert_eq!(runs.len(), 3);
        assert_eq!(
            runs.iter().map(|run| run.pin_band).collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn metadata_is_bounded_deduplicated_and_first_group_wins() {
        let columns = (0..=MAX_COLUMNS_PER_HEADER_GROUP as u64).map(ColumnId);
        let first = HeaderGroup::new("First", columns.chain([ColumnId(0)]));
        assert_eq!(first.columns.len(), MAX_COLUMNS_PER_HEADER_GROUP);
        let second = HeaderGroup::new("Second", [ColumnId(0)]);
        let runs = header_group_runs_by(&[first, second], [(ColumnId(0), 1)]);
        assert_eq!(runs[0].title, "First");
    }

    #[test]
    fn title_is_bounded_by_unicode_scalar_count() {
        let group = HeaderGroup::new("易".repeat(MAX_HEADER_GROUP_TITLE_CHARS + 1), []);
        assert_eq!(group.title.chars().count(), MAX_HEADER_GROUP_TITLE_CHARS);
    }
}
