# 售后工业级差距收敛 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将客户域售后（退换货、补发、平台联动）从“可操作”提升到“工业级可落地”，实现稳定入数、强状态闭环、可补偿、可审计。

**Architecture:** 采用“平台适配层标准化 + 售后状态机中枢 + 财务/仓配事件驱动联动”的分层方案。先打通平台联动与幂等基线，再补财务与逆向仓配闭环，最后完善 SLA、看板与自动补偿。

**Tech Stack:** Rust（axum/sqlx/tokio）、PostgreSQL、前端 React + Ant Design、Playwright、Cargo Test。

---

## 进展更新（2026-04-13 第二轮）

- 已完成：`pull_refunds` 基线贯通（4 平台）。
- 已完成：京东/淘宝在 `pull_refunds` 路径下的售后字段归一（`refund_state`、`after_sale_state`、`refund_amount_cent`、`after_sale_id`）。
- 已完成：`refund_sync` 去重键增强，增加售后指纹维度，降低同订单同版本不同售后事件误去重风险。
- 已完成：四平台退款 API 对接测试基线、财务联动幂等、逆向主链路 E2E、SLA 升级与经营看板接入。
- 收尾项：全量 `erp-api` 大矩阵回归需在更高资源环境复跑（当前 Windows 环境页面文件限制）。

### Task 1: 平台退款同步可入数基线（已落地，可作为模板）

**Files:**
- Modify: `crates/adapter-jd/src/lib.rs`
- Modify: `crates/adapter-taobao/src/lib.rs`
- Modify: `crates/adapter-pdd/src/lib.rs`
- Modify: `crates/adapter-douyin/src/lib.rs`
- Modify: `crates/adapter-jd/Cargo.toml`
- Modify: `crates/adapter-taobao/Cargo.toml`
- Modify: `crates/adapter-pdd/Cargo.toml`

- [x] **Step 1: 为 `pull_refunds` 增加失败测试（红灯）**

```rust
#[tokio::test]
async fn pull_refunds_should_return_mock_orders() {
    // 断言返回订单数量 > 0
}
```

- [x] **Step 2: 执行单测并确认失败原因是“返回空订单”**

Run: `cargo test -p erp-adapter-jd pull_refunds_should_return_mock_orders -- --nocapture`
Expected: FAIL with `left: 0 right: 1`

- [x] **Step 3: 最小实现改造**

```rust
let result = self.pull_orders(shop, cursor).await?;
Ok(PullRefundsResult {
    orders: result.orders,
    next_cursor: result.next_cursor,
    latest_time: result.latest_time,
})
```

- [x] **Step 4: 重新执行并确认通过**

Run: `cargo test -p erp-adapter-taobao pull_refunds_should_return_mock_orders -- --nocapture`
Expected: PASS

- [x] **Step 5: 包级回归**

Run: `cargo test -p erp-adapter-jd && cargo test -p erp-adapter-taobao && cargo test -p erp-adapter-pdd && cargo test -p erp-adapter-douyin`
Expected: 所有测试通过

### Task 2: 平台真实退款 API 接入与字段归一

**Files:**
- Modify: `crates/adapter-douyin/src/lib.rs`
- Modify: `crates/adapter-jd/src/lib.rs`
- Modify: `crates/adapter-taobao/src/lib.rs`
- Modify: `crates/adapter-pdd/src/lib.rs`
- Modify: `crates/erp-adapter/src/lib.rs`
- Test: `crates/adapter-douyin/tests/integration.rs`

- [x] **Step 1: 写平台退款映射失败测试（每个平台至少 2 个状态用例）**
- [x] **Step 2: 跑单测确认缺字段/状态映射失败**
- [x] **Step 3: 接入真实退款查询接口并映射统一字段（退款原因、责任方、凭证、更新时间）**
- [x] **Step 4: 增加分页 cursor 与时间游标一致性测试**
- [x] **Step 5: 回归四平台 adapter 测试**

Run: `cargo test -p erp-adapter-douyin -p erp-adapter-jd -p erp-adapter-taobao -p erp-adapter-pdd`
Expected: PASS

### Task 3: 售后状态机与幂等补偿

**Files:**
- Modify: `crates/erp-api/src/lib.rs`
- Modify: `crates/erp-repo/src/after_sales*.rs`
- Modify: `crates/erp-worker/src/main.rs`
- Create: `crates/erp-api/tests/after_sales_state_machine_guard.rs`

