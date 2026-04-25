# Workbench Template Industrialization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将当前 ERP 工作台模板收敛为可批量复刻、可长期演进、可上线验收的工业级壳层，并以 `OrderReviewPage` 与 `ChannelDirectoryPage` 两个黄金样板页完成验证。

**Architecture:** 先统一菜单、路由、壳层白名单、路由别名、页面能力声明的真相源，再发布标准化的 workbench page contract，随后拆分 `WorkbenchLayout` 并修复测试资产漂移，最后用两个黄金样板页验证订单型与目录型工作台都能在同一模板标准下稳定落地。

**Tech Stack:** React 18, TypeScript, React Router, Ant Design, React Query, AG Grid, Vitest, Playwright

---

## 背景结论

- 当前工作台壳层已经具备工业级雏形，但“模板能力、当前正式接入范围、菜单路由、测试资产”之间存在明显漂移。
- 历史 `A` / `A+` 报告可以证明某一阶段的壳层能力，不再能直接证明当前收口后的正式工作台模板已经达到可批量复刻的工业级标准。
- 本计划不再继续堆叠零散功能，而是先恢复“真相一致、契约清晰、可验证、可复制”这四个工业化前提。

## 工业级退出清单

- [ ] 菜单、路由、壳层白名单、路由别名、页面清单来自同一真相源。
- [ ] `WorkbenchLayout` 完成模块化拆分，核心壳层不再承载页面特例。
- [ ] `components/page-templates/workbench` 发布统一页面契约，新增页面接入不需要直接改壳层内部。
- [ ] 黄金样板页 1 `OrderReviewPage` 达到 A 级模板引用标准。
- [ ] 黄金样板页 2 `ChannelDirectoryPage` 达到 A 级模板引用标准。
- [ ] 关键回归测试以行为断言为主，不再依赖源码字符串断言。
- [ ] 核心工作台路由具备导航、a11y、交互、发布门禁。
- [ ] 命令面板、快捷键、分栏、面板状态、URL 分享能力都具备契约和回归验证。
- [ ] 路由诊断、预热、加载态、错误态、空态实现统一并可观测。
- [ ] 新页面复制有清晰接入清单、子任务边界和验收口径。

## 总体策略

- 先打磨 1 到 2 个黄金样板页，再做批量复刻。这一策略保持不变。
- 第一黄金样板页固定为 `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`，因为它覆盖了最复杂的三面板、订单主表、详情面板、底部明细、快捷动作和预热场景。
- 第二黄金样板页本期推荐使用 `apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`，因为它能验证目录型 CRUD 工作台，不再只证明“订单页能跑”。如果产品主数据页在本期恢复到正式路由，可把它替换或追加为 2B 样板页。
- `2B` 扩展样板页必须在 `M5` 前锁定日期和范围。优先级最高的候选是恢复后的产品主数据页；如果该页仍未恢复到正式路由，则使用 `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx` 作为过渡扩展样板页，用来补齐“搜索 + 过滤 + 详情 + 编辑/回写”混合场景的验证广度。
- 任何批量复刻动作都必须排在本计划的 `P0` 和黄金样板页验收之后，否则只会把当前漂移和测试债复制出去。

## Task 1: P0 统一工作台真相源

