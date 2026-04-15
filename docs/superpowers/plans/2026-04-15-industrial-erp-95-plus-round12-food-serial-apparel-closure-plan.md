# 工业级电商 ERP 95%+ Round 12 食品批次、序列号、鞋服映射闭环计划

> 面向 Codex / 多子任务执行：本轮仍以现有主计划为准，聚焦 mock API、前端页面、WMS 出库流程和自动化测试的闭环验证。

## 目标

- 将食品行业的 `production_date` 从“推算展示”推进到“真字段管理 + 冻结联动 + 出库阻断”。
- 将数码行业的序列号治理从“生成与可见”推进到“唯一校验 + 打包绑定 + 发货闭环”。
- 将鞋服行业的颜色 / 尺码治理从“摘要展示”推进到“模板库 + 字典治理 + 渠道映射联动”。
- 通过 fresh 单测与 E2E 证明本轮改造已进入工业级 ERP 可验证状态。

## 当前判断

- 项目整体完成度基线约为 `98.7%`。
- 本轮不是大而全扩展，而是补齐 3 条高价值短板，让行业治理从“页面可见”升级到“流程可阻断、规则可复用、测试可回归”。
- 由于当前 Rust 真后端链路未完全下沉，本轮优先把 `mock + API + 页面 + WMS` 做成强闭环，避免空转。

## Round 12 五个子任务

### 子任务 1：食品批次 production_date 真字段化

- 范围
  - `apps/erp-web/src/api/skuBatches.ts`
  - `apps/erp-web/src/api/mocks/data.ts`
  - `apps/erp-web/src/api/mocks/skus.ts`
  - `apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`
- 目标
  - 本地类型扩展 `production_date?: string | null`
  - 食品批次样本补生产日期
  - 新建 / 编辑批次时持久化生产日期
  - 批次治理摘要和列表优先读取真字段

### 子任务 2：库存冻结 mock + 禁售批次出库阻断

- 范围
  - `apps/erp-web/src/api/inventoryFreezes.ts`
  - `apps/erp-web/src/api/mocks/data.ts`
  - `apps/erp-web/src/api/mocks/wms.ts`
  - `apps/erp-web/src/pages/wms/WmsPackPage.tsx`
- 目标
  - `inventory/freezes` 在 mock 下可查询、创建、取消
  - 禁售食品批次自动存在冻结记录
  - WMS 打包遇到禁售批次直接阻断
  - 页面提示出阻断原因与治理动作

### 子任务 3：数码序列号唯一校验与出库绑定闭环

- 范围
  - `apps/erp-web/src/api/mocks/skus.ts`
  - `apps/erp-web/src/api/mocks/wms.ts`
  - `apps/erp-web/src/pages/wms/WmsPackPage.tsx`
  - `apps/erp-web/src/pages/skuSerials/SkuSerialsPage.tsx`
- 目标
  - 生成序列号时校验重复
  - 打包时校验序列号存在、SKU 匹配、未被其他出库单占用
  - 打包成功后绑定 `OUTBOUND`
  - 发货成功后状态进入 `SHIPPED`

### 子任务 4：鞋服模板库 + 字典治理 + 渠道映射联动

- 范围
  - `apps/erp-web/src/api/mappingRules.ts`
  - `apps/erp-web/src/api/mocks/data.ts`
  - `apps/erp-web/src/api/mocks/shopProducts.ts`
  - `apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`
- 目标
  - mock 下支持映射规则查询 / 新建 / 更新
  - 规则中沉淀鞋服模板、颜色字典、尺码字典
  - 自动匹配根据 `spec_text` 解析颜色 / 尺码
  - 冲突原因从“无候选 SKU”细化到“颜色字典未命中 / 尺码字典未命中 / 模板未命中”

### 子任务 5：Round 12 测试与评估

- 范围
  - `apps/erp-web/src/api/mocks/*.test.ts`
  - `apps/erp-web/e2e/sku-runtime-governance.spec.ts`
  - `apps/erp-web/e2e/shop-product-match-governance.spec.ts`
  - `apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`
  - `docs/ops/tasks/2026-04-15/erp95-round12-exec-log.md`
- 目标
  - 先补失败测试，再做实现
  - fresh 跑通 Vitest + Playwright
  - 回写本轮完成度与工业级差距

## 实施顺序

1. 先写 Round 12 文档、任务卡、执行日志。
2. 先补失败单测，卡住食品冻结阻断、序列号绑定、鞋服模板匹配。
3. 再补 WMS / mock / 页面实现。
4. 最后跑 fresh 自动化验证并更新日志。

## 验证命令

```powershell
pnpm --filter erp-web exec vitest run src/api/mocks/skus.runtime-governance.test.ts
pnpm --filter erp-web exec vitest run src/api/mocks/wms.runtime-governance.test.ts
pnpm --filter erp-web exec vitest run src/api/mocks/shopProducts.match-governance.test.ts
pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium
pnpm --filter erp-web exec playwright test e2e/shop-product-match-governance.spec.ts --project=chromium
pnpm --filter erp-web exec playwright test e2e/outbound-fulfillment-flow.spec.ts --project=chromium
```

## 交付标准

- 食品批次页面能维护真生产日期，禁售批次存在冻结并阻断出库。
- 数码序列号不会重复生成，且完成出库绑定和发货状态流转。
- 鞋服自动匹配不再只依赖人工兜底，能体现模板和字典治理价值。
- 所有本轮结论必须附 fresh 测试证据，不能只给静态代码说明。
