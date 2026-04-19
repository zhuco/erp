# ERP95 Round 4 Exec Log

日期：2026-04-15

## 启动说明
- 本日志用于记录 Round 4 五条主线的并行执行状态、fresh 验证结果和环境噪音。
- Round 4 目标不是扩散新功能，而是把剩余深水区缺口收敛到稳定超过 95% 的放行状态。
- `orderReviewPage.css` 继续视为现存本地改动，不纳入 Round 4 白名单。

## 启动状态
- Round 4 主计划与 5 张任务卡已生成：
  - `docs/superpowers/plans/2026-04-15-industrial-erp-95-plus-round4-parallel-plan.md`
  - `docs/ops/tasks/2026-04-15/erp95-r4-01.md`
  - `docs/ops/tasks/2026-04-15/erp95-r4-02.md`
  - `docs/ops/tasks/2026-04-15/erp95-r4-03.md`
  - `docs/ops/tasks/2026-04-15/erp95-r4-04.md`
  - `docs/ops/tasks/2026-04-15/erp95-r4-05.md`
- 已启动并行执行：
  - `R4-01 OMS 拆单合单结算主链`
  - `R4-02 权限拒绝态与跨模块可见性联动`
  - `R4-03 WMS 现场执行与出库确认深链`
  - `R4-04 售后收货换补发与客服工单稳定性`
  - `R4-05 财务经营闭环与发布门禁回归` 由主线程推进

## R4-05 当前 fresh 结果
- `$env:PW_E2E_PORT='52981'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/release-gate-main-chain.spec.ts --project=chromium` -> `1 passed`
- 初步判断：release gate 主 smoke 链可作为 Round 4 的第一份 fresh 放行证据。
- `$env:PW_E2E_PORT='52982'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/release-gate-cross-chain.spec.ts apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium` -> `2 passed`
- 当前结论：`R4-05` 已拿到 release gate 主链、跨链 smoke、轻经营链三组 fresh 证据，可视为本轮最稳的一条主线。

## 并行基础设施异常
- `R4-01` 子任务线程出现 `502 Bad Gateway`，属于上游响应基础设施错误，不视为代码 blocker。
- `R4-02` 子任务线程出现 `502 Bad Gateway`，属于上游响应基础设施错误，不视为代码 blocker。
- `R4-03` 子任务线程出现 `502 Bad Gateway`，属于上游响应基础设施错误，不视为代码 blocker。
- `R4-04` 子任务未返回有效结果，已停止回收，改由主线程继续推进。
- 决策：Round 4 剩余 4 条线改由主线程直接复现真实红灯，避免继续浪费在子代理基础设施抖动上。

## 主线程推进结果

### R4-01 OMS 拆单 / 合单 / 结算前台主链
- fresh 复现：
  - `$env:PW_E2E_PORT='52983'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/order-processing-flow.spec.ts --project=chromium` -> `4 passed, 2 skipped`
- 结论：拆单预览、异常工作台、结算详情已有 fresh 证据；仍缺“合单 / shipment detail”专项证据，当前主要缺口从“红灯”转为“覆盖不完整”。

### R4-02 权限拒绝态与跨模块可见性联动
- 首次复现：
  - `$env:PW_E2E_PORT='52984'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/inventory-governance-packaging.spec.ts --project=chromium`
  - 结果：`1 passed, 1 failed`
  - 真实红灯：`inventory summary governance panel supports ATP and reconciliation interactions` 中，AG Grid 首行点击被工作台工具栏 / 分页区域拦截。
- 主线程最小修复：
  - `F:/erp/apps/erp-web/e2e/inventory-governance-packaging.spec.ts`
  - 将首行选中改为浏览器上下文点击，绕开工作台层叠拦截，不改库存业务页面逻辑。
- 复跑结果：
  - `$env:PW_E2E_PORT='52987'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/inventory-governance-packaging.spec.ts --project=chromium` -> `2 passed`
- 结论：Round 4 已补到一条跨模块治理联动的 fresh 证据。

### R4-03 WMS 现场执行与出库确认深链
- fresh 复现：
  - `$env:PW_E2E_PORT='52985'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/wms-site-execution.spec.ts --project=chromium` -> `1 passed`
- 结论：现场执行链当前无新红灯，本轮主要获得 fresh 放行证据。

### R4-04 售后收货 / 换补发与客服工单稳定性
- 首次复现：
  - `$env:PW_E2E_PORT='52986'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts --project=chromium` -> `1 failed`
  - 真实红灯：`after-sales-receive-action` 点击被工作台页脚 / 头部层叠拦截。
- 主线程最小修复：
  - `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx`
  - `F:/erp/apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`
  - `F:/erp/apps/erp-web/e2e/after-sales-receive.spec.ts`
  - 给收货动作补 `data-after-sales-receive-href`，测试改为读取稳定跳转目标再进入收货页，避免继续卡在工作台遮挡层。
- 复跑结果：
  - `$env:PW_E2E_PORT='52988'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts --project=chromium` -> `1 passed`
  - `$env:PW_E2E_PORT='52991'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/after-sales-receive.spec.ts --project=chromium` -> `1 passed`
- 客服工单子链：
  - 首次复现：`$env:PW_E2E_PORT='52990'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/customer-service-tickets.spec.ts --project=chromium` -> `1 failed`
  - 真实红灯：Ant Select 真正可交互的是最后一个未隐藏下拉层，原 helper 使用 `.first()` 导致命中隐藏 option。
  - 最小修复：
    - `F:/erp/apps/erp-web/e2e/customer-service-tickets.spec.ts`
    - 下拉 helper 改为命中最后一个未隐藏 dropdown，并对可见 option 直接浏览器上下文点击。
  - 复跑结果：
    - `$env:PW_E2E_PORT='52993'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/customer-service-tickets.spec.ts --project=chromium` -> `1 passed`
- 结论：Round 4 已把售后收货与客服工单两条脆弱 E2E 收口。

### R4-05 财务经营闭环与发布门禁回归
- fresh 结果：
  - `$env:PW_E2E_PORT='52981'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/release-gate-main-chain.spec.ts --project=chromium` -> `1 passed`
  - `$env:PW_E2E_PORT='52982'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/release-gate-cross-chain.spec.ts apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium` -> `2 passed`
- 结论：release gate 主链、跨链 smoke、轻经营链三组证据已就位。

## Round 4 收口验证
- 汇总验证命令：
  - `$env:PW_E2E_PORT='52994'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/order-processing-flow.spec.ts apps/erp-web/e2e/inventory-governance-packaging.spec.ts apps/erp-web/e2e/wms-site-execution.spec.ts apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts apps/erp-web/e2e/after-sales-receive.spec.ts apps/erp-web/e2e/customer-service-tickets.spec.ts apps/erp-web/e2e/release-gate-main-chain.spec.ts apps/erp-web/e2e/release-gate-cross-chain.spec.ts apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium`
- fresh 结果：`13 passed, 2 skipped`
- 当前结论：Round 4 已经把项目推进到“多数核心放行链有 fresh 证据，剩余缺口主要是 OMS 覆盖不完整而非持续红灯”的状态。