**Files:**
- Create: `apps/erp-web/src/app/navigation/workbenchRegistry.ts`
- Test: `apps/erp-web/src/app/navigation/workbenchRegistry.test.ts`
- Modify: `apps/erp-web/src/app/router.tsx`
- Modify: `apps/erp-web/src/app/layout/menu.tsx`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`
- Modify: `apps/erp-web/src/app/navigation/routeAliases.ts`

**Goal:** 把菜单、路由、壳层可达性、路由别名、命令面板可见性收敛为一份可推导的 registry，消除当前多处手工维护导致的漂移。

**Current Gap:** 当前正式菜单、主工作台路由、`WorkbenchLayout` 白名单、历史 E2E 断言都各自维护页面集合，页面删减之后出现了菜单仍收口、路由仍存在、测试仍验证旧模块的分叉状态。

**Dependencies:** 无。这是全局阻塞项。

**Parallelization:** 不适合与任何会同时改 `router.tsx`、`menu.tsx`、`WorkbenchLayout.tsx` 的写入型子任务并行。其它 lane 可以先做只读分析，但不能抢先合并。

- [ ] 盘点当前正式可达页面、历史测试资产、legacy alias、壳层白名单的偏差，形成 registry 字段清单。
- [ ] 设计 registry descriptor，至少包含 `path`、`label`、`module`、`permission`、`feature`、`shellEligible`、`panelCapabilities`、`commandPaletteActions`、`legacyAliases`。
- [ ] 让 `router.tsx`、`menu.tsx`、`WorkbenchLayout.tsx` 的路径判定都只消费 registry，而不是分别维护一套 route list。
- [ ] 让 `routeAliases.ts` 从 registry 派生或至少与 registry 对齐，避免新增页面时忘记补 alias。
- [ ] 新增 parity test，验证菜单叶子页、router path、shell eligible path、legacy redirect 四者一致。
- [ ] 修正 `workbench-menu-navigation.spec.ts`，让它只覆盖 registry 中真实存在的正式页面。

**Acceptance:**

- [ ] 新增、下线、隐藏一个工作台页面时，不再需要同时改三份以上路径列表。
- [ ] `WorkbenchLayout` 不再通过手写白名单决定某个页面是否可进入工作台壳层。
- [ ] 任意一个 registry 路由都能通过 unit parity test 和导航 E2E 被追踪到。
- [ ] 权限字段与菜单可见性字段一致，避免“菜单隐藏但路由可直达”或“菜单可见但路由未挂载”。
- [ ] 页面删除时，菜单入口、router path、shell eligibility、legacy alias、测试期望必须在同一个变更里一起消失，否则 parity test 直接失败。

## Task 2: P0 发布标准化工作台页面契约

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/contracts.ts`
- Create: `apps/erp-web/src/components/page-templates/workbench/createWorkbenchPageConfig.ts`
- Test: `apps/erp-web/src/components/page-templates/workbench/contracts.test.ts`
- Modify: `apps/erp-web/src/components/page-templates/workbench/index.ts`
- Modify: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.ts`
- Modify: `apps/erp-web/src/app/layout/workbenchPanels.tsx`

**Goal:** 明确工作台页面接入模板所需的契约，让左/右/底部面板、工具栏、搜索、状态缓存、命令面板动作都由页面配置声明，而不是散落在页面代码和壳层判断里。

**Current Gap:** 现有接入路径已经存在，但页面能力声明仍然分散，壳层必须知道太多页面细节，导致页面一收口或一复刻就容易出现可达但不可用、可渲染但功能不完整的问题。

**Dependencies:** Task 1。

**Parallelization:** 可以在 Task 1 registry 字段冻结后由单独 lane 实现，但不要与 Task 3 同时改壳层接口。

- [ ] 定义 page config contract，至少覆盖工具栏、搜索模型、选择模型、`stateCacheKey`、`savedViewKey`、left/right/bottom panel factory、overlay factory、diagnostics metadata。
- [ ] 对 `panelCapabilities`、默认布局预设、页面快捷动作、命令面板可暴露动作建立显式声明，不再依赖壳层推断。
- [ ] 让 `useWorkbenchLayoutController.ts` 以 contract 为入口，而不是让页面直接拼接壳层所需的内部结构。
- [ ] 让 `workbenchPanels.tsx` 退化为标准装配层，而不是继续承载页面特例逻辑。
- [ ] 新增 invalid config guard，缺少关键配置时在开发期直接报错，而不是运行时静默降级到 `NotFound` 或空白面板。

**Acceptance:**

- [ ] 新页面通过 `createWorkbenchPageConfig` 即可接入，不需要直接修改 `WorkbenchLayout` 内部实现。
- [ ] 一个页面是否拥有左侧树、右侧详情、底部明细、命令动作，都能从 contract 直接看出。
- [ ] 任意 contract 缺字段时都有明确的测试失败或开发时错误提示。

## Task 3: P0 拆分 `WorkbenchLayout` 并完成壳层瘦身

**Files:**
- Create: `apps/erp-web/src/app/layout/useWorkbenchShellState.ts`
- Create: `apps/erp-web/src/app/layout/WorkbenchShellChrome.tsx`
- Create: `apps/erp-web/src/app/layout/WorkbenchShellPanels.tsx`
- Create: `apps/erp-web/src/app/layout/WorkbenchShellResize.tsx`
- Create: `apps/erp-web/src/app/layout/WorkbenchCommandPalette.tsx`
- Create: `apps/erp-web/src/app/layout/WorkbenchLiveRegion.tsx`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`
- Modify: `apps/erp-web/src/app/layout/workbenchLayout.css`
- Test: `apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts`
- Test: `apps/erp-web/src/app/layout/WorkbenchLayout.command-palette.test.ts`
- Test: `apps/erp-web/src/app/layout/RouteTabs.keyboard-flow.test.ts`

