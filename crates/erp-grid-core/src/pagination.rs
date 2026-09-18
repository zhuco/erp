use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pagination {
    page: usize,
    page_size: usize,
}
impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
        }
    }
}
impl Pagination {
    pub fn page(&self) -> usize {
        self.page
    }
    pub fn page_size(&self) -> usize {
        self.page_size
    }
    pub fn page_count(&self, total: usize) -> usize {
        total.div_ceil(self.page_size)
    }
    pub fn set_page(&mut self, page: usize, total: usize) {
        self.page = page.min(self.page_count(total).saturating_sub(1));
    }
    pub fn set_page_size(&mut self, page_size: usize) {
        self.page_size = page_size.clamp(1, 10_000);
        self.page = 0;
    }
    pub fn range(&self, total: usize) -> Range<usize> {
        let page = self.page.min(self.page_count(total).saturating_sub(1));
        let start = page.saturating_mul(self.page_size).min(total);
        start..start.saturating_add(self.page_size).min(total)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_ranges_cover_results_once_and_clamp_empty_last_page_and_size() {
        let mut pager = Pagination::default();
        pager.set_page_size(10);
        assert_eq!(pager.page_count(23), 3);
        let mut visited = Vec::new();
        for page in 0..3 {
            pager.set_page(page, 23);
            visited.extend(pager.range(23));
        }
        assert_eq!(visited, (0..23).collect::<Vec<_>>());
        pager.set_page(999, 23);
        assert_eq!(pager.range(23), 20..23);
        pager.set_page(999, 0);
        assert_eq!(pager.page(), 0);
        assert_eq!(pager.range(0), 0..0);
        pager.set_page_size(0);
        assert_eq!(pager.page_size(), 1);
        pager.set_page_size(50);
        assert_eq!(pager.page(), 0);
        assert_eq!(pager.range(10), 0..10);
    }
}
