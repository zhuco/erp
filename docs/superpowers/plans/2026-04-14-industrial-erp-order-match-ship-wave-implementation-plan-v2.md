# Industrial ERP Order/Match/Ship/Wave Implementation Plan (V2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 以工业级标准完成“订单处理、商品匹配、订单修改、发货单发货、波次发货”五条主链路，并将“关键动作必须落库、可审计、可发布门禁阻断”固化为上线硬规则。  
**Architecture:** 采用“业务主链路 + 数据持久化守卫 + OpenAPI 契约门禁 + 集成测试 + E2E 发布门禁”五层闭环。所有写操作必须同时满足：API 契约声明、Repo 持久化写入、集成测试断言、发布脚本阻断。  
**Tech Stack:** React + TypeScript + Playwright + Rust + Axum + SQLx + PostgreSQL + OpenAPI + pnpm + cargo

---

## 0. 当前进度快照（2026-04-14）

### 0.1 已完成（P0）
- [x] `release_gate_backend.mjs` 升级为硬门禁：无 `DATABASE_URL/TEST_DATABASE_URL` 直接 NO-GO，且阻断 `skipping ... DATABASE_URL`/`pool timed out` 伪通过
- [x] OpenAPI 补齐 shop-products 主链路端点：`skus/download/tasks/create-skus/auto-match/manual-match/delete-match`
- [x] OpenAPI 新增/补齐匹配治理字段：`mapping_status`、`conflict_reason`、`match_source`
- [x] OpenAPI 门禁脚本升级（v2）：校验路径/权限/状态码/持久化声明
- [x] `pnpm.cmd run repo:verify:openapi` 验证通过
- [x] `pnpm.cmd run test:backend:release-gate` 在无 DB 配置下按预期阻断（NO-GO）

### 0.2 进行中（P0）
- [ ] 使用真实数据库环境执行后端关键链路门禁（当前环境未配置 DB）
- [ ] 订单审核主工作台动作闭环（review/split/merge）与真实落库联调
- [ ] 发货与波次跨模块状态一致性回归

### 0.3 风险提示
- 旧版 `docs/ops/*.md` 与旧 `verify_openapi_gate.mjs` 存在写入限制，已通过 V2 文件与入口切换规避，不影响当前门禁执行。

---

## 1. 企业级硬约束（不可协商）

### 1.1 动作落库强制规则
- 任一业务写操作返回成功前，必须至少命中一类持久化证据表：
  - `order_action_log`
  - `order_status_history`
  - `operation_logs`
  - `audit_logs`
- 商品匹配链路必须补充命中：
  - `shop_product_tasks`
  - `shop_products`
  - `shop_product_skus`
  - `channel_sku_mappings`

### 1.2 发布门禁强制规则
- 缺少 `DATABASE_URL/TEST_DATABASE_URL`：**NO-GO**
- 关键测试出现 `skipping ... DATABASE_URL` 或 `pool timed out`：**NO-GO**
- OpenAPI 路径/权限/状态码/持久化声明任一缺失：**NO-GO**
- 仅前端成功、无后端落库证据：**NO-GO**

---

## 2. 数据库完整结构检查矩阵（Order/Match/Ship/Wave）

| 链路 | 必需核心表 | 必需索引/约束方向 | 落库断言（测试必须覆盖） |
| --- | --- | --- | --- |
| 订单处理/修改 | `orders`, `order_action_log`, `order_status_history` | 幂等键唯一约束、状态时间序索引 | 审核/拆单/合单/修改后存在动作日志与状态历史 |
| 商品匹配 | `shop_products`, `shop_product_skus`, `shop_product_tasks`, `channel_sku_mappings`, `operation_logs` | 匹配状态/来源索引、任务时间索引 | auto/manual/delete match 后任务记录与映射记录一致 |
| 发货单发货 | `shipments`, `shipment_events`(如有), `operation_logs`, `audit_logs` | 发货单状态索引、幂等键 | confirm/cancel 后状态快照与日志记录一致 |
| 波次发货 | `wms_waves`, `wms_outbound_docs`, `operation_logs`, `audit_logs` | 波次状态索引、单据关联索引 | wave ship 后单据级执行结果可查询、失败可重试 |

---

## 3. 并行执行编排（Codex 多子任务）

### Worker-A（Order Workbench）
**Owner Scope:** `apps/erp-web/src/pages/orders/*review*`  
**目标:** 恢复并加固 review/split/merge 动作闭环（含错误提示与重试）  
**DoD:** 前端单测 + E2E + 后端落库断言全通过

### Worker-B（Product Match Governance）
**Owner Scope:** `apps/erp-web/src/pages/shopProducts/*`, `apps/erp-web/src/api/shopProducts.ts`, `crates/erp-api/src/lib.rs`(match相关)  
**目标:** 冲突治理、手工修复、回写日志一致  
**DoD:** 匹配治理 E2E 通过，`operation_logs` 元数据可追踪