**Goal:** 把当前 2418 行的 `WorkbenchLayout.tsx` 按职责拆成稳定模块，降低壳层复杂度，给后续子任务提供安全的并行边界。

**Current Gap:** 当前壳层文件过大，既负责路由识别、布局状态、面板装配、快捷键、命令面板、可访问性播报、预热、诊断，又夹带页面接入逻辑，是并行开发最大的冲突热点。

**Dependencies:** Task 1 和 Task 2。

**Parallelization:** 这是第二个不适合多人同时写入的核心收敛项。建议只允许一个 Codex worker 负责壳层拆分，其它 lane 通过新增文件和页面侧接入并行推进。

- [ ] 先冻结 `WorkbenchLayout` 的公共输入输出边界，再开始拆分实现。
- [ ] 把布局状态和 URL 回放逻辑抽到 `useWorkbenchShellState.ts`。
- [ ] 把顶部壳层结构、主内容区、面板容器、拖拽分栏、命令面板、live region 分别拆到独立模块。
- [ ] 把页面特例判断清空，改为读取 Task 2 定义的 page contract。
- [ ] 保持 `WorkbenchLayout` 只承担壳层装配责任，不再直接理解业务页细节。
- [ ] 对拆出的模块分别补最小行为测试，避免未来再次长回一个超大文件。

**Acceptance:**

- [ ] `WorkbenchLayout.tsx` 退化为壳层装配文件，代码规模显著下降。
- [ ] 命令面板、分栏、面板装配、screen reader 播报、URL 状态回放都有独立模块和测试。
- [ ] 黄金样板页接入过程中不再需要为页面差异反复回改壳层主文件。

## Task 4: P0 修复测试真相漂移，替换脆弱断言

