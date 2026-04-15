# ERP95 Five-Track Migration Map

## 2026-04-14 本轮已吸收到主仓

- `5track-03` 已吸收：
  - `crates/erp-api/src/orders.rs`
  - `crates/erp-api/tests/oms_orders.rs`
- `5track-04` 已吸收：
  - `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
  - `apps/erp-web/src/pages/wms/WmsShipPage.tsx`
  - `apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
  - `apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`
- `5track-05` 已吸收：
  - `apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- `5track-02` 已吸收最小低风险修正：
  - `apps/erp-web/src/pages/products/ProductDiffPage.tsx`
- `5track-01` 已补齐最小编译基线：
  - `crates/erp-repo/src/data_scopes.rs`
  - `crates/erp-api/tests/api_rbac_tenant_isolation.rs`

## 本轮验证结论

- 已通过：
  - `cargo test -p erp-api --test oms_orders -- --nocapture`
  - `cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture`
  - `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts --project=chromium`
- 仍需单独记录为环境噪音：
  - Rust 集成测试中的 `pool timed out while waiting for an open connection`
  - 个别前端验证命令因 CLI 用法或依赖环境不完整导致的误失败

## 目的

这份清单用于把旧的 `erp95-01` 到 `erp95-10` 工作树成果迁移到新的 5 张主任务卡中，避免继续按旧 10 轨碎片化推进。

适用范围：

- 新主计划：[2026-04-14-industrial-erp-95-plus-five-track-codex-plan.md](F:/erp/docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-five-track-codex-plan.md)
- 新任务卡：
  - [erp95-5track-01.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-01.md)
  - [erp95-5track-02.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-02.md)
  - [erp95-5track-03.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-03.md)
  - [erp95-5track-04.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-04.md)
  - [erp95-5track-05.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-05.md)

## 迁移总原则

1. 旧 worktree 中已有价值的改动优先吸收，不重做。
2. 旧 worktree 中越界的改动必须先剥离，再进入新卡开发。
3. 新卡的 `allowed_files` 是唯一执行边界，旧卡的边界定义不再生效。
4. 任何“已暂存但未提交”的变更也必须纳入迁移判断，不能只看工作区未暂存部分。
5. 主仓中的脏改动如果与新卡边界重叠，必须先说明来源与去向，再决定是否吸收。

## 主仓当前重叠风险

主仓当前存在与 `5track-01` 重叠的未提交改动：

- `apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- `apps/erp-web/src/app/router.tsx`
- `apps/erp-web/src/pages/system/DataScopesPage.tsx`
- `apps/erp-web/src/pages/system/UsersPage.tsx`
- `crates/erp-api/src/lib.rs`
- `crates/erp-api/tests/api_rbac_tenant_isolation.rs`
- `crates/erp-repo/src/tenants.rs`

说明：

1. 这些文件与 `erp95-01` / `erp95-02` 的 worktree 改动范围高度重叠。
2. 在正式启动 `5track-01` 编码前，必须先明确“以主仓为准”还是“以 worktree 为准”。
3. `apps/erp-web/src/pages/orders/orderReviewPage.css` 和 `scripts/.dev-processes.json` 不属于当前 5-track 任务卡边界，迁移时应忽略。

## 旧 Worktree 到新卡映射

| 旧 worktree | 当前分支 | 当前状态 | 对应新卡 | 迁移结论 |
| --- | --- | --- | --- | --- |
| `erp95-01` | `codex/erp95-01-ia` | 有有效改动 | `5track-01` | 直接吸收 |
| `erp95-02` | 未单独核对分支名 | 有有效改动且严重越界 | `5track-01` | 先边界消毒再吸收 |
| `erp95-03` | `codex/erp95-03-channel-ops` | 有有效改动 | `5track-02` | 直接吸收 |
| `erp95-04` | 未单独核对分支名 | 有有效改动 | `5track-02` | 直接吸收 |
| `erp95-05` | `codex/erp95-05-oms` | 有有效改动 | `5track-03` | 直接吸收 |
| `erp95-06` | `codex/erp95-06-wms` | 有页面改动，另含越界文档残留 | `5track-04` | 清理越界文档后吸收 |
| `erp95-07` | 未见有效改动 | 空 | `5track-04` | 作为后续补货扩展承接点 |
| `erp95-08` | `codex/erp95-08-aftersales` | 空 | `5track-05` | 新起点 |
| `erp95-09` | 未见有效改动 | 空 | `5track-05` | 新起点 |
| `erp95-10` | 未见有效改动 | 空 | `5track-05` | 新起点 |

