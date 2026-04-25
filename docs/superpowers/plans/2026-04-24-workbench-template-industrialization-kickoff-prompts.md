# Workbench Template Industrialization Codex Kickoff Prompts

用途：将 [2026-04-24-workbench-template-industrialization.md](/F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md) 直接拆成可投喂给 Codex 子任务的启动 prompt。

主计划：`F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md`

适用场景：
- 需要按 lane 并行推进工作台模板工业化改造
- 需要严格控制共享文件的写入边界
- 需要把“依赖冻结后再启动”的顺序写清楚

---

## 0. 派发顺序

不要一次性把 10 个 worker 全部放出去写代码。按下面波次启动：

1. `Wave 1`
   - Lane A `真相源统一`
2. `Wave 2`
   - Lane B `页面契约标准化`
   - Lane D `测试资产修复`
3. `Wave 3`
   - Lane C `壳层模块化`
4. `Wave 4`
   - Lane E `黄金样板页 1 / OrderReviewPage`
   - Lane F `黄金样板页 2 / ChannelDirectoryPage`
   - Lane G `a11y 与交互硬化`
   - Lane H `预热、性能、可观测性`
5. `Wave 5`
   - Lane I `文案与空错态治理`
6. `Wave 6`
   - Lane J `CI、报告与复制门禁`

补充说明：
- Lane A 完成前，不要启动任何会修改 `router.tsx`、`menu.tsx`、`routeAliases.ts` 的写入型 worker。
- Lane C 是唯一可以大改 `WorkbenchLayout.tsx` 的 worker。
- Lane D 独占 `WorkbenchLayout*.test.ts*` 与 `workbench-menu-navigation.spec.ts`。
- Lane E 只改订单域；Lane F 只改渠道域；不要顺手回改壳层接口。

---

## 1. 通用广播模板

先把下面这段发给所有 worker，再发各自专属 prompt。

```text
你在 F:/erp 的独立 worktree 中工作，目标是执行这份主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

硬性约束：
1) 你不是独自在代码库中工作。不要回滚、覆盖、重命名或清理其他 worker 的改动。
2) 只允许修改你被分配的白名单文件；如果你发现必须越界，先停止并返回 NEEDS_CONTEXT。
3) 先补失败测试，再做最小实现，再回归验证。不要跳过验证。
4) 不要通过加大 timeout、删除断言、屏蔽测试、关闭功能来“修绿”。
5) 如果依赖前置 lane 尚未冻结，直接返回 NEEDS_CONTEXT，不要自行假设。
6) 完成后必须输出四段：
   A. Changed Files
   B. Commands Run
   C. Test Results
   D. Risks / Follow-ups

状态约定：
- DONE：完成并验证
- DONE_WITH_CONCERNS：完成但有明确风险
- NEEDS_CONTEXT：依赖未满足或上下文不足
- BLOCKED：遇到无法自行解决的阻塞

首条回复必须先回显：
1) 你的目标
2) 你的白名单文件
3) 你准备先执行的第一条命令
```

---

## 2. Lane A 启动 Prompt

建议分支：`codex/workbench-ia`

启动条件：立即可启动。这是全局前置 lane。

```text
你是 Lane A，负责“工作台真相源统一”。直接开始实现，不要先写泛化方案。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
把菜单、router、shell eligibility、legacy aliases 收敛为统一 registry，消除当前前端页面删减后的漂移。

你的白名单文件：
- F:/erp/apps/erp-web/src/app/navigation/workbenchRegistry.ts (create)
- F:/erp/apps/erp-web/src/app/navigation/workbenchRegistry.test.ts (create)
- F:/erp/apps/erp-web/src/app/router.tsx
- F:/erp/apps/erp-web/src/app/layout/menu.tsx
- F:/erp/apps/erp-web/src/app/navigation/routeAliases.ts

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout*.test.ts*
- 任何 orders/channels 页面文件

必须完成的工作：
1) 盘点当前正式路由、菜单叶子页、legacy alias 的真实差异。
2) 新建 workbench registry，并让 router/menu/aliases 从 registry 派生或至少与其对齐。
3) 让 registry 能表达 permission、module、shellEligible、legacyAliases。
4) 写 registry 单测，证明路由与菜单源头统一。
5) 不要碰共享测试文件，把测试改造需求留给 Lane D。

建议执行步骤：
1) 先写 `workbenchRegistry.test.ts` 的失败断言。
2) 再创建 `workbenchRegistry.ts` 并回接 `router.tsx`、`menu.tsx`、`routeAliases.ts`。
3) 跑最小单测确认主真相源已经成立。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/app/layout/menu.test.ts src/app/navigation/routeAliases.test.ts src/app/navigation/workbenchRegistry.test.ts

建议提交信息：
feat(workbench): unify route menu and alias registry
```