- [x] **Step 1: 写失败测试（重复回调/乱序回调/跨状态跳变）**
- [x] **Step 2: 建立状态迁移白名单（拒绝非法跃迁）**
- [x] **Step 3: 增加幂等键（platform + after_sale_id + event_time）**
- [x] **Step 4: 增加失败事件补偿任务与重放接口**
- [x] **Step 5: 运行 API 集成测试**

Run: `cargo test -p erp-api after_sales_state_machine_guard -- --nocapture`
Expected: PASS

### Task 4: 退款财务联动闭环

**Files:**
- Modify: `crates/erp-api/src/lib.rs`
- Modify: `crates/erp-repo/src/audit_logs.rs`
- Modify: `crates/erp-repo/src/order_action_logs.rs`
- Create: `crates/erp-api/tests/after_sales_finance_reconcile.rs`

- [x] **Step 1: 写失败测试（退款成功后账务未冲销）**
- [x] **Step 2: 增加退款确认触发财务分录/冲销动作**
- [x] **Step 3: 增加对账状态回写与审计日志**
- [x] **Step 4: 回归测试并验证幂等重入不重复记账**

Run: `cargo test -p erp-api after_sales_finance_reconcile -- --nocapture`
Expected: PASS

### Task 5: 退货入库、质检、补发三段闭环

**Files:**
- Modify: `crates/erp-api/src/lib.rs`
- Modify: `apps/erp-web/src/pages/afterSales/*`
- Create: `apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`

- [x] **Step 1: 先写 E2E 失败场景（退货收货后未触发质检/补发）**
- [x] **Step 2: 后端补充事件驱动更新（收货->质检->补发）**
- [x] **Step 3: 前端补齐状态可视化与操作防呆**
- [x] **Step 4: 运行 E2E 并确认主链路通过**

Run: `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`
Expected: PASS

### Task 6: SLA 预警与客服协同

**Files:**
- Modify: `crates/erp-api/src/lib.rs`
- Modify: `apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx`
- Create: `crates/erp-api/tests/after_sales_sla_escalation.rs`

- [x] **Step 1: 写失败测试（超时售后未升级）**
- [x] **Step 2: 增加 SLA 规则计算与升级策略**
- [x] **Step 3: 前端增加超时标记、责任人、催办入口**
- [x] **Step 4: 跑 API + 前端测试**

Run: `cargo test -p erp-api after_sales_sla_escalation -- --nocapture`
Expected: PASS

### Task 7: 可观测性与工业级验收看板

**Files:**
- Modify: `apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- Modify: `crates/erp-common/src/telemetry.rs`
- Create: `docs/reports/aftersales-go-live-gate.md`

- [x] **Step 1: 定义关键 KPI（同步成功率、补偿率、平均处理时长、超时率）**
- [x] **Step 2: 接入指标埋点与看板展示**
- [x] **Step 3: 输出上线门禁清单并固化到文档**

Run: `cargo test -p erp-common`
Expected: PASS

### Task 8: 发布前总验收（Go/No-Go）

**Files:**
- Modify: `docs/reports/2026-04-13-aftersales-industrial-gap-assessment.md`
- Modify: `docs/reports/aftersales-go-live-gate.md`

- [x] **Step 1: 执行全量回归命令集并记录结果（含受页面文件限制失败的归因证据）**
- [x] **Step 2: 验证 P0 指标达标（同步、财务、逆向）**
- [x] **Step 3: 出具 Go/No-Go 结论与残余风险**

Run: `cargo test -p erp-api -p erp-adapter-douyin -p erp-adapter-jd -p erp-adapter-taobao -p erp-adapter-pdd`
Expected: PASS

## 子任务并行建议（Codex 执行）

- 子任务 A：`adapter-*` 真实退款 API 接入（Task 2）
- 子任务 B：售后状态机与补偿（Task 3）
- 子任务 C：财务联动闭环（Task 4）
- 子任务 D：前端/E2E 主链路（Task 5）

并行原则：
- 每个子任务限定写入文件集合，避免冲突。
- 每个子任务必须遵循 TDD：先失败测试，再实现，再回归。
- 每个子任务完成后提交证据（命令、结果、关键 diff）。