## 新卡 1: 平台与生产入口基线

**目标卡：** [erp95-5track-01.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-01.md)

**主要来源：**

- `F:/erp/.worktrees/erp95-01`
- `F:/erp/.worktrees/erp95-02`
- 主仓当前同边界未提交改动

**可直接吸收的变更：**

- `erp95-01`
  - `apps/erp-web/src/app/router.tsx`
  - `apps/erp-web/src/app/layout/routeMeta.ts`
  - `apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- `erp95-02`
  - `apps/erp-web/src/pages/system/DataScopesPage.tsx`
  - `apps/erp-web/src/pages/system/UsersPage.tsx`
  - `crates/erp-api/src/lib.rs`
  - `crates/erp-api/tests/api_rbac_tenant_isolation.rs`
  - `crates/erp-repo/src/data_scopes.rs`
  - `crates/erp-repo/src/tenants.rs`
  - `migrations/0095_multi_merchant_org_baseline.sql`

**必须先剥离的越界改动：**

- `crates/adapter-douyin/src/lib.rs`
- `crates/adapter-douyin/tests/integration.rs`
- `crates/adapter-jd/src/lib.rs`
- `crates/adapter-pdd/src/lib.rs`
- `crates/adapter-taobao/src/lib.rs`
- `crates/erp-api/src/orders/exception_workbench.rs`
- `crates/erp-api/src/orders/intake.rs`
- `crates/erp-repo/src/mapping_rules.rs`
- `crates/erp-repo/src/stock_counts.rs`
- `crates/erp-repo/src/stock_transfers.rs`
- `crates/erp-worker/src/main.rs`

**接手策略：**

1. 先以 `erp95-01` 为生产入口基线来源。
2. 再从 `erp95-02` 中只提取白名单内文件。
3. 对 `erp95-02` 的越界内容不做顺手整合，留在原 worktree 待人工判断或废弃。
4. 在开始编码前，必须明确主仓重叠改动是否要和 worktree 合并。

**迁移优先级：** 最高

## 新卡 2: 渠道接入与商品治理

**目标卡：** [erp95-5track-02.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-02.md)

**主要来源：**

- `F:/erp/.worktrees/erp95-03`
- `F:/erp/.worktrees/erp95-04`

**可直接吸收的变更：**

- `erp95-03`
  - `apps/erp-web/src/pages/system/SystemSyncJobsPage.tsx`
  - `apps/erp-web/e2e/system-integrations.spec.ts`
- `erp95-04`
  - `apps/erp-web/src/pages/products/ProductDiffPage.tsx`
  - `apps/erp-web/e2e/product-change-control.spec.ts`

**当前判断：**

1. `erp95-03` 与 `erp95-04` 的变更面比较干净。
2. 两个 worktree 目前主要是前端页面和 E2E 收口，适合直接合并到新的渠道商品治理主线。
3. 这条新卡是当前最容易继续推进编码的一条线，因为边界最清楚，冲突最少。

**接手策略：**

1. 以 `erp95-03` 作为主工作树承载新的 `5track-02`。
2. 吸收 `erp95-04` 的 `ProductDiffPage.tsx` 与对应测试。
3. 后续如果需要补后端仓储层与集成测试，再在同一新卡边界内继续推进。

**迁移优先级：** 高

## 新卡 3: OMS 主链收口

**目标卡：** [erp95-5track-03.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-03.md)

**主要来源：**

- `F:/erp/.worktrees/erp95-05`

**可直接吸收的变更：**

- `crates/erp-api/src/orders.rs`
- `crates/erp-api/tests/oms_orders.rs`

**当前判断：**

1. `erp95-05` 的变更集中、干净，聚焦在 OMS 后端。
2. 目前还没有看到同轨前端页面层推进，因此后续收口时还需要补 UI 或行为验证。
3. 这条线可以与 `5track-02` 并行推进，但合并顺序应晚于 `5track-01`。

**接手策略：**

1. 保持 `erp95-05` 作为 OMS 继续推进的主 worktree。
2. 先锁定 `orders.rs` 与 `oms_orders.rs` 已有进展。
3. 后续再补审单、异常、拆合单、结算相关页面或测试。

**迁移优先级：** 高

## 新卡 4: 履约与补货执行闭环

**目标卡：** [erp95-5track-04.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-04.md)

**主要来源：**

- `F:/erp/.worktrees/erp95-06`
- `F:/erp/.worktrees/erp95-07`

**可直接吸收的变更：**

- `erp95-06` 已暂存：
  - `apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
  - `apps/erp-web/src/pages/wms/WmsShipPage.tsx`
  - `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`