**Files:**
- Create: `apps/erp-web/src/app/navigation/workbenchRouteParity.test.ts`
- Create: `apps/erp-web/e2e/workbench-route-reachability.spec.ts`
- Modify: `apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.command-palette.test.ts`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts`
- Modify: `apps/erp-web/src/app/layout/ModuleHeader.industrial-upgrade.test.ts`

**Goal:** 让工作台模板的回归验证以行为测试为主，修掉当前已知的重复标题、旧模块断言、源码字符串断言等低可靠资产。

**Current Gap:** 当前至少存在三个问题：`workbench-menu-navigation.spec.ts` 有重复 test title；它仍验证已不在正式菜单中的旧模块；部分 unit test 通过读取源码字符串断言模板能力，无法真实证明运行行为。

**Dependencies:** Task 1 必须完成。Task 2 的 contract 必须冻结。Task 3 的壳层公共 API 必须冻结后，Task 4 才能合并最终断言版本。

**Parallelization:** 适合独立 lane。它的写入范围与黄金样板页、性能、a11y lane 冲突较少，但共享测试文件必须明确归 Lane D 独占。

- [ ] 修复 `workbench-menu-navigation.spec.ts` 的重复 test title。
- [ ] 以 registry 为输入重写菜单导航 E2E，让测试随着正式页面清单自动更新。
- [ ] 将 `WorkbenchLayout.command-palette.test.ts` 改成基于真实渲染和可见文本的行为断言，移除对旧 `title="Command Palette"` 之类源码字面量的依赖。
- [ ] 逐步替换 `industrial-upgrade` 类测试中的源码字符串匹配，保留必要的结构性快照，重点转向行为和 DOM 语义断言。
- [ ] 增加 route reachability smoke test，验证所有正式工作台页面都能被打开、命令面板可用、壳层不会误判为 `NotFound`。
- [ ] 增加“页面删除负向漂移”测试：页面被下线后，菜单不可见、路由按预期 404/redirect、shell 不再挂载、alias 不再遗留、旧测试名不能继续通过。

**Acceptance:**

- [ ] 当前已知失败项全部清零。
- [ ] 核心壳层测试可以直接证明“页面可达、壳层可交互、命令面板可用、路由不漂移”。
- [ ] 关键测试文件不再依赖手工维护的旧模块名称。
- [ ] `workbench-menu-navigation.spec.ts`、`WorkbenchLayout*.test.ts` 由 Lane D 独占维护，Lane A 只提供 registry 和 contract 变更，不直接接管共享测试文件。

## Task 5: P0 黄金样板页 1 `OrderReviewPage`

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewDetailPanel.tsx`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewBottomPanel.tsx`
- Modify: `apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx`
- Test: `apps/erp-web/src/pages/orders/OrderReviewPage.*.test.tsx`
- Test: `apps/erp-web/e2e/order-review-*.spec.ts`

**Goal:** 把订单审核页打磨成“模板标准页”，作为后续订单查询、复核、异常、下载等订单型工作台的复刻母版。

**Current Gap:** 订单审核页能力最完整，但它同时也是壳层耦合最深的页面。若不先把它收敛成标准页，后面复刻只会复制特殊处理而不是复制模板能力。

**Dependencies:** Task 2、Task 3、Task 4。

**Parallelization:** 适合独立 lane，但只允许这个 lane 写订单审核主文件，避免与性能/a11y lane 产生大面积冲突。

- [ ] 将订单审核页完整迁移到新的 page contract，收敛工具栏、搜索、选中态、右侧详情、底部明细、overlay、命令动作的声明方式。
- [ ] 统一主表、详情、底部面板之间的 sync key 和状态回放口径。
- [ ] 明确审核页的加载态、错误态、空态、无权限态，避免壳层成功渲染但页面实际不可操作。
- [ ] 让订单审核页成为命令面板、快捷键、URL 布局回放、右侧详情和底部明细联动的标准示例页。
- [ ] 补充页面级回归测试，覆盖至少主表聚焦、双击、详情切换、底部标签切换、保存视图、布局回放、命令面板动作。

**Acceptance:**

- [ ] 订单审核页不再依赖壳层内部的页面特例分支。
- [ ] 它可以作为后续订单型页面复刻的直接参考。
- [ ] 页面级单测和 E2E 可以证明它已经达到“模板标准页”而不是“能跑的个例页”。

## Task 6: P0 黄金样板页 2 `ChannelDirectoryPage`

**Files:**
- Modify: `apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`
- Modify: `apps/erp-web/src/pages/channels/ChannelDirectoryPanels.tsx`
- Modify: `apps/erp-web/src/pages/channels/channelDirectoryColumns.tsx`
- Modify: `apps/erp-web/src/pages/channels/channelDirectoryModel.ts`
- Test: `apps/erp-web/src/pages/channels/ChannelDirectoryPage.*.test.tsx`
- Test: `apps/erp-web/e2e/channel-directory-*.spec.ts`

**Goal:** 用渠道目录页验证“目录型 CRUD 工作台”也能在同一模板下稳定成立，避免工业化标准只对订单型页面有效。

**Current Gap:** 当前很多模板能力是在订单场景中验证出来的。如果第二个样板页仍然选订单页，只能证明模板对订单链路有效，不能证明它是通用工作台模板。

**Dependencies:** Task 2、Task 3、Task 4。

**Parallelization:** 可以与 Task 5 并行，但写入范围必须限制在渠道目录域，不要同时改壳层主文件。

- [ ] 将渠道目录页迁移到统一 page contract，收敛工具栏、左右面板、表格动作、抽屉编辑器、右键菜单的接入方式。
- [ ] 验证 `WorkbenchPageTemplate` 对目录型数据页同样成立，包括分页、筛选、右侧详情、抽屉编辑、批量动作。
- [ ] 明确目录页的 empty/error/no-permission/loading 状态模板。
- [ ] 让渠道目录页成为非订单域的标准复刻参考。
- [ ] 如果产品主数据页在本期恢复到正式路由，评估是否追加为 2B 黄金样板页；否则保持渠道目录作为第二样板页不变。

**Acceptance:**

- [ ] 渠道目录页证明模板不仅适配订单场景，也适配主数据目录场景。
- [ ] 页面不再需要额外的壳层特例支持。
- [ ] 后续目录型页面可以沿用同一套 contract 和测试脚手架。

## Task 7: P1 无障碍与操作效率硬化

**Files:**
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`
- Modify: `apps/erp-web/src/app/layout/ModuleHeader.tsx`
- Modify: `apps/erp-web/src/app/layout/RouteTabs.tsx`
- Modify: `apps/erp-web/src/app/layout/workbenchLayout.css`
- Modify: `apps/erp-web/e2e/workbench-a11y.spec.ts`
- Test: `apps/erp-web/src/app/layout/WorkbenchLayout.splitter-a11y.test.tsx`
- Test: `apps/erp-web/src/app/layout/RouteTabs.keyboard-flow.test.ts`