### Worker-C（Order Edit + Audit）
**Owner Scope:** `crates/erp-api/tests/oms_orders.rs`, `crates/erp-repo/src/order_action_logs.rs`, `crates/erp-repo/src/order_status_history.rs`  
**目标:** 订单修改强制审计落库，持久化失败即失败响应  
**DoD:** `oms_orders` 关键测试覆盖并通过

### Worker-D（Shipment Consistency）
**Owner Scope:** `crates/erp-api/tests/shipment_confirm.rs`, `crates/erp-repo/src/shipments.rs`, `apps/erp-web/src/pages/wms/WmsShipPage.tsx`  
**目标:** OMS/WMS/Shipment 状态一致性 + 补偿策略  
**DoD:** shipment confirm 主链路全绿，失败分支可回放

### Worker-E（Wave Ship Execution）
**Owner Scope:** `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`, `crates/erp-repo/src/wms.rs`, `crates/erp-api/src/lib.rs`(waves)  
**目标:** 波次发货端到端可执行，部分失败可见且可重试  
**DoD:** wave ship E2E 通过且单据级结果落库可查

### Worker-F（Release Gate & Contract）
**Owner Scope:** `scripts/release_gate_backend.mjs`, `scripts/verify_openapi_gate_v2.mjs`, `openapi/erp_api_v1.yaml`, `docs/ops/*V2.md`  
**目标:** 发布门禁阻断能力与契约一致性常态化  
**DoD:** `repo:verify:openapi` 稳定通过，门禁在无 DB 场景正确失败

---

## 4. 执行任务清单（剩余）

### Task 1: 真实数据库门禁验收（P0）
**Files:**
- Modify: `F:/erp/scripts/release_gate_backend.mjs`（仅在需调整关键测试集时）
- Test: `F:/erp/crates/erp-api/tests/order_match_ship_wave_persistence_guard.rs`
- Test: `F:/erp/crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/oms_orders.rs`
- Test: `F:/erp/crates/erp-api/tests/shipment_confirm.rs`

- [ ] Step 1: 配置 `DATABASE_URL`（或 `TEST_DATABASE_URL`）并确认可连通
- [ ] Step 2: 运行 `pnpm.cmd run test:backend:release-gate`
- [ ] Step 3: 若失败，按测试输出逐项修复后重跑直至全绿
- [ ] Step 4: 记录门禁证据（日志片段 + 关键表命中）

### Task 2: 订单审核主工作台动作闭环（P0）
**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- Create/Modify: `F:/erp/apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.ts`
- Create/Modify: `F:/erp/apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.test.ts`
- Test: `F:/erp/apps/erp-web/e2e/order-processing-flow.spec.ts`

- [ ] Step 1: 补失败测试，锁定 review/split/merge 行为
- [ ] Step 2: 最小实现绑定动作并加错误处理
- [ ] Step 3: 运行单测与 E2E
- [ ] Step 4: 验证后端动作日志落库

### Task 3: 发货与波次一致性回归（P0）
**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/crates/erp-repo/src/shipments.rs`
- Modify: `F:/erp/crates/erp-repo/src/wms.rs`
- Test: `F:/erp/crates/erp-api/tests/shipment_confirm.rs`
- Test: `F:/erp/crates/erp-repo/tests/wms_wave_rules.rs`

- [ ] Step 1: 补失败测试覆盖部分失败和重试分支
- [ ] Step 2: 实现状态回写一致性
- [ ] Step 3: 验证日志/审计落库与前端可见性一致

### Task 4: 发布清单归并回主文档（P1）
**Files:**
- Modify: `F:/erp/docs/ops/RELEASE_GATE_CHECKLIST.md`
- Modify: `F:/erp/docs/ops/GO_NO_GO_CHECKLIST.md`
- Modify: `F:/erp/docs/ops/TASK_COMPLETION_MATRIX.md`
- Source: `F:/erp/docs/ops/*_V2.md`

- [ ] Step 1: 解除旧文件写锁（或权限策略）
- [ ] Step 2: 将 V2 内容并入主文档
- [ ] Step 3: 校验文档与脚本行为一致

---

## 5. 里程碑与门禁

### M1（已达成）
- OpenAPI 主链路与门禁脚本升级完成
- 无 DB 环境下后端门禁可正确阻断

### M2（本周目标）
- 真实 DB 环境下后端关键测试全通过
- 订单审核主工作台动作闭环恢复

### M3（发布前）
- 发货/波次一致性全链路回归通过
- E2E 发布门禁全绿
- Go/No-Go 清单证据完整

---

## 6. 标准验收命令

```bash
pnpm.cmd run repo:verify:openapi
pnpm.cmd run test:backend:release-gate
pnpm --dir apps/erp-web run generate:api
pnpm --dir apps/erp-web build
pnpm --dir apps/erp-web exec playwright test e2e/release-gate-main-chain.spec.ts e2e/release-gate-cross-chain.spec.ts
```

---

## 7. 交付判定（工业级）

- 五条主链路均有“业务动作成功 + 持久化证据 + 审计可追溯 + 门禁可阻断”
- 任一链路缺少数据库证据，不得进入 Go
- 上线前必须有可复核的日志与 SQL 证据归档
