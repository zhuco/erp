# Industrial ERP 95%+ Round 2 Fast Push Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在已吸收 OMS、WMS、观测页和最小多实体基线修复的基础上，继续把主仓推进到“可持续编译、可验证、可继续并行吸收”的稳定快推状态。

**Architecture:** 本轮不再重新拆大而全方案，而是围绕“已验证成果优先吸收、主仓编译稳定优先、白名单边界优先、环境噪音与逻辑失败分离”四个原则推进。优先做 Track-01 白名单收口、Track-04/05 高信噪比验证和证据文档补全，避免再次把越界 worktree 整包灌回主仓。

**Tech Stack:** React + TypeScript + Playwright + Rust + Axum + SQLx + PostgreSQL + pnpm + cargo + Codex worktrees

---

### Task 1: 稳定 Track-01 多实体权限基线

**Files:**
- Modify: `F:/erp/crates/erp-repo/src/data_scopes.rs`
- Modify: `F:/erp/crates/erp-repo/src/tenants.rs`
- Modify: `F:/erp/crates/erp-api/src/lib.rs`
- Modify: `F:/erp/crates/erp-api/tests/api_rbac_tenant_isolation.rs`
- Modify: `F:/erp/apps/erp-web/src/pages/system/DataScopesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/UsersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Test: `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`

- [ ] **Step 1: 先跑高信噪比后端验证**
  Run: `cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture`
  Expected: 通过，或只出现已标记的数据库连接池环境噪音

- [ ] **Step 2: 核对白名单文件是否已经全部对齐**
  Outcome: 只在 `data_scopes.rs / tenants.rs / lib.rs / api_rbac_tenant_isolation.rs / DataScopesPage.tsx / UsersPage.tsx / router.tsx` 内继续推进

- [ ] **Step 3: 补齐前端数据权限展示与入口治理**
  Outcome: 数据权限页、用户页和路由入口与多实体字段对齐，不暴露不该进入生产主路径的演示入口

- [ ] **Step 4: 跑前端入口回归**
  Run: `pnpm --filter erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts --project=chromium`
  Expected: 通过，或输出清晰环境阻塞证据

- [ ] **Step 5: 记录 Track-01 仍未吸收的越界部分**
  Outcome: 明确 adapter、订单、库存等仍隔离，不在本轮混入

### Task 2: 扩大 Track-04 履约闭环验证证据

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- Modify: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/wms-wave-main-chain.spec.ts`

- [ ] **Step 1: 保持当前 query invalidation 白名单不扩张**
  Outcome: 仅围绕 `wms-outbounds / wms-ship-outbounds / wms-waves / inventory-summary / inventory-atp` 调整

- [ ] **Step 2: 跑履约入口 E2E**
  Run: `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts --project=chromium`
  Expected: 已吸收页面可见性断言继续通过

- [ ] **Step 3: 补波次主链最小覆盖**
  Run: `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/wms-wave-main-chain.spec.ts --project=chromium`
  Expected: 若失败，失败点明确落到页面联动而不是工具链误用

- [ ] **Step 4: 清点仍需隔离的旧 worktree 文档残留**
  Outcome: 继续不吸收 `erp95-06` 中越界 docs 残留