---

## 3. Lane B 启动 Prompt

建议分支：`codex/workbench-contract`

启动条件：Lane A 完成，registry 字段已经冻结。

```text
你是 Lane B，负责“工作台页面契约标准化”。直接开始实现，不要等待壳层拆分完成。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
定义统一的 workbench page contract，让页面通过声明接入壳层，不再把页面细节散落在壳层内部。

你的白名单文件：
- F:/erp/apps/erp-web/src/components/page-templates/workbench/contracts.ts
- F:/erp/apps/erp-web/src/components/page-templates/workbench/createWorkbenchPageConfig.ts (create)
- F:/erp/apps/erp-web/src/components/page-templates/workbench/contracts.test.ts
- F:/erp/apps/erp-web/src/components/page-templates/workbench/index.ts
- F:/erp/apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.ts
- F:/erp/apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.test.ts
- F:/erp/apps/erp-web/src/app/layout/workbenchPanels.tsx

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout*.test.ts*
- F:/erp/apps/erp-web/src/app/router.tsx
- F:/erp/apps/erp-web/src/app/layout/menu.tsx

必须完成的工作：
1) 定义 page config contract，至少覆盖 toolbar、search、state cache、saved view、panels、overlays、diagnostics metadata。
2) 显式声明 panel capabilities 与 command palette actions，不允许壳层继续猜。
3) 让 `useWorkbenchLayoutController.ts` 和 `workbenchPanels.tsx` 以 contract 为输入。
4) 为 contract 缺失关键字段时增加开发期 guard。
5) 产出可供 Lane C、Lane E、Lane F 消费的稳定接口。

建议执行步骤：
1) 先补 `contracts.test.ts` / `useWorkbenchLayoutController.test.ts` 的失败断言。
2) 再实现 contract 与 config factory。
3) 最后仅做必要的接入层回接，不要顺手改页面。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/components/page-templates/workbench/contracts.test.ts src/components/page-templates/workbench/useWorkbenchLayoutController.test.ts src/components/page-templates/workbench/workbenchContracts.test.ts

建议提交信息：
feat(workbench): standardize page contract and config factory
```

---

## 4. Lane C 启动 Prompt

建议分支：`codex/workbench-shell`

启动条件：Lane B 完成，page contract 已冻结。你是唯一可大改 `WorkbenchLayout.tsx` 的 worker。

```text
你是 Lane C，负责“WorkbenchLayout 壳层模块化”。直接开始实现，不要改测试归属文件。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
拆分超大 `WorkbenchLayout.tsx`，让壳层只负责装配，不再直接承载页面特例。

你的白名单文件：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/layout/useWorkbenchShellState.ts (create)
- F:/erp/apps/erp-web/src/app/layout/WorkbenchShellChrome.tsx (create)
- F:/erp/apps/erp-web/src/app/layout/WorkbenchShellPanels.tsx (create)
- F:/erp/apps/erp-web/src/app/layout/WorkbenchShellResize.tsx (create)
- F:/erp/apps/erp-web/src/app/layout/WorkbenchCommandPalette.tsx (create)
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLiveRegion.tsx (create)
- F:/erp/apps/erp-web/src/app/layout/workbenchLayout.css
- F:/erp/apps/erp-web/src/app/layout/useWorkbenchShellState.test.ts (create)
- F:/erp/apps/erp-web/src/app/layout/WorkbenchShellResize.test.tsx (create)
- F:/erp/apps/erp-web/src/app/layout/WorkbenchCommandPalette.behavior.test.tsx (create)

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.command-palette.test.ts
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts
- F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts
- F:/erp/apps/erp-web/src/app/router.tsx
- F:/erp/apps/erp-web/src/app/layout/menu.tsx

必须完成的工作：
1) 抽离布局状态与 URL 回放逻辑到 hook。
2) 抽离 chrome、panels、resize、command palette、live region 到独立模块。
3) 清理页面特例分支，统一改读 Lane B 产出的 contract。
4) 让 `WorkbenchLayout.tsx` 退化成壳层装配文件。
5) 只创建新的模块级测试，不要改 D 独占的共享测试文件。

建议执行步骤：
1) 先建立模块骨架与新测试。
2) 再从 `WorkbenchLayout.tsx` 分批迁移代码。
3) 最后做 contract 接线和回归。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/app/layout/useWorkbenchShellState.test.ts src/app/layout/WorkbenchShellResize.test.tsx src/app/layout/WorkbenchCommandPalette.behavior.test.tsx src/app/layout/RouteTabs.keyboard-flow.test.ts

建议提交信息：
refactor(workbench): modularize shell layout and state
```

