use crate::data::{DataSet, fixture_date};
use erp_grid_core::*;
#[derive(Default)]
pub struct Summary {
    pub count: usize,
    pub quantity: i128,
    pub amount: i128,
}
impl Summary {
    pub fn add(&mut self, data: &DataSet, row: usize) {
        self.count += 1;
        self.quantity += data.text(row, ColumnId(2)).parse::<i128>().unwrap_or(0);
        self.amount += money_minor(&data.text(row, ColumnId(3)))
            .ok()
            .flatten()
            .unwrap_or(0) as i128;
    }
    pub fn label(&self) -> String {
        let amount = self.amount;
        format!(
            "{} rows · Qty {} · Amount {}{}.{:02}",
            self.count,
            self.quantity,
            if amount < 0 { "-" } else { "" },
            amount.abs() / 100,
            amount.abs() % 100
        )
    }
}
pub struct QueryJob {
    next: usize,
    matches: Vec<usize>,
    pub filters: Vec<(ColumnId, Filter)>,
    sorts: Vec<SortRule>,
    sorter: Option<SortJob>,
    pub summary: Summary,
}
impl QueryJob {
    pub fn new(filters: Vec<(ColumnId, Filter)>, sorts: Vec<SortRule>) -> Self {
        Self {
            next: 0,
            matches: Vec::new(),
            filters,
            sorts,
            sorter: None,
            summary: Summary::default(),
        }
    }
    pub fn advance(&mut self, data: &DataSet, columns: &[Column]) -> Option<Vec<usize>> {
        if self.next < data.rows {
            let end = (self.next + 2000 / self.filters.len().max(1)).min(data.rows);
            for row in self.next..end {
                if self.filters.iter().all(|(col, filter)| match filter {
                    Filter::Money { .. } => money_minor(&data.text(row, *col))
                        .ok()
                        .flatten()
                        .is_some_and(|value| filter.matches_number(value)),
                    Filter::DateTime { .. } => fixture_date(row)
                        .key()
                        .is_ok_and(|value| filter.matches_number(value)),
                    _ => filter.matches_text(&data.text(row, *col)),
                }) {
                    self.matches.push(row);
                    if data.kind != crate::data::Kind::Logs {
                        self.summary.add(data, row);
                    }
                }
            }
            self.next = end;
            if self.next < data.rows {
                return None;
            }
        }
        if self.sorts.is_empty() {
            return Some(std::mem::take(&mut self.matches));
        }
        let sorter = self
            .sorter
            .get_or_insert_with(|| SortJob::new(std::mem::take(&mut self.matches)));
        if sorter.advance(6000, |a, b| {
            for rule in &self.sorts {
                if let Some(column) = columns.iter().find(|c| c.id == rule.column) {
                    let cmp = compare_values(
                        &data.text(a, rule.column),
                        &data.text(b, rule.column),
                        column.value_kind,
                        rule.descending,
                    );
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
            }
            std::cmp::Ordering::Equal
        }) {
            Some(std::mem::take(&mut sorter.indices))
        } else {
            None
        }
    }
}
