# Industrial ERP 95%+ Round 6 Product Industry Closure Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于上一轮已经落地的商品行业模板能力，继续把食品、鞋服、数码三类商品主档推进到“可维护、可批量操作、可回显验证”的工业级收口状态。

**Architecture:** 当前项目整体工业级电商 ERP 完成度可评估在 `96%` 左右，OMS / WMS / 售后 / 财务主链已经具备较强 fresh 证据，当前高性价比缺口转移到了“商品行业化主档闭环不足”。Round 6 不再大范围扩模块，而是围绕 `ProductsPage.tsx` 的现有能力做组件化抽离、鞋服规格生成效率提升、详情摘要强化和 E2E 保存回显闭环补证，降低大文件风险并为后续食品批次流转、序列号强校验继续铺路。

**Tech Stack:** React、TypeScript、Ant Design、React Query、Playwright、Mock API

---

## 当前项目完成度复评估

**整体完成度判断：** `96%` 左右

**已基本达到工业级门槛的域：**
- OMS 订单处理、异常流、审核流、结算工作台已有 fresh 证据
- WMS 波次、出库执行、履约动作链已有 fresh 证据
- 售后收货、补发、客服工单、逆向与轻经营链条基本可放行
- 财务经营页已补到独立前台 E2E 证据
- 系统治理、多实体治理、发布门禁链条已不再是主要短板

**当前仍需继续完善的高收益缺口：**
- 商品行业化主档还缺“保存并回显”的完整闭环证据
- 食品模板已可配置，但缺更清晰的效期策略沉淀与后续对批次治理的挂点
- 鞋服模板已有颜色 / 尺码语义，但缺批量规格生成效率工具
- 数码模板已有序列号策略字段，但缺更清晰的规则摘要与治理提示
- `ProductsPage.tsx` 体量继续膨胀，后续开发与并行集成风险偏高

## Round 6 五条主线

### Task 1: 行业规则模型抽离
**价值：** 把行业字段解析、摘要生成、默认值策略从大页内联逻辑抽离出来，降低后续持续开发冲突。

### Task 2: 行业表单区块组件化
**价值：** 让食品 / 鞋服 / 数码区块从 `ProductsPage.tsx` 中分离，便于后续继续扩充校验和治理提示。

### Task 3: 鞋服规格矩阵快速生成
**价值：** 将鞋服从“可手工录入”推进到“可快速生成颜色 / 尺码规格矩阵”，提升主数据维护效率。

### Task 4: 行业摘要组件化与详情强化
**价值：** 把详情面板中的行业摘要变成独立可复用模块，为后续列表标签、审批对比、变更控制复用打基础。

### Task 5: 商品行业化保存回显闭环 E2E
**价值：** 让本轮能力从“UI 联动存在”升级为“新建、保存、回显、再次打开仍正确”的 fresh 证据。

## 并行执行原则

1. 子任务优先改自己的新增文件或白名单文件，避免重复改 `ProductsPage.tsx`
2. 新增组件与工具函数可以并行开发，主线程最后统一集成
3. 若子任务再次遇到 `502 Bad Gateway`，立即停掉该线并回切主线程，不做无效重试
4. 每条任务卡都要写明验证命令与未覆盖边界
5. 本轮不碰 `router.tsx`、订单/WMS/财务主链文件，避免扩散改动范围

### Task 1: 行业规则模型抽离

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/products/productIndustry.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 抽离行业字段类型、attrs 读写、摘要格式化函数**
- [ ] **Step 2: 在 `ProductsPage.tsx` 中改为引用抽离后的工具模块**
- [ ] **Step 3: 保证现有食品 / 鞋服 / 数码联动不回退**
- [ ] **Step 4: 运行 `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "行业模板"`**
- [ ] **Step 5: 记录仍未下钻到批次 / 序列号运行期校验的边界**

### Task 2: 行业表单区块组件化

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/products/components/ProductIndustrySections.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 把食品 / 鞋服 / 数码表单区块拆成独立组件**
- [ ] **Step 2: 通过 props 承接 `form`、`industryProfile`、`batchEnabled`、`serialEnabled`**
- [ ] **Step 3: 保持现有 testid 与文案兼容**
- [ ] **Step 4: 运行行业模板联动用例**
- [ ] **Step 5: 确认组件化后不引入视觉回归**

### Task 3: 鞋服规格矩阵快速生成

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/products/components/ProductApparelSpecComposer.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 设计最小可用的颜色 / 尺码批量输入器**
- [ ] **Step 2: 生成规格草稿并写回 `specs`**
- [ ] **Step 3: 自动生成 `spec_name` 预览，避免手工重复录入**
- [ ] **Step 4: 运行商品页 E2E，必要时补最小断言**
- [ ] **Step 5: 记录仍未实现的高级矩阵能力边界**

### Task 4: 行业摘要组件化与详情强化

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/products/components/ProductIndustrySummary.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 将详情面板中的行业摘要抽成独立组件**
- [ ] **Step 2: 保留食品效期、鞋服规格、数码序列号三个摘要块**
- [ ] **Step 3: 让摘要优先展示业务语义，而不是原始 attrs**
- [ ] **Step 4: 运行商品页 E2E，确认详情区域不报错**
- [ ] **Step 5: 记录后续可复用到变更控制页的接口**

### Task 5: 商品行业化保存回显闭环 E2E

**Files:**
- Modify: `F:/erp/apps/erp-web/e2e/products.spec.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/products.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/data.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`

- [ ] **Step 1: 先写“新建商品并保存后回显行业摘要”的失败用例**
- [ ] **Step 2: 跑红灯，确认失败点是保存链路或回显链路**
- [ ] **Step 3: 做最小 mock / 页面修复**
- [ ] **Step 4: fresh 重跑 `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium`**
- [ ] **Step 5: 写明仍未覆盖的食品批次流转和序列号出库强校验**
