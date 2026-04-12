# Observability

## Local dashboard stack
- Run `docker compose -f infra/docker-compose.observability.yml up -d`.
- Prometheus: `http://localhost:9091`
- Grafana: `http://localhost:3000` (admin/admin)
- Dashboard provisioning: `infra/observability/grafana/dashboards/erp-overview.json`
- Alert rules: `infra/observability/prometheus/alerts.yml`

## Metrics endpoints
- API: `http://<host>:${HTTP_PORT_API}/metrics`
- Worker: `http://<host>:${WORKER_METRICS_PORT:-9090}/metrics`

## Metrics
- `erp_webhook_inbox_received_total`: Inbox webhook events processed by workers.
- `erp_webhook_inbox_dedup_hit_total`: Duplicate webhook hits observed when processing inbox events.
- `erp_outbox_send_total`: Outbox envelopes delivered to workers.
- `erp_outbox_send_failed_total`: Outbox envelopes that failed in workers.
- `erp_outbox_send_retry_total`: Outbox envelopes delivered with retry attempts (attempt > 1).
- `erp_outbox_dispatch_attempt_total`: Outbox dispatch attempts (worker side).
- `erp_outbox_dispatch_failed_total`: Outbox dispatch failures.
- `erp_worker_consume_success_total`: Worker messages processed successfully.
- `erp_worker_consume_failed_total`: Worker messages that failed processing.
- `erp_worker_dlq_total`: Worker messages moved to DLQ or invalid envelopes.
- `erp_shipping_success_total`: Shipments completed successfully.
- `erp_shipping_failed_total`: Shipments failed or blocked.
- `erp_shipping_push_success_total`: Shipping push callbacks succeeded.
- `erp_shipping_push_failed_total`: Shipping push callbacks failed.
- `erp_http_request_error_total`: HTTP 5xx responses from API.
- `erp_inventory_adjust_total`: Inventory adjustments applied via API.
- `erp_inventory_reserve_total`: Inventory reservations created via API.
- `erp_inventory_consume_total`: Inventory reservations consumed during shipping.
- `erp_inventory_return_total`: Inventory reservations released via API.
- `erp_print_jobs_success_total`: Print jobs marked `SUCCESS`.
- `erp_print_jobs_failed_total`: Print jobs marked `FAILED`.

## Recommended alerts
- Worker failure rate: `erp_worker_consume_failed_total` / `erp_worker_consume_success_total` > 1% for 5m.
- Worker DLQ growth: `increase(erp_worker_dlq_total[10m]) > 0`.
- Outbox retry spike: `increase(erp_outbox_send_retry_total[10m])` above normal baseline.
- Shipping failures: `rate(erp_shipping_failed_total[5m]) / rate(erp_shipping_success_total[5m]) > 5%`.
- Inventory adjust spikes: `increase(erp_inventory_adjust_total[10m])` above baseline.
- Inventory reserve/consume imbalance: sustained `erp_inventory_reserve_total` increase without matching `erp_inventory_consume_total`.
- Print job failures: `rate(erp_print_jobs_failed_total[10m]) / rate(erp_print_jobs_success_total[10m]) > 5%`.

## Key chain alerts (order → outbound → ship → inventory → export)
> 结合 Prometheus 指标与 Grafana/Postgres 数据源进行告警，确保关键链路可追踪。

| 链路 | 指标/SQL（示例） | 阈值建议 | 说明 |
| --- | --- | --- | --- |
| 订单审核积压 | `select count(*) from orders where review_status = 'NOT_REVIEWED' and order_created_at < now() - interval '30 min';` | > 100 | Grafana SQL 告警，提示订单审核未及时处理 |
| 出库积压 | `select count(*) from outbound_docs where status <> 'SHIPPED' and created_at < now() - interval '30 min';` | > 50 | 监控出库未完成 |
| 发货确认失败 | `rate(erp_shipping_failed_total[5m]) / clamp_min(rate(erp_shipping_success_total[5m]), 1)` | > 5% 持续 5m | 已在 `infra/observability/prometheus/alerts.yml` 配置 |
| 库存调整异常 | `increase(erp_inventory_adjust_total[10m])` | > 200 | 监控异常调整峰值 |
| 导出任务失败 | `select count(*) from export_jobs where status = 'FAILED' and created_at > now() - interval '30 min';` | > 0 | Grafana SQL 告警，提示导出失败 |

## Alertmanager 通知编排
- Prometheus 告警规则维护在 `infra/observability/prometheus/alerts.yml`。
- 建议按严重级别路由通知（例：warning → Slack/飞书，critical → PagerDuty/短信）。

示例（Alertmanager 路由）：
```yaml
route:
  receiver: "slack-ops"
  group_by: ["alertname"]
  routes:
    - matchers:
        - severity="critical"
      receiver: "pagerduty"
receivers:
  - name: "slack-ops"
    slack_configs:
      - channel: "#erp-alerts"
  - name: "pagerduty"
    pagerduty_configs:
      - routing_key: "<PAGERDUTY_KEY>"
```