**必须处理的异常：**

- `erp95-06` 暂存中还包含：
  - `docs/superpowers/plans/2026-04-14-fulfillment-chain-closure.md`
- 同一 worktree 中该文件还显示为 `AD`，说明存在“先删后又暂存旧版本”的残留状态，不能直接吸收。

**当前判断：**

1. `erp95-06` 真正有价值的是 WMS 三个页面的改动。
2. `erp95-07` 目前没有看到实际改动，因此补货链仍需后续补建。
3. 这条线可以推进，但开始前必须先把 `erp95-06` 的文档残留状态清理干净。

**接手策略：**

1. 保留 `erp95-06` 作为履约主线来源。
2. 先丢弃或重新整理越界文档残留状态。
3. 再决定是否以同一 worktree 继续补采购与计划执行闭环。

**迁移优先级：** 中高

## 新卡 5: 售后财务与集成门禁

**目标卡：** [erp95-5track-05.md](F:/erp/docs/ops/tasks/2026-04-14/erp95-5track-05.md)

**主要来源：**

- `F:/erp/.worktrees/erp95-08`
- `F:/erp/.worktrees/erp95-09`
- `F:/erp/.worktrees/erp95-10`

**当前判断：**

1. 这三个旧 worktree 目前没有看到有效改动。
2. 这意味着 `5track-05` 基本属于“新起点收口轨”，不是吸收型任务。
3. 但这条线承担最终门禁和证据口径，因此必须最后统一收口，不能提前抢跑定义完成标准。

**接手策略：**

1. 以 `erp95-08` 作为新的承载 worktree 起步。
2. 后续按新卡边界逐步并入售后、财务、观测和门禁脚本。
3. 能力矩阵、进度表和 Go/No-Go 口径全部归这条线统一维护。

**迁移优先级：** 最后

## 建议启动顺序

### 立即可启动

1. `5track-02` 渠道接入与商品治理
2. `5track-03` OMS 主链收口

原因：

1. 两条线的现有 worktree 改动干净，吸收成本低。
2. 不会马上撞到主仓现有脏改动。
3. 适合继续累积“真实收口进度”。

### 启动前必须先处理边界

1. `5track-01` 平台与生产入口基线
2. `5track-04` 履约与补货执行闭环

原因：

1. `5track-01` 同时牵涉主仓脏改动和 `erp95-02` 越界污染。
2. `5track-04` 需要先清理 `erp95-06` 的文档残留状态。

### 最后启动

1. `5track-05` 售后财务与集成门禁

原因：

1. 该线缺少已有实质改动。
2. 需要依赖前面几条主线输出的状态契约和实际证据。

## 当前建议

如果继续直接推进开发，建议采用以下顺序：

1. 先做 `5track-01` 的边界消毒方案确认
2. 同步开始 `5track-02` 的吸收与续开发
3. 紧接着推进 `5track-03`
4. 待 `erp95-06` 清理干净后启动 `5track-04`
5. 最后统一收口到 `5track-05`

## 本轮真实验证现状

