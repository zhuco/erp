#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub top: usize,
    pub left: usize,
    pub bottom: usize,
    pub right: usize,
}

impl CellRect {
    pub fn between(a: Cell, b: Cell) -> Self {
        Self {
            top: a.row.min(b.row),
            left: a.column.min(b.column),
            bottom: a.row.max(b.row).saturating_add(1),
            right: a.column.max(b.column).saturating_add(1),
        }
    }

    pub fn contains(self, cell: Cell) -> bool {
        self.top <= cell.row
            && cell.row < self.bottom
            && self.left <= cell.column
            && cell.column < self.right
    }

    fn valid(self) -> bool {
        self.top < self.bottom && self.left < self.right
    }

    /// Subtraction creates at most four disjoint rectangles, never one entry per cell.
    fn subtract(self, other: Self, output: &mut Vec<Self>) {
        let top = self.top.max(other.top);
        let left = self.left.max(other.left);
        let bottom = self.bottom.min(other.bottom);
        let right = self.right.min(other.right);
        if top >= bottom || left >= right {
            output.push(self);
            return;
        }
        for rect in [
            Self {
                bottom: top,
                ..self
            },
            Self {
                top: bottom,
                ..self
            },
            Self {
                top,
                bottom,
                right: left,
                ..self
            },
            Self {
                top,
                bottom,
                left: right,
                ..self
            },
        ] {
            if rect.valid() {
                output.push(rect);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionOperation {
    Replace,
    Add,
    Subtract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    TooManyRegions,
    InvalidRectangle,
}

#[derive(Clone, Debug, Default)]
pub struct Selection {
    regions: Vec<CellRect>,
}

impl Selection {
    pub const MAX_REGIONS: usize = 128;
    pub fn regions(&self) -> &[CellRect] {
        &self.regions
    }
    pub fn contains(&self, cell: Cell) -> bool {
        self.regions.iter().any(|r| r.contains(cell))
    }
    pub fn clear(&mut self) {
        self.regions.clear();
    }
    pub fn cell_count(&self) -> u128 {
        self.regions
            .iter()
            .map(|r| (r.bottom - r.top) as u128 * (r.right - r.left) as u128)
            .sum()
    }

    /// Atomic change: if fragmentation exceeds the limit, preserve the previous selection.
    pub fn apply(
        &mut self,
        rect: CellRect,
        operation: SelectionOperation,
    ) -> Result<(), SelectionError> {
        if !rect.valid() {
            return Err(SelectionError::InvalidRectangle);
        }
        let mut next = Vec::new();
        if operation != SelectionOperation::Replace {
            for existing in &self.regions {
                existing.subtract(rect, &mut next);
            }
        }
        if operation != SelectionOperation::Subtract {
            next.push(rect);
        }
        if next.len() > Self::MAX_REGIONS {
            return Err(SelectionError::TooManyRegions);
        }
        self.regions = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtract_middle_splits_and_readding_does_not_double_count() {
        let mut selection = Selection::default();
        let all = CellRect {
            top: 0,
            left: 0,
            bottom: 10,
            right: 10,
        };
        let hole = CellRect {
            top: 2,
            left: 2,
            bottom: 8,
            right: 8,
        };
        selection.apply(all, SelectionOperation::Replace).unwrap();
        selection.apply(hole, SelectionOperation::Subtract).unwrap();
        assert_eq!(selection.regions().len(), 4);
        assert_eq!(selection.cell_count(), 64);
        selection.apply(hole, SelectionOperation::Add).unwrap();
        assert_eq!(selection.cell_count(), 100);
    }

    #[test]
    fn rectangle_algebra_matches_cell_oracle() {
        let mut selection = Selection::default();
        let mut expected = [[false; 12]; 12];
        let mut seed = 72_u64;
        for step in 0..800 {
            let mut next = || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed >> 32) as usize % 12
            };
            let rect = CellRect::between(
                Cell {
                    row: next(),
                    column: next(),
                },
                Cell {
                    row: next(),
                    column: next(),
                },
            );
            let op = [
                SelectionOperation::Replace,
                SelectionOperation::Add,
                SelectionOperation::Subtract,
            ][step % 3];
            selection.apply(rect, op).unwrap();
            for (row, cells) in expected.iter_mut().enumerate() {
                for (column, selected) in cells.iter_mut().enumerate() {
                    let cell = Cell { row, column };
                    if op == SelectionOperation::Replace {
                        *selected = rect.contains(cell);
                    } else if rect.contains(cell) {
                        *selected = op == SelectionOperation::Add;
                    }
                    assert_eq!(selection.contains(cell), *selected);
                }
            }
            assert_eq!(
                selection.cell_count(),
                expected.iter().flatten().filter(|v| **v).count() as u128
            );
        }
    }

    #[test]
    fn limit_failure_is_atomic_and_large_selection_is_constant_size() {
        let mut selection = Selection::default();
        for row in 0..128 {
            selection
                .apply(
                    CellRect::between(
                        Cell {
                            row: row * 2,
                            column: 0,
                        },
                        Cell {
                            row: row * 2,
                            column: 0,
                        },
                    ),
                    SelectionOperation::Add,
                )
                .unwrap();
        }
        let before = selection.regions().to_vec();
        assert_eq!(
            selection.apply(
                CellRect::between(
                    Cell {
                        row: 999,
                        column: 0
                    },
                    Cell {
                        row: 999,
                        column: 0
                    }
                ),
                SelectionOperation::Add
            ),
            Err(SelectionError::TooManyRegions)
        );
        assert_eq!(selection.regions(), before);
        selection
            .apply(
                CellRect {
                    top: 0,
                    left: 0,
                    bottom: 1_000_000,
                    right: 100,
                },
                SelectionOperation::Replace,
            )
            .unwrap();
        assert_eq!(selection.regions().len(), 1);
        assert_eq!(selection.cell_count(), 100_000_000);
    }
}
