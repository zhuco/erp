# Industrial ERP 95%+ Round 9 Runtime Governance UX Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Round 8 已完成“运行期治理挂点前台化”的基础上，继续把食品批次页与数码序列号页推进到“按钮有禁用态、阻断有可见原因、策略有摘要透出、生成体验更顺滑”的工业级可用状态。

**Architecture:** 当前项目整体工业级电商 ERP 完成度约为 `97.2%`。Round 9 不再扩散到 OMS、WMS、财务主链，也不新增后端治理中心，而是聚焦 `SkuBatchesPage.tsx`、`SkuSerialsPage.tsx` 与 `sku-runtime-governance.spec.ts` 三个运行期入口文件，通过“测试先行 + 前端最小增强 + fresh 回归”完成一轮高性价比 UX 补缺，让治理能力从“能挡错”提升到“少误操作、少试错、少解释成本”。

**Tech Stack:** React、TypeScript、Ant Design、React Query、Playwright、Mock API

---

## 当前完成度重新评估

**整体完成度判断：** `97.2%` 左右

**本轮前已经形成的工业级证据：**
- 商品主档已支持食品保质期/生产日期、鞋服尺码颜色、数码序列号策略。
- `sku-batches` 与 `sku-serials` 已正式挂到路由与菜单。
- 食品 SKU 缺失失效日期、未启用批次管理等基础阻断已经可用。
- 数码 SKU 的序列号治理提示、启停能力校验与规则接口降级已经可用。
- `e2e/sku-runtime-governance.spec.ts` 已有 4 条 fresh 用例覆盖基础运行期治理链路。

**本轮最值得继续补的缺口：**
- 食品批次页当前更多是“点保存后报错”，缺少保存前的禁用态与明确原因提示。
- 数码序列号页仍需自动前缀建议，减少一线操作员手工输入和误输。
- 页面级治理摘要还不够前置，策略信息更多停留在抽屉内，不利于培训和巡检。
- 运行期 E2E 还没有直接证明“按钮禁用态”和“推荐前缀”两类 UX 行为。

## Round 9 五条主线

### Task 1: 食品批次页治理禁用态与保存前可见原因
**价值：** 把“保存时报错”前移成“保存前就知道为什么不能保存”，降低食品批次建档误操作。

### Task 2: 数码序列号页自动前缀建议与治理禁用态
**价值：** 让数码 SKU 进入生成抽屉后就具备推荐前缀和明确阻断原因，减少反复试错。

### Task 3: 运行期治理摘要与页头策略透出
**价值：** 在页面主视图先暴露治理规则，让班组长、客服、运营在进入抽屉前就知道当前治理边界。

### Task 4: Round 9 运行期治理 E2E 扩展
**价值：** 用 fresh Playwright 证据证明本轮不是单纯文案调整，而是实打实提升了禁用态与默认建议能力。

### Task 5: 文档、任务卡与执行记录
**价值：** 把 Round 9 的目标、边界、验证命令、完成度变化与剩余缺口沉淀下来，方便 Round 10 继续推进。

## 并行执行原则

1. 页面实现只允许聚焦 `skuBatches`、`skuSerials`、`sku-runtime-governance.spec.ts` 与 Round 9 文档，不扩散到订单、WMS、财务主链。
2. 本轮继续采用测试先行，必须先看到 E2E 红灯，再进入页面实现。
3. 若子线继续出现 `502 Bad Gateway`、无回包或状态不明确，主线程立即接管，不空等。
4. 新增提示、按钮、摘要、日志、任务卡统一使用中文并保持 UTF-8 编码。
5. 所有“完成”判断都必须建立在 fresh Playwright 结果之上，不接受“理论应当通过”。

### Task 1: 食品批次页治理禁用态与保存前可见原因

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 先把食品批次页的失败 E2E 扩到禁用态与原因提示**
- [ ] **Step 2: 运行 `pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium --grep "食品"`，确认红灯**
- [ ] **Step 3: 新增 `batchBlockReason`、保存按钮禁用态与原因提示区**
- [ ] **Step 4: 把抽屉标题、按钮、字段标签统一中文化并补稳定 `data-testid`**
- [ ] **Step 5: 重跑食品治理用例，确认保存前阻断转绿**

### Task 2: 数码序列号页自动前缀建议与治理禁用态

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuSerials/SkuSerialsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 先把数码序列号页的失败 E2E 扩到推荐前缀与生成禁用态**
- [ ] **Step 2: 运行 `pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium --grep "数码"`，确认红灯**
- [ ] **Step 3: 新增推荐前缀推导、自动回填与生成阻断原因**
- [ ] **Step 4: 保持 `sku-serial-rules` 加载失败时主流程仍可用**
- [ ] **Step 5: 重跑数码治理用例，确认推荐前缀与生成阻断全部转绿**

### Task 3: 运行期治理摘要与页头策略透出

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/skuSerials/SkuSerialsPage.tsx`

- [ ] **Step 1: 为批次页增加页头运行治理摘要**
- [ ] **Step 2: 为序列号页增加页头运行治理摘要**
- [ ] **Step 3: 保持摘要语义聚焦“前置校验”和“避免误操作”**
- [ ] **Step 4: 为摘要区域补稳定 `data-testid`，便于后续巡检与 E2E 扩展**
- [ ] **Step 5: 确保摘要不替代抽屉内细粒度治理提示**

### Task 4: Round 9 运行期治理 E2E 扩展

**Files:**
- Modify: `F:/erp/apps/erp-web/e2e/sku-runtime-governance.spec.ts`

- [ ] **Step 1: 把食品治理用例从“保存失败”升级为“保存前禁用 + 原因可见”**
- [ ] **Step 2: 把数码治理用例从“手填前缀后成功”升级为“自动建议前缀 + 按钮可用”**
- [ ] **Step 3: 优先使用 `data-testid` 和按钮禁用态，减少文案脆弱性**
- [ ] **Step 4: 跑整组 `sku-runtime-governance` fresh 验证**
- [ ] **Step 5: 补跑 `products.spec.ts` 回归，确认商品主档无连带回归**

### Task 5: 文档、任务卡与执行记录

**Files:**
- Create: `F:/erp/docs/superpowers/plans/2026-04-15-industrial-erp-95-plus-round9-runtime-governance-ux-plan.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r9-01.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r9-02.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r9-03.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r9-04.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r9-05.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-round9-exec-log.md`

- [ ] **Step 1: 记录本轮聚焦点、并行接管策略和验证命令**
- [ ] **Step 2: 记录 E2E 红灯到绿灯的事实证据**
- [ ] **Step 3: 记录 fresh 回归结果与完成度变化**
- [ ] **Step 4: 评估 Round 9 完成后仍待补的链路缺口**
- [ ] **Step 5: 为 Round 10 留出清晰的继续推进方向**
