# ERP 前端 UI 统一性下一步开发计划

> **适用范围：** `apps/erp-web` 前端工作台、业务列表页、设置页、订单/商品/仓储/WMS/营销/基础资料等 ERP 高频操作页面。  
> **执行方式：** 使用 Codex 多子任务并行推进，最多 5 个子任务；子任务启动时不要指定模型；子任务结束后必须关闭并输出交付摘要。  
> **核心目标：** 将 ERP 前端从“模板接入率达标”推进到“用户感知统一性 95% 以上”，降低新用户学习成本、老用户跨模块切换成本和后续页面扩展成本。

---

## 0. 文档导航、最新评分与当前差距

### 0.1 配套文档

- 最新基线与差距：[`ERP_FRONTEND_UI_UNIFORMITY_BASELINE.md`](./ERP_FRONTEND_UI_UNIFORMITY_BASELINE.md)。
- 总体计划与里程碑：[`ERP_FRONTEND_UI_UNIFORMITY_PLAN.md`](./ERP_FRONTEND_UI_UNIFORMITY_PLAN.md)。
- 新页面接入与 Code Review 清单：[`ERP_FRONTEND_UI_PAGE_CHECKLIST.md`](./ERP_FRONTEND_UI_PAGE_CHECKLIST.md)。
- UI 文案与术语规范：[`ERP_FRONTEND_UI_TERMINOLOGY.md`](./ERP_FRONTEND_UI_TERMINOLOGY.md)。

### 0.2 2026-05-17 最新完成度评估

| 项目 | 当前值 | 目标 | 差距/判断 |
|---|---:|---:|---|
| UI 统一性综合评分 | `68.5 / 100` | `95 / 100` | 还差 `26.5` 分，未达标 |
| 模板治理口径采用率 | `180/180`，`100.00%` | `>= 95%` | 已达标，继续防倒退 |
| P0 适用页面模板治理 | `80/80`，`100.00%` | `100%` | 已达标 |
| P0 裸 Table 页面 | `11` | `0` | 还需清理 11 页 |
| P0 裸 Card 页面 | `43` | 阶段目标 `< 30`，最终尽量归零 | 下一批至少压降 13 页 |
| P0 工业紧凑表格契约 | `71/82` | P0 主表全覆盖 | 还需覆盖剩余 P0 主表/子表 |
| P0 工具栏统一标识 | `61/82` | P0 全覆盖 | 还需补齐约 21 页 |
| P0 `data-testid` | `79/82` | P0 全覆盖 | 基本达标，补齐剩余页 |
| P0 列设置标识 | `73/82` | P0 主表全覆盖 | 还需补齐约 9 页 |

### 0.3 评分拆分与差距来源

| 维度 | 权重 | 当前得分 | 满分差距 | 下一步提分方式 |
|---|---:|---:|---:|---|
| 模板接入统一性 | 20 | 20.0 | 0.0 | 保持 `lint:page-templates` 不倒退 |
| 操作流程统一性 | 25 | 10.9 | 14.1 | 补 `toolbarLeft/toolbarRight/actionBar/header`、统一主操作顺序 |
| 表格体验统一性 | 20 | 10.6 | 9.4 | P0 裸 Table 清零，子表迁入工业表格/底部表格契约 |
| 视觉与文案统一性 | 15 | 11.0 | 4.0 | 压降裸 Card，统一 section、空态、错误态和说明文案 |
| 可访问性与键盘效率 | 10 | 4.9 | 5.1 | 补焦点返回、`aria-label`、键盘关闭、E2E/a11y 断言 |
| 工程治理与回归门禁 | 10 | 9.7 | 0.3 | 将评分脚本从趋势工具升级为逐步阻断门禁 |

当前结论：模板采用率已经不是主要矛盾；真正的差距来自“同类页面的操作入口、表格行为、局部容器和键盘可用性仍不够一致”。要达到 95%，下一轮应优先把 P0 裸 Table 清零，并把 P0 裸 Card 从 `43` 降到 `30` 以下。

### 0.4 五个任务当前推进状态

| 任务 | 当前状态 | 已有交付 | 下一步重点 |
|---|---|---|---|
| 子任务 1：基线评分 | 已可运行并用于趋势评估 | `apps/erp-web/scripts/assess-ui-uniformity.mjs`、最新基线报告 | 增加评分规则单测，将 P0 裸 Table/裸 Card 阈值逐步接入门禁 |
| 子任务 2：模板契约 | 已形成主表/工具栏契约，仍需补状态和键盘契约 | `WorkbenchPageTemplate`、`UiModule03MainTableSection`、模板采用率脚本 | 统一 loading/empty/error/permission slot，禁止页面绕过主操作区 |
| 子任务 3：P0 页面统一改造 | 持续推进中，已完成多批页面 | 财务、售后、采购、库存、商品、销售、WMS 多页契约迁移 | 下一批清理 `ProductBarcodesPage`、`ProductChannelAvailabilityPage`、`StockCountsPage`、`StockMovesPage`、`StockTransfersPage` |
| 子任务 4：文档固化和交叉链接 | 进行中 | 本计划、基线报告、页面接入清单、术语表 | 每轮改造后同步更新分数、差距、下一批页面和门禁口径 |
| 子任务 5：E2E 与 CI 门禁 | 基础具备，统一性专用门禁不足 | Playwright、a11y、性能、release gate 等脚本基础 | 增加工作台统一性 E2E 集合，并将评分趋势输出接入 CI/PR 注释 |