**Goal:** 把当前已有的 a11y 和快捷键能力从“局部亮点”升级为模板门禁，确保所有正式工作台页面都具备一致的键盘可达性和屏幕阅读器语义。

**Current Gap:** 模板已经有 live region、快捷键帮助、命令面板、部分 a11y 测试，但这些能力还没有完全沉淀为“所有工作台页默认继承”的工业级标准。

**Dependencies:** Task 3。Task 5 和 Task 6 做页面接入时可同步兑现。

**Parallelization:** 适合独立 lane，但需要避免与 Task 3 在同一时间段同时大量改 `WorkbenchLayout.tsx`。

- [ ] 确认分栏拖拽条具备完整键盘语义、焦点管理、数值边界和可见 hit target。
- [ ] 统一命令面板、快捷键帮助、弹窗/抽屉关闭后的 focus return 策略。
- [ ] 审核 `ModuleHeader`、`RouteTabs`、工作台搜索、分页、批量动作按钮的 aria label 和键盘触达路径。
- [ ] 把 dashboard、订单审核、订单查询、渠道目录纳入统一 a11y smoke gate。
- [ ] 补齐模板层的 empty/error/loading/read-only 文案语义和 landmark 标记。

**Acceptance:**

- [ ] `test:e2e:a11y` 覆盖的核心页面全部通过。
- [ ] 关键工作台操作不依赖鼠标。
- [ ] a11y 语义由模板默认提供，页面只补业务描述，不重复造轮子。

## Task 8: P1 预热、性能、可观测性标准化

**Files:**
- Create: `apps/erp-web/src/app/navigation/workbenchTelemetry.ts`
- Modify: `apps/erp-web/src/app/navigation/routePrefetch.ts`
- Modify: `apps/erp-web/src/app/navigation/RouteDiagnosticsReporter.tsx`
- Modify: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.ts`
- Test: `apps/erp-web/e2e/order-review-performance-budget.spec.ts`
- Test: `apps/erp-web/e2e/order-review-interaction-latency-evaluation.spec.ts`

**Goal:** 把现有的路由诊断和预热能力沉淀为模板公共能力，并把性能回归纳入正式门禁，而不是只在单个页面里临时测量。

**Current Gap:** 当前已经存在 `routePrefetch`、`RouteDiagnosticsReporter`、性能预算测试脚本，但模板层还没有清晰的“何时预热、如何诊断、哪些事件必须上报、哪些预算必须守住”的标准。

**Dependencies:** Task 2、Task 3。Task 5 和 Task 6 负责提供真实页面样本。

**Parallelization:** 适合独立 lane，与页面 lane 并行推进。

- [ ] 统一定义工作台级 telemetry 事件，如 route start/ready/timeout、prefetch hit/miss、panel mount、contract invalid、shell reject。
- [ ] 为预热建立白名单和触发条件，不让模板无差别预热所有页面。
- [ ] 明确右侧面板、底部明细、命令面板等重组件的挂载策略和时机。
- [ ] 将 `test:e2e:perf`、`test:e2e:latency` 纳入工业化验收，不再只作为附加脚本存在。
- [ ] 为黄金样板页建立最低性能基线和可追踪的诊断输出。

**Acceptance:**

- [ ] 所有正式工作台页面都有统一的路由与壳层诊断事件。
- [ ] 性能预算测试在黄金样板页上稳定可跑。
- [ ] 出现加载超时、壳层拒绝、页面 contract 缺失时，可在诊断层直接定位原因。

## Task 9: P1 文案、编码、空错态治理

**Files:**
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`
- Modify: `apps/erp-web/src/app/layout/ModuleHeader.tsx`
- Modify: `apps/erp-web/src/app/layout/menu.tsx`
- Modify: `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx`
- Modify: `apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`
- Test: `apps/erp-web/src/app/layout/WorkbenchLayout.command-palette.test.ts`
- Test: `apps/erp-web/src/app/layout/menu.test.ts`

**Goal:** 统一模板层的文案来源、编码策略、加载/错误/空态表述，杜绝用户可见的乱码、混语和不一致提示。

**Current Gap:** 当前代码和历史报告都暴露出文案分散、旧字符串断言、局部中英文混用的风险。终端抓取里还能看到乱码迹象，是否为真实 UI 问题需要先确认，但模板层的文案集中化仍是必做项。

