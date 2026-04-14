# 售后 Go-Live Gate（工业级门禁清单）

> 适用范围：客户域售后（退货退款、换货、补发、平台联动、财务联动、SLA 协同）  
> 更新时间：2026-04-14  
> 责任团队：售后产品 + 平台适配 + ERP API + Worker + QA

## 1. 发布目标

本门禁用于在发布前确认售后链路具备工业级上线最小条件：

- 可入数：平台退款同步不空转
- 可闭环：状态机、财务、逆向链路可追踪
- 可补偿：异常可识别、可重试、可人工兜底
- 可观测：关键 KPI 可看、可告警
- 可回归：关键 API/E2E 有自动化证据

## 2. KPI 门槛（P0）

| 指标 | 公式 | 门槛 | 数据源 |
| --- | --- | --- | --- |
| 同步成功率 | `sync_success / (sync_success + compensation)` | `>= 99%` | `erp_after_sales_sync_success_total`、`erp_after_sales_compensation_total` |
| 补偿率 | `compensation / sync_success` | `<= 1%` | 同上 |
| 平均处理时长 | `cycle_time_ms_sum / cycle_time_ms_count` | 按租户 SLA 配置，不高于基线 | `erp_after_sales_cycle_time_ms_sum/count` |
| SLA 超时告警 | `sla_overdue > 0` 必须可见且可催办 | 100% 可见 | `erp_after_sales_sla_overdue_total` + 售后列表超时标记 |

## 3. 功能门禁（P0/P1）

### 3.1 P0 必须通过

- [x] 4 平台退款同步可入数（抖店/京东/淘宝/拼多多）
- [x] 退款 API 成功/失败回退路径可测
- [x] `cursor/latest_time` 一致性可测
- [x] 售后状态机非法跃迁拦截（防乱序/跨状态跳变）
- [x] 退款重复回调幂等（不重复触发）
- [x] 财务联动幂等（重复退款请求不重复触发出账）
- [x] SLA 超时识别 + 升级责任角色 + 前端催办入口
- [x] 退货-质检-补发主链路 E2E 可执行

### 3.2 P1 建议通过（发布后首周内补齐）

- [ ] 财务分录级对账断言（按凭证维度核对）
- [ ] SLA 超时率分母指标（售后总量）并入看板
- [ ] 逆向链路异常补偿回放演练（演练脚本与操作手册）

## 4. 自动化验证证据（2026-04-14）

### 4.1 已通过命令

1. `cargo test -p erp-adapter-douyin -p erp-adapter-jd -p erp-adapter-taobao -p erp-adapter-pdd`
2. `cargo test -p erp-api --test after_sales_state_machine_guard --test after_sales_finance_reconcile --test after_sales_sla_escalation -- --nocapture`
3. `cargo test -p erp-common`
4. `pnpm --filter erp-web exec playwright test apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`

### 4.2 受环境阻塞命令

1. `cargo test -p erp-api -p erp-adapter-douyin -p erp-adapter-jd -p erp-adapter-taobao -p erp-adapter-pdd`
- 状态：未通过
- 阻塞原因：Windows 页面文件不足（`os error 1455`）
- 处置：在 CI 或高资源机器重跑全量矩阵后再签署最终 Go

## 5. 发布判定规则

1. Go（可发布）
- 4.1 全部通过
- 4.2 阻塞项在 CI/高资源机器复跑通过
- KPI 指标接入并可在观测页读取

2. Conditional Go（有条件发布）
- 4.1 全部通过
- 4.2 仅受环境资源阻塞，且已给出替代验证与复跑计划

3. No-Go（禁止发布）
- P0 任一条失败
- 关键链路出现“接口成功但无状态落库/无审计记录”
- 关键测试无法复现通过且无清晰归因

## 6. 当前结论（2026-04-14）

- 当前门禁状态：**Conditional Go**
- 主要依据：
  - 本轮售后核心功能与新增测试均通过
  - 全量测试失败归因到执行环境页面文件不足，不是售后改动引入回归
- 发布前动作：
  1. 在 CI 或高资源机重跑全量 `cargo test` 矩阵
  2. 复核售后观测页指标曲线（同步成功率、补偿率、处理时长、超时量）
  3. 按本清单签署最终 Go/No-Go