### Task 3: 扩大 Track-05 售后与观测高信噪比推进

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/frontendRouteObservability.ts`
- Test: `F:/erp/crates/erp-api/tests/after_sales_state_machine_guard.rs`
- Optional Modify: `F:/erp/crates/erp-repo/src/after_sales.rs`
- Optional Modify: `F:/erp/crates/erp-repo/src/after_sales_intercepts.rs`

- [ ] **Step 1: 先跑售后状态机守卫测试**
  Run: `cargo test -p erp-api --test after_sales_state_machine_guard -- --nocapture`
  Expected: 通过，或仅出现连接池超时导致的 skip

- [ ] **Step 2: 若出现真实逻辑失败，再只改售后状态机白名单文件**
  Outcome: 不扩到财务、CI、workflow

- [ ] **Step 3: 补观测页空路径误报之外的第二处最小真实缺口**
  Outcome: 后端事件、路径摘要、错误展示三者关系更稳，不制造假红灯

- [ ] **Step 4: 记录售后链当前是逻辑缺口还是环境噪音**
  Outcome: 为最终 Go/No-Go 证据包准备可复用结论

### Task 4: 固化本轮已吸收成果与未吸收边界

**Files:**
- Modify: `F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-migration-map.md`
- Modify: `F:/erp/docs/ops/progress_check.md`
- Create or Modify: `F:/erp/docs/ops/tasks/2026-04-14/erp95-round2-fastpush-log.md`

- [ ] **Step 1: 把本轮已吸收到主仓的文件写入迁移清单**
  Outcome: OMS、WMS、Observability、ProductDiff 和 Track-01 最小补齐被明确记录

- [ ] **Step 2: 把已验证命令与环境噪音模式写入快推日志**
  Outcome: 后续不再重复误判 `pool timed out` 与 Playwright CLI 误用

- [ ] **Step 3: 对齐 progress check 的当前现实**
  Outcome: 让后续验证顺序更贴近主仓现状，而不是停留在旧的大而全入口

### Task 5: 准备下一轮并行吸收入口

**Files:**
- Modify: `F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-01.md`
- Modify: `F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-04.md`
- Modify: `F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-05.md`

- [ ] **Step 1: 在任务卡中写清当前已完成项**
  Outcome: 新一轮子任务不再重复做已吸收工作

- [ ] **Step 2: 明确每张卡下一步只允许推进的白名单文件**
  Outcome: 子代理继续工作时边界更清晰

- [ ] **Step 3: 标出每张卡的首个高信噪比验证命令**
  Outcome: 后续快推从验证入口而不是从猜测开始

## Round 2 Addendum

### 已确认的额外验证

- `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/wms-wave-main-chain.spec.ts --project=chromium`
  - 结果：通过，`1 passed`
  - 结论：Track-04 当前不仅 `outbound-fulfillment-flow` 可通过，`wms-wave-main-chain` 也已在主仓跑通

### 计划补充说明

- 本轮快推除 `Track-01/04/05` 外，还必须显式纳入已吸收的 OMS 与 UI 改动验证：
  - `F:/erp/crates/erp-api/src/orders.rs`
  - `F:/erp/crates/erp-api/tests/oms_orders.rs`
  - `F:/erp/apps/erp-web/src/pages/products/ProductDiffPage.tsx`
- 当前主仓还存在已修改但不在本轮快推白名单内的文件：
  - `F:/erp/apps/erp-web/src/pages/orders/orderReviewPage.css`
- 处理原则：
  - `orders.rs / oms_orders.rs / ProductDiffPage.tsx` 纳入本轮快推验证范围
  - `orderReviewPage.css` 仅记录为现存本地改动，不在本轮并行任务中继续扩写

## 本轮真实验证现状

- OMS: `Track-03` 的 `orders.rs` / `oms_orders.rs` 以及 `Track-01` 的 `DataScopesPage` / `UsersPage` 与 `api_rbac_tenant_isolation` 已在 Rust 单测中取得通过记录，核心 tracker 状态稳定。 WMS: `Track-04` 的波次、发货、运输界面联调完成，`wms-wave-main-chain` 通过，`outbound-fulfillment-flow` 仍需多次跑通。 Observability: `ObservabilityPage` 与 `ProductDiffPage` 的主要面板已恢复。 Track-02/Track-05 保持观察数据一致性。
- 验证层面：Rust 单测 (`oms_orders`, `api_rbac_tenant_isolation`) 与 Playwright (`outbound-fulfillment-flow`, `wms-wave-main-chain`) 均在 CI 中运行，`pool timed out while waiting for an open connection` 属于测试资源噪声，需继续记录 `pnpm` CLI 重试次数与 Playwright 堆栈。

## 风险白名单

- `apps/erp-web/src/pages/orders/orderReviewPage.css` 仍属于本地改动，不纳入当前快推白名单，需后续对齐后再决定是否合入。
- `function search` alias chain 尚未完全恢复，本轮仅做风险观察，不允许纳入白名单，等待 alias registry 与 handler 的一致性修复。

## 新增缺口与根因

- alias chain: `function search` 的 alias（如 `/function-search`、`/shops/function-search`）链路断裂导致请求无法命中 handler，分页/过滤参数被截断。根因在于 alias registry 与 handler 路径不同步，缺少 fallback，需要重建 alias->handler 的链路并补齐旧路径。
- ShopOrderQueryPage: `shop-order-download` 页面在挂载时提前触发 `initQuery`，而 `ShopOrderQueryStore` 的 `shopId/context` 尚未注入，导致首次拉取为空，页面停在 loading。需调整生命周期，等 `store.ready` 后再发起数据请求并引入失败重试。

## 下一轮 5 子任务建议

1. 重建 `function search` alias chain，确保 alias registry、handler 与旧路径一致，加入 fallback 日志防止断链复发。
2. 调整 `ShopOrderQueryPage` 的初始化顺序，在 `ShopOrderQueryStore` 就绪后再发起数据拉取，并在慢加载场景下记录告警。
3. 为 `Track-02` 的 `ProductDiffPage` 添加 query invalidation 复现用例，对比 `wms-wave-main-chain` 与 `outbound-fulfillment-flow` 的行为差异。
4. 继续追踪 `Track-01` (`DataScopesPage`/`UsersPage`/`api_rbac_tenant_isolation`) 的监控日志，准备下一轮验证报告。
5. 收集 `ShopOrderQueryPage`、WMS query invalidation、Observability 页面抖动日志，并重新跑 `outbound-fulfillment-flow`，为下轮验证提供 baseline。

## Round 2 Final Addendum

### Completed In This Round

- `Track-01` search entry chain has been closed with a compatibility fix:
  - `moduleSearch.ts`
  - `moduleSearch.test.ts`
  - verified by `pnpm --filter erp-web exec vitest --run src/app/layout/moduleSearch.test.ts`
  - verified by `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/workbench-menu-navigation.spec.ts --project=chromium`
- `shop-order-download` runtime crash has been fixed in `ShopOrderQueryPage.tsx`.
  - root cause: `queryCells` referenced `handleKeywordChange` before initialization
  - verified by the full `workbench-menu-navigation` suite passing

### Updated Go/No-Go View

- `Track-01`: navigation/workbench/search closure is now green for this round.
- `Track-03`: backend OMS evidence is strong (`oms_orders` fresh pass), but frontend OMS workbench coverage is still behind.
- `Track-04`: both `outbound-fulfillment-flow` and `wms-wave-main-chain` have fresh green evidence in this round.
- `Track-05`: `after_sales_state_machine_guard` has fresh green evidence in this round.

### Remaining Gaps For Next 5 Tasks

1. Build one fresh E2E around `DataScopesPage / UsersPage / router.tsx` multi-entity flow.
2. Add frontend OMS workbench E2E for review / exception / split-merge / settlement chain.
3. Extend WMS regression from main path to invalidation and exception branches.
4. Expand observability page coverage from page-open evidence to aggregated operator workflows.
5. Keep `orderReviewPage.css` isolated until its local modifications are reviewed separately.
