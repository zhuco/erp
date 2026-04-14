# 工业级电商 ERP 95% 目标计划（订单/匹配/修改/发货/波次）

> 文档版本：V95.0
> 更新日期：2026-04-14
> 适用范围：订单处理、商品匹配、订单修改、发货单发货、波次发货
> 执行方式：Codex 多子线程并行开发 + 发布门禁硬阻断

---

## 1. 目标定义（什么叫 95% 工业级）

本计划中的 `95%` 不是“页面数量”或“接口数量”，而是“可收费、可审计、可持续发布”的综合成熟度。

### 1.1 工业级 95% 评分模型（总分 100）

| 维度 | 权重 | 达标定义 |
| --- | --- | --- |
| 五条主链路功能闭环 | 35 | 订单处理、商品匹配、订单修改、发货单发货、波次发货均可端到端执行并可回归 |
| 数据持久化与审计 | 15 | 所有关键写操作有 `x-persistence-required` + 可查表证据 + 审计留痕 |
| 契约一致性（OpenAPI） | 10 | 前后端接口、权限、状态码、持久化声明一致且门禁可阻断 |
| 质量保障（集成/E2E） | 10 | 后端集成测试、前端 E2E 覆盖主链路，禁止 skip 伪通过 |
| 稳定性与补偿 | 10 | 失败可重试、回放、幂等、部分失败可见、补偿可执行 |
| 权限与租户隔离 | 8 | 租户隔离、关键动作权限、跨租户越权防护验证通过 |
| 可观测性与运维就绪 | 7 | 关键业务指标、日志、告警、runbook、Go/No-Go 证据齐全 |
| 并行交付治理能力 | 5 | 多线程并行可控，冲突率低，按里程碑稳定集成 |

### 1.2 当前基线（2026-04-14）

- 当前综合水平：`约 72/100`（以仓库实证为准）。
- 短板集中：`波次发货前端闭环`、`真实数据库门禁全绿`、`跨链路一致性回归`。

### 1.3 95% 达标硬标准

1. 五条链路各自评分不低于 `90`，且任一链路不得低于 `88`。
2. `pnpm run test:backend:release-gate` 在真实 DB 环境下连续 `3` 次全绿。
3. `test:e2e:release-gate` 全绿，且关键场景（含异常）通过率 `100%`。
4. 任一关键测试出现 `skipping ... DATABASE_URL` 或 `pool timed out` 直接 `NO-GO`。

---

## 2. 五条主链路 95% 目标拆解

## 2.1 订单处理（目标 95）

- 目标：审单、异常、转态流转、日志审计、失败补偿完整闭环。
- 必达：
  - `oms_orders.rs` 真库全绿。
  - 审核动作（PASS/FORCE/REQUEUE）均有 `order_action_log + order_status_history + operation_logs`。
  - 审核失败场景有明确错误码与可恢复路径。

## 2.2 商品匹配（目标 95）

- 目标：下载、自动匹配、手动匹配、删除匹配、冲突治理、日志追踪闭环。
- 必达：
  - `platform_sync_and_mapping_flow_integration.rs` 真库全绿。
  - `AUTO/MANUAL/DELETE` 三类操作均有任务记录与日志元数据。
  - 冲突原因与匹配来源在 UI、API、DB 三层一致。

## 2.3 订单修改（目标 95）

- 目标：修改动作（拆单/合单/编辑）与审计、状态历史完全一致。
- 必达：
  - 审核工作台动作统一 guard（权限、选择数量、租户上下文）。
  - 修改动作失败不产生“成功提示”，并返回可追踪请求标识。
  - `order_match_ship_wave_persistence_guard.rs` 真库全绿。

## 2.4 发货单发货（目标 95）

- 目标：发货确认、出库发货、状态回写、事件投递一致。
- 必达：
  - `shipment_confirm.rs`、`shipments.rs` 真库回归通过。
  - `SHIPMENT_CONFIRMED` 事件、`ship_status`、OMS/WMS 状态映射一致。
  - 部分失败可见且可补偿。

## 2.5 波次发货（目标 95，当前核心）

- 目标：波次发货端到端可执行（前端可操作、后端可审计、结果可回放）。
- 必达：
  - 前端接入 `/wms/waves/{wave_id}/ship`。
  - 增加波次发货正向集成测试（成功、部分失败、中断策略、权限）。
  - 落库证据包含 `operation_logs`、`audit_logs`，并更新波次状态。

---

## 3. Codex 多子线程并行开发总编排

## 3.1 编排原则

1. 单线程单写域：每个线程只改指定文件，避免冲突。
2. 关键路径优先：`真实DB门禁 -> 波次发货闭环 -> 全链路回归`。
3. 日集成：每天至少 2 次主干合流，防止长时间分叉。
4. 证据先于结论：每个线程必须提交“命令 + 结果 + 风险”。

## 3.2 线程矩阵（12 线程）

### T0（主控线程 / Orchestrator）
- 责任：调度、合并、冲突仲裁、里程碑打点。
- 写域：`docs/ops/*V2.md`、`docs/superpowers/plans/*`（仅计划与证据）。

