//! Single source of truth for workspace page metadata.
use super::Page;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PageSpec {
    pub page: Page,
    pub module: &'static str,
    pub group: &'static str,
    pub name: &'static str,
    pub permission: &'static str,
    pub connected: bool,
}

pub(crate) const PAGES: &[PageSpec] = &[
    PageSpec {
        page: Page::Products,
        module: "货品",
        group: "查询",
        name: "货品查询",
        permission: "products:read",
        connected: true,
    },
    PageSpec {
        page: Page::Orders,
        module: "销售",
        group: "订单中心",
        name: "订单查询",
        permission: "orders:read",
        connected: true,
    },
    PageSpec {
        page: Page::Reports,
        module: "开发工具",
        group: "预览",
        name: "报表预览",
        permission: "dev:preview",
        connected: true,
    },
    PageSpec {
        page: Page::WarehouseTree,
        module: "开发工具",
        group: "预览",
        name: "仓库树预览",
        permission: "dev:preview",
        connected: true,
    },
    PageSpec {
        page: Page::Lab,
        module: "开发工具",
        group: "Lab",
        name: "网格实验台",
        permission: "dev:lab",
        connected: true,
    },
];

pub(crate) fn spec(page: Page) -> &'static PageSpec {
    PAGES
        .iter()
        .find(|item| item.page == page)
        .unwrap_or(&PAGES[0])
}