**Dependencies:** Task 3、Task 4。Task 5 和 Task 6 在页面侧兑现。

**Parallelization:** 适合独立 lane，但第一步必须先通过真实浏览器确认“乱码是否真实出现在 UI”，避免做成无意义的全仓字符扫除。

- [ ] 先在浏览器核对 dashboard、订单审核、订单查询、渠道目录的真实文案表现，确认乱码是运行时问题还是终端编码采样问题。
- [ ] 将模板层通用文案集中到统一 i18n / copy source，不再把关键提示散落在多个组件和页面里。
- [ ] 统一 empty/error/loading/no-permission/default-help 文案口径。
- [ ] 修掉模板层用户可见的旧英文残留和过时占位文本。
- [ ] 调整测试，断言真实可见文案和语义，而不是源码字面量。

**Acceptance:**

- [ ] 核心工作台页面不出现用户可见的乱码和混乱提示。
- [ ] 模板文案有统一来源，新增页面不会重复抄写一套提示词。
- [ ] 空态、错态、加载态在订单型和目录型样板页上表现一致。

## Task 10: P2 发布门禁、复制波次与文档固化

**Files:**
- Modify: `apps/erp-web/package.json`
- Modify: `.github/workflows/ci.yml`
- Create: `docs/reports/workbench-template-industrialization-verification-2026-04-24.md`
- Modify: `docs/reports/README.md`

**Goal:** 将工业化验收标准固化到脚本、CI、验证报告和后续复制节奏中，避免“这次手工看起来没问题”之后再次失控。

**Current Gap:** 当前已有 `test:e2e:a11y`、`test:e2e:perf`、`test:e2e:release-gate` 和前端 CI gate，但模板工业化的验收项还没有以“双黄金样板页 + registry parity + shell contract + release gate”统一串起来。

**Dependencies:** Task 1 到 Task 9。

**Parallelization:** 适合收尾 lane，在主要改造完成后集中落地。

- [ ] 将 registry parity、黄金样板页 focused unit、a11y、menu navigation、release gate 组合成工业化验收命令集。
- [ ] 新增黄金样板页专用门禁命令，例如 `test:unit:golden-pages` 与 `test:e2e:golden-pages`，或者等价的聚合命令，并要求它们进入 CI 必跑集。
- [ ] 更新 CI，让工业化模板相关门禁成为正式流水线的一部分。
- [ ] 产出验证报告，记录通过项、残余风险、暂缓项、复制条件。
- [ ] 明确复制波次：第一波订单链路相关页，第二波目录型页，第三波配置型页。
- [ ] 为 `2B` 扩展样板页设置单独的日期化 release gate，不允许无限期后移。
- [ ] 为后续 Codex 并行开发补充“写入范围、禁止并行写入文件、验收脚本、回滚边界”的执行说明。

**Acceptance:**

- [ ] 任何后续模板变更都必须通过工业化验收命令集。
- [ ] CI 能在合并前拦住 route 漂移、壳层退化、黄金样板页回归、a11y 回退。
- [ ] `OrderReviewPage` 和 `ChannelDirectoryPage` 的 focused unit + E2E 都必须为绿色，任一失败都不得视为模板工业化通过。
- [ ] 复制到其它页面前有清晰的放行条件，而不是继续靠人工拍板。

## 非功能门禁

- [ ] `pnpm --dir apps/erp-web lint`
- [ ] `pnpm --dir apps/erp-web typecheck`
- [ ] `pnpm --dir apps/erp-web test:unit -- src/app/layout/menu.test.ts src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts src/app/layout/WorkbenchLayout.command-palette.test.ts src/app/layout/RouteTabs.keyboard-flow.test.ts`
- [ ] `pnpm --dir apps/erp-web test:unit -- src/pages/orders/OrderReviewPage*.test.tsx src/pages/channels/ChannelDirectoryPage*.test.tsx`
- [ ] `pnpm --dir apps/erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts --project=chromium`
- [ ] `pnpm --dir apps/erp-web exec playwright test e2e/workbench-a11y.spec.ts --project=chromium`
- [ ] `pnpm --dir apps/erp-web exec playwright test e2e/order-review-*.spec.ts e2e/channel-directory-*.spec.ts --project=chromium`
- [ ] `pnpm --dir apps/erp-web test:e2e:perf`
- [ ] `pnpm --dir apps/erp-web test:e2e:latency`
- [ ] `pnpm --dir apps/erp-web test:e2e:release-gate`
- [ ] 根仓 `pnpm run accept:web`

