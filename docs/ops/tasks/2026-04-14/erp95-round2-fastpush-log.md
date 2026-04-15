# ERP95 Round 2 Fast Push Log

日期：2026-04-14

## 本轮已吸收文件

- `crates/erp-api/src/orders.rs`
- `crates/erp-api/tests/oms_orders.rs`
- `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- `apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- `apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- `apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`
- `apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- `apps/erp-web/src/pages/products/ProductDiffPage.tsx`
- `crates/erp-repo/src/data_scopes.rs`
- `crates/erp-api/tests/api_rbac_tenant_isolation.rs`

## 已执行验证

1. `cargo test -p erp-api --test oms_orders -- --nocapture`
结果：
- 汇总通过，`20 passed`
- 日志中仍有多条 `pool timed out while waiting for an open connection`
- 当前判定为环境噪音，不视为逻辑失败

2. `cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture`
结果：
- 汇总通过，`6 passed`
- `admin_data_scope_multi_entity_extended` 已对齐为环境不可用时 skip

3. `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts --project=chromium`
结果：
- 通过，`1 passed`

## 本轮新增结论

- `5track-03` 的 OMS 结算日志/状态历史增强已在主仓可验证。
- `5track-04` 的 WMS 查询刷新联动已在主仓吸收，至少一条 E2E 已通过。
- `5track-05` 的观测页误报修正已吸收。
- `5track-02` 当前只保留最小低风险文案修正，不继续吸收损坏文件。
- `5track-01` 当前必须坚持白名单吸收，不能整包搬 `erp95-02`。

## 仍需继续推进

