# 订单工作台完整模板化设计

**日期**: 2026-04-11

## 目标

将订单审核页面当前已经稳定的 ERP 工作台结构抽取为一套完整工作台模板，模板必须覆盖以下区域：

- 顶部工具栏
- 左侧筛选区
- 主表区
- 右侧详情区
- 底部子表区

本次设计采用重抽法，但不直接做成一个不可控的超大业务组件，而是拆成一组可组合、可复用、可扩展的模板组件。目标是让后续其它 UI 页面可以在保持工业级交互一致性的前提下，快速复用这一整套工作台结构。

## 范围

### 包含

- 完整工作台页面模板
- 顶部工具栏模板
- 主表模板
- 右侧详情区模板
- 底部子表区模板
- 主表、右侧、底部之间的联动协议
- 订单审核页面迁移到完整模板
- 后续应用到其它页面的复用边界设计

### 不包含

- 本轮不统一所有业务字段定义
- 本轮不把所有页面一次性替换到新模板
- 本轮不将左侧筛选树的业务规则完全抽象成 schema 引擎
- 本轮不重写订单审核和其它页面的业务接口层

## 当前现状与问题

订单审核页已经形成了一套比较成熟的工业级工作台交互，但能力仍然散落在多个页面实现中：

- 顶部工具栏使用 `UiModule02ActionSearchToolbar`，但它仍以页面直接消费为主，未上升为完整工作台协议的一部分。
- 主表 `OrderReviewMainGrid.tsx` 内部承载了大量工业级交互：单行高亮、勾选多选、Shift/Ctrl 连选、Space 勾选、ArrowUp/ArrowDown 聚焦联动、双击、右键、列筛选、加载态、空态、错误态。
- 底部 `OrderReviewBottomPanel.tsx` 同时包含了可复用的容器能力和订单审核专属的货品编辑/日志展示逻辑。
- 右侧详情与底部子表、主表活动行之间的联动，目前仍由页面手工编排。
- 页面整体结构已经符合 ERP 工作台形态，但模板化边界尚未稳定，后续其它页面想复用时仍会发生复制粘贴和交互偏移。

如果继续以页面级复制的方式扩展，会出现以下问题：

- 页面之间视觉相似但交互不一致
- 主表键盘/多选/右键协议被重复实现
- 右侧详情与底部子表联动逻辑继续分散
- 工具栏“左按钮 / 中搜索 / 右通用动作”继续重复接线
- 后续回归成本随着页面数增长而增加

## 设计结论

本次采用“完整工作台模板 + 业务注入”的方案，推荐作为后续工作台类页面的统一骨架。

### 推荐方案

建立一组模板组件，由模板固定结构与交互协议，由页面注入业务内容与数据：

1. `WorkbenchPageTemplate`
2. `WorkbenchToolbarTemplate`
3. `WorkbenchMainGridTemplate<T>`
4. `WorkbenchRightPanelTemplate`
5. `WorkbenchBottomTabsTemplate`
6. `WorkbenchInteractionController<TMain, TDetail>`

这套方案的核心是：

- 布局由模板统一
- 工具栏骨架由模板统一
- 主表工业级交互由模板统一
- 右侧与底部区域由模板统一
- 具体列、数据、按钮、详情内容、子表内容由页面注入

### 不采用其它方案的原因

#### 不采用“只抽样式外壳”

只抽样式无法复用交互协议，页面仍然要各自重写主表选择、键盘、右键、联动逻辑，无法满足工业级 ERP 的一致性目标。

#### 不采用“单一 schema 驱动的超大模板引擎”

当前系统仍处于交互固化阶段，如果直接做成全站 schema 引擎，会过早把业务演进中的差异固化到配置层，导致模板复杂度和维护成本快速上升。

## 模板边界

### 1. WorkbenchPageTemplate

职责：

- 负责完整工作台五区布局：顶部工具栏、左侧、主区、右侧、底部
- 统一控制左右下三块区域的显隐和尺寸
- 统一承载 footer / summary / table stack 容器
- 只负责结构，不负责订单、商品、库存等业务字段

建议 props：

- `toolbar: ReactNode`
- `leftPanel: ReactNode`
- `mainContent: ReactNode`
- `rightPanel: ReactNode`
- `bottomPanel: ReactNode`
- `leftWidth?: number`
- `rightVisible?: boolean`
- `bottomVisible?: boolean`
- `footer?: ReactNode`
- `className?: string`

### 2. WorkbenchToolbarTemplate

职责：

- 固定“左按钮 / 中搜索 / 右通用动作”结构
- 将打印、导出、帮助纳入统一协议
- 页面只注入业务动作和搜索行为

