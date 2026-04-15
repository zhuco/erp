# Industrial ERP 95%+ Round 5 Parallel Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 2026-04-15 Round 4 fresh 结果，把工业级电商 ERP 从“已跨过 95% 门槛”继续推进到“95%~96% 区间更稳、核心放行链更完整、剩余缺口更聚焦”的状态。

**Architecture:** Round 5 继续采用“5 条主线并行 + 主线程先打最关键缺口 + 每条线至少交付一份 fresh 证据”的方式推进，但与 Round 4 不同，本轮重心从“消灭持续红灯”转向“收口剩余覆盖空洞”。优先级依次为 OMS skipped / shipment 证据、权限深拒绝态、WMS 深动作链、财务经营页证据、E2E 共性稳定性模式沉淀。

**Tech Stack:** React + TypeScript + Playwright + Vitest + Rust + Axum + SQLx + PostgreSQL + pnpm + cargo

---

## 当前完成度重评估（Round 5 起点）

**总体完成度判断：** 约 `95%~96%`

**已经达到工业级门槛的部分：**
1. OMS 异常工作台、订单复核、拆单预览、结算详情已有 fresh 证据。
2. WMS 波次、现场执行、handover、出库主链已有可复用验证基础。
3. 售后收货、换补发、客服工单、逆向与轻经营链已基本可放行。
4. 发布门禁主链、跨链 smoke、observability 回归链已可作为上线门禁证据。
5. 多实体治理、库存治理和系统治理已经不再是连续红灯区域。

**仍未完全打透的工业级缺口：**
1. OMS 仍有 `skipped` 用例，订单状态筛选、发货明细下游链、合单证据还不完整。
2. 权限侧已有 happy path，但拒绝态、禁用态、跨页面可见性联动还缺更深证据。
3. WMS 深动作链虽能通过主链，但 `outbound-fulfillment-actions` 一类现场动作还缺 fresh 收口。
4. 财务经营页已有页面能力，但围绕对账 / 利润 / 发票的独立前台证据还偏薄。
5. 多个 E2E 仍依赖“工作台遮挡规避”“下拉层选择规避”等模式，需继续沉淀稳定策略。

## Round 5 五条主线

### Task 1: OMS skipped 收口与 shipment/downstream 证据补齐
**价值：** 直接决定我们对“订单处理主链已工业级完成”的结论是否站得住。

### Task 2: 权限深拒绝态与治理联动增强
**价值：** 决定系统是否具备工业级多实体 / 多角色 / 多页面治理韧性，而非仅页面可见。

### Task 3: WMS 深动作链补证据
**价值：** 决定履约现场动作链是否不只停留在 happy path，而是真正覆盖拣货、打包、验货、发货等串行动作。

### Task 4: 财务经营页独立证据补齐
**价值：** 决定“已过 95%”是只靠 release gate smoke，还是具备真实经营页前台证据。

### Task 5: 共性 E2E 稳定性模式沉淀
**价值：** 决定后续 Round 6+ 是继续被脆弱选择器拖慢，还是开始进入低噪音收口阶段。

## 并行执行原则

1. 每张任务卡只允许修改自己的白名单文件，不碰别的业务域。
2. 先复现真实缺口，再做最小修复；禁止为了“多过几条”而大范围重构。
3. 本轮继续把 `apps/erp-web/src/pages/orders/orderReviewPage.css` 视为现存用户改动，不纳入白名单。
4. `apps/erp-web/src/app/router.tsx` 已有本地改动，除非必要，不在本轮作为主修点。
5. 子代理若再次出现 `502 Bad Gateway`，立刻回收并切回主线程，不做无效重试。
6. 每条线至少写出“已补到什么证据 / 还缺什么边界”，避免只汇报通过数。
