---
name: ERP95-5T-03 OMS 主链收口
goal: 收口审单、异常、拆合单、复核、结算与审计落库，形成首发版本最关键的订单主链闭环。
background: 本任务卡吸收原 ERP95-05 的内容，聚焦订单域自身的可执行、可追踪、可回归，不把履约和财务问题混入本卡片。
owner: Codex-5T-03
branch: codex/erp95-5t-03-oms-main-chain
worktree: .worktrees/erp95-05
action: active
source_plan: docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-five-track-codex-plan.md
replaces:
  - docs/ops/tasks/2026-04-14/erp95-track-05.md
allowed_files:
  - apps/erp-web/src/pages/orders/OrderReviewPage.tsx
  - apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx
  - apps/erp-web/src/pages/orders/OrderSettlementPage.tsx
  - apps/erp-web/src/pages/orders/OrderSplitMergeLegacyPage.tsx
  - apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx
  - apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.ts
  - crates/erp-api/src/orders.rs
  - crates/erp-repo/src/orders.rs
  - crates/erp-repo/src/order_reviews.rs
  - crates/erp-repo/src/order_action_logs.rs
  - crates/erp-repo/src/order_status_history.rs
  - crates/erp-api/tests/oms_orders.rs
  - crates/erp-api/tests/order_exception_sla_integration.rs
  - crates/erp-api/tests/order_match_ship_wave_persistence_guard.rs
  - apps/erp-web/e2e/order-processing-flow.spec.ts
  - apps/erp-web/e2e/order-exceptions.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/**
  - apps/erp-web/src/pages/wms/**
  - apps/erp-web/src/pages/procurement/**
  - apps/erp-web/src/pages/planning/**
  - apps/erp-web/src/pages/afterSales/**
  - apps/erp-web/src/pages/finance/**
  - package.json
  - scripts/**
  - docs/ops/**
dependencies:
  - 优先吸收 erp95-05 中已经推进的 orders.rs 与 oms_orders.rs 改动。
  - 与履约链通过状态契约对接，不直接修改 WMS 文件。
  - 任何让测试通过的修复都必须保留持久化与审计证据。
dod:
  - 审单、异常、拆合单、复核、结算共享一致状态链。
  - 操作日志与状态历史可查询。
  - OMS 相关后端测试和 E2E 具备通过路径。
  - 无越界修改。
test_commands:
  - cargo test -p erp-api --test oms_orders --test order_exception_sla_integration --test order_match_ship_wave_persistence_guard -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/order-processing-flow.spec.ts e2e/order-exceptions.spec.ts --project=chromium
delivery: commit hash + 变更摘要 + 测试结果 + 剩余风险
---

## 任务目标
把订单域从“页面和接口大体都有”推进到“关键动作能写入、能回看、能验证”，为后续履约和售后提供稳定状态输出。

## 执行步骤
- [ ] Step 1: 盘点 `erp95-05` 的现有改动和可直接吸收的测试。
- [ ] Step 2: 先跑失败测试，锁定状态链、异常处理和审计落库问题。
- [ ] Step 3: 完成审单、异常、拆合单、复核、结算动作的统一收口。
- [ ] Step 4: 补齐操作日志与状态历史查询能力。
- [ ] Step 5: 校验 OMS 输出状态是否可被履约链消费。
- [ ] Step 6: 跑通 OMS 后端测试与前端 E2E，输出证据清单。

## 特别注意
- 不允许通过删除逻辑或弱化校验来“测通”。
- 发现需要修改 WMS 或财务文件时，停止扩张，改为记录接口契约问题。
- 本卡片完成标准是“主链闭环 + 审计证据”，不是“页面交互看起来能点”。