### T1（订单处理后端）
- 写域：
  - `crates/erp-api/tests/oms_orders.rs`
  - `crates/erp-repo/src/order_action_logs.rs`
  - `crates/erp-repo/src/order_status_history.rs`
- DoD：订单审核/异常动作真库全绿。

### T2（订单处理前端）
- 写域：
  - `apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.ts`
  - `apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.test.ts`
  - `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- DoD：review/merge/split 操作可用且错误处理一致。

### T3（商品匹配后端）
- 写域：
  - `crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs`
  - `crates/erp-repo/src/shop_product_tasks.rs`
  - `crates/erp-repo/src/channel_sku_mappings.rs`
- DoD：AUTO/MANUAL/DELETE 全链路可追踪。

### T4（商品匹配前端）
- 写域：
  - `apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`
  - `apps/erp-web/src/api/shopProducts.ts`
- DoD：匹配治理视图完整（冲突、来源、任务、筛选一致）。

### T5（订单修改与持久化守卫）
- 写域：
  - `crates/erp-api/tests/order_match_ship_wave_persistence_guard.rs`
  - `crates/erp-repo/src/operation_logs.rs`
- DoD：写接口持久化守卫不可绕过。

### T6（发货单发货后端）
- 写域：
  - `crates/erp-api/tests/shipment_confirm.rs`
  - `crates/erp-api/tests/shipments.rs`
- DoD：事件、状态、日志三者一致。

### T7（发货单发货前端）
- 写域：
  - `apps/erp-web/src/pages/wms/WmsShipPage.tsx`
  - `apps/erp-web/src/api/wms.ts`（仅 outbound ship 范围）
- DoD：发货流程异常反馈与状态刷新稳定。

### T8（波次发货后端）
- 写域：
  - `crates/erp-api/src/lib.rs`（`ship_wms_wave` 范围）
  - `crates/erp-api/tests/wave_ship_integration.rs`（新增）
- DoD：波次发货成功/部分失败/权限场景可验证。

### T9（波次发货前端）
- 写域：
  - `apps/erp-web/src/api/wms.ts`（新增 `shipWmsWave`）
  - `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- DoD：可执行波次发货并展示逐单结果。

### T10（契约与门禁）
- 写域：
  - `openapi/erp_api_v1.yaml`
  - `scripts/verify_openapi_gate_v2.mjs`
  - `scripts/release_gate_backend.mjs`
- DoD：契约缺失/伪通过可稳定阻断。

### T11（E2E 与证据归档）
- 写域：
  - `apps/erp-web/e2e/release-gate-main-chain.spec.ts`
  - `apps/erp-web/e2e/release-gate-cross-chain.spec.ts`
  - `docs/ops/RELEASE_GATE_CHECKLIST_V2.md`
- DoD：一键输出 Go/No-Go 证据。

---

## 4. 并行执行波次（Wave）与依赖关系

## 4.1 Wave-1（基础收敛，D1-D3）

并行线程：T1、T2、T3、T4、T6、T7、T10

目标：
- 订单处理/匹配/发货三链路在前后端双侧稳定。
- OpenAPI 与后端门禁规则固定。

退出条件：
- `repo:verify:openapi` 通过。
- 关键测试在真实 DB 环境无 skip。

## 4.2 Wave-2（短板攻坚，D4-D7）

并行线程：T5、T8、T9、T10

目标：
- 波次发货前后端完全打通。
- 持久化守卫和权限守卫覆盖到位。

退出条件：
- 波次发货集成测试通过。
- 前端可执行波次发货并可视化结果。

## 4.3 Wave-3（总回归与上线证据，D8-D10）

并行线程：T11 + T0（主控收口）

目标：
- 五链路 E2E 全绿。
- 完成发布证据包与 Go/No-Go 结论。

退出条件：
- 发布门禁全绿，且连续 3 次稳定通过。

---

## 5. 95% 里程碑与评分提升路径

| 里程碑 | 时间窗 | 目标分数 | 核心结果 |
| --- | --- | --- | --- |
| M0 基线冻结 | Day 0 | 72 | 锁定范围、线程边界、门禁口径 |
| M1 主链路稳定 | Day 3 | 82 | 订单/匹配/发货链路真库可复跑 |
| M2 波次闭环完成 | Day 7 | 88 | 波次发货前后端与测试闭环 |
| M3 全链路门禁 | Day 10 | 92 | E2E + 后端门禁 + 持久化证据齐全 |
| M4 工业级冲刺 | Day 14 | 95 | 连续稳定发布能力与证据归档 |

---

## 6. 统一门禁与验收命令

```bash
pnpm run repo:verify:openapi
pnpm run test:backend:release-gate
pnpm --dir apps/erp-web run generate:api
pnpm --dir apps/erp-web build
pnpm run test:e2e:release-gate
```

### 6.1 NO-GO 条件（任一命中即阻断）

1. 未配置 `DATABASE_URL/TEST_DATABASE_URL`。
2. 出现 `skipping ... DATABASE_URL` 或 `pool timed out`。
3. 写接口缺失持久化声明或无落库证据。
4. 波次发货仅后端可调、前端不可执行。
5. 五条主链路任一链路无法完整回归。