### 0.5 当前基线口径

当前 `assess:ui-uniformity` 已可作为统一性趋势和差距判断工具；综合评分仍不应单独替代人工验收。硬门槛仍是 `lint:page-templates`、`lint`、`typecheck`、`build`、UTF-8/乱码检查和关键契约测试。下一阶段建议先把 P0 裸 Table 清零作为阻断项，再逐步提高 P0 裸 Card、工具栏和列设置阈值。

---

## 1. 背景与现状判断

当前项目已经具备工业级 UI 工作台基础，且模板治理口径已达到 `100.00%`。但 ERP 系统的统一性不能只看“有没有套模板”。ERP 用户每天会在订单、商品、仓储、财务、售后、系统配置等模块之间频繁切换，真正降低学习成本的是：页面结构、操作顺序、表格行为、详情入口、状态反馈、文案术语和快捷键长期一致。

结合最新评分脚本与多轮 Codex 子任务改造，得到以下现状：

- `apps/erp-web/scripts/check-page-template-usage.mjs` 当前模板治理输出为：`215` 个页面文件、`180` 个适用页面、`180` 个模板页面，采用率 `100.00%`，baseline 例外 `30` 个，超过 `95%` 目标。
- 最新 UI 统一性综合评分为 `68.5 / 100`，目标 `95`，差距 `26.5`；模板接入已满分，但操作流程、表格体验、视觉文案和键盘效率仍有明显缺口。
- 完整 `WorkbenchPageTemplate` 直接采用页面仍约 `31/215`，约 `14.4%`；大量页面继续以 `UiModule03MainTableSection` 和页面内局部容器组合实现，统一性依赖契约测试和门禁约束。
- P0 当前仍有 `11` 个裸 Table 页面和 `43` 个裸 Card 页面，是下一轮最直接、最可量化的提分来源。
- 已有统一模板能力：`IndustrialWorkbenchTemplate`、`WorkbenchPageTemplate`、`WorkbenchToolbar`、工作台面板尺寸、列设置、选择控制、交互控制、视图状态控制、模板采用率指标和可见文案 Unicode 约束。
- 现有门禁较完整：`lint:page-templates`、ESLint、TypeScript、Vitest、Playwright、a11y、性能预算、交互延迟、UTF-8/乱码检查等已经存在。
- 主要短板不在“有没有模板”，而在“统一性是否能被结构化约束”：工具栏顺序、搜索区、主表结构、列设置、空态/加载态/错误态、术语词典、设计 token、键盘效率、跨页面 E2E 和视觉基线还缺少全局门禁。

因此，下一阶段应从“模板接入率 95%+”升级为“用户感知统一性 95%+”。

---

## 2. 当前统一性能力清单

### 2.1 模板与布局能力

- `apps/erp-web/src/components/page-templates/workbench/IndustrialWorkbenchTemplate.tsx`：工业工作台通用布局骨架，覆盖顶部工具区、主表区、左右面板、底部面板、覆盖层、分页/底部滚动条、稳定 `data-testid` 与 `region`。
- `apps/erp-web/src/components/page-templates/workbench/WorkbenchPageTemplate.tsx`：工作台页面统一入口，约束页面通过 `WorkbenchToolbar` 接入主操作区。
- `apps/erp-web/src/components/page-templates/workbench/WorkbenchToolbar.tsx`：统一工具栏对外封装。
- `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`：ERP 外层工作台布局，包含顶部模块导航、功能搜索、路由标签、左右/底部面板拖拽、字体缩放、快捷键、可访问性公告等。
- `apps/erp-web/src/app/layout/workbenchPanelSizing.ts`：统一面板默认尺寸、最小值、最大值。
- `apps/erp-web/src/app/layout/workbenchPanels.tsx`：统一工作台面板上下文。

### 2.2 表格与交互能力

- `apps/erp-web/src/components/page-templates/workbench/workbenchGridColumnPolicy.ts`：统一选择列、序号列、固定列、不可隐藏/不可移动策略。
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchColumnSettings.ts`：列显示、排序、持久化、重置能力。
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchSelectionController.ts`：统一选择状态控制。
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchInteractionController.ts`：统一快捷键、右键菜单、公告交互控制。
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchViewStateController.ts`：统一视图状态序列化与记忆。

### 2.3 工程门禁能力

- `apps/erp-web/scripts/check-page-template-usage.mjs`：模板采用率阈值为 `0.95`，并禁止部分旧模板 token 和旧设置布局导入。
- `apps/erp-web/src/components/page-templates/workbench/workbenchTemplateMetrics.ts`：模板采用率计算与 `>= 0.95` 达标判断。
- `apps/erp-web/src/app/layout/visibleText.ts`：运行时处理用户可见 CJK Unicode 转义。
- `apps/erp-web/src/app/layout/visibleTextEncoding.test.ts`：静态约束前端用户可见中文不得继续写成 `\uXXXX` 字面量。
- `scripts/verify_utf8_encoding.mjs`：UTF-8、BOM 和疑似中文乱码检查。

---

## 3. 主要问题与学习成本来源

### 3.1 模板采用率与用户感知统一性不等价

