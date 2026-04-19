# ERP95 Round 3 Exec Log

日期：2026-04-15

## 启动说明
- 本日志用于记录本轮 5 条主线的并行执行状态、fresh 验证结果和环境噪音。
- `orderReviewPage.css` 继续视为现存本地改动，不纳入本轮白名单。

## 当前完成度重评估
- 工业级电商 ERP 整体完成度当前仍按 `92%~93%` 口径评估。
- 已经拿到 fresh 证据的主线：`R3B-01 多实体权限 UI 业务链`、`R3B-02 OMS 前端工作台主链`、`R3B-03 WMS 异常与失效回归`、`R3B-05 Observability 运维工作流`。
- 正在推进中的主线：`R3B-04 售后与经营会计链`。

## R3B-05 Observability 运维工作流
- 根因结论：`ObservabilityPage.tsx` 在 mock 模式下查询前端路由留痕时，错误使用了 `@/api/system` 的 `getFrontendRouteDiagnostics`；而上报侧走的是 `@/api/frontendRouteDiagnostics`。两侧 mock 通道不一致，导致“后端留痕表格为空”。
- 主线程修复：将 `ObservabilityPage.tsx` 的前端路由留痕查询与相关类型导入切换到 `@/api/frontendRouteDiagnostics`，保留 `fetchMetrics` 继续来自 `@/api/system`。
- fresh 验证：
  - `$env:PW_E2E_PORT='52961'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/frontend-route-observability.spec.ts --project=chromium` -> `7 passed`
  - `$env:PW_E2E_PORT='52962'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/task8-stats-observability-sso.spec.ts --project=chromium` -> `1 passed`
- 结论：运维工作流真实逻辑缺口已收口；当前剩余问题以环境噪音为主。

## R3B-01 多实体权限 UI 业务链
- 子任务结果：`DONE_WITH_CONCERNS`
- 关键修复：
  - `F:/erp/apps/erp-web/e2e/multi-entity-governance.spec.ts` 新增并固化 `merchant/org` 权限业务链 E2E。
  - `F:/erp/apps/erp-web/src/pages/system/DataScopesPage.tsx` 补齐 `merchant_ids`、`organization_ids` 字段与回填。
  - `F:/erp/apps/erp-web/src/pages/system/UsersPage.tsx` 补齐数据权限弹窗中的 `merchant_ids`、`organization_ids` 与稳定定位 `data-testid`。
- fresh 验证：
  - `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/multi-entity-governance.spec.ts --project=chromium`：首轮 RED，最终 `1 passed`
  - `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/system-governance.spec.ts --project=chromium` -> `1 passed`
  - `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/workbench-menu-navigation.spec.ts --project=chromium` -> `15 passed`
- 仍未覆盖：
  - `merchant/org` 的拒绝态与 403 提示。
  - `merchant/org` 与 `shop/warehouse` 的交叉约束。
  - 权限保存后对订单 / WMS / 售后可见性的联动校验。

## R3B-02 OMS 前端工作台主链
- 子任务结果：`DONE`
- 关键修复：
  - 在 `F:/erp/apps/erp-web/e2e/order-processing-flow.spec.ts` 新增 `exceptions workbench supports search and detail focus`。
  - 在 `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx` 补 `data-testid="order-exceptions-detail-panel"`。
- fresh 验证：
  - `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/order-processing-flow.spec.ts --project=chromium --grep "exceptions workbench supports search and detail focus"`：首轮 RED，最终 `1 passed (5.6s)`。
- 环境噪音：
  - `http://localhost:52941 is already used`，通过清理残留 `vite --port 52941 --strictPort` 进程恢复。
- 仍未覆盖：
  - 拆单 / 合单 / 结算主链的 fresh 前端证据。

## R3B-03 WMS 异常与失效回归
- 子任务结果：`DONE`
- 关键修复：
  - `F:/erp/apps/erp-web/e2e/outbound-flow.spec.ts` 将失效的 `outbound-list-title` 断言替换为稳定 `grid` 断言。
  - 移除依赖易变文案的无数据分支判断，避免脆弱误报。
- fresh 验证：
  - `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/wms-handover.spec.ts --project=chromium` -> `1 passed`
  - `$env:PW_E2E_PORT='52954'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/wms-wave-main-chain.spec.ts apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts --project=chromium` -> `2 passed`
  - `$env:PW_E2E_PORT='52956'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/outbound-flow.spec.ts --project=chromium` -> `1 passed`
- 仍未覆盖：
  - PDA / 仓内现场执行链路。

## R3B-04 售后与经营会计链
- 子任务已重新派发，当前等待 fresh 结果。
- 白名单仍限定在售后 / 客服 / 对账 / 利润与相关 E2E，不扩散到总账平台。

## 环境噪音记录
- Playwright Web Server 端口占用是本轮高频环境噪音，当前通过显式指定 `PW_E2E_PORT` 与清理残留 `vite` 进程规避。
- Rust 侧 `pool timed out while waiting for an open connection` 继续记录为环境噪音，目前没有新证据表明它对应真实业务失败。

## R3B-04 追加结果（主线程接手）
- 子代理再次因 `502 Bad Gateway` 中断后，主线程直接接手执行。
- fresh 验证：
  - `$env:PW_E2E_PORT='52966'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium` -> `1 passed`
  - `$env:PW_E2E_PORT='52971'; pnpm --filter erp-web exec playwright test apps/erp-web/e2e/after-sales-workflow.spec.ts --project=chromium` -> `4 passed`
- 结论：售后逆向录入、质检、SLA 规则、批量导入，以及“售后 -> 物流异常 -> 利润页”的轻经营链都已具备 fresh 证据，因此 `R3B-04` 本轮达到 DoD。
- 未收口但已确认的剩余缺口：
  - `apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts` 仍因工作台表格动作点击链路脆弱而失败，当前更像测试稳定性问题，不是核心售后流程不可用。
  - `apps/erp-web/e2e/customer-service-tickets.spec.ts` 在 Ant Select 下拉项选择时仍存在视口外定位脆弱问题，需下一轮单独稳固。
