use std::ops::Range;

/// Sparse inserted details keep million-row grids independent of total row count.
pub(crate) struct RowLayout {
    rows: usize,
    expanded: Vec<usize>,
    row_height: f32,
}
pub(crate) const DETAIL_HEIGHT: f32 = 260.0;
impl RowLayout {
    pub fn with_row_height(
        rows: usize,
        expanded: impl Iterator<Item = usize>,
        row_height: f32,
    ) -> Self {
        let mut expanded: Vec<_> = expanded.filter(|row| *row < rows).collect();
        expanded.sort_unstable();
        expanded.dedup();
        Self {
            rows,
            expanded,
            row_height: row_height.clamp(16.0, 64.0),
        }
    }
    pub fn top(&self, row: usize) -> f32 {
        row as f32 * self.row_height
            + self.expanded.partition_point(|index| *index < row) as f32 * DETAIL_HEIGHT
    }
    pub fn height(&self) -> f32 {
        self.top(self.rows)
    }
    pub fn expanded(&self, row: usize) -> bool {
        self.expanded.binary_search(&row).is_ok()
    }
    fn containing(&self, y: f32) -> usize {
        let (mut lo, mut hi) = (0, self.rows);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.top(mid + 1) <= y {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }
    pub fn hit(&self, y: f32) -> Option<usize> {
        let row = self.containing(y.max(0.0));
        (row < self.rows && y < self.top(row) + self.row_height).then_some(row)
    }
    pub fn visible(&self, offset: f32, height: f32) -> Range<usize> {
        self.containing(offset.max(0.0)).saturating_sub(1)
            ..self
                .containing((offset + height).max(0.0))
                .saturating_add(1)
                .min(self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn details_do_not_become_data_rows_or_change_row_numbers() {
        let layout = RowLayout::with_row_height(1_000_000, [0, 5].into_iter(), 28.0);
        assert_eq!(layout.hit(27.0), Some(0));
        assert_eq!(layout.hit(28.0), None);
        assert_eq!(layout.hit(287.0), None);
        assert_eq!(layout.hit(288.0), Some(1));
        assert_eq!(layout.top(6), 6.0 * 28.0 + 520.0);
        assert_eq!(layout.visible(100.0, 100.0), 0..1);
        assert_eq!(layout.height(), 28_000_520.0);
    }

    #[test]
    fn compact_metrics_are_used_for_hit_testing_and_scroll_geometry() {
        let layout = RowLayout::with_row_height(3, [].into_iter(), 21.0);
        assert_eq!(layout.hit(20.9), Some(0));
        assert_eq!(layout.hit(21.0), Some(1));
        assert_eq!(layout.top(2), 42.0);
        assert_eq!(layout.height(), 63.0);
        assert_eq!(layout.visible(21.0, 21.0), 0..3);
    }
}