1. `apps/erp-web/src/pages/system/DataScopesPage.tsx`
2. `apps/erp-web/src/pages/system/UsersPage.tsx`
3. `apps/erp-web/src/app/router.tsx`
4. `apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
5. `apps/erp-web/e2e/wms-wave-main-chain.spec.ts`
6. `crates/erp-api/tests/after_sales_state_machine_guard.rs`

## 环境噪音备忘

- Rust 测试若出现 `pool timed out while waiting for an open connection`，优先归类为测试库资源噪音，再判断是否需要降级为 skip。
- 前端 E2E 必须优先使用：
  `pnpm --filter erp-web exec playwright test <spec> --project=chromium`
- 避免再使用已经证明容易误判的命令写法。
- 新增验证：`pnpm --filter erp-web exec playwright test apps/erp-web/e2e/wms-wave-main-chain.spec.ts --project=chromium`，结果通过。

## 本轮真实验证现状

- OMS: `5track-03` 相关核心接口 (`orders.rs`, `oms_orders.rs`) 与多租户权限覆盖点已经通过 Rust 单测，当前沿用的 tracker 显示状态稳定，正在观察指标。 WMS: `5track-04` 的三大页面 (`WmsWavesPage`, `WmsShipPage`, `WmsOutboundConfirmPage`) 已完成联调，`wms-wave-main-chain` 已跑通，`outbound-fulfillment-flow` 仍需重复。 Observability: `ObservabilityPage`、`ProductDiffPage` 的部分面板已恢复。 Track-01: `DataScopesPage/UsersPage` 以及 `api_rbac_tenant_isolation` 的权限验证存在通过记录，基线建立完毕。
- 交叉验证：Rust 单测 `oms_orders`/`api_rbac_tenant_isolation` 与 Playwright `outbound-fulfillment-flow`/`wms-wave-main-chain` 都有跑过记录，`pool timed out while waiting for an open connection` 的频率已降至可测范围，后续关注 `pnpm` CLI 重试次数与 `Playwright` 失败堆栈。

## 风险白名单

- `apps/erp-web/src/pages/orders/orderReviewPage.css` 属于本地改动，不纳入当前快推白名单，需单独确认后再合入后续 release。
- `function search` 别名链尚未回归完备，暂不放入白名单，留给下一轮任务打通 alias registry 与 handler。

## 新增缺口与根因

- alias chain：`function search` 的别名链对旧路径 (`/function-search`、`/shops/function-search`) 断流，分页/过滤参数无法到达 handler，根因是 alias registry 和实际 handler 路径不同步，缺乏 fallback。
- ShopOrderQuery：`ShopOrderQueryPage`（shop-order-download）初始化顺序逆，组件先触发 `initQuery`，后填充 `ShopOrderQueryStore` 的 store context，导致首次加载卡住，需要等 `store.ready` 再拉取，同时补充失败重试机制。

## 下一轮 5 子任务建议

1. 重铺 `function search` alias chain，确保 alias registry、handler 与旧路径一致，加 fallback 保障旧页面不会坏。
2. 修复 `ShopOrderQueryPage` 的初始化顺序，等待 `ShopOrderQueryStore` 构建完成再拉数据并在失败时纪录慢加载。
3. 为 `Track-02` 的 `ProductDiffPage` 查询新增 query invalidation 用例，观察 `wms-wave-main-chain` 与 `outbound-fulfillment-flow` 差异。
4. 持续关注 `Track-01` 的监控日志 (`DataScopesPage`/`UsersPage`/`api_rbac_tenant_isolation`)，在下一轮报告中整理验证面板。
5. 重新跑 `outbound-fulfillment-flow` 并收集 `ShopOrderQueryPage`、WMS query invalidation、Observability 页面抖动日志，作为下轮验证基础。

## Round 2 Completion Update

### 本轮新增完成

- `Track-01` 搜索链已完成最小兼容修复：
  - `apps/erp-web/src/app/layout/moduleSearch.ts`
  - `apps/erp-web/src/app/layout/moduleSearch.test.ts`
  - 保留菜单显示名“审核订单”，同时补齐 `/orders/review` 的搜索别名：`订单审核`、`单审`
- `shop-order-download` 页面已完成初始化时序修复：
  - `apps/erp-web/src/pages/sales/ShopOrderQueryPage.tsx`
  - 根因不是路由失效，而是 `queryCells` 提前引用尚未初始化的 `handleKeywordChange`，导致运行时错误：`Cannot access 'handleKeywordChange' before initialization`

### 本轮 fresh 验证

1. `pnpm --filter erp-web exec vitest --run src/app/layout/moduleSearch.test.ts`
   - 结果：`9 passed`
2. `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/workbench-menu-navigation.spec.ts --project=chromium`
   - 结果：`15 passed`
3. `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts --project=chromium`
   - 结果：`1 passed`
4. `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/wms-wave-main-chain.spec.ts --project=chromium`
   - 结果：`1 passed`
5. `cargo test -p erp-api --test oms_orders -- --nocapture`
   - 结果：`20 passed`
   - 备注：仍有多条 `pool timed out while waiting for an open connection`，继续按环境噪音记录
6. `cargo test -p erp-api --test after_sales_state_machine_guard -- --nocapture`
   - 结果：`3 passed`
   - 备注：个别用例存在连接池超时 skip 日志，但最终结果为通过

### 本轮结束后的真实缺口

1. `Track-01`：入口链已经收口，但 `DataScopesPage / UsersPage / router.tsx` 的多实体 UI 贯通仍缺少一条完整的 fresh E2E 业务链。
2. `Track-03`：OMS 后端测试稳定，但前端订单审核、异常订单、拆合单、结算链路还缺少本轮新增 E2E 证据。
3. `Track-04`：两条 WMS 主链 E2E 已通过，但 query invalidation 与更多异常路径仍未形成系统性回归集。
4. `Track-05`：观测页与售后状态机已有高信噪通过证据，但更广的运维观测聚合视图尚未收口。

### 风险白名单结论

- `apps/erp-web/src/pages/orders/orderReviewPage.css`
  - 仍视为现存本地改动
  - 不纳入本轮快推白名单
- 本轮已从“待观察”切换为“已修复并验证”的项：
  - `function search` alias chain
  - `shop-order-download` 页面初始化异常
