//! Bounded lookup over a host-owned query result. Column identities are stable
//! IDs, not positional indexes; callers cancel on data-generation changes.
use crate::ColumnId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindRequest {
    pub query: String,
    pub case_sensitive: bool,
    pub direction: FindDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindMatch {
    pub row: usize,
    pub column: ColumnId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindStep {
    Pending,
    Found(FindMatch),
    Exhausted,
}

pub struct FindCursor {
    request: FindRequest,
    needle: String,
    total_rows: usize,
    start: Option<FindMatch>,
    columns: Option<Vec<ColumnId>>,
    next: usize,
    remaining: usize,
    cells: usize,
}

impl FindCursor {
    /// Search after/before the current cell, wrapping once, including that
    /// cell last. Unknown starting identities begin at the result boundary.
    pub fn new_from(request: FindRequest, total_rows: usize, start: Option<FindMatch>) -> Self {
        let needle = if request.case_sensitive {
            request.query.clone()
        } else {
            request.query.to_lowercase()
        };
        Self {
            request,
            needle,
            total_rows,
            start,
            columns: None,
            next: 0,
            remaining: 0,
            cells: 0,
        }
    }

    pub fn step(
        &mut self,
        cell_budget: usize,
        columns: &[ColumnId],
        cell_text: impl Fn(usize, ColumnId) -> String,
    ) -> FindStep {
        if self.columns.is_none() {
            self.cells = self.total_rows.checked_mul(columns.len()).unwrap_or(0);
            self.remaining = if self.needle.is_empty() {
                0
            } else {
                self.cells
            };
            let start = self
                .start
                .filter(|cell| self.cells > 0 && cell.row < self.total_rows)
                .and_then(|cell| {
                    columns
                        .iter()
                        .position(|id| *id == cell.column)
                        .map(|column| cell.row * columns.len() + column)
                });
            if self.cells > 0 {
                self.next = match (self.request.direction, start) {
                    (FindDirection::Forward, Some(position)) => {
                        if position == self.cells - 1 {
                            0
                        } else {
                            position + 1
                        }
                    }
                    (FindDirection::Backward, Some(position)) => {
                        if position == 0 {
                            self.cells - 1
                        } else {
                            position - 1
                        }
                    }
                    (FindDirection::Forward, None) => 0,
                    (FindDirection::Backward, None) => self.cells - 1,
                };
            }
            self.columns = Some(columns.to_vec());
        }
        if self.columns.as_deref() != Some(columns) {
            self.remaining = 0;
        }
        for _ in 0..cell_budget.min(self.remaining) {
            let row = self.next / columns.len();
            let column = columns[self.next % columns.len()];
            self.remaining -= 1;
            self.next = match self.request.direction {
                FindDirection::Forward => {
                    if self.next == self.cells - 1 {
                        0
                    } else {
                        self.next + 1
                    }
                }
                FindDirection::Backward => {
                    if self.next == 0 {
                        self.cells - 1
                    } else {
                        self.next - 1
                    }
                }
            };
            let value = cell_text(row, column);
            let matched = if self.request.case_sensitive {
                value.contains(&self.needle)
            } else {
                value.to_lowercase().contains(&self.needle)
            };
            if matched {
                return FindStep::Found(FindMatch { row, column });
            }
        }
        if self.remaining == 0 {
            FindStep::Exhausted
        } else {
            FindStep::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(direction: FindDirection) -> FindRequest {
        FindRequest {
            query: "hit".into(),
            case_sensitive: false,
            direction,
        }
    }
    #[test]
    fn cell_budget_preserves_position_and_visits_each_cell_once() {
        let columns = [ColumnId(90), ColumnId(7), ColumnId(300)];
        let mut cursor = FindCursor::new_from(request(FindDirection::Forward), 2, None);
        let calls = std::cell::RefCell::new(Vec::new());
        for step in 0..6 {
            let result = cursor.step(1, &columns, |row, column| {
                calls.borrow_mut().push((row, column));
                "absent".into()
            });
            assert_eq!(
                result,
                if step == 5 {
                    FindStep::Exhausted
                } else {
                    FindStep::Pending
                }
            );
        }
        assert_eq!(calls.borrow().len(), 6);
        assert_eq!(calls.borrow()[3], (1, ColumnId(90)));
    }
    #[test]
    fn next_previous_and_wrap_use_column_identity_not_numeric_id() {
        let columns = [ColumnId(90), ColumnId(7), ColumnId(300)];
        let start = Some(FindMatch {
            row: 1,
            column: ColumnId(7),
        });
        for (direction, expected) in [(FindDirection::Forward, 300), (FindDirection::Backward, 90)]
        {
            let mut cursor = FindCursor::new_from(request(direction), 3, start);
            assert_eq!(
                cursor.step(1, &columns, |_, _| "HIT".into()),
                FindStep::Found(FindMatch {
                    row: 1,
                    column: ColumnId(expected)
                })
            );
        }
        let mut cursor = FindCursor::new_from(
            request(FindDirection::Forward),
            1,
            Some(FindMatch {
                row: 0,
                column: ColumnId(300),
            }),
        );
        assert_eq!(
            cursor.step(1, &columns, |_, _| "HIT".into()),
            FindStep::Found(FindMatch {
                row: 0,
                column: ColumnId(90)
            })
        );
    }
    #[test]
    fn empty_changed_columns_and_case_sensitive_search_terminate() {
        let mut query = request(FindDirection::Forward);
        query.case_sensitive = true;
        let mut cursor = FindCursor::new_from(query, 1, None);
        assert_eq!(
            cursor.step(1, &[ColumnId(1)], |_, _| "HIT".into()),
            FindStep::Exhausted
        );
        let mut cursor = FindCursor::new_from(request(FindDirection::Backward), usize::MAX, None);
        assert_eq!(
            cursor.step(1, &[], |_, _| unreachable!()),
            FindStep::Exhausted
        );
        let mut cursor = FindCursor::new_from(request(FindDirection::Forward), 10, None);
        assert_eq!(
            cursor.step(0, &[ColumnId(1)], |_, _| unreachable!()),
            FindStep::Pending
        );
        assert_eq!(
            cursor.step(1, &[ColumnId(2)], |_, _| unreachable!()),
            FindStep::Exhausted
        );
    }
}
