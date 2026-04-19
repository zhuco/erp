# Industrial ERP 95%+ Round 7 Product Governance Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Round 6 已完成商品行业化主档组件化的基础上，继续把食品、鞋服、数码三类商品推进到“可保存回显、可再次编辑回填、可通过规则测试验证”的工业级治理状态。

**Architecture:** 当前项目整体工业级电商 ERP 完成度可评估为 `96%` 左右，订单、仓储、售后、财务等主链已具备较强 fresh 证据，商品主档已经进入最后一段高性价比补缺阶段。Round 7 不扩散到 OMS/WMS/财务主链，而是继续围绕 `ProductsPage.tsx`、`productIndustry.ts`、`products.spec.ts` 与商品 mock 做小范围、高收益闭环：补数码模板保存回显、增强鞋服规格生成器、强化食品效期策略表达、补行业规则单测，并把下一阶段运行期治理挂点文档化。

**Tech Stack:** React、TypeScript、Ant Design、React Query、Vitest、Playwright、Mock API

---

## 当前完成度重新评估

**整体完成度判断：** `96%` 左右

**已经接近工业级闭环的域：**
- OMS 审单、异常单、履约链路已有 fresh E2E 证据。
- WMS 波次、出库、发运主链已有 fresh E2E 证据。
- 售后、客服、财务经营前台能力已具备较完整的页面与验证闭环。
- 商品主档基础字段、行业模板切换、鞋服规格批量生成、食品效期摘要已具备第一轮落地证据。

**当前最需要继续补强的功能缺口：**
- 数码商品仍缺“序列号策略保存后回显 + 再次打开编辑回填”的 fresh 证据。
- 鞋服规格生成器仍偏最小可用，缺“保留已有规格、智能去重、尽量继承已有扩展字段”的治理能力。
- 食品模板已能配置，但摘要表达还可以更突出生产日期、临期预警、禁售提前等主档语义。
- 行业规则目前主要由 E2E 间接覆盖，缺少 `productIndustry.ts` 层面的单元测试。
- 运行期治理（食品批次流转、数码序列号出入库强校验）还未下钻到后续链路，需要挂点文档防止后续扩散失控。

## Round 7 五条主线

### Task 1: 数码模板保存回显与编辑回填闭环
**价值：** 让数码商品的序列号治理从“字段存在”提升到“保存可见、再次编辑可回填、后续运行期可挂接”。

### Task 2: 鞋服规格生成器合并去重增强
**价值：** 让鞋服主档维护从“覆盖式批量生成”升级为“合并式批量生成”，减少人工重复录入与误覆盖。

### Task 3: 食品效期策略表达增强
**价值：** 在商品详情摘要与模板提示中明确生产日期、临期预警、禁售提前三类食品治理语义，为后续批次治理打基础。

### Task 4: 行业规则单元测试补齐
**价值：** 为 `productIndustry.ts` 建立快速反馈层，降低未来继续补字段或调整摘要时的回归风险。

### Task 5: 运行期治理挂点文档与执行记录
**价值：** 把“食品批次流转”和“数码序列号出入库强校验”的下一阶段接入点文档化，避免后续开发再次散点推进。

## 并行执行原则

1. 子任务优先各自拥有清晰文件边界，避免多人同时改 `ProductsPage.tsx` 和 `products.spec.ts`。
2. 若子线再次出现 `502 Bad Gateway`，立即停止重试，由主线程接管，不做机械空转。
3. 所有新增行为先补失败测试，再做最小实现，最后执行 fresh 验证。
4. 本轮仍不触碰 `router.tsx`、订单、WMS、财务主链文件，保持变更聚焦。

### Task 1: 数码模板保存回显与编辑回填闭环

**Files:**
- Modify: `F:/erp/apps/erp-web/e2e/products.spec.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/components/ProductIndustrySummary.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/productIndustry.ts`

- [ ] **Step 1: 先写数码模板保存回显与再次编辑回填的失败 E2E 用例**
- [ ] **Step 2: 运行 `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "数码模板"`，确认红灯**
- [ ] **Step 3: 只补最小页面/摘要/回填实现使其转绿**
- [ ] **Step 4: 再次运行同一条 E2E，确认保存、回显、回填全部通过**
- [ ] **Step 5: 记录仍未下钻到出入库强校验的边界**

### Task 2: 鞋服规格生成器合并去重增强

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/products/productIndustry.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/components/ProductApparelSpecComposer.tsx`
- Test: `F:/erp/apps/erp-web/src/pages/products/productIndustry.test.ts`

- [ ] **Step 1: 先写规格合并去重的失败单元测试**
- [ ] **Step 2: 运行 `pnpm --filter erp-web test:unit -- productIndustry.test.ts`，确认红灯**
- [ ] **Step 3: 实现保留已有规格、按颜色/尺码去重、尽量继承原扩展字段的最小逻辑**
- [ ] **Step 4: 再跑单测转绿**
- [ ] **Step 5: 记录仍未覆盖的高级矩阵能力边界**

### Task 3: 食品效期策略表达增强

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/products/productIndustry.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/components/ProductIndustrySections.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/components/ProductIndustrySummary.tsx`
- Modify: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 先补食品策略摘要与提示的失败断言**
- [ ] **Step 2: 运行食品相关用例确认红灯**
- [ ] **Step 3: 用最小改动强化生产日期、临期预警、禁售提前的表达**
- [ ] **Step 4: 重跑食品相关 E2E 转绿**
- [ ] **Step 5: 记录仍未下钻到批次台账的边界**

### Task 4: 行业规则单元测试补齐

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/products/productIndustry.test.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/productIndustry.ts`

- [ ] **Step 1: 先为食品摘要、数码摘要、鞋服规格合并生成测试样例**
- [ ] **Step 2: 运行 `pnpm --filter erp-web test:unit -- productIndustry.test.ts`，确认失败原因正确**
- [ ] **Step 3: 补最小实现，不顺手扩功能**
- [ ] **Step 4: 再跑单测确认通过**
- [ ] **Step 5: 保持测试聚焦在规则层，不扩散到页面快照**

### Task 5: 运行期治理挂点文档与执行记录

**Files:**
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-round7-exec-log.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r7-01.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r7-02.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r7-03.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r7-04.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-15/erp95-r7-05.md`

- [ ] **Step 1: 写清本轮目标、文件边界、验证命令**
- [ ] **Step 2: 在执行日志中记录子线失败即主线程接管的策略**
- [ ] **Step 3: 明确食品批次流转与数码序列号运行期治理的接入点**
- [ ] **Step 4: 随执行过程更新本轮进展与 fresh 证据**
- [ ] **Step 5: 输出本轮后完成度与剩余缺口评估**