---

## 5. Lane D 启动 Prompt

建议分支：`codex/workbench-tests`

启动条件：Lane A 完成；Lane B contract 冻结；Lane C 壳层公共 API 冻结后再合并最终版本。

```text
你是 Lane D，负责“测试资产修复与行为断言替换”。你独占共享测试文件。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
把当前工作台测试从漂移的旧断言和源码字符串匹配，升级为基于真实行为的稳定回归资产。

你的白名单文件：
- F:/erp/apps/erp-web/src/app/navigation/workbenchRouteParity.test.ts (create)
- F:/erp/apps/erp-web/e2e/workbench-route-reachability.spec.ts (create)
- F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.command-palette.test.ts
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts
- F:/erp/apps/erp-web/src/app/layout/ModuleHeader.industrial-upgrade.test.ts
- F:/erp/apps/erp-web/e2e/workbench-copy-integrity.spec.ts (create, if needed)

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/router.tsx
- F:/erp/apps/erp-web/src/app/layout/menu.tsx
- 任何 orders/channels 页面文件

必须完成的工作：
1) 修复 `workbench-menu-navigation.spec.ts` 的重复标题和旧模块漂移。
2) 让菜单导航 E2E 只覆盖正式 registry 页面。
3) 把命令面板和 industrial-upgrade 测试改成行为断言。
4) 增加 route parity / reachability smoke test。
5) 增加“页面删除负向漂移”测试：页面被移除后，菜单、路由、shell、alias、测试期望同步失败。

建议执行步骤：
1) 先把现有失败项收敛成明确的红测。
2) 再改共享测试文件。
3) 不要改生产代码；如确需新增 test hook，返回 NEEDS_CONTEXT。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/app/layout/WorkbenchLayout.command-palette.test.ts src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts src/app/layout/ModuleHeader.industrial-upgrade.test.ts src/app/navigation/workbenchRouteParity.test.ts
pnpm --dir apps/erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/workbench-route-reachability.spec.ts --project=chromium

建议提交信息：
test(workbench): replace brittle assertions with route parity behaviors
```

---

## 6. Lane E 启动 Prompt

建议分支：`codex/workbench-order-review`

启动条件：Lane B、Lane C、Lane D 全部完成并冻结公共接口。