## 里程碑顺序

1. `M1 / 真相收敛`: Task 1 + Task 2
2. `M2 / 壳层瘦身`: Task 3 + Task 4
3. `M3 / 双黄金样板页`: Task 5 + Task 6
4. `M4 / 模板硬化`: Task 7 + Task 8 + Task 9
5. `M5 / 放量复制`: Task 10

## Codex 子任务并行拆分建议

### Lane A: 真相源统一

- Scope: Task 1
- Ownership: `router.tsx`、`menu.tsx`、`routeAliases.ts`、`workbenchRegistry.ts`
- Can Start: 立即
- Blocking: 阻塞所有写入壳层和测试基线的 lane
- Handoff: 共享测试文件不归 Lane A 持有，Lane A 完成 registry 后将测试改造需求移交 Lane D

### Lane B: 页面契约标准化

- Scope: Task 2
- Ownership: `components/page-templates/workbench/**`、`workbenchPanels.tsx`
- Can Start: Lane A 的 registry 字段冻结后
- Blocking: 阻塞黄金样板页接入

### Lane C: 壳层模块化

- Scope: Task 3
- Ownership: `WorkbenchLayout.tsx` 及其拆分出的 `layout` 子模块
- Can Start: Lane B 的 contract 冻结后
- Blocking: 阻塞大部分模板能力稳定化

### Lane D: 测试资产修复

- Scope: Task 4
- Ownership: `src/app/layout/*.test.*`、`e2e/workbench-*.spec.ts`
- Can Start: 可先读代码，正式写入建议在 Lane A 完成后
- Blocking: 阻塞工业化验收
- Exclusive Files: `apps/erp-web/e2e/workbench-menu-navigation.spec.ts`、`apps/erp-web/src/app/layout/WorkbenchLayout*.test.ts*`

### Lane E: 黄金样板页 1 订单审核

- Scope: Task 5
- Ownership: `src/pages/orders/**`
- Can Start: Lane B 和 Lane C 的公共接口稳定后
- Blocking: 订单型页面复制

### Lane F: 黄金样板页 2 渠道目录

- Scope: Task 6
- Ownership: `src/pages/channels/**`
- Can Start: Lane B 和 Lane C 的公共接口稳定后
- Blocking: 目录型页面复制

### Lane G: a11y 与交互硬化

- Scope: Task 7
- Ownership: `src/app/layout/**` 中的交互语义层和对应测试
- Can Start: Lane C 第一轮拆分完成后
- Blocking: 发布门禁

### Lane H: 预热、性能、可观测性

- Scope: Task 8
- Ownership: `src/app/navigation/**`、性能测试脚本
- Can Start: Lane C 第一轮拆分完成后
- Blocking: 发布门禁

### Lane I: 文案与空错态治理

- Scope: Task 9
- Ownership: 模板层通用文案与黄金样板页文案
- Can Start: 真实 UI 文案核对完成后
- Blocking: 用户可用性和演示质量

### Lane J: CI、报告与复制节奏

- Scope: Task 10
- Ownership: `package.json`、`.github/workflows/ci.yml`、`docs/reports/**`
- Can Start: M3 后即可并行准备
- Blocking: 正式放量复制

## 并行开发红线

- [ ] 任一时刻只允许一个写入型子任务修改 `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`。
- [ ] 任一时刻只允许一个写入型子任务修改 `apps/erp-web/src/app/router.tsx` 或 `apps/erp-web/src/app/layout/menu.tsx`。
- [ ] 黄金样板页 lane 只能修改各自业务域文件，不得顺手回改壳层接口。
- [ ] 测试 lane 优先修真相漂移和行为断言，不得用扩大 timeout 或注释断言的方式“修绿”。
- [ ] 性能 lane 不得通过关闭诊断、禁用预热、移除功能来换取表面通过。

## 放量复制前提

- [ ] Task 1 到 Task 6 全部完成并通过验证。
- [ ] 两个黄金样板页都被团队确认可作为后续复刻母版。
- [ ] 工业化验收命令集已经接入 CI。
- [ ] 文档中已经记录哪些文件可以并行改、哪些文件必须串行改。
- [ ] 剩余页面的复制顺序和验收脚本已明确。