- OMS: `5track-03` 相关核心接口 (`orders.rs`, `oms_orders.rs`) 已通过 Rust 单测，现有 tracker 未再上报新异常。 WMS: `5track-04` 中的 `WmsWavesPage`, `WmsShipPage`, `WmsOutboundConfirmPage` 联调完成，`wms-wave-main-chain` 通过，`outbound-fulfillment-flow` 仍需重复。 Observability: `ObservabilityPage` 以及 `ProductDiffPage` 的主要面板已能渲染。 Track-01: `DataScopesPage/UsersPage` 与 `api_rbac_tenant_isolation` 权限验证达成初步通过。
- 测试层面：Rust 单测 (`oms_orders`, `api_rbac_tenant_isolation`) 与 Playwright (`outbound-fulfillment-flow`, `wms-wave-main-chain`) 持续在 CI 中运行，`pool timed out while waiting for an open connection` 仍为主要噪声，需继续观察 `pnpm` CLI 重试与 Playwright 失败堆栈。

## 风险白名单

- `apps/erp-web/src/pages/orders/orderReviewPage.css` 仍为本地改动，暂不纳入快推白名单，后续需要同步清楚再决定是否合入。
- `function search` alias chain 未完全回归，当前只列入风险观察，不放入白名单，并预留后续任务覆盖 alias registry 与 handler。

## 新增缺口与根因

- alias chain: `function search` 的别名链（例如 `/function-search` 和 `/shops/function-search`）断裂导致请求无法到达 handler，分页/过滤参数被截断，根因是 alias registry 与实际 handler 路径不同步，缺少 fallback。需要补齐 alias 路径与 handler，增加断链监控。
- ShopOrderQueryPage: `shop-order-download` 页面在挂载时提前触发 `initQuery`，但 `ShopOrderQueryStore` 中的 `shopId/context` 尚未初始化，结果首次拉取为空，页面陷入 loading 状态；需调整生命周期以等 store ready 后再启动，并添加失败重试指标。

## 下一轮 5 子任务建议

1. 修复 `function search` alias chain，确保 alias registry、handler 与旧路径维持一致，加入 fallback 记录以防未来断链。
2. 调整 `ShopOrderQueryPage` 初始化顺序，使 `ShopOrderQueryStore` 就绪后再发起拉取，并在初始化延迟时记录 slow-loading 日志。
3. 在 `Track-02` 的 `ProductDiffPage` 增加 query invalidation 用例，关注 `wms-wave-main-chain` 与 `outbound-fulfillment-flow` 的行为差异。
4. 深入观察 `Track-01` 的监控日志（`DataScopesPage`、`UsersPage`、`api_rbac_tenant_isolation`），准备下一轮的验证报告。
5. 带着 `ShopOrderQueryPage`、WMS query invalidation、Observability 页面抖动日志重新跑一次 `outbound-fulfillment-flow`，作为下轮验证基线。

## Round 2 Verified Closure Update

### 已确认收口

- `Track-01`：`moduleSearch` 已补齐 `/orders/review` 的搜索别名，`workbench-menu-navigation.spec.ts` 已 fresh 通过 `15 passed`。
- `Track-04`：`wms-wave-main-chain.spec.ts` 与 `outbound-fulfillment-flow.spec.ts` 本轮均已 fresh 通过。
- `Track-05`：`after_sales_state_machine_guard` 本轮 fresh 通过 `3 passed`，环境噪音继续单独记录。
- `Track-03`：`oms_orders` 本轮 fresh 通过 `20 passed`。

### 本轮新增迁移结论

- 可计入本轮主仓新增收口的文件：
  - `apps/erp-web/src/app/layout/moduleSearch.ts`
  - `apps/erp-web/src/app/layout/moduleSearch.test.ts`
  - `apps/erp-web/src/pages/sales/ShopOrderQueryPage.tsx`
- 不再继续记录为“本轮真实缺口”的项：
  - `function search` alias chain
  - `shop-order-download` 页面初始化异常

### 轮后剩余缺口

1. `DataScopesPage / UsersPage / router.tsx` 的多实体 UI 链路还缺少完整业务级 E2E。
2. OMS 前端工作台链路仍缺少新增 E2E 覆盖，当前主要是后端证据更强。
3. WMS 已有两条主链回归，但异常路径和 query invalidation 仍未系统化收口。
4. `orderReviewPage.css` 继续视为本地改动，不纳入本轮快推白名单。
