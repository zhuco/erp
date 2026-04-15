# Industrial ERP 95%+ Round 3 Parallel Push Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 2026 年 4 月 15 日这一轮，把项目从“主链可用”推进到“95%+ 可验证交付”，优先补齐多实体权限 E2E、OMS 前端主链 E2E、WMS 异常回归、商品治理回归与运维观测工作流证据。

**Architecture:** 本轮继续坚持“白名单边界 + 高信噪验证 + 并行推进”的方法，不再扩散大范围业务面。5 条子任务线分别服务于 5 条主轨，每条任务卡都要以最小文件集合产出 fresh 测试证据，并把环境噪音与真实逻辑缺口分开记录。

**Tech Stack:** React + TypeScript + Playwright + Vitest + Rust + Axum + SQLx + PostgreSQL + pnpm + cargo

---

### Task 1: Track-01 多实体权限 UI 业务链 E2E

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/system/DataScopesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/UsersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Create or Modify: `F:/erp/apps/erp-web/e2e/system-governance.spec.ts`
- Optional Create: `F:/erp/apps/erp-web/e2e/multi-entity-governance.spec.ts`

- [ ] **Step 1: 先写或扩展失败 E2E，锁定多实体用户/数据权限业务链**
- [ ] **Step 2: 运行目标 E2E，确认当前失败点属于页面贯通而非测试脆弱**
- [ ] **Step 3: 用最小 UI/路由修复打通入口、列表和详情展示**
- [ ] **Step 4: 重跑目标 E2E，并补一条最小回归命令**
- [ ] **Step 5: 记录仍未覆盖的 merchant/org/shop 深层权限分支**

### Task 2: Track-02 商品治理与差异页回归补强

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductDiffPage.tsx`
- Create or Modify: `F:/erp/apps/erp-web/e2e/product-change-control.spec.ts`
- Optional Create: `F:/erp/apps/erp-web/e2e/product-diff-governance.spec.ts`

- [ ] **Step 1: 先写失败测试，锁定差异页筛选、刷新或查询回填的真实缺口**
- [ ] **Step 2: 运行 Playwright 定位缺口是否来自 query invalidation / 页面稳定性 / 选择器不足**
- [ ] **Step 3: 做最小页面修复，不扩展新功能面**
- [ ] **Step 4: 运行目标 E2E 收口，并确认不会回归既有商品治理链**
- [ ] **Step 5: 记录与 OMS/WMS 链的边界，不跨到订单或仓储页面**

### Task 3: Track-03 OMS 前端主链 E2E 收口

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrdersPage.tsx`
- Create or Modify: `F:/erp/apps/erp-web/e2e/order-processing-flow.spec.ts`
- Optional Create: `F:/erp/apps/erp-web/e2e/order-workbench-pages.spec.ts`

- [ ] **Step 1: 先补失败 E2E，覆盖审核、异常、查询三段中的至少一条强链路**
- [ ] **Step 2: 运行目标 E2E，确认问题是页面行为还是断言不稳定**
- [ ] **Step 3: 做最小修复，优先补 test id、稳定选择器或必要的页面状态处理**
- [ ] **Step 4: 跑通目标 E2E，并与既有 `oms_orders` 后端证据形成对应关系**
- [ ] **Step 5: 输出尚未覆盖的拆单/合单/结算前端缺口**

### Task 4: Track-04 WMS 异常路径与失效回归补强

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- Create or Modify: `F:/erp/apps/erp-web/e2e/outbound-flow.spec.ts`
- Create or Modify: `F:/erp/apps/erp-web/e2e/wms-handover.spec.ts`

- [ ] **Step 1: 先写失败测试，覆盖波次/发货/出库确认中的异常或 query invalidation 链**
- [ ] **Step 2: 运行目标 E2E，确认失败属于真实状态同步缺口**
- [ ] **Step 3: 做最小页面修复，继续严格限制在现有 invalidation 白名单内**
- [ ] **Step 4: 重跑目标 E2E，并复验主链 `wms-wave-main-chain` 或 `outbound-fulfillment-flow` 不回退**
- [ ] **Step 5: 记录仍未纳入本轮的 PDA / 现场执行 / 自动化设备边界**

### Task 5: Track-05 运维观测工作流与证据收口

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/frontendRouteObservability.ts`
- Create or Modify: `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`
- Create or Modify: `F:/erp/apps/erp-web/e2e/frontend-route-observability.spec.ts`
- Optional Modify: `F:/erp/docs/ops/tasks/2026-04-15/erp95-round3-exec-log.md`

- [ ] **Step 1: 先补失败 E2E，覆盖页面摘要、前端路由观测和运维排障入口中的一条工作流**
- [ ] **Step 2: 运行目标 E2E，区分是假红灯、接口噪音还是页面聚合逻辑缺口**
- [ ] **Step 3: 用最小修复收口观测摘要与运维入口行为**
- [ ] **Step 4: 重跑目标 E2E，并确认不会回退本轮既有 Observability 修复**
- [ ] **Step 5: 输出“仍属于环境噪音”的证据和“仍是逻辑缺口”的证据**

## 并行执行原则

1. 每张任务卡只允许在自己的 `allowed_files` 内修改，不得跨轨争抢文件。
2. 一律先写失败测试或复现失败命令，再实施修复。
3. 每张卡至少产出一条 fresh 验证结果，不能只提交代码不交证据。
4. `apps/erp-web/src/pages/orders/orderReviewPage.css` 继续视为现存本地改动，不纳入本轮白名单。
5. Rust 测试中的 `pool timed out while waiting for an open connection` 继续优先按环境噪音记录，但不能拿来掩盖真实逻辑失败。
