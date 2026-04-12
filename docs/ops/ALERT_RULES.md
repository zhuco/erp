# 告警规则（Alert Rules）

## 1. API 可用性

1. `api_error_rate_high`
- 条件：5 分钟窗口错误率 > 2%
- 等级：P1
- 动作：电话 + IM + 值班升级

2. `api_p95_latency_high`
- 条件：5 分钟窗口 P95 > 800ms
- 等级：P2
- 动作：IM 通知 + 自动采样分析

## 2. 数据与队列

1. `db_connection_exhausted`
- 条件：连接池使用率 > 90% 持续 3 分钟
- 等级：P1

2. `outbox_backlog_high`
- 条件：Outbox backlog > 5000 持续 10 分钟
- 等级：P2

3. `dlq_growth_abnormal`
- 条件：DLQ 每分钟新增 > 100 持续 5 分钟
- 等级：P1

## 3. 平台接入

1. `webhook_verify_fail_spike`
- 条件：Webhook 验签失败率 > 5% 持续 5 分钟
- 等级：P2

2. `oauth_refresh_fail`
- 条件：Token 刷新连续失败 >= 3 次
- 等级：P2

## 4. 统计与身份

1. `clickhouse_unavailable`
- 条件：`/stats/health` 返回 `clickhouse_ready=false` 且配置了 ClickHouse URL
- 等级：P2

2. `sso_unavailable`
- 条件：`/stats/health` 返回 `sso_ready=false` 且配置了 Keycloak Issuer
- 等级：P1

## 5. 通知策略

- P1：立即电话 + IM + 15 分钟未恢复自动升级
- P2：IM + 工单，30 分钟未恢复升级
- P3：仅工单与日报汇总
