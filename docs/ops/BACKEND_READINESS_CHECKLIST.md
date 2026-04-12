# 后端上线必备功能检查

评分标准：PASS=已覆盖关键流程，WEAK=部分覆盖，MISSING=未覆盖。

## 检查清单

| 项目 | 评分 | 代码证据 | 备注 |
| --- | --- | --- | --- |
| 订单链路（inbox/outbox/worker） | PASS | `crates/erp-channel/src/main.rs`<br>`crates/erp-repo/src/inbox.rs`<br>`crates/erp-repo/src/outbox.rs`<br>`crates/erp-outbox/src/dispatcher.rs`<br>`crates/erp-worker/src/main.rs` | Webhook → Inbox → Outbox → Dispatcher → Worker 完整链路 |
| 幂等 | WEAK | `crates/erp-repo/src/inbox.rs`<br>`crates/erp-repo/src/shipments.rs`<br>`crates/erp-repo/src/inventory.rs` | 有去重/唯一约束，但缺少接口级幂等键或全局请求去重 |
| 乱序 | PASS | `crates/erp-repo/src/orders/store.rs` | upsert 使用 `platform_updated_at` 过滤旧数据写入 |
| 失败重试 | WEAK | `crates/erp-outbox/src/dispatcher.rs`<br>`crates/erp-repo/src/shipments.rs` | Outbox 有重试/退避，但 worker 处理失败无重投机制 |
| 库存流水 | PASS | `crates/erp-repo/src/inventory.rs`<br>`crates/erp-repo/src/after_sale_cases.rs` | RESERVE/SHIP/RETURN/ADJUST 全流程流水 |
| 多仓 | PASS | `crates/erp-repo/src/warehouses.rs`<br>`crates/erp-repo/src/inventory.rs`<br>`crates/erp-repo/src/shipments.rs` | warehouse_id 贯穿库存与发货 |
| 发货幂等 | PASS | `crates/erp-repo/src/shipments.rs`<br>`crates/erp-worker/src/main.rs`<br>`crates/erp-repo/src/inventory.rs` | 发货请求/库存扣减具备幂等保护 |
| 退款入库幂等 | PASS | `crates/erp-repo/src/after_sale_cases.rs`<br>`migrations/0009_inventory_return_idempotency.sql` | 退货入库唯一约束防重复入账 |
| 打印任务/审计 | WEAK | `crates/erp-repo/src/print_jobs.rs`<br>`crates/erp-api/src/lib.rs` | 有任务与状态流转，但缺少操作审计与结果回溯 |
| RBAC / tenant 隔离 | WEAK | `crates/erp-api/src/lib.rs`<br>`crates/erp-repo/src/auth.rs` | 多数接口有权限/租户校验，但订单相关权限标注不足 |
| 可观测（指标/日志） | PASS | `crates/erp-common/src/telemetry.rs`<br>`infra/observability/prometheus/alerts.yml`<br>`docs/ops/OBSERVABILITY.md` | 指标+告警规则与通知编排已落地 |
| 回退开关（feature flags） | PASS | `docs/ops/FEATURE_FLAGS.md`<br>`crates/erp-repo/src/feature_flags.rs`<br>`crates/erp-api/src/lib.rs` | READ_ONLY 等开关可用于回退 |
| 数据保留与备份建议 | PASS | `docs/ops/DATA_RETENTION.md`<br>`crates/erp-worker/src/jobs/purge.rs`<br>`scripts/backup_postgres.sh` | 保留策略与清理脚本已明确 |

## WEAK 项补强建议

### 幂等
1. 为写接口补充幂等键或请求去重（参考 `crates/erp-repo/src/idempotency.rs`）。
2. 在高频写入（发货、库存）链路补测试覆盖。

### 失败重试
1. 为 worker 处理失败增加重投/退避策略与告警。
2. 对 DLQ 回放补充自动化流程。