---

## 7. 子线程执行规范（Codex）

## 7.1 每个线程必须遵守

1. 不回退其他线程修改。
2. 只修改分配写域文件。
3. 先补失败测试，再最小实现，再全量验证。
4. 输出必须包含：
   - 变更文件列表
   - 执行命令与结果摘要
   - 残余风险

## 7.2 标准线程提示词模板

```text
你是线程 <Tx>，负责文件：
<文件绝对路径列表>

目标：<DoD>
约束：
1) 你不是唯一开发者，不要回退别人改动。
2) 仅修改你负责的文件。
3) 先测试后实现，最后再次运行测试。

交付：
1) 变更清单
2) 核心实现点（3-5条）
3) 命令与结果
4) 风险与待办
```

---

## 8. 冲突治理与集成策略

1. 分支策略：`codex/erp95/t1-*` 至 `codex/erp95/t11-*`。
2. 合并节奏：每日两次主干合流（中午、收工前）。
3. 冲突优先级：门禁脚本 > OpenAPI > 后端接口 > 前端调用 > E2E。
4. 回滚策略：仅回滚本线程提交，禁止跨线程硬回退。

---

## 9. 证据包模板（上线必备）

每次里程碑输出一个证据包，至少包含：

1. 命令执行结果摘要（含时间戳）。
2. 关键测试通过截图或日志片段。
3. 关键表 SQL 抽样证据：
   - `order_action_log`
   - `order_status_history`
   - `operation_logs`
   - `audit_logs`
4. 缺陷清单与处理结论。
5. Go/No-Go 判定。

---

## 10. 最终交付定义（Definition of Done @95）

以下条件全部满足才可宣告达到 `95%`：

1. 五条主链路端到端执行通过。
2. 关键写操作持久化证据完整且可审计。
3. OpenAPI 与实现一致，门禁可阻断不合规变更。
4. 后端发布门禁、前端构建、E2E 发布门禁全部全绿。
5. 在真实 DB 环境连续 3 次稳定通过，且无关键 skip。
6. 证据包完整归档，可供上线评审复核。

---

## 11. 实施进展快照（2026-04-14，主线程集成后）

### 11.1 已完成（本轮并行实现）

- [x] `T2 订单处理前端`：新增并加固工具栏动作 guard 与错误处理，覆盖 `review/merge/split` 的租户、权限、选择数量、异常分支。
  - 文件：
    - `apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.ts`
    - `apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.test.ts`
  - 结果：`30` 条单测通过。
- [x] `T3 商品匹配后端`：补齐 `DELETE_MATCH` 治理断言（任务结果、API 状态、DB 映射清理、operation_logs metadata 一致性）。
  - 文件：
    - `crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs`
  - 结果：在可连库环境下为严格断言；当前无 DB 环境下保持既有 skip 逻辑。
- [x] `T11 E2E 与证据归档`：发布门禁 smoke 新增波次链路覆盖（`/wms/waves` + 关键 `data-testid`）。
  - 文件：
    - `apps/erp-web/e2e/release-gate-main-chain.spec.ts`
    - `apps/erp-web/e2e/release-gate-cross-chain.spec.ts`
  - 结果：Playwright `2 passed`。

### 11.2 前序已完成（本计划批次）

- [x] `T8/T9 波次发货闭环`：前端接入 `shipWmsWave`、波次详情发货 UI、后端波次发货集成测试与前端 API 单测已落地。
  - 关键文件：
    - `apps/erp-web/src/api/wms.ts`
    - `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
    - `apps/erp-web/src/api/wms.waveShip.test.ts`
    - `crates/erp-api/tests/wave_ship_integration.rs`

### 11.3 本轮主线程复验证据

```bash
pnpm --dir apps/erp-web exec vitest run src/pages/orders/review-native/orderReviewToolbarActions.test.ts
pnpm --dir apps/erp-web exec playwright test e2e/release-gate-main-chain.spec.ts e2e/release-gate-cross-chain.spec.ts --project=chromium
cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture
pnpm run test:backend:release-gate
```

- 结果摘要：
  - `orderReviewToolbarActions.test.ts`: `30 passed`
  - release-gate E2E: `2 passed`
  - `platform_sync_and_mapping_flow_integration`: 进程通过，但在无 DB 环境输出 skip 信息（不计作发布证据）
  - `test:backend:release-gate`: 按门禁设计直接 `NO-GO`（`DATABASE_URL/TEST_DATABASE_URL` 未配置）

### 11.4 距离 95% 的剩余关键工作

1. 提供可用的真实数据库环境并跑通 `test:backend:release-gate`（连续 3 次全绿）。
2. 将 `platform_sync_and_mapping_flow_integration` 从“可 skip 通过”升级到“必须连库执行”，并纳入门禁硬证据。
3. 继续补齐 `oms_orders` 与 `shipment_confirm/shipments` 的异常链路断言，覆盖失败补偿与幂等回放。
4. 输出里程碑证据包（SQL 抽样 + 日志元数据 + Go/No-Go）并归档到 `docs/ops/`。
