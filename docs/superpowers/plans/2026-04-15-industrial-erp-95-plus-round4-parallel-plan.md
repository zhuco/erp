# Industrial ERP 95%+ Round 4 Parallel Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 2026-04-15 Round 3 执行日志，把工业级电商 ERP 从约 94%~95% 推进到“稳定超过 95%，且核心放行链不依赖脆弱选择器与偶发环境噪音”的状态。

**Architecture:** Round 4 不再扩展新模块，而是聚焦 5 条最接近上线门槛的深水区链路：OMS 深链、权限拒绝态联动、WMS 现场执行、售后/客服稳定性、财务经营与发布门禁。策略上采用“继续白名单并行 + 一条主线至少一份 fresh 证据 + 对脆弱 E2E 优先做根因收敛”的方式，避免继续堆表面通过数。

**Tech Stack:** React + TypeScript + Playwright + Vitest + Rust + Axum + SQLx + PostgreSQL + pnpm + cargo

---

## 当前完成度重评估（Round 4 起点）

**总体完成度判断：** 约 `94%~95%`

**已经较稳的部分：**
1. 多实体权限 UI 已有 `multi-entity-governance`、`system-governance`、`workbench-menu-navigation` fresh 证据。
2. OMS 异常工作台已有 `exceptions workbench supports search and detail focus` fresh 证据。
3. WMS 波次、出库主链与 handover 回归已有 fresh 证据。
4. 售后逆向录入、质检、SLA、批量导入与轻经营链已有 fresh 证据。
5. Observability 运维工作流两组 Playwright 已 fresh 全绿。

**冲刺 95%+ 仍差的关键能力：**
1. OMS 前台仍缺拆单 / 合单 / 结算主链的成组 fresh 证据。
2. 权限治理仍缺拒绝态、403 提示和跨模块数据可见性联动验证。
3. WMS 仍缺现场执行 / PDA / dock-yard / slotting 类工业现场证据。
4. 售后收货换补发与客服工单仍有脆弱 E2E，影响稳定放行。
5. 财务对账 / 利润与 release gate 还缺一组更明确的经营闭环与放行门禁证据。

## 本轮 5 条主线

### Task 1: OMS 拆单 / 合单 / 结算前台主链
**价值：** 把 OMS 从“异常工作台可用”推进到“订单处理主链可放行”。

### Task 2: 权限拒绝态与跨模块可见性联动
**价值：** 决定平台是否具备工业级多实体治理，而非仅有表单字段。

### Task 3: WMS 现场执行与出库确认深链
**价值：** 决定履约链是否真正覆盖工业现场，而不只是 happy path 页面可见。

### Task 4: 售后收货 / 换补发与客服工单稳定性
**价值：** 决定逆向链能否稳定放行，避免脆弱 E2E 成为上线门槛短板。

### Task 5: 财务经营闭环与发布门禁回归
**价值：** 决定项目是否具备“利润 / 对账 / 跨链 smoke gate”级别的上线证据。

## 并行执行原则

1. 每张任务卡只允许改自己的白名单文件。
2. 一律先跑失败用例或先复现真实缺口，再做最小修复。
3. 每条线至少交付一条 fresh 验证结果，优先 2 条以上。
4. `apps/erp-web/src/pages/orders/orderReviewPage.css` 继续视为现存本地改动，不纳入本轮白名单。
5. Playwright 端口占用继续作为环境噪音治理对象，但不能掩盖真实逻辑失败。
6. 若某条线连续 3 次最小修复仍未收口，必须停止堆补丁，改写日志并转为下一轮专项任务。