虽然模板采用率脚本当前显示 `100.00%`，但页面内仍可能存在以下问题：

- 页面只是引用模板，但工具栏、搜索、主表、状态区仍自定义；
- 大量半统一页面使用 `UiModule03MainTableSection`，缺少完整工作台的左筛选、右详情、底部明细和统一列设置；
- 直接使用 Ant Design `Table` / `Card` 的页面仍多，用户在不同模块中看到不同的信息密度与操作入口；
- `36` 个 baseline exceptions 仍需要治理，至少要做到不可新增、原因可追踪、逐步清零。

### 3.2 高频用户操作存在跨页偏差

典型偏差包括：

- **工具栏不一致：** 搜索框、刷新、导入、导出、批量操作、列设置的位置和顺序不固定。
- **详情入口不一致：** 有些页面右侧详情，有些页面 Drawer，有些页面 Modal，有些跳转详情页。
- **表格选择不一致：** `selectionCellMode="custom"` 与 `selectionHeaderMode="allInvert"` 只覆盖少数页面。
- **状态展示不一致：** 空态、加载态、错误态、权限态和部分失败状态缺少统一结构。
- **设置类页面模型不清晰：** `SettingsPageTemplate` 与工作台页面之间需要建立统一的“设置工作台”学习模型。
- **术语与文案风险：** 已解决 `\uXXXX` 显示问题，但“查询/搜索/检索”“刷新/重新加载”“列设置/列配置”等术语仍需要词典约束。
- **键盘与可访问性覆盖不足：** a11y 当前只覆盖少数路线，且部分 axe 规则仍被全局禁用。

---

## 4. 下一阶段目标

### 4.1 总目标

在保持模板采用率 `>= 95%` 且当前 `100%` 不倒退的前提下，建立 ERP 前端 UI 统一性治理体系，使核心 ERP 页面达到“用户感知统一性 95% 以上”。

### 4.2 用户目标

- 新用户进入任意业务模块后，能复用同一套操作习惯完成查询、筛选、列表浏览、详情查看、批量处理、导出和返回。
- 老用户从订单模块切换到商品、仓储、WMS、售后、财务、基础资料时，不需要重新寻找常用按钮和详情入口。
- 高频操作不因页面差异产生额外认知负担。
- 异常、空数据、加载、权限不足、部分失败等状态在全系统中呈现一致。

### 4.3 工程目标

- 统一模板不仅被使用，还能约束页面结构、交互行为、文案规范、状态展示和测试门禁。
- 新增页面默认通过模板契约和 lint 检查，不允许绕过统一工作台规范。
- 高频页面具备稳定 E2E 回归，覆盖用户真实操作路径。
- UI 统一性评分可持续评估、可追踪、可在 CI 或本地验收中作为门禁。

---

## 5. 95% 完成度定义

下一阶段建议把“ERP 前端 UI 统一性完成度”拆成 6 个维度，总分 100 分，目标 `>= 95`。

| 维度 | 权重 | 达标说明 |
| --- | ---: | --- |
| 模板接入统一性 | 20 | 页面使用统一工作台/设置页模板，不绕开模板自组装布局；baseline exceptions 不增加 |
| 操作流程统一性 | 25 | 查询、筛选、刷新、导出、批量操作、详情、编辑、返回、快捷键跨页一致 |
| 表格体验统一性 | 20 | 列设置、固定列、选择列、序号列、分页、空态、加载态、横向滚动一致 |
| 视觉与文案统一性 | 15 | 标题、按钮、状态标签、提示语、密度、间距、颜色语义、Unicode/乱码约束一致 |
| 可访问性与键盘效率 | 10 | 焦点顺序、中文 aria-label、Esc/Tab、快捷键、焦点返回一致 |
| 工程治理与回归门禁 | 10 | lint、typecheck、unit、E2E、统一性扫描、报告生成稳定 |

### 5.1 达标硬门槛

- 模板接入率保持 `>= 95%`，目标保持当前 `100%`。
- 核心 P0 页面统一性评分 `>= 95`。
- 所有强约束页面通过统一工作台契约测试。
- 高频操作路径 E2E 全部通过。
- 不出现用户可见 Unicode 转义、乱码、英文混杂按钮、同义不同名按钮。
- 新增页面无法绕过模板治理脚本和文案约束测试。
- `lint`、`typecheck`、`build`、关键单测、关键 E2E 通过。

---

## 6. 页面分级与改造范围

### 6.1 P0 页面：必须优先达标

P0 是高频、跨岗位、误操作成本高的页面，建议优先包括：

- **订单/销售：** `OrderReviewPage`、`OrderQueryPage`、`OrderExceptionsPage`、`SalesOrderQueryWorkbenchPage`、`ShopOrderQueryPage`、`SalesShopConfigPage`、`LogisticsUploadPage`、`SyncJobRunsPage`。
- **商品主数据：** `ProductsPage`、`ProductBarcodesPage`、`ProductUnitsPage`、`ProductLifecyclePage`、`GoodsProductsQueryPage`。
- **WMS/仓储：** `WarehouseDirectoryPage`、`ContainersPage`、`WmsOutboundsPage`、`WmsPickTasksPage`、`WmsPackPage`、`WmsShipPage`、`WmsWavesPage`。
- **系统与基础资料：** `PermissionsPage`、`RolesPage`、`UsersPage`、`DataScopesPage`、客户/标签/供应商/物流费用/渠道目录页。
- **设置类核心页：** `SalesSettingsPage`、`SystemSettingsPage`、`SystemIntegrationsPage`、`SystemCustomFieldsPage`。