建议 props：

- `leftActions: ReactNode`
- `searchValue: string`
- `onSearchChange: (value: string) => void`
- `onSearchSubmit: () => void`
- `searchPlaceholder?: string`
- `rightActions?: { print?: () => void; export?: () => void; help?: () => void }`
- `extraRightContent?: ReactNode`
- `searchInputTestId?: string`
- `searchButtonTestId?: string`

### 3. WorkbenchMainGridTemplate<T>

职责：

- 承载主表 AG Grid 容器和工业级交互协议
- 统一以下行为：
  - 单行高亮
  - 勾选多选
  - Shift/Ctrl 连选与反选
  - Space 勾选
  - ArrowUp/ArrowDown 切换活动行
  - 双击业务动作
  - 右键菜单
  - 受控列筛选
  - 加载态 / 空态 / 错误态

建议 props：

- `rows: T[]`
- `columnDefs: ColDef<T>[]`
- `getRowId: (row: T) => string`
- `selectedRowKeys: Key[]`
- `focusedRowKey: string | null`
- `columnFilters: Record<string, unknown>`
- `onColumnFiltersChange: (filters: Record<string, unknown>) => void`
- `onSelectionChange: (keys: Key[]) => void`
- `onRowActive: (record: T) => void`
- `onRowDoubleClick: (record: T) => void`
- `onFocus: (record: T) => void`
- `onContextMenu: (record: T, event: MouseEvent) => void`
- `loading: boolean`
- `error: unknown`
- `onRetry: () => void`
- `summary?: ReactNode`
- `pagination?: ReactNode`

### 4. WorkbenchRightPanelTemplate

职责：

- 提供统一右侧面板容器与状态呈现
- 只承载结构和 loading/error/empty，不绑定业务字段

建议 props：

- `visible?: boolean`
- `title?: ReactNode`
- `content: ReactNode`
- `loading?: boolean`
- `error?: unknown`
- `empty?: ReactNode`

### 5. WorkbenchBottomTabsTemplate

职责：

- 提供统一底部多 tab 工作区容器
- 每个 tab 内可挂 AG Grid、表单、日志、轻编辑区
- 工具栏、右键、编辑弹窗能力通过页面注入

建议 props：

- `tabs: Array<{ key: string; label: ReactNode; toolbar?: ReactNode; content: ReactNode; badge?: ReactNode }>`
- `activeTabKey?: string`
- `onActiveTabChange?: (key: string) => void`
- `height?: number | string`

### 6. WorkbenchInteractionController<TMain, TDetail>

职责：

- 编排主表活动行、右侧详情、底部子表之间的联动
- 页面提供数据获取和持久化回调，控制器负责组织协议

最小职责：

- 主表活动行切换后，同步触发右侧详情刷新
- 主表活动行切换后，同步触发底部子表刷新
- 主表勾选集与活动行分离
- 批量动作基于勾选集
- 详情与子表基于活动行

## 交互协议

模板必须固化以下规则，不允许每个页面自行漂移。

### 主表规则

- 鼠标单击只切换活动行，不自动勾选
- 勾选框只负责多选集合，不负责活动行高亮
- 任意时刻只允许一条活动高亮行
- 双击触发页面提供的业务动作
- ArrowUp/ArrowDown 等同于鼠标选中活动行，不触发勾选
- Space 只切换当前活动行勾选状态，不影响分页
- Shift/Ctrl 保持当前已实现的连续选择与反选协议

### 主表筛选规则

- 固定值字段采用下拉筛选
- 文本字段采用模糊搜索筛选
- 时间字段采用时间范围选择器
- 主表列筛选必须同步进入页面业务过滤状态，不能只停留在 AG Grid 本地 model

### 联动规则

- 右侧详情与底部子表均绑定“当前活动行”
- 勾选集合不驱动右侧详情和底部子表
- 右侧详情、底部子表、主表活动行必须保持同一条主线

### 工具栏规则

- 左侧放业务操作按钮
- 中间放统一搜索入口
- 右侧放打印、导出、帮助等通用动作
- 页面可以加业务按钮，但不破坏三区布局

## 数据流

### 输入数据

- 页面负责查询和预处理数据
- 模板只消费已经成型的 `rows`、`detail`、`tabs`、`toolbar actions`

### 输出事件

- 模板向页面抛出交互事件
- 页面负责决定是否请求接口、是否更新右侧详情、是否保存子表

### 单向流转原则

- 页面传入数据与回调
- 模板负责交互与结构
- 模板不直接发业务 API 请求
- 业务请求始终由页面或业务 hook 承担