```text
你是 Lane E，负责“黄金样板页 1 / OrderReviewPage”。直接开始实现，不要改壳层文件。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
把 OrderReviewPage 收敛成订单型工作台的标准模板页，验证 contract、三面板、命令动作、状态回放都能在模板上成立。

你的白名单文件：
- F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx
- F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.keyboard-efficiency.test.ts
- F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx
- F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.test.ts
- F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewDetailPanel.tsx
- F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewDetailPanel.test.ts
- F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewBottomPanel.tsx
- F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewBottomPanel.contract.test.ts
- F:/erp/apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.ts
- F:/erp/apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.test.ts
- F:/erp/apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx
- F:/erp/apps/erp-web/src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts
- F:/erp/apps/erp-web/src/pages/orders/shared/orderWorkbenchEditorAdapter.ts
- F:/erp/apps/erp-web/src/pages/orders/shared/orderWorkbenchEditorAdapter.test.ts
- F:/erp/apps/erp-web/e2e/order-review-interactions.spec.ts
- F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts
- F:/erp/apps/erp-web/e2e/order-review-query.spec.ts
- F:/erp/apps/erp-web/e2e/order-review-golden-page.spec.ts (create)

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/router.tsx
- F:/erp/apps/erp-web/src/app/layout/menu.tsx
- 任何 channels 页面文件

必须完成的工作：
1) 将 OrderReviewPage 迁移到新 contract。
2) 收敛主表、右侧详情、底部明细、toolbar、overlays、selection、saved view 的接入方式。
3) 明确 loading / error / empty / no-permission 状态。
4) 补一个黄金样板页级 E2E，证明它可以作为复刻母版。
5) 不要改壳层；发现 contract 缺口时返回 NEEDS_CONTEXT。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/pages/orders/OrderReviewPage.keyboard-efficiency.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/review-native/OrderReviewDetailPanel.test.ts src/pages/orders/review-native/OrderReviewBottomPanel.contract.test.ts src/pages/orders/review-native/orderReviewToolbarActions.test.ts src/pages/orders/shared/orderWorkbenchEditorAdapter.test.ts
pnpm --dir apps/erp-web exec playwright test e2e/order-review-interactions.spec.ts e2e/order-review-selection.spec.ts e2e/order-review-query.spec.ts e2e/order-review-golden-page.spec.ts --project=chromium

建议提交信息：
feat(workbench-order): elevate order review to golden template page
```

---

## 7. Lane F 启动 Prompt

建议分支：`codex/workbench-channel-directory`

启动条件：Lane B、Lane C、Lane D 全部完成并冻结公共接口。

```text
你是 Lane F，负责“黄金样板页 2 / ChannelDirectoryPage”。直接开始实现，不要改订单域和壳层主文件。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
把 ChannelDirectoryPage 收敛成目录型 CRUD 工作台的标准模板页，用于证明模板不只适用于订单页。

你的白名单文件：
- F:/erp/apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx
- F:/erp/apps/erp-web/src/pages/channels/channelDirectoryPage.test.ts
- F:/erp/apps/erp-web/src/pages/channels/ChannelDirectoryPanels.tsx
- F:/erp/apps/erp-web/src/pages/channels/ChannelWorkbenchPanels.tsx
- F:/erp/apps/erp-web/src/pages/channels/ChannelWorkbenchPanels.test.ts
- F:/erp/apps/erp-web/src/pages/channels/channelDirectoryColumns.tsx
- F:/erp/apps/erp-web/src/pages/channels/channelDirectoryModel.ts
- F:/erp/apps/erp-web/src/pages/channels/channelDirectoryModel.test.ts
- F:/erp/apps/erp-web/src/pages/channels/channelWorkbenchModel.ts
- F:/erp/apps/erp-web/src/pages/channels/channelWorkbenchModel.test.ts
- F:/erp/apps/erp-web/e2e/channels-shops-list-visibility.spec.ts
- F:/erp/apps/erp-web/e2e/channel-directory-golden-page.spec.ts (create)

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/router.tsx
- F:/erp/apps/erp-web/src/app/layout/menu.tsx
- 任何 orders 页面文件

必须完成的工作：
1) 将 ChannelDirectoryPage 迁移到新 contract。
2) 统一目录页的 toolbar、left/right panel、drawer editor、context menu、pagination、empty/error 状态。
3) 让目录型工作台也通过模板能力，而不是页面内自拼。
4) 新增黄金样板页级 E2E。
5) 保持写入范围只在渠道域，不要回改壳层或订单页。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/pages/channels/channelDirectoryPage.test.ts src/pages/channels/channelDirectoryModel.test.ts src/pages/channels/channelWorkbenchModel.test.ts src/pages/channels/ChannelWorkbenchPanels.test.ts
pnpm --dir apps/erp-web exec playwright test e2e/channels-shops-list-visibility.spec.ts e2e/channel-directory-golden-page.spec.ts --project=chromium

建议提交信息：
feat(workbench-channel): elevate channel directory to golden template page
```

---

## 8. Lane G 启动 Prompt

建议分支：`codex/workbench-a11y`

启动条件：Lane C 已完成第一轮壳层拆分并冻结公共结构。