P0 页面必须满足：统一模板、统一工具栏、统一搜索区、统一表格行为、统一列设置、统一状态展示、基础键盘可用、单测或 E2E 覆盖核心路径。

### 6.2 P1 页面：阶段内统一

P1 是中频页面，适合在 P0 后批量推进：

- 采购：`PurchaseOrdersPage`、`PurchaseReceiptsPage`、`PurchaseReturnsPage`、`PurchaseSuggestionsPage`。
- 库存：`InventoryPage`、`InventoryLedgerPage`、`InventoryLocksPage`、`StocktakesPage`。
- 售后：`AfterSalesListPage`、`AfterSaleCaseListPage`、`RefundDocsPage`。
- 财务：`ApManagementPage`、`ArManagementPage`、`InvoicesPage`、`TaxManagementPage`。

P1 页面目标：模板接入达标、操作入口一致、表格和状态展示不明显偏离，至少具备静态契约检查或组件单测。

### 6.3 P2 页面：豁免与专用模板治理

P2 不强制套工作台模板，但必须登记原因并避免复制业务工作台骨架：

- 登录页、异常页、纯展示页、临时诊断页；
- 创建/编辑表单页，例如商品创建/编辑、套装创建/编辑，建议定义“编辑器模板”；
- 详情阅读页，例如订单详情、采购详情，建议定义“详情阅读模板”；
- 指标看板页，例如 insights、stats，建议定义“指标看板模板”。

---

## 7. 统一工作台标准

### 7.1 页面骨架

工作台页面固定为以下顺序：

1. 顶部模块导航；
2. 当前路由标签/快捷入口；
3. 页面标题与说明；
4. 工具栏；
5. 查询/筛选区；
6. 主表；
7. 右侧详情、底部明细、左侧筛选等扩展面板；
8. 分页、汇总、状态栏；
9. 列设置、视图保存、密度切换等辅助控制。

### 7.2 工具栏标准

按钮按固定分组排序：

1. 主操作：新增、保存、提交、审核；
2. 查询操作：查询、重置、刷新；
3. 数据操作：导入、导出、同步；
4. 批量操作：批量审核、批量取消、批量标记；
5. 视图操作：列设置、密度、保存视图；
6. 更多操作：低频操作统一收纳。

禁止同一业务含义使用多个按钮名称；禁止主按钮在不同页面改变位置；禁止页面单独实现工作台工具栏样式。

### 7.3 查询区标准

查询区统一搜索框位置、高级筛选入口、展开/收起行为、重置含义、查询后焦点返回、快捷键行为和默认筛选值展示方式。

### 7.4 面板标准

左侧、右侧、底部面板统一折叠入口、拖拽分割条、最小/最大尺寸、键盘可访问性、本地记忆策略和空内容展示方式。

---

## 8. 统一交互标准

| 操作 | 统一要求 |
| --- | --- |
| 查询 | 固定入口、固定按钮文案、Enter 可触发 |
| 重置 | 清空筛选并回到默认查询态 |
| 刷新 | 保留筛选条件、当前视图和页签上下文 |
| 导出 | 统一在数据操作组或更多菜单中 |
| 新增 | 主按钮样式一致 |
| 编辑 | 行内、详情侧栏或详情页入口保持同类一致 |
| 删除/取消 | 必须确认，危险色和文案统一 |
| 批量操作 | 必须依赖明确选中态，展示选中数量 |
| 列设置 | 固定入口、固定中文 aria-label、固定浮动/弹窗行为 |
| 返回/关闭 | 关闭详情、抽屉、弹窗后焦点返回触发入口 |

### 8.1 快捷键标准

| 快捷键 | 行为 |
| --- | --- |
| `Ctrl / Cmd + K` | 打开命令面板或功能搜索 |
| `Enter` | 在查询区触发查询 |
| `Esc` | 关闭弹窗、抽屉或非阻断浮层 |
| `Tab` | 按视觉顺序移动焦点 |
| `Shift + Tab` | 反向移动焦点 |
| 方向键 | 在表格、菜单、列表中移动 |

---

## 9. 统一表格标准

### 9.1 主表标准

主表统一：行选择方式、行高、表头高度、固定列策略、序号列名称、空态、加载态、错误态、横向滚动条、分页、汇总信息、列设置入口。

### 9.2 工业表格契约

工作台列表页应优先满足：

- 使用工业紧凑表格预设，例如 `gridPreset="industrial-review-compact"`。
- 使用统一选择契约，例如 `selectionCellMode="custom"` 与 `selectionHeaderMode="allInvert"`。
- 选择列、序号列不可隐藏、不可移动，并由统一策略控制。
- 金额、数量、时间、状态、空值展示遵循统一列渲染规则。
- 主列表禁止绕过统一模板裸用 `<Table`，除非位于右侧详情、底部明细等登记白名单区域。

### 9.3 表格状态标准

| 状态 | 统一展示 |
| --- | --- |
| 加载中 | 骨架屏或统一 Spin，不改变页面结构 |
| 无数据 | 说明当前筛选下无数据，并提供重置/刷新建议 |
| 查询失败 | 展示错误原因和重试入口 |
| 权限不足 | 展示权限说明和联系管理员建议 |
| 部分失败 | 展示成功数、失败数和失败明细入口 |
| 后台处理中 | 展示任务状态和刷新方式 |

