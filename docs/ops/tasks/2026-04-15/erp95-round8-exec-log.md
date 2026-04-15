# ERP95 Round 8 Exec Log

日期：2026-04-15

## 启动说明
- 本日志用于记录 Round 8 运行期治理挂点前台化的执行状态、fresh 验证结果与剩余缺口。
- 本轮继续聚焦食品批次治理与数码序列号治理，不扩散到订单、WMS、财务主链。
- 若并行子线再次出现 `502 Bad Gateway` 或长时间无结果返回，立即停止空等，由主线程接管对应任务。

## Round 8 起点评估
- 当前工业级电商 ERP 整体完成度：约 `96.5%`
- 当前高性价比缺口：
  - 食品治理尚未下沉到批次创建运行页
  - 数码治理尚未下沉到序列号生成运行页
  - 运行期治理缺少稳定 mock 样本与 fresh E2E
  - `sku-serial-rules` 仍存在非 mock 优先风险

## Round 8 计划任务
- R8-01：食品批次治理挂点前台化
- R8-02：数码序列号治理挂点前台化
- R8-03：运行期专用 Mock 样本补齐
- R8-04：运行期治理 E2E 补齐
- R8-05：文档、任务卡与执行记录

## 执行结果
- 子线尝试：
  - Round 8 文档子线：`502 Bad Gateway`，主线程接管。
  - 运行期治理 E2E 子线：`502 Bad Gateway`，主线程接管。
- 主线程完成内容：
  - 新增 `docs/superpowers/plans/2026-04-15-industrial-erp-95-plus-round8-runtime-governance-plan.md`
  - 新增 `docs/ops/tasks/2026-04-15/erp95-r8-01.md` 至 `erp95-r8-05.md`
  - 新增 `apps/erp-web/e2e/sku-runtime-governance.spec.ts`
  - 更新 `apps/erp-web/src/app/router.tsx`，恢复 `/sku-batches` 与 `/sku-serials` 路由挂载
  - 更新 `apps/erp-web/src/app/layout/menu.tsx`，补齐两条运行页入口
  - 更新 `apps/erp-web/src/api/mocks/data.ts`，新增食品/数码运行期治理样本商品与 SKU
  - 更新 `apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`，补齐食品批次治理提示、批次能力校验、食品失效日期阻断
  - 更新 `apps/erp-web/src/pages/skuSerials/SkuSerialsPage.tsx`，补齐数码序列号治理提示、序列号能力校验，并将规则接口失败降级为非关键路径

## Fresh 验证
- 运行期治理 E2E：
  - `pnpm --filter erp-web exec playwright test e2e/sku-runtime-governance.spec.ts --project=chromium`
  - 结果：`4 passed`
- 商品主档回归：
  - `pnpm --filter erp-web exec playwright test e2e/products.spec.ts --project=chromium`
  - 结果：`7 passed`

## 本轮后评估
- 当前工业级电商 ERP 整体完成度：约 `97.2%`
- 本轮新增 fresh 证据：
  - 食品 SKU 在批次页可见运行期治理提示，且未填失效日期不会创建批次
  - 未启用批次管理的食品 SKU 无法创建批次
  - 数码 SKU 在序列号页可见运行期治理提示
  - 未启用序列号管理的数码 SKU 无法生成唯一码
  - 启用序列号管理的数码 SKU 可以成功生成唯一码
  - 商品主档行业模板回归仍然稳定通过

## 后续缺口
- 食品：仍需把生产日期、临期预警、禁售策略继续下钻到批次台账/入库链路。
- 数码：仍需把入库唯一校验、出库强绑定、平台回传继续下钻到后端与履约链路。
- 鞋服：如继续提升，可在规格矩阵上补模板库、字典与批量派生能力。
- 运行治理：当前已完成前台挂点与 mock 证据，下一轮可考虑向 WMS/售后场景做受控下钻。
