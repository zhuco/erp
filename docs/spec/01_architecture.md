# 架构概要（SaaS 多店铺 / 日百万单 / 平台订单为准）

## 核心原则
1) **平台订单为准**：订单投影仅反映平台最新状态；ERP 侧只做增强（审计、聚合、对账、履约任务）
2) **幂等优先**：Webhook、Polling、重试都可能产生重复与乱序，系统必须从数据层开始幂等。
3) **Webhook + Polling 补偿**：Webhook 追求低延迟；Polling/对账用于补偿丢失与一致性校验。
4) **Inbox/Outbox**：入站事件落库（可审计、可重放）；出站事件用 outbox 保证“写 DB 与发消息”的一致性。

## 本仓库当前实现的通路
- erp-channel：
  - 接收 `/webhooks/{platform}/{shop_id}` 
  - raw 入库 -> inbox_dedup 幂等 -> inbox_events -> outbox_events(INBOX_EVENT_RECEIVED)
- erp-dispatcher：
  - 扫描 outbox_events -> 发布 NATS subject `erp.outbox.{tenant}.{event_type}`
- erp-worker：
  - 订阅 `erp.outbox.>` -> 处理 INBOX_EVENT_RECEIVED：
    - verify_webhook -> parse order -> upsert orders + order_items
    - outbox_events(ORDER_UPSERTED)

## 必须补齐（市场级）
- 四平台正式验签/鉴权/字段映射（任务卡：INBOX-002 & 平台任务）
- 失败补偿重放（WORKER-RETRY-001）
- 订单查询/导出/权限细化（ORDERS-002 & RBAC-001）