---

## 10. 统一视觉与文案标准

### 10.1 文案标准

- 所有用户可见文案使用中文。
- 禁止页面显示 `\uXXXX` 形式 Unicode 转义。
- 同一动作使用同一文案：
  - 使用“查询”，避免混用“搜索/检索/筛选”；
  - 使用“重置”，避免混用“清空/恢复/初始化”；
  - 使用“刷新”，避免混用“重新加载/更新”；
  - 使用“导出”，避免混用“下载/输出”；
  - 使用“列设置”，避免混用“字段设置/表头设置/列配置”。
- 错误提示应说明“发生了什么 + 用户能做什么”。

### 10.2 视觉标准

应将页面背景、卡片背景、分割线、主按钮、危险操作、成功/警告/异常/禁用状态色、字号、行高、间距、圆角、阴影、表格密度沉淀为统一 token 或集中样式，禁止页面级随意硬编码。

### 10.3 密度标准

ERP 默认应保持高信息密度但不牺牲可读性：主表紧凑、搜索区可折叠、详情区不挤压主操作、低频操作收纳到更多菜单。

---

## 11. 工程约束与质量门禁

### 11.1 必跑命令

涉及 UI 统一性改造后至少执行：

```bash
pnpm --dir apps/erp-web run lint:page-templates
pnpm --filter erp-web typecheck
pnpm --filter erp-web lint
pnpm --filter erp-web build
```

### 11.2 建议单测集合

```bash
pnpm --dir apps/erp-web exec vitest --run   src/app/layout/visibleText.test.ts   src/app/layout/visibleTextEncoding.test.ts   src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts   src/app/layout/WorkbenchLayout.interactions.test.ts   src/app/layout/WorkbenchLayout.splitter-a11y.test.ts   src/components/page-templates/workbench/workbenchContracts.test.ts   src/components/page-templates/workbench/IndustrialWorkbenchTemplate.test.ts   src/components/page-templates/workbench/workbenchColumnSettings.test.ts
```

### 11.3 建议 E2E 集合

```bash
pnpm --filter erp-web test:e2e -- --project=chromium   e2e/workbench-a11y.spec.ts   e2e/workbench-layout-presets.spec.ts   e2e/order-workbench-pages.spec.ts   e2e/order-review-interactions.spec.ts
```

### 11.4 建议新增命令

- `pnpm --filter erp-web test:ui-contract`：轻量 UI 合同门禁，覆盖模板采用率、编码约束、菜单路由、核心 workbench contract、typography 单测。
- `pnpm --filter erp-web test:e2e:workbench-uniformity`：统一性 E2E 专项，聚合 a11y、typography、layout presets、菜单导航、订单工作台页面。
- `pnpm --dir apps/erp-web run assess:ui-uniformity`：统一性评分报告，输出模板接入率、P0/P1/P2 达标情况、豁免数量、表格契约、文案约束、总评分。

---


### 11.5 基线评分脚本使用方法

首版 UI 统一性评分脚本已经接入 `apps/erp-web/package.json`，用于快速查看当前页面统一性基线。

```bash
pnpm --dir apps/erp-web run assess:ui-uniformity
```

需要给 CI、机器人或后续报告消费时，使用 JSON 输出：

```bash
pnpm --dir apps/erp-web run assess:ui-uniformity -- --json
```

在 Windows PowerShell 中，如果需要把 JSON 继续交给 Node/Python 解析，优先直接调用脚本，避免管道编码在少数环境中附加 BOM：

```bash
node apps/erp-web/scripts/assess-ui-uniformity.mjs --json
```

脚本当前输出重点包括：

- 页面总数、适用页面数、baseline 例外数。
- 治理口径模板采用数和推荐模板采用率。
- `WorkbenchPageTemplate` 直接采用数。
- `UiModule03MainTableSection`、裸 `Table`、裸 `Card` 页面数量。
- `industrial-review-compact`、`selectionCellMode="custom"`、`selectionHeaderMode="allInvert"` 等工业表格契约数量。
- P0/P1/P2 建议分类概要。
- 首版 UI 统一性评分和维度拆分。

使用约束：

- 脚本用于趋势跟踪和差异定位，不替代人工产品验收。
- 总评分在规则固化前不作为唯一发布门禁。
- 新增豁免必须能在报告中定位到页面和原因。
- 当脚本输出与真实 UI 感知冲突时，优先修正规则，不通过降低标准绕过。

### 11.6 本轮文档固化验收命令

只修改文档时，至少执行以下轻量验收：

```bash
python -c "from pathlib import Path; files=[Path('docs/ERP_FRONTEND_UI_UNIFORMITY_PLAN.md'),Path('docs/ERP_FRONTEND_UI_PAGE_CHECKLIST.md'),Path('docs/ERP_FRONTEND_UI_TERMINOLOGY.md')]; [p.read_text(encoding='utf-8') for p in files]; print('utf8 ok')"
pnpm --dir apps/erp-web run assess:ui-uniformity
pnpm --dir apps/erp-web run assess:ui-uniformity -- --json
pnpm --dir F:\erp run check:utf8
```

