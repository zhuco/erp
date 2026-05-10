---
name: erp-ui
description: Use when 在 ERP Web 中新建或重构 UI 页面、落地统一工作台模板、调整左侧/底部/右侧模块、替换主表数据源/工具栏常用按钮/中间搜索数据源，或判断应修改工作台模板还是具体页面。
---

# erp-ui

## 概览

指导在 `apps/erp-web` 中基于模块化规范完成 ERP Web UI 的新建与重构，确保工作台布局、设置页布局与组件复用一致。

## 快速流程

1. 明确需求类型（新建/重构）与页面位置，确认页面属于工作台、设置页、策略列表或普通业务页。
2. 阅读 `docs/FRONTEND_GUIDE.md` 获取工作台布局、复用方式、表格决策与测试要求；用户提到“UI 开发文档”默认指该文件。
3. 新设计工作台页面必须以 `WorkbenchPageTemplate` 为唯一推荐入口，并对照 `OrderReviewPage.tsx`、`WorkbenchLayout`、`useWorkbenchPanels()` 与 `ui-modules` 组件确认骨架。
4. 设置类页面必须以 `SettingsPageTemplate` / `SettingsPageSection` 为推荐入口，不要在页面中直接使用 `UiModule07SettingsLayout` / `UiModule07SettingsSection`。
5. 严格按统一工作台模板一比一复制页面骨架，只替换业务数据源、主表列模型、主表上方工具栏左侧常用按钮和中间搜索数据源。
6. 通过配置或 props 控制左侧模块、底部子表、右侧模块显示或隐藏，不要为单页随意删改工作台结构。
7. 修改 UI 前先判断应改“工作台模板/共享组件”还是“落地 UI 页/页面适配层”，再动代码。
8. 处理样式与交互细节，保持已有命名、结构、间距、测试标识和状态模型，避免重复造轮子。

## 推荐入口

- **工作台页唯一推荐入口**：页面从 `@/components/page-templates` 导入 `WorkbenchPageTemplate`、`WorkbenchToolbar` 及相关 hooks/types；禁止页面直接导入 `@/components/workbench/templates` 或使用 `OrderReviewWorkbenchTemplate` / `OrderReviewWorkbenchToolbarTemplate`。
- **工具栏统一入口**：`WorkbenchPageTemplate` 的 `toolbar` 必须使用 `WorkbenchToolbar`；左侧用 `leftContent` 放常用按钮，中间用搜索 props / `centerContent` 接业务搜索数据源，右侧保留模板通用能力。
- **设置页推荐入口**：设置类页面从 `@/components/page-templates` 导入 `SettingsPageTemplate`、`SettingsPageSection`；页面不要直接使用模块 7 的低层 `UiModule07SettingsLayout` / `UiModule07SettingsSection`。
- **低层模块只做实现细节**：`OrderReviewWorkbenchTemplate`、`OrderReviewWorkbenchToolbarTemplate`、`UiModule07SettingsLayout`、`UiModule07SettingsSection` 仅供 page-template 包装层或共享模块内部使用。

## 统一工作台模板一比一规则

- **必须保留模板骨架**：顶部信息区、左侧模块、主表上方工具栏、主表、底部子表、右侧模块的顺序、层级、间距、类名语义和折叠行为应与统一工作台模板一致。
- **只替换业务适配点**：新页面只替换列表/详情/子表/统计等数据源，主表列定义，工具栏左侧常用按钮，以及工具栏中间搜索的数据源、字段映射、占位文案和触发逻辑。
- **工具栏位置固定**：主表上方工具栏左侧放当前业务常用按钮，中间放当前业务搜索入口，右侧复用模板已有通用能力；不得把常用按钮、搜索框或通用操作挪到自定义区域。
- **模块显隐走配置**：左侧模块、底部子表、右侧模块需要显示或隐藏时，优先通过 `useWorkbenchPanels()`、页面适配配置或组件 props 控制；没有数据时使用隐藏/折叠/空态，不复制一份新布局。
- **数据源替换不改模板**：业务查询参数、React Query key、API 调用、表格行类型、子表数据、详情数据和统计指标都属于页面适配层，不能为了接数据改动模板结构。

## 模板还是落地页决策

- **改工作台模板/共享组件**：布局骨架、面板尺寸/折叠/拖拽、工具栏区域划分、通用按钮位置、全局视觉间距、共享空态/加载态、多个工作台页都会受益的缺陷或能力。
- **改落地 UI 页/适配层**：业务数据源、筛选字段、搜索字段、常用按钮清单、权限控制、列定义、行操作、左/底/右模块是否显示、页面文案、`data-testid` 后缀。
- **先写判断再实现**：动手修改 UI 前，在计划或说明中明确“本次改模板”或“本次改落地页”，并说明依据；不确定时优先保持模板不变，只做页面适配。
- **模板改动要回归**：凡是改 `WorkbenchLayout`、`page-templates`、`ui-modules` 或订单工作台共享入口，必须考虑所有采用该模板的页面，并补/跑对应共享组件或页面回归测试。

## 模块选型与复用要点

- 订单类新页面通常复用模块 1-5（左侧快速查询、三栏工具栏、主表、底部子表、右侧详情）。
- 策略列表类页面复用模块 6（策略表）。
- 设置类页面复用 `SettingsPageTemplate` / `SettingsPageSection`，模块 7 只作为其内部实现。
- 需要工作台面板时，使用 `useWorkbenchPanels()` 挂载 left/right/bottom 面板。
- 左侧工作台模板优先复用 `UiModule08OrderReviewLeftPanelTemplate` 或同类共享封装。
- 主表上方工具栏统一复用 `WorkbenchToolbar`；需要更底层能力时才在共享组件内部使用 `UiModule02ActionSearchToolbar`。
- 主表、底部子表、右侧详情优先复用 `UiModule03MainTableSection`、`UiModule04BottomTable`、`UiModule05OrderReviewRightPanelTemplate` 或现有页面共享入口。

## 参考入口

- 规范文档：`docs/FRONTEND_GUIDE.md`
- 推荐模板入口：`apps/erp-web/src/components/page-templates/`
- 模块组件目录：`apps/erp-web/src/components/ui-modules/`
- 参考页面：`apps/erp-web/src/pages/orders/OrderReviewPage.tsx`（模块 1-5）、`apps/erp-web/src/pages/sales/SalesStrategySettingsPage.tsx`（模块 6）、`apps/erp-web/src/pages/settings/SettingsPage.tsx`（模块 7）

## 测试准备（当前不强制）

当前需求不要求补 E2E 或单测，但页面交互应预留 `data-testid` 等测试标识，便于后续补充测试时快速接入（规范可参考 `docs/FRONTEND_GUIDE.md`）。
如果修改工作台模板或共享模块，即使需求未强制 E2E，也应优先补/跑共享组件测试，确认其他工作台页面未被破坏。