```text
你是 Lane G，负责“a11y 与交互硬化”。直接开始实现，不要去改订单域和渠道域页面。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
把工作台的键盘可达性、分栏语义、focus return、aria labels 和 a11y smoke gate 固化成模板标准。

你的白名单文件：
- F:/erp/apps/erp-web/src/app/layout/ModuleHeader.tsx
- F:/erp/apps/erp-web/src/app/layout/ModuleHeader.a11y-labels.test.ts
- F:/erp/apps/erp-web/src/app/layout/RouteTabs.tsx
- F:/erp/apps/erp-web/src/app/layout/RouteTabs.keyboard-flow.test.ts
- F:/erp/apps/erp-web/src/app/layout/workbenchLayout.css
- F:/erp/apps/erp-web/src/app/layout/useFocusReturn.ts
- F:/erp/apps/erp-web/src/app/layout/ShortcutToolbar.tsx
- F:/erp/apps/erp-web/src/app/layout/ShortcutToolbar.keyboard-reorder.test.ts
- F:/erp/apps/erp-web/e2e/workbench-a11y.spec.ts

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/router.tsx
- 任何 orders/channels 页面文件

必须完成的工作：
1) 审核并修复模板层 aria label、keyboard flow、focus return。
2) 确认分栏、tabs、toolbar、search、快捷帮助都能被键盘完整操作。
3) 扩展 `workbench-a11y.spec.ts`，把 dashboard、orders review、sales query、channel directory 纳入 smoke gate。
4) 不改共享测试归属文件之外的壳层主逻辑。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/app/layout/ModuleHeader.a11y-labels.test.ts src/app/layout/RouteTabs.keyboard-flow.test.ts src/app/layout/ShortcutToolbar.keyboard-reorder.test.ts
pnpm --dir apps/erp-web exec playwright test e2e/workbench-a11y.spec.ts --project=chromium

建议提交信息：
feat(workbench-a11y): harden keyboard and accessibility baseline
```

---

## 9. Lane H 启动 Prompt

建议分支：`codex/workbench-telemetry`

启动条件：Lane C 已完成第一轮壳层拆分；Lane E/F 可提供真实页面样本时优先。

```text
你是 Lane H，负责“预热、性能、可观测性标准化”。直接开始实现，不要修改业务页布局。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
把 route prefetch、diagnostics reporting、性能预算和交互延迟测试收敛成模板公共标准。

你的白名单文件：
- F:/erp/apps/erp-web/src/app/navigation/routePrefetch.ts
- F:/erp/apps/erp-web/src/app/navigation/routePrefetch.test.ts
- F:/erp/apps/erp-web/src/app/navigation/RouteDiagnosticsReporter.tsx
- F:/erp/apps/erp-web/src/app/navigation/routeDiagnosticsReporting.ts
- F:/erp/apps/erp-web/src/app/navigation/routeDiagnosticsReporting.test.ts
- F:/erp/apps/erp-web/src/app/navigation/workbenchTelemetry.ts (create)
- F:/erp/apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.ts
- F:/erp/apps/erp-web/e2e/order-review-performance-budget.spec.ts
- F:/erp/apps/erp-web/e2e/order-review-interaction-latency-evaluation.spec.ts

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/router.tsx
- 任何 channels 页面文件

必须完成的工作：
1) 定义工作台 telemetry 事件。
2) 收敛 route prefetch 的触发边界与诊断输出。
3) 让性能预算和延迟评估与工作台模板直接关联。
4) 保持业务行为不变，不要为过测试而禁用诊断或关闭功能。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/app/navigation/routePrefetch.test.ts src/app/navigation/routeDiagnosticsReporting.test.ts src/components/page-templates/workbench/useWorkbenchLayoutController.test.ts
pnpm --dir apps/erp-web exec playwright test e2e/order-review-performance-budget.spec.ts e2e/order-review-interaction-latency-evaluation.spec.ts --project=chromium

建议提交信息：
feat(workbench-obs): standardize telemetry prefetch and performance gates
```

---

## 10. Lane I 启动 Prompt

建议分支：`codex/workbench-copy`

启动条件：Lane E、Lane F、Lane G 完成首轮并冻结页面结构后再启动。先做浏览器核对，再动代码。