涉及代码、模板、页面或门禁脚本时，仍需执行完整 Web 验收：

```bash
pnpm --dir apps/erp-web run lint:page-templates
pnpm --filter erp-web typecheck
pnpm --filter erp-web lint
pnpm --filter erp-web build
```

## 12. 阶段里程碑

### 阶段 0：基线盘点与评分模型

**目标：** 建立可量化的 UI 统一性基线，明确哪些页面达标、哪些页面需要改造。

**工作内容：**

- 梳理 `apps/erp-web/src/pages` 页面清单，按 P0/P1/P2 分级；
- 读取现有模板治理脚本结果；
- 建立 UI 统一性评分模型；
- 建立页面豁免登记表；
- 输出基线报告。

**交付物：**

- `docs/ERP_FRONTEND_UI_UNIFORMITY_BASELINE.md`；
- 可选 `apps/erp-web/scripts/assess-ui-uniformity.mjs`；
- 可选脚本测试或最小 Node 测试。

**验收标准：** 页面分级清晰、评分模型可复现、豁免页面有原因、基线报告能指导后续改造。

### 阶段 1：统一工作台契约加强

**目标：** 让统一模板不仅是页面壳，而是能约束工具栏、搜索区、主表、面板、状态和列设置的契约。

**工作内容：**

- 补强 `IndustrialWorkbenchTemplate` 与 `WorkbenchPageTemplate` 契约测试；
- 明确 toolbar、search、mainGrid、leftPanel、rightPanel、bottomPanel、footer slot；
- 抽象统一空态、加载态、错误态、权限态接口；
- 明确业务页面可替换点和禁止自定义点；
- 抽象真正通用的 `WorkbenchToolbar`，逐步摆脱订单审核旧命名实现。

**验收标准：** 模板契约测试通过、页面不复制工作台骨架、模板接入率保持当前目标、`typecheck` 通过。

### 阶段 2：P0 页面统一性改造

**目标：** 让最高频 ERP 页面先达到 95% 用户感知统一性。

**工作内容：**

- 逐页核对 P0 页面工具栏、搜索区、主表、列设置、分页、详情面板；
- 修正按钮顺序和文案；
- 接入统一空态、加载态、错误态；
- 统一焦点返回和键盘关闭行为；
- 补关键单测和必要 E2E。

**验收标准：** P0 页面统一性评分 `>= 95`、关键单测通过、无用户可见 Unicode 转义、`lint` 与 `typecheck` 通过。

### 阶段 3：P1 页面统一性改造与 P2 豁免收敛

**目标：** 将中频页面纳入统一规范，同时清理不必要的特殊豁免。

**工作内容：**

- 对 P1 页面应用统一模板和统一交互；
- 清理可迁移的 baseline exceptions；
- 将确需保留的 P2 页面写明豁免原因；
- 补充文案和状态统一检查；
- 更新统一性报告。

**验收标准：** P1 页面主要入口统一、baseline exceptions 不增加且最好减少、所有豁免有明确原因、UI 统一性总分 `>= 95`。

### 阶段 4：质量门禁固化与长期治理

**目标：** 将 UI 统一性从一次性改造变成长期工程机制。

**工作内容：**

- 将统一性评估脚本接入本地验收或 CI；
- 建立页面新增 checklist；
- 建立设计 token 使用约束；
- 建立文案词典；
- 建立最小 E2E 冒烟集合；
- 在开发文档中明确新增页面接入流程。

**验收标准：** 新增页面默认进入模板治理、文案转义会失败、非统一按钮文案可被发现、评分可持续生成、开发者能按文档快速新建统一工作台页面。

---

## 13. Codex 多子任务执行方案

> 执行要求：最多 5 个子任务；启动子任务时不要指定模型；每个子任务结束后必须关闭，并输出交付摘要、修改文件、验证命令和风险点。

### 子任务 1：UI 统一性基线与评分脚本

**写集建议：** `docs/ERP_FRONTEND_UI_UNIFORMITY_BASELINE.md`、`apps/erp-web/scripts/assess-ui-uniformity.mjs`、对应脚本测试。

**任务：** 扫描页面清单；建立 P0/P1/P2 分级配置；读取模板接入率；输出统一性评分、baseline exceptions、风险页面清单。

**验收：** 能稳定输出统一性评分；模板接入率读取准确；豁免页面可追踪；评分结果能指导后续改造。

### 子任务 2：工作台模板契约与统一状态能力

**写集建议：** `apps/erp-web/src/components/page-templates/workbench/**` 及对应测试。

**任务：** 强化 `IndustrialWorkbenchTemplate` / `WorkbenchPageTemplate` 契约；统一 toolbar/search/mainGrid/panels/footer slot；补空态、加载态、错误态、权限态接口；抽象通用 `WorkbenchToolbar`。

**验收：** 模板契约测试通过；页面模板治理脚本通过；`typecheck` 通过；不破坏现有页面。

### 子任务 3：P0 页面交互与表格统一改造

**写集建议：** P0 页面文件和 P0 页面相关测试，避免修改模板核心。

**任务：** 核对工具栏、搜索区、表格、列设置、分页、详情面板；统一按钮顺序、文案、列设置入口、表格状态和键盘关闭行为。

**验收：** P0 页面统一性评分 `>= 95`；关键单测通过；无用户可见 Unicode 转义；`lint`、`typecheck` 通过。

