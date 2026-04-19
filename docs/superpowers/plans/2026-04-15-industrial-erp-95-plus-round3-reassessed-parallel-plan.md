# Industrial ERP 95%+ Round 3 Reassessed Parallel Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 2026 年 4 月 15 日当前主仓状态，围绕工业级电商 ERP 的核心交付标准，把项目从约 92%~93% 推进到 95%+ 所需的“核心业务链可验证、关键异常链可解释、运维证据可放行”状态。

**Architecture:** 本轮采用“重新评估缺口 -> 聚焦 5 条最影响 95%+ 的主线 -> 白名单并行推进 -> 每条线都要带 fresh 验证”的策略。优先收口工业 ERP 最关键的治理、OMS、WMS、售后经营会计、运维观测五条链，而不是继续扩散零散页面。

**Tech Stack:** React + TypeScript + Playwright + Vitest + Rust + Axum + SQLx + PostgreSQL + pnpm + cargo

---

## 当前完成度重评估（2026-04-15）

**总体完成度判断：** 约 `92%~93%`

**已经较稳的部分：**
1. `Track-01` 工作台导航、模块搜索、店铺订单查询入口已 fresh 通过。
2. `Track-03` OMS 后端 `oms_orders` 已 fresh 通过 `20 passed`。
3. `Track-04` WMS 主链 `outbound-fulfillment-flow` 与 `wms-wave-main-chain` 已 fresh 通过。
4. `Track-05` 售后状态机守卫已 fresh 通过 `3 passed`。

**离 95%+ 仍差的关键能力：**
1. 多实体权限 UI 还缺业务级 E2E 证据。
2. OMS 前端工作台还缺审核/异常/查询主链的 fresh E2E。
3. WMS 还缺异常路径与失效同步回归。
4. 售后与经营会计链缺少 fresh 前端/端到端证据。
5. Observability 还缺运维视角工作流级证据。

## 本轮 5 条主线

### Task 1: 多实体权限 UI 业务链
**价值：** 决定平台治理是否达到工业级多主体可用标准。

### Task 2: OMS 前端工作台主链
**价值：** 决定订单中台是否从“后端稳”真正走到“前台可验证”。

### Task 3: WMS 异常与失效回归
**价值：** 决定履约链是否具备工业级异常处理能力，而不仅是 happy path。

### Task 4: 售后与经营会计链
**价值：** 决定项目是否具备完整闭环，而不是只有正向订单链。

### Task 5: Observability 运维工作流
**价值：** 决定项目能否被运维和发布门禁解释与放行。

## 并行执行原则

1. 每张任务卡只允许改自己的白名单文件。
2. 一律先写失败测试或先复现失败命令，再做修复。
3. 每条线至少交付一条 fresh 验证结果。
4. `apps/erp-web/src/pages/orders/orderReviewPage.css` 继续视为现存本地改动，不纳入本轮白名单。
5. Rust 测试里的 `pool timed out while waiting for an open connection` 继续单独记为环境噪音，但不能掩盖真实逻辑失败。
