# 商品行业化主档实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不重构后端 schema 的前提下，为商品资料页补齐食品、鞋服、数码三类行业化主档配置，并完成可保存、可回填、可展示、可验证的闭环。

**Architecture:** 继续复用商品顶层字段承接批次与序列号治理能力，行业模板语义统一写入 `attrs`，鞋服规格增强继续沿用 `specs`。前端集中改造 `ProductsPage.tsx`，通过 `attrsSnapshotRef` 保持未知扩展字段不丢失，并用 Playwright 对行业模板联动做回归保护。

**Tech Stack:** React、TypeScript、Ant Design、React Query、Playwright、Mock API

---

### Task 1: 行业模板与字段映射

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 写出失败用例**

在 `products.spec.ts` 中新增“商品弹窗支持行业模板切换”的用例，断言可见：

- 行业模板
- 食品效期管理字段
- 鞋服尺码体系字段
- 数码序列号规则字段

- [ ] **Step 2: 运行用例确认红灯**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium`

Expected: 新增断言失败，提示行业模板字段尚不存在。

- [ ] **Step 3: 最小实现行业模板表单**

在 `ProductsPage.tsx` 的基础信息表单中新增表单字段并完成回填与默认值：

- `industry_profile`
- `production_date_enabled`
- `shelf_life_unit`
- `near_expiry_warning_days`
- `variant_mode`
- `apparel_size_system`
- `serial_tracking_mode`
- `serial_outbound_required`
- `serial_inbound_unique_required`

- [ ] **Step 4: 运行用例确认转绿**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "行业模板"`

Expected: 新增行业模板联动用例通过。

- [ ] **Step 5: 本地检查**

确认新字段默认值不破坏现有新建、编辑、复制流程。

### Task 2: 食品效期策略落地

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 写出失败用例**

新增“食品模板显示生产日期与效期字段”的用例，至少断言：

- 生产日期管理
- 保质期单位
- 临期预警天数
- 禁售提前天数

- [ ] **Step 2: 运行用例确认红灯**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "食品模板"`

Expected: 字段缺失导致断言失败。

- [ ] **Step 3: 最小实现食品区块**

在商品基础信息中新增食品区块，并将以下字段写入 `attrs`：

- `production_date_enabled`
- `shelf_life_unit`
- `near_expiry_warning_days`

继续复用顶层字段：

- `batch_enabled`
- `shelf_life_days`
- `forbid_sale_days`

- [ ] **Step 4: 运行用例确认转绿**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "食品模板"`

Expected: 食品字段联动通过。

- [ ] **Step 5: 详情摘要补齐**

在详情面板中增加食品主档摘要，确保业务人员可读。

### Task 3: 鞋服规格矩阵增强

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 写出失败用例**

新增“鞋服模板强化颜色尺码矩阵”的用例，断言：

- 尺码体系字段出现
- 规格区域出现颜色/尺码提示
- 新增规格后可见颜色与尺码输入框

- [ ] **Step 2: 运行用例确认红灯**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "鞋服模板"`

Expected: 鞋服模板字段或提示缺失。

- [ ] **Step 3: 最小实现鞋服区块**

在 `ProductsPage.tsx` 中：

- 增加 `variant_mode` 与 `apparel_size_system`
- 对规格区增加鞋服语义提示
- 为 `spec_name` 增加颜色/尺码自动预览提示

- [ ] **Step 4: 运行用例确认转绿**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "鞋服模板"`

Expected: 鞋服模板联动通过。

- [ ] **Step 5: 回填与保存验证**

确认编辑已有鞋服商品时，规格与行业字段能同时回填。

### Task 4: 数码序列号规则增强

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/products.spec.ts`

- [ ] **Step 1: 写出失败用例**

新增“数码模板显示序列号规则”的用例，断言：

- 序列号追踪模式
- 出库必须绑定序列号
- 入库序列号唯一校验
- 平台回传序列号

- [ ] **Step 2: 运行用例确认红灯**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "数码模板"`

Expected: 规则字段尚未渲染。

- [ ] **Step 3: 最小实现数码区块**

在 `ProductsPage.tsx` 中：

- 复用 `serial_enabled`
- 新增 `serial_tracking_mode`
- 新增 `serial_outbound_required`
- 新增 `serial_inbound_unique_required`
- 与 `serial_push_platform` 联动禁用态

- [ ] **Step 4: 运行用例确认转绿**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium --grep "数码模板"`

Expected: 数码模板联动通过。

- [ ] **Step 5: 详情摘要补齐**

在详情面板中增加数码主档摘要，方便运营与售后查看。

### Task 5: 整体验证与缺口复盘

**Files:**
- Modify: `F:/erp/apps/erp-web/e2e/products.spec.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`

- [ ] **Step 1: 运行商品页整组 E2E**

Run: `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium`

Expected: 商品页原有用例与新增行业化用例全部通过。

- [ ] **Step 2: 检查是否引入现有行为回归**

重点回归：

- 新建商品弹窗
- 搜索与重置
- 快捷筛选设置
- 商品详情右侧面板

- [ ] **Step 3: 必要时微调文案与默认值**

仅修正本轮引入的回归，不扩大改动范围。

- [ ] **Step 4: 记录剩余功能缺口**

输出下一轮建议补齐项：

- 食品批次生产日期流转
- 鞋服批量规格组合器
- 数码出入库序列号强校验

- [ ] **Step 5: 准备收尾说明**

整理本轮变更文件、验证命令、仍待补齐范围，确保可以继续进入下一轮推进。