### 子任务 4：文档、规范与新页面 checklist

**写集建议：** `docs/ERP_FRONTEND_UI_UNIFORMITY_PLAN.md`、`docs/ERP_FRONTEND_UI_PAGE_CHECKLIST.md`、`docs/ERP_FRONTEND_UI_TERMINOLOGY.md`。

**任务：** 完善 UI 统一性规范、新页面接入 checklist、按钮/状态/表格术语词典、豁免登记规则、Code Review checklist。

**验收：** 文档能直接指导新增页面开发；checklist 可用于 code review；术语表可执行；文档路径和链接有效。

### 子任务 5：E2E 回归与 CI 门禁固化

**写集建议：** `apps/erp-web/e2e/**`、`apps/erp-web/package.json`、`apps/erp-web/scripts/**`。

**任务：** 补工作台 a11y、布局预设、菜单路由结构、视觉基线和最小冒烟集合；新增 `test:ui-contract` 与 `test:e2e:workbench-uniformity`；将统一性评分接入验收说明。

**验收：** 最小 E2E 集合通过；`lint`、`typecheck`、`build` 通过；失败能定位具体页面或规则。

---

## 14. 风险与回滚策略

### 14.1 模板改造影响多个页面

**风险表现：** 布局错位、面板尺寸异常、表格高度异常、滚动条消失或重复、工具栏换行异常。

**缓解策略：** 先补模板契约测试；每次只改一个模板能力点；P0 页面优先视觉和 E2E 回归；保持旧 props 兼容；避免一次性删除旧适配层。

**回滚策略：** 保留旧模板 props；单个页面临时切回旧配置；回滚公共 CSS 高风险变更；失败页面临时加入带原因和期限的豁免。

### 14.2 统一文案影响业务理解

**风险表现：** 业务用户熟悉按钮名称被替换；同一术语在不同语境含义不同；过度统一导致业务精度下降。

**缓解策略：** 建立术语表；按 P0/P1/P2 分批确认；保留必要业务术语；只统一通用动作，不强行统一业务专有名词。

**回滚策略：** 文案改造集中提交，便于按页面或词条回滚；保留术语映射记录；争议词条允许临时豁免。

### 14.3 E2E 不稳定导致门禁噪音

**风险表现：** 本地通过、CI 失败；等待条件不稳定；数据依赖 mock 状态。

**缓解策略：** E2E 优先覆盖结构和关键交互；使用稳定 selector；避免依赖动画时间；长链路 E2E 与冒烟 E2E 分层。

**回滚策略：** 新增 E2E 先作为非阻断报告；稳定后升级门禁；flaky 用例临时降级并登记修复原因。

### 14.4 评分脚本误判

**风险表现：** 合规页面被判失败；特殊页面无法表达豁免；扫描规则与真实 UI 脱节。

**缓解策略：** 评分脚本先输出报告再阻断；支持页面分级和豁免原因；关键规则配套单测；不用单一字符串扫描替代真实契约测试。

**回滚策略：** 保留上一个评分阈值；新规则先设为 warning；误判页面增加带原因例外；修复脚本后恢复阻断。

### 14.5 并行子任务冲突

**风险表现：** 多个子任务同时改模板核心、CSS、测试或 `package.json`。

**缓解策略：** 明确写集边界：子任务 1 负责基线/评分；子任务 2 负责模板核心；子任务 3 负责 P0 页面；子任务 4 负责文档；子任务 5 负责 E2E 和门禁。每个子任务结束必须关闭并汇报。

**回滚策略：** 按子任务独立回滚；模板核心优先回滚；文档和脚本可保留；页面适配按页面粒度回滚。

---

## 15. 新页面接入流程

新增 ERP 页面必须按以下流程：

1. 判断页面类型：工作台、设置页、详情页、编辑器、看板、特殊页；
2. 优先选择统一模板；
3. 只填写业务数据源、列定义、工具栏按钮、搜索项和面板内容；
4. 使用统一文案词典；
5. 补最小单测；
6. 运行页面模板治理脚本；
7. 如需豁免，登记原因、归属模块、负责人和有效期。

---

## 16. Code Review Checklist

每个 UI PR 必查：

- 是否使用统一模板；
- 是否复制了工作台骨架；
- 工具栏按钮顺序是否一致；
- 查询、重置、刷新语义是否一致；
- 表格列设置入口是否一致；
- 空态、加载态、错误态是否一致；
- 是否有中文 aria-label；
- 是否有用户可见 Unicode 转义；
- 是否硬编码页面级字号、颜色、间距、圆角、阴影；
- 是否影响 P0 页面 E2E；
- 是否更新文档、checklist 或术语表。

---

## 17. 最终完成定义

当以下条件全部满足时，本轮“ERP 前端 UI 统一性下一步开发”可以关闭：

- UI 统一性总分从当前 `66.4` 提升到 `>= 95`，差距 `26.5` 已清零；
- 模板接入率 `>= 95%`，目标保持 `100%`；
- P0 页面统一性评分 `>= 95`，且 P0 裸 Table 清零、P0 裸 Card 降到门禁阈值以下；
- baseline exceptions 不增加，最好减少；
- 用户可见 Unicode 转义约束通过；
- P0 高频操作 E2E 通过；
- `lint`、`typecheck`、`build` 通过；
- 文档、checklist、术语表完整；
- 新增页面有明确接入流程和门禁；
- 每个 Codex 子任务均已关闭并提交交付摘要。