## 错误处理

模板需要统一支持以下状态：

- 主表加载中
- 主表加载失败
- 主表空数据
- 右侧详情加载中/失败/空态
- 底部子表加载中/失败/空态

错误处理原则：

- 模板负责显示统一状态外观
- 页面负责传递错误对象和重试回调
- 模板不解析业务错误码，只展示页面提供的错误语义

## 目录建议

建议将模板放到以下目录：

- `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.tsx`
- `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.tsx`
- `apps/erp-web/src/components/workbench/templates/WorkbenchMainGridTemplate.tsx`
- `apps/erp-web/src/components/workbench/templates/WorkbenchRightPanelTemplate.tsx`
- `apps/erp-web/src/components/workbench/templates/WorkbenchBottomTabsTemplate.tsx`
- `apps/erp-web/src/components/workbench/templates/types.ts`

页面业务组件继续保留在各自页面目录下，例如：

- `apps/erp-web/src/pages/orders/review-native/*`

## 迁移方案

### 第一阶段：抽完整工作台骨架

本阶段只落地：

- `WorkbenchPageTemplate`
- `WorkbenchToolbarTemplate`

订单审核页面先改为使用这两个模板，但主表、右侧详情、底部子表仍沿用当前订单审核专属组件。

目标：

- 先把完整工作台结构模板化
- 先让订单审核成为完整模板的第一位消费者
- 不同时动主表、右侧、底部的内部实现

### 第二阶段：抽主表模板

将订单审核当前主表中与业务字段无关的工业级交互协议抽出到 `WorkbenchMainGridTemplate<T>`。

订单审核保留：

- 列定义工厂
- 筛选定义工厂
- 与订单字段相关的 value formatter / filter params

### 第三阶段：抽右侧与底部模板

将底部子表容器和右侧容器抽象成统一模板，保留订单审核专属内容实现。

订单审核保留：

- 货品合并数量规则
- 日志列定义
- 持久化文案与业务接口调用
- 右侧详情字段布局

## 在其它页面的复用策略

模板不要求其它页面与订单审核拥有相同字段，只要求页面满足相同工作台结构：

- 顶部工具栏
- 左侧筛选
- 主表
- 右侧详情
- 底部子表

因此后续页面只需提供：

- 该页面自己的列定义
- 自己的详情区
- 自己的底部子表内容
- 自己的工具栏按钮与搜索动作

即可复用这套模板。

## 测试与验收

### 回归要求

每次迁移步骤都必须至少执行：

- `pnpm typecheck`
- `pnpm playwright test e2e/order-review-selection.spec.ts`

### 需要保住的核心行为

- 主表单击只高亮不勾选
- 勾选多选与活动行分离
- Shift/Ctrl/Space 正常
- ArrowUp/ArrowDown 联动右侧和底部
- 双击业务动作正常
- 列筛选正常
- 分页与 summary 正常
- 底部子表跟随活动行切换

### 模板装配验收

模板落地后需新增一条 smoke test，验证完整工作台模板五区都能正常装配：

- 顶部工具栏
- 左侧筛选
- 主表
- 右侧详情
- 底部子表

## 风险与约束

### 主要风险

- 模板边界过大，导致业务差异被错误吸收到模板层
- 模板边界过小，导致复用收益不足
- 主表协议抽象不当，导致键盘和选择行为回退
- 底部子表若把订单审核业务逻辑抽过头，会让其它页面难以接入

### 控制策略

- 先抽骨架，再抽主表，再抽右侧/底部
- 订单审核先作为模板第一消费者
- 不在第一轮就做 schema 引擎
- 用现有 E2E 锁住主表核心行为

## 实施建议

建议本次真正进入编码时，只完成以下第一轮落地：

- 新增 `WorkbenchPageTemplate`
- 新增 `WorkbenchToolbarTemplate`
- 让订单审核页面先接入这两个模板
- 不修改订单审核主表、右侧详情、底部子表的内部业务逻辑

这样做的收益是：

- 已经完成“完整工作台模板化”的第一步
- 风险最低
- 后续你可以继续把它应用到想要的其它页面
- 主表和子表模板化可以在下一轮继续推进

## 本轮结论

本次设计确认采用：

- 完整工作台模板化
- 顶部工具栏纳入模板边界
- 模板以组件组合而非 schema 引擎为主
- 迁移顺序为：页面骨架 -> 工具栏 -> 主表 -> 右侧/底部
- 订单审核作为第一落地页

这条路径在当前阶段是风险最低、复用价值最高、最符合工业级 ERP 目标的方案。
