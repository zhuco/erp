# Industrial ERP 95%+ Round 10 Shelf-Life And Serial Visibility Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 v0.1.3 已完成商品行业化主档与运行期治理基础上，继续把食品批次页推进到“临期/禁售可视化”，把数码序列号页推进到“治理要求可视化”，形成更接近工业级一线作业体验的前台闭环。

**Architecture:** 当前项目整体工业级电商 ERP 完成度约为 `97.8%`。Round 10 不扩散到 OMS、WMS、财务主链，也不触碰后端 schema，而是围绕 `SkuBatchesPage.tsx`、`SkuSerialsPage.tsx`、`data.ts` 与 `sku-runtime-governance.spec.ts` 做一轮受控增强：食品侧补足临期/禁售状态标签、效期筛选与摘要统计；数码侧补足入库唯一校验、出库强绑、平台回传等治理要求的前台透出，让运行期页面更适合培训、巡检与一线操作。

**Tech Stack:** React、TypeScript、Ant Design、React Query、Playwright、Vitest、Mock API

---

## 当前完成度重新评估

**整体完成度判断：** `97.8%` 左右

**已形成较强 fresh 证据的域：**
- 商品主档已经支持食品、鞋服、数码行业模板和行业摘要。
- 食品批次页已经支持失效日期阻断、未启用批次管理阻断。
- 数码序列号页已经支持推荐前缀、未启用序列号管理阻断。
- 商品行业化回归与运行期治理回归已形成 fresh E2E 证据。

**当前最值得继续补的高价值缺口：**
- 食品批次页仍缺少“临期/禁售”状态标签和可筛选视图，一线看不到效期风险分层。
- 食品效期治理仍停留在创建抽屉，批次台账缺少页头级治理摘要和风险统计。
- 数码序列号页虽然已经能推荐前缀，但“入库唯一校验 / 出库强绑定 / 平台回传”仍然只是隐含策略，缺少更直接的前台透出。
- 运行期治理 E2E 还没有覆盖“临期/禁售状态可见”和“数码治理要求标签可见”。

## Round 10 五条主线

### Task 1: 食品批次临期/禁售状态标签前台化
**价值：** 让批次台账从“只是列表”提升为可读的效期风险看板。

### Task 2: 食品批次页效期摘要与治理筛选
**价值：** 让班组长和运营能够快速筛出临期批次、禁售批次和正常批次。

### Task 3: 数码序列号治理要求透出
**价值：** 把入库唯一校验、出库强绑定、平台回传等策略前台化，降低培训成本。

### Task 4: Round 10 运行期治理 E2E 扩展
**价值：** 用 fresh Playwright 证据覆盖食品效期状态可见与数码治理要求可见，不让本轮停留在文案层。

### Task 5: 文档、任务卡与执行记录
**价值：** 沉淀 Round 10 的目标、边界、验证命令、完成度变化与剩余差距。

## 并行执行原则

1. 本轮只允许聚焦 `skuBatches`、`skuSerials`、`mock data`、`sku-runtime-governance.spec.ts` 与 Round 10 文档，不扩散到 OMS、WMS、财务主链。
2. 先补失败 E2E，再做页面实现，最后跑 fresh 验证。
3. 子任务写入范围必须互不重叠；若子线失败或超时，主线程立即接管。
4. 新增提示、状态、摘要、任务卡和日志统一使用中文并保持 UTF-8。
5. 所有完成判断都必须建立在 fresh Playwright/Vitest 证据之上。

### Task 1: 食品批次临期/禁售状态标签前台化

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/data.ts`
- Test: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 先写食品批次状态标签失败 E2E**
- [ ] **Step 2: 运行 `pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium --grep "临期|禁售"`，确认红灯**
- [ ] **Step 3: 在批次台账中增加临期、禁售、正常三类状态标签**
- [ ] **Step 4: 补稳定 mock 批次样本，确保临期/禁售场景可稳定命中**
- [ ] **Step 5: 重跑食品状态标签用例，确认转绿**

### Task 2: 食品批次页效期摘要与治理筛选

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 为批次页新增效期风险摘要**
- [ ] **Step 2: 新增临期/禁售/正常筛选项**
- [ ] **Step 3: 保持筛选逻辑前台可验证，不要求后端新增查询参数**
- [ ] **Step 4: 为摘要和筛选补 `data-testid`，便于 E2E 断言**
- [ ] **Step 5: 重跑食品治理用例，确认摘要与筛选稳定**

### Task 3: 数码序列号治理要求透出

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuSerials/SkuSerialsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 先写数码治理要求可见失败 E2E**
- [ ] **Step 2: 运行 `pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium --grep "治理要求"`，确认红灯**
- [ ] **Step 3: 在生成抽屉或页头前台化入库唯一校验、出库强绑定、平台回传三类策略**
- [ ] **Step 4: 保持推荐前缀与生成禁用态行为不回退**
- [ ] **Step 5: 重跑数码治理用例，确认转绿**

### Task 4: Round 10 运行期治理 E2E 扩展

**Files:**
- Modify: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 扩展食品批次状态可见断言**
- [ ] **Step 2: 扩展食品批次治理筛选断言**
- [ ] **Step 3: 扩展数码治理要求标签断言**
- [ ] **Step 4: 跑整组运行期治理 fresh 验证**
- [ ] **Step 5: 补跑商品行业化回归与行业单测**

### Task 5: 文档、任务卡与执行记录

**Files:**
- Create: `F:/erp/docs/superpowers/plans/2026-04-15-industrial-erp-95-plus-round10-shelf-life-serial-visibility-plan.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r10-01.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r10-02.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r10-03.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r10-04.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r10-05.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-round10-exec-log.md`

- [ ] **Step 1: 记录本轮聚焦点、并行策略与验证命令**
- [ ] **Step 2: 记录红灯到绿灯证据**
- [ ] **Step 3: 记录 fresh 结果与完成度变化**
- [ ] **Step 4: 评估本轮后距离工业级 ERP 的剩余差距**
- [ ] **Step 5: 为 Round 11 留出清晰接续点**