## 12. 本轮执行结果与下一批开发入口（2026-05-17）

### 12.1 已完成

- 已完成 `10` 个 P0 页面工业表格契约/工具栏/header 迁移，重点消除页面级裸 `Card`、把筛选区迁入 `UiModule03MainTableSection.header`，并补齐契约测试。
- 当前评分脚本实测为 `68.5 / 100`，目标 `95`，差距 `26.5`，状态仍为 `未达标`。
- 当前 P0 指标：裸 `Table` 为 `3`，裸 `Card` 为 `11`，统一工具栏标识为 `79/82`，列设置标识为 `73/82`。

### 12.2 下一批 P0 开发任务

1. 继续清理 `OrderReviewRulesPage.tsx` 的裸 `Card`，将筛选区和主操作迁入主表 `toolbarLeft/toolbarRight/header`。
2. 批量迁移采购域：`ProcurementInspectionsPage.tsx`、`ProcurementPlansPage.tsx`、`ProcurementReconciliationsPage.tsx`、`ProcurementRequestsPage.tsx`、`ProcurementSourcingPage.tsx`。
3. 批量迁移采购执行页：`PurchaseOrdersPage.tsx`、`PurchaseSuggestionsPage.tsx`、`SupplierPerformancePage.tsx`。
4. 批量迁移商品治理页：`ProductChangeControlPage.tsx`、`ProductLifecyclePage.tsx`、`ProductUnitsPage.tsx`。
5. 每页必须同步补契约测试，断言 `toolbarLeft`、`toolbarRight`、`header/actionBar`、`dataTestId`、`columnSettingsKey`，并禁止页面级裸 `<Card>` / `<Table>`。

### 12.3 95 分目标判断

- 当前主要瓶颈不再是模板采用率，而是全量页面操作一致性、表格一致性、视觉文案一致性与键盘可访问性。
- 要达到 `95`，至少需要继续压降全量裸 `Card`/`Table`、把 P0/P1 列表页主表迁入统一工业契约，并新增 E2E/a11y 门禁覆盖核心路径。
- 后续每轮应以评分脚本 `nextBatch.P0` 和 `nextBatch.P1` 为输入，按 8-12 页一批推进，直到 P0 裸 `Table` 清零、P0 裸 `Card` 低于 `10`、P1 工具栏与工业表格契约覆盖显著提升。


## 10. 本轮继续开发结果（2026-05-17）

- 本轮继续迁移 `OrderReviewRulesPage`、采购域 8 页、`SyncJobRunsPage`、`InvoicesPage`、`ProductPackagingSpecsPage` 以及商品治理 3 页的主表工具栏/筛选区/裸 Card 契约，共新增覆盖 `16` 个页面。
- 最新评分为 `68.5 / 100`，目标 `95`，差距 `26.5`；由于评分模型还强依赖全量裸 Table、P1 覆盖、键盘/a11y 与视觉治理，单轮大量 P0 工具栏迁移无法带来 10 分级跃升。
- P0 当前：裸 `Table` 为 `3`，裸 `Card` 为 `11`，统一工具栏标识为 `79/82`，P0 工具栏覆盖率已达到 `96.34%`。
- 下一步提分重点应从 P0 工具栏转向：清零 P0 裸 `Table`、压降商品治理页剩余详情裸 `Card`、批量推进 P1 裸 `Table/Card`、补键盘/a11y 门禁。


## 11. P0 裸表格继续压降（2026-05-17）

- 已将 `PurchaseReceiptsPage`、`PurchaseReturnsPage`、`LogisticsUploadPage` 中的页面级裸 `<Table>` 明细区迁移为统一 `ErpDataGrid`。
- 最新评分为 `68.5 / 100`，目标 `95`，差距 `26.5`，仍为 `未达标`。
- 全量裸 `Table` 从 `50` 降至 `41`，P0 裸 `Table` 从 `6` 降至 `3`；P0 仍需继续清理剩余裸表和裸 Card。
- 下一轮建议优先处理 `SalesProductMatchPage`、`ReconcilePage`、商品治理详情裸 Card、以及 WMS 策略页中的裸 Card/低一致性区域。


## 12. Reconcile / WMS / P1 样本继续治理（2026-05-17）

- 已解包 `ReconcilePage` 的 4 个表单裸 Card、商品治理页剩余主流程/详情裸 Card、WMS 策略/列表页筛选裸 Card。
- 已将 6 个规划 P1 样本页从裸 `Table/Card` 迁移为 `ErpDataGrid + section`，并补齐契约测试。
- 最新评分为 `68.5 / 100`，目标 `95`，差距 `26.5`，仍为 `未达标`。
- 全量裸 `Table` 降至 `41`，全量裸 `Card` 降至 `72`；P0 裸 `Card` 降至 `11`，P1 裸 `Table` 降至 `28`、P1 裸 `Card` 降至 `50`。
- 下一轮应继续处理 `MarketingPriceListsPage`、`SystemSyncJobsPage`、`PrintJobsPage`、`SkuListPage`、`ReportsPage`，并开始补键盘/a11y 门禁，否则评分中的 `a11yKeyboardConsistency=4.9/10` 会持续卡住。