```text
你是 Lane I，负责“文案与空错态治理”。先做浏览器核对，再决定是否修改代码。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
确认真实 UI 是否存在乱码、旧英文残留、空错态文案漂移，并将模板层通用文案收敛到统一来源。

你的白名单文件：
- F:/erp/apps/erp-web/src/app/layout/workbenchI18n.ts
- F:/erp/apps/erp-web/src/app/layout/ModuleHeader.tsx
- F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx
- F:/erp/apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx
- F:/erp/apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx
- F:/erp/apps/erp-web/e2e/workbench-copy-integrity.spec.ts (create)

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx
- F:/erp/apps/erp-web/src/app/router.tsx
- F:/erp/apps/erp-web/src/app/layout/menu.tsx

必须完成的工作：
1) 先用真实浏览器确认 dashboard、order review、sales order query、channel directory 是否真的出现乱码或混语。
2) 只在确认存在真实 UI 问题后再改代码；如果只是终端编码采样问题，明确写清结论。
3) 将模板层通用 copy 收敛到统一来源。
4) 新增文案完整性 E2E，覆盖命令面板、工具栏、空态、错态、帮助文案。

必须执行命令：
pnpm --dir apps/erp-web test:unit -- src/app/layout/menu.test.ts src/app/layout/WorkbenchLayout.command-palette.test.ts
pnpm --dir apps/erp-web exec playwright test e2e/workbench-copy-integrity.spec.ts --project=chromium

建议提交信息：
feat(workbench-copy): normalize visible copy and state messaging
```

---

## 11. Lane J 启动 Prompt

建议分支：`codex/workbench-gates`

启动条件：至少 Lane E、Lane F 已达到可回归状态。推荐在 M3 后启动。

```text
你是 Lane J，负责“CI、报告与复制门禁”。你是收口 worker，不负责大块业务实现。

主计划：
F:/erp/docs/superpowers/plans/2026-04-24-workbench-template-industrialization.md

你的目标：
把工业化验收命令、黄金样板页门禁、CI、验证报告、复制条件固化下来。

你的白名单文件：
- F:/erp/apps/erp-web/package.json
- F:/erp/.github/workflows/ci.yml
- F:/erp/docs/reports/workbench-template-industrialization-verification-2026-04-24.md (create)
- F:/erp/docs/reports/README.md

禁止修改：
- F:/erp/apps/erp-web/src/app/layout/**
- F:/erp/apps/erp-web/src/pages/orders/**
- F:/erp/apps/erp-web/src/pages/channels/**

必须完成的工作：
1) 新增黄金样板页专用命令，如 `test:unit:golden-pages` / `test:e2e:golden-pages`，或等价聚合脚本。
2) 更新 CI，让 golden pages、route parity、workbench a11y 进入必跑门禁。
3) 产出验证报告，记录通过项、残余风险、2B 扩展样板页的后续日期化 gate。
4) 不要改业务实现，只做门禁与报告收口。

必须执行命令：
pnpm --dir apps/erp-web lint
pnpm --dir apps/erp-web typecheck
pnpm --dir apps/erp-web test:unit -- src/pages/orders/OrderReviewPage*.test.tsx src/pages/channels/ChannelDirectoryPage*.test.tsx
pnpm --dir apps/erp-web exec playwright test e2e/order-review-*.spec.ts e2e/channel-directory-*.spec.ts --project=chromium
pnpm --dir apps/erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/workbench-a11y.spec.ts --project=chromium
pnpm --dir apps/erp-web test:e2e:perf
pnpm --dir apps/erp-web test:e2e:latency
pnpm --dir apps/erp-web test:e2e:release-gate

建议提交信息：
chore(workbench): enforce golden-page gates and industrial verification
```

---

## 12. 发放建议

建议按下面方式发放：

1. 先发“通用广播模板”。
2. 再发 Lane A。
3. Lane A 交付并确认 registry 冻结后，再发 Lane B。
4. Lane B 冻结后，单独发 Lane C。
5. Lane C 完成公共壳层拆分后，再并行发 Lane D、Lane E、Lane F、Lane G、Lane H。
6. Lane E/F/G 首轮稳定后，再发 Lane I。
7. 所有黄金样板页可回归后，再发 Lane J。

每个 worker 首条回复必须先回显：
- 目标
- 白名单文件
- 第一条命令

如果任意 worker 发现需要越界改文件，统一返回 `NEEDS_CONTEXT`，由主控重新切分，不允许自己扩边界。
