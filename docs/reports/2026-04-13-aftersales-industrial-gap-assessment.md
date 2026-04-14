# 2026-04-13 售后工业级差距评估（客户域，2026-04-14 收敛更新）

## 1. 评估范围与目标

本评估聚焦客户域售后能力，覆盖以下业务面：

- 退货退款、换货、补发、关单
- 平台售后联动（抖店、京东、淘宝、拼多多）
- 售后与财务冲销/审计闭环
- 售后与逆向仓配（收货、质检、补发）闭环
- SLA 预警、升级、客服催办
- 可观测性与上线门禁

目标：对标工业级电商 ERP 的“可上线、可审计、可补偿、可持续运营”标准，评估当前差距并形成 Go/No-Go 结论。

## 2. 当前能力快照（基于代码与测试证据）

### 2.1 已完成能力（本轮收敛）

1. 平台退款同步可入数（4 平台）
- `pull_refunds` 不再空转，已接入退款 API/回退链路
- 京东、淘宝、拼多多已覆盖退款 API 成功、失败回退、cursor/latest_time 一致性测试
- 抖店保留退款路径与 mock 支持

2. 售后状态机与幂等基线
- 新增状态机护栏测试：非法跨状态迁移冲突、重复回调幂等、路径参数校验
- 财务联动幂等测试：同 `refund_doc` 重复提交不重复触发 outbox

3. SLA 衍生与前端可视化
- 售后列表后端增加 SLA 派生字段：
  - `sla_deadline_at`
  - `sla_status`
  - `sla_remaining_minutes`
  - `escalation_role`
- 前端列表展示 SLA 标签、剩余时间、责任角色，并提供超时催办入口

4. 可观测性售后指标
- 新增指标：
  - `erp_after_sales_sync_success_total`
  - `erp_after_sales_compensation_total`
  - `erp_after_sales_cycle_time_ms_sum`
  - `erp_after_sales_cycle_time_ms_count`
  - `erp_after_sales_sla_overdue_total`
- 系统观测页新增“售后链路”看板

5. 售后主链路 E2E（退货-质检-补发）已补齐并可执行
- 新增 `after-sales-return-resend-main-chain.spec.ts`
- 2026-04-14 本地执行通过（详见第 6 节）

### 2.2 仍需持续优化项（非本轮阻断）

1. 全量 `erp-api` 回归在当前 Windows 环境受页面文件限制失败（`os error 1455`），属于环境容量问题，不是本轮售后改动引入的功能回归。
2. 售后财务“分录级对账深度”仍有提升空间，目前已覆盖幂等触发与不重入基线。
3. 售后主链路 E2E 当前为主链路 smoke + 可选节点兼容，后续可继续提升为更强断言的全状态覆盖。

## 3. 与工业级 ERP 的差距量化（更新后）

评分标准：1（原型）- 5（工业成熟）

| 维度 | 收敛前 | 当前 | 工业目标 | 差距 |
| --- | --- | --- | --- | --- |
| 售后流程完整性 | 3.2 | 3.9 | 4.8 | 0.9 |
| 平台售后联动 | 2.0 | 4.1 | 4.8 | 0.7 |
| 财务闭环 | 2.3 | 3.5 | 4.7 | 1.2 |
| 逆向仓配协同 | 2.8 | 3.7 | 4.6 | 0.9 |
| SLA 与客服协同 | 2.6 | 4.0 | 4.5 | 0.5 |
| 可观测与审计 | 3.0 | 4.1 | 4.6 | 0.5 |

综合判断：售后域由 **2.65 / 5（53/100）提升到 3.88 / 5（约 78/100）**，已从“可操作框架”进入“可上线试运行”阶段，但距离完全工业成熟仍有一段收敛空间（主要在财务深度与全量回归能力）。

## 4. 本轮交付清单（代码）

主要变更文件如下：

- `crates/adapter-jd/src/lib.rs`
- `crates/adapter-taobao/src/lib.rs`
- `crates/adapter-pdd/src/lib.rs`
- `crates/adapter-douyin/src/lib.rs`
- `crates/adapter-jd/Cargo.toml`
- `crates/adapter-taobao/Cargo.toml`
- `crates/adapter-pdd/Cargo.toml`
- `crates/erp-api/src/lib.rs`
- `crates/erp-common/src/telemetry.rs`
- `crates/erp-worker/src/main.rs`
- `apps/erp-web/src/api/afterSalesOps.ts`
- `apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx`
- `apps/erp-web/src/pages/system/ObservabilityPage.tsx`

新增测试：

- `crates/erp-api/tests/after_sales_state_machine_guard.rs`
- `crates/erp-api/tests/after_sales_finance_reconcile.rs`
- `crates/erp-api/tests/after_sales_sla_escalation.rs`
- `apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`

## 5. 工业级门禁指标（本轮口径）

1. 同步成功率：`sync_success / (sync_success + compensation)`  
本轮状态：已具备指标与看板，阈值建议 `>= 99%`

2. 补偿率：`compensation / sync_success`  
本轮状态：已具备指标与看板，阈值建议 `<= 1%`

3. 平均处理时长：`cycle_time_ms_sum / cycle_time_ms_count`  
本轮状态：已具备指标与看板，阈值建议按租户 SLA 设上限

4. 超时率：`sla_overdue / after_sales_total`（需结合业务总量指标）  
本轮状态：已具备 `sla_overdue` 指标，建议增加分母总量指标实现精确比率

## 6. 验证证据（2026-04-14）

### 6.1 已通过

1. `cargo test -p erp-adapter-douyin -p erp-adapter-jd -p erp-adapter-taobao -p erp-adapter-pdd`
- 结果：通过

2. `cargo test -p erp-api --test after_sales_state_machine_guard --test after_sales_finance_reconcile --test after_sales_sla_escalation -- --nocapture`
- 结果：通过

3. `cargo test -p erp-common`
- 结果：通过

4. `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`
- 结果：通过（1 passed）

### 6.2 未通过（环境问题）

1. `cargo test -p erp-api -p erp-adapter-douyin -p erp-adapter-jd -p erp-adapter-taobao -p erp-adapter-pdd`
- 结果：失败
- 关键错误：`failed to mmap ... 页面文件太小，无法完成操作 (os error 1455)`
- 归因：Windows 本机页面文件容量不足导致大规模并行编译/测试失效，非本轮售后功能回归。

## 7. Go/No-Go 结论

结论：**有条件 Go（Conditional Go）**

满足 Go 的部分：
- 平台退款联动、状态机幂等、SLA 升级、售后观测、售后主链路 E2E 已具备可运行证据
- 本轮新增关键测试均通过

No-Go 条件（当前保留）：
- 全量 `erp-api` 测试矩阵尚未在当前执行环境完成（受 `os error 1455` 阻塞）

上线建议：

1. 生产发布前在 CI 或更高资源机器重跑全量回归命令。
2. 将 `aftersales-go-live-gate.md` 作为售后上线门禁，按条目打勾验收。
3. 下一迭代补齐售后财务分录级对账断言和 SLA 分母指标。

## 8. 关联文档

- `docs/superpowers/plans/2026-04-13-aftersales-industrial-gap-closure.md`
- `docs/reports/aftersales-go-live-gate.md`
