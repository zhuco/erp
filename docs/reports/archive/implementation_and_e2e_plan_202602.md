# ERP 营销模块实现补全与 E2E 测试计划（2026-02）

## 1. 背景与目标

在不依赖电商平台联调的前提下，本轮聚焦三件事：

1. 补齐营销模块接口契约（OpenAPI），降低前后端认知偏差。
2. 补齐前端 Mock 能力，保证核心流程可开发、可回归、可演示。
3. 建立可执行的自动化测试清单（后端权限 + 前端 E2E + 前端单测）。

## 2. 两周可执行排期（按模块 / 人天）

> 周期：2 周（10 个工作日）
>  
> 角色按企业 ERP 常见配置：后端 1 人、前端 1 人、测试 1 人（可并行）

| 模块 | 主要工作 | 后端人天 | 前端人天 | 测试人天 | 小计 |
|---|---|---:|---:|---:|---:|
| 营销 API 契约 | OpenAPI 路径 + Schema 补齐，和路由/权限对齐 | 2.0 | 0.5 | 0.5 | 3.0 |
| 营销前端 Mock | 六大子域 CRUD Mock + API 接入 | 0.0 | 2.5 | 0.5 | 3.0 |
| 营销 UI 交互稳态 | 关键按钮/弹窗/表格刷新稳定化 | 0.0 | 1.5 | 0.5 | 2.0 |
| 后端权限回归 | 403/租户隔离场景补测 | 1.5 | 0.0 | 0.5 | 2.0 |
| 前端单测补齐 | Mock 分支 API 行为单测 | 0.0 | 1.0 | 0.5 | 1.5 |
| 前端 E2E 补测 | 促销/价表/赠品 端到端 CRUD 主链路 | 0.0 | 1.0 | 2.0 | 3.0 |
| CI 门禁与回归 | typecheck / unit / e2e / cargo test 收敛 | 0.5 | 0.5 | 1.0 | 2.0 |

**合计：16.5 人天**（可在 2 周内通过并行完成）

## 3. 两周日历化安排（建议）

### 第 1 周

- Day 1-2：营销 OpenAPI 契约补齐、后端权限点核对。
- Day 2-4：前端营销 Mock CRUD 补齐并接入所有 marketing API。
- Day 4-5：补前端 API 单测，覆盖 mock 分支核心行为。

### 第 2 周

- Day 6-8：E2E 用例落地（促销、价表+明细、赠品策略）。
- Day 8-9：后端权限测试补齐（无权限/租户隔离）。
- Day 10：统一回归，修复 flaky 与选择器稳定性，整理验收报告。

## 4. 本轮实现清单（已落地）

### 4.1 营销 Mock 与 API 接入

- 新增 `apps/erp-web/src/api/mocks/marketing.ts`
  - 覆盖价表、促销、赠品、优惠券、费用、复盘六类数据的内存 CRUD。
  - 支持分页、关键字过滤、状态过滤等基础查询能力。
- 已接入 Mock 的前端 API 文件：
  - `apps/erp-web/src/api/marketingPriceLists.ts`
  - `apps/erp-web/src/api/marketingPromotions.ts`
  - `apps/erp-web/src/api/marketingGifts.ts`
  - `apps/erp-web/src/api/marketingCoupons.ts`
  - `apps/erp-web/src/api/marketingCosts.ts`
  - `apps/erp-web/src/api/marketingPostmortems.ts`
  - `apps/erp-web/src/api/mocks/handlers.ts`

### 4.2 OpenAPI 契约补齐

- 已补齐营销路由（共 14 个）：
  - 价表、价表明细、促销、赠品、优惠券、费用、复盘。
- 已补齐营销 Schema：
  - `MarketingPriceList*`、`MarketingPriceItem*`
  - `MarketingPromotion*`
  - `MarketingGiftPolicy*`
  - `MarketingCoupon*`
  - `MarketingCost*`
  - `MarketingPostmortem*`
- 权限别名对齐：
  - `marketing_prices.*`
  - `marketing_promotions.*`
  - `marketing_gifts.*`
  - `marketing_coupons.*`
  - `marketing_costs.*`
  - `marketing_postmortems.*`

### 4.3 自动化测试补齐

- 新增后端权限测试：
  - `crates/erp-api/tests/marketing_permissions.rs`
  - 覆盖无权限与租户不匹配 403 场景。
- 新增前端 E2E：
  - `apps/erp-web/e2e/marketing-crud.spec.ts`
  - 覆盖促销（新建/编辑）、价表（新建/详情/新增明细）、赠品（新建/删除）。

## 5. 建议继续补齐的测试清单（下一步）

### P0（必须）

- 前端 E2E：优惠券、促销费用、活动复盘各补 1 条 CRUD 主路径。
- 后端权限：对 `costs` 与 `postmortems` 再补租户隔离测试。
- 前端单测：营销 API 的异常分支（非法 ID、删除不存在记录）。

### P1（建议）

- 前端 E2E：筛选条件回归（状态、关键字、分页）。
- 后端接口测试：参数边界（limit clamp、空字符串归一化）。
- OpenAPI 一致性检查：生成客户端并做 schema smoke test。

## 6. 验收门禁（建议）

### 前端

- `pnpm --filter erp-web typecheck`
- `pnpm --filter erp-web test:unit`
- `pnpm --filter erp-web test:e2e -- marketing-crud.spec.ts`

### 后端

- `cargo test -p erp-api marketing_permissions -- --nocapture`

## 7. 风险与治理

- 风险：E2E 选择器对 UI 文案依赖高，易受文案改动影响。
  - 治理：优先补 `data-testid`，关键交互用结构化定位。
- 风险：Mock 与真实后端可能出现默认值偏差。
  - 治理：每次后端字段变更同步更新 Mock 与 OpenAPI。
- 风险：营销 JSON 字段（scope/rule/metrics）结构非强约束。
  - 治理：逐步引入领域 JSON Schema 校验与前端编辑器约束。

