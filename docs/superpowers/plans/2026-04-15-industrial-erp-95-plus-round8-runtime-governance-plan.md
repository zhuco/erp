# Industrial ERP 95%+ Round 8 Runtime Governance Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在商品行业化主档已完成的基础上，把食品批次治理与数码序列号治理正式挂到运行期页面，形成“可见提示 + 前置校验 + fresh E2E 证据”的工业级闭环。

**Architecture:** 当前项目整体工业级电商 ERP 完成度约为 `96.5%`，订单、仓储、售后、财务与商品主档已经具备较强的 fresh 证据。Round 8 不扩散到 OMS/WMS/财务主链，也不重造后端规则中心，而是围绕 `SkuBatchesPage.tsx`、`SkuSerialsPage.tsx`、`data.ts` 与新增运行期治理 E2E 做一轮高收益补缺：把食品效期策略和数码序列号策略挂到运行页的真实操作入口，让前台先具备工业级治理触点。

**Tech Stack:** React、TypeScript、Ant Design、React Query、Vitest、Playwright、Mock API

---

## 当前完成度重新评估

**整体完成度判断：** `96.5%` 左右

**已接近工业级闭环的域：**
- OMS 审单、异常单、履约主链已有多轮 fresh E2E 证据。
- WMS 波次、出库、发运主链已有 fresh E2E 证据。
- 售后、客服、财务经营前台已有稳定页面与主流程验证。
- 商品主档已支持食品保质期/生产日期、鞋服尺码颜色、数码序列号策略的行业化建档与回显。

**当前最需要继续补强的功能缺口：**
- 食品治理尚未真正落到批次页，缺少“食品 SKU 建批次必须带失效日期”的运行期证据。
- 数码治理尚未真正落到序列号页，缺少“未启用序列号管理的 SKU 不能生成唯一码”的运行期证据。
- 运行期页面依赖的 mock 样本还不够稳定，E2E 不宜继续依赖随机 SKU 命中。
- `sku-serial-rules` 接口当前不是 mock 优先，生成抽屉关键路径不能再依赖它成功返回。

## Round 8 五条主线

### Task 1: 食品批次治理挂点前台化
**价值：** 把食品行业规则从商品主档下沉到批次创建动作，形成“看到规则就能执行规则”的运行期闭环。

### Task 2: 数码序列号治理挂点前台化
**价值：** 把数码商品的序列号治理从静态摘要下沉到唯一码生成动作，形成可阻断错误操作的前台能力。

### Task 3: 运行期专用 mock 样本补齐
**价值：** 为食品批次治理与数码序列号治理提供稳定、可搜索、可回归的专用商品/SKU 样本，降低 E2E 随机性。

### Task 4: 运行期治理 E2E 补齐
**价值：** 用 fresh Playwright 证据覆盖食品批次与数码序列号两条运行期治理路径，确保本轮不是“只改页面文案”。

### Task 5: 文档、任务卡与执行记录
**价值：** 把本轮文件边界、验证命令、接管策略与功能缺口沉淀下来，方便继续推进 Round 9。

## 并行执行原则

1. 若子线再次出现 `502 Bad Gateway` 或长时间无返回，立即停止重试，由主线程接管，不做机械空耗。
2. 本轮页面实现集中在 `skuBatches`、`skuSerials` 与 `mock data`，不扩散到订单、WMS、财务主链。
3. 所有行为先补失败 E2E，再做最小实现，最后做 fresh 验证。
4. `sku-serial-rules` 不作为关键成功路径依赖；即便规则接口失败，数码运行期治理也必须可用。
5. 新增 UI 提示、日志、说明统一使用中文并保持 UTF-8。

### Task 1: 食品批次治理挂点前台化

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/productIndustry.ts`
- Test: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 先写食品批次治理失败 E2E**
- [ ] **Step 2: 运行 `pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium --grep "食品批次"`，确认红灯**
- [ ] **Step 3: 页面上增加食品效期治理提示、SKU 批次能力判断与失效日期校验**
- [ ] **Step 4: 重跑食品批次用例，确认提示与阻断行为转绿**
- [ ] **Step 5: 记录仍未下钻到入库/临期作业链路的边界**

### Task 2: 数码序列号治理挂点前台化

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuSerials/SkuSerialsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/productIndustry.ts`
- Test: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 先写数码序列号治理失败 E2E**
- [ ] **Step 2: 运行 `pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium --grep "数码序列号"`，确认红灯**
- [ ] **Step 3: 页面上增加序列号治理提示、SKU 序列号能力判断，并让生成流程不强依赖规则接口**
- [ ] **Step 4: 重跑数码序列号用例，确认禁用阻断与启用生成全部转绿**
- [ ] **Step 5: 记录仍未下钻到入库唯一校验与出库强绑后端链路的边界**

### Task 3: 运行期专用 mock 样本补齐

**Files:**
- Modify: `F:/erp/apps/erp-web/src/api/mocks/data.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/skus.ts`

- [ ] **Step 1: 增加食品治理样本商品与样本 SKU**
- [ ] **Step 2: 增加数码治理样本商品与样本 SKU**
- [ ] **Step 3: 确保搜索接口稳定命中这些样本**
- [ ] **Step 4: 如有必要补对应批次/序列号初始样本**
- [ ] **Step 5: 保持改动只服务运行期治理，不重构整套 mock 架构**

### Task 4: 运行期治理 E2E 补齐

**Files:**
- Create: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/fixtures.ts`（仅在必要时）

- [ ] **Step 1: 新建食品批次治理场景**
- [ ] **Step 2: 新建数码序列号治理场景**
- [ ] **Step 3: 先确认选择器因页面未实现而失败，证明测试有效**
- [ ] **Step 4: 页面实现后再次跑通整组 E2E**
- [ ] **Step 5: 保持断言聚焦治理行为，不写与本轮无关的大而全回归**

### Task 5: 文档、任务卡与执行记录

**Files:**
- Create: `F:/erp/docs/superpowers/plans/2026-04-15-industrial-erp-95-plus-round8-runtime-governance-plan.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r8-01.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r8-02.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r8-03.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r8-04.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r8-05.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-round8-exec-log.md`

- [ ] **Step 1: 写清本轮目标、文件边界、验证命令与接管策略**
- [ ] **Step 2: 记录子线失败由主线程接管的事实**
- [ ] **Step 3: 记录 fresh 验证结果与完成度变化**
- [ ] **Step 4: 评估本轮后仍待补的运行期缺口**
- [ ] **Step 5: 为下一轮 Round 9 留出清晰接续点**
