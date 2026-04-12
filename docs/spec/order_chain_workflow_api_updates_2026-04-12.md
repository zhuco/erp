# 订单主链路与接口规划（工业级版本，2026-04-12）

> 文档定位：本文件是订单主链路的“上线级蓝图”，用于统一产品、后端、前端、测试、SRE 的交付口径。  
> 关联基线：`docs/spec/04_order_status_machine.md`、`docs/spec/14_action_permission_matrix.md`、`docs/spec/15_integration_event_contract.md`、`docs/spec/security_and_audit.md`。

## 1. 目标、范围与成功标准

### 1.1 业务目标

围绕以下链路构建端到端可运营能力：

1. 网店订单入站（下载/手工建单）
2. 订单审核（自动审核 + 人工审核）
3. 复核队列（命中策略后进入人工复核）
4. 递交发货（OMS -> WMS）
5. WMS 发货（预约物流、打印、直接发货）
6. 出库验货（按单验货、一单多包、扫码校验）
7. 打包登记（配货员/打包员/包材/重量）

### 1.2 非功能目标（工业级）

1. 一致性：关键状态转换不回退，跨系统最终一致。
2. 可恢复：接口重试可幂等，异常可补偿，任务可重放。
3. 可追踪：全链路有审计日志 + 操作日志 + TraceId。
4. 可观测：具备 SLO、告警阈值、错误预算与值班 Runbook。
5. 可演进：接口契约稳定，支持灰度发布与无损回滚。

### 1.3 成功判定（验收门槛）

1. P0 接口 100% 有 OpenAPI 契约、权限定义、审计落库。
2. 写接口 100% 支持 `Idempotency-Key`（语义幂等，而非仅防重）。
3. 关键动作（审核/复核/递交发货/验货）95 分位耗时 < 300ms（不含外部系统调用）。
4. 关键链路日可用性 >= 99.9%，且具备自动重试 + 手工补偿。

## 2. 领域边界与职责划分

### 2.1 OMS 责任

1. 订单入站、审核策略决策、复核队列管理。
2. 发货前校验与向 WMS 下发履约意图。
3. 维护订单主状态与审核轨迹。

### 2.2 WMS 责任

1. 出库单、波次、拣配、验货、打包、发货执行。
2. 回传执行结果与异常。
3. 保证仓内执行事件有序、可追溯。

### 2.3 集成原则

1. OMS/WMS 间通过“命令 + 事件”解耦。
2. OMS 不直接依赖 WMS 事务，采用 Outbox + 重试 + 幂等。
3. 回传事件按“单据维度有序，跨单据并行”处理。

## 3. 端到端状态机与不变量

### 3.1 主状态建议

1. `DOWNLOADED`
2. `PENDING_REVIEW`
3. `AUTO_APPROVED` / `APPROVED`
4. `MANUAL_REVIEW`
5. `REJECTED`
6. `PENDING_OUTBOUND`
7. `PICKING` / `PACKING` / `VERIFYING`
8. `SHIPPED`
9. `CLOSED`

### 3.2 关键不变量（必须在代码中强约束）

1. 未通过审核的订单不得递交发货（`PENDING_REVIEW`/`REJECTED` 禁止 `submit-shipment`）。
2. `SHIPPED` 后仅允许售后相关子状态推进，主履约状态不得回退。
3. 验货完成后不得继续写入该包裹的扫码明细。
4. 一单多包场景中，`package_no` 必须可追溯到扫码与包材记录。
5. 审核策略变更不影响历史审核结论，仅影响后续订单。

### 3.3 反回退策略

1. 状态写入统一带 `version`（乐观锁）。
2. 回传事件落库时进行“时间戳 + 版本号 + 状态优先级”校验。
3. 旧事件或低优先级事件记录为 `ignored`，但保留审计痕迹。

## 4. 接口分层与交付现状

## 4.1 P0（契约已补齐 + 后端骨架已落地）

1. `GET /v1/tenants/{tenant_id}/oms/review-settings`
2. `PUT /v1/tenants/{tenant_id}/oms/review-settings`
3. `GET /v1/tenants/{tenant_id}/oms/orders/recheck-queue`
4. `POST /v1/tenants/{tenant_id}/oms/orders/recheck-queue/batch-review`
5. `POST /v1/tenants/{tenant_id}/oms/orders/{order_id}/submit-shipment`
6. `POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/verify/start`
7. `POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/verify/scan-item`
8. `POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/verify/finish`

### 4.2 P0 当前完成度（本轮）

1. 路由：已完成。
2. 请求/响应结构：已完成（OpenAPI + Handler 对齐）。
3. 权限校验：已完成（最小权限原则）。
4. 审计日志：已完成（审计日志 + 操作日志）。
5. 业务持久化：部分完成（当前为骨架实现，需下一阶段补齐持久化与状态推进）。

### 4.3 P1（建议新增）

1. `POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/pack/register`
2. `POST /v1/tenants/{tenant_id}/oms/review-rules/{rule_id}/simulate`
3. `POST /v1/tenants/{tenant_id}/wms/waves/{wave_id}/print-summary`

## 5. 数据模型补齐（工业级必做）

### 5.1 新增/扩展实体

1. `oms_review_settings`（或沿用 settings 表 + 固定 key）  
   字段：`tenant_id`, `key`, `value_json`, `version`, `updated_at`, `updated_by`
2. `oms_recheck_batch_job`  
   字段：`job_id`, `tenant_id`, `action`, `reason`, `requested_by`, `requested_at`, `status`
3. `oms_recheck_batch_item`  
   字段：`job_id`, `order_id`, `result`, `error_code`, `error_message`
4. `wms_verify_session`  
   字段：`session_id`, `tenant_id`, `outbound_doc_id`, `shipment_no`, `tracking_no`, `package_no`, `status`, `started_by`, `started_at`, `finished_at`
5. `wms_verify_scan_line`  
   字段：`session_id`, `barcode`, `sku_id`, `qty`, `scanned_at`, `scanner_user_id`

### 5.2 数据约束

1. `wms_verify_session(tenant_id, outbound_doc_id, package_no, status=VERIFYING)` 唯一，防止并发开启重复会话。
2. `wms_verify_scan_line` 不允许负数数量，且单行数量上限受配置保护。
3. `oms_recheck_batch_item` 全量记录成功/失败，支持补偿重试。

## 6. 幂等与并发控制

### 6.1 幂等规则

1. 所有写接口必须接收 `Idempotency-Key`。
2. 幂等键建议作用域：`tenant_id + endpoint + business_id + idempotency_key`。
3. 首次请求落 `idempotency_records`，重复请求直接回放首次响应（状态码与响应体一致）。
4. 幂等记录保留期建议 72 小时，过期后允许重新执行。

### 6.2 并发冲突处理

1. 使用 `version` 乐观锁控制关键状态写入。
2. 冲突统一返回 `409 CONFLICT`，错误码 `STATE_VERSION_CONFLICT`。
3. 批量复核接口按“逐单处理 + 局部失败可见”策略，禁止全量 silent fail。

## 7. 权限矩阵（最小授权）

1. `orders.review.settings.read`：读取审核总配置
2. `orders.review.settings.write`：修改审核总配置
3. `orders.recheck.read`：查询复核队列
4. `orders.recheck.write`：批量复核处理
5. `orders.submit`：递交发货
6. `wms.verify.write`：验货三段式操作
7. `wms.pack.register`：打包登记（P1）

> 所有接口除权限外，必须执行 `tenant`、`shop`、`warehouse` 数据域校验。

## 8. 审计、操作日志与链路追踪

### 8.1 审计最小字段

1. `tenant_id`
2. `user_id` / `actor_type`
3. `action`
4. `entity_type` + `entity_id`
5. `before_json` + `after_json`
6. `ip` + `ua`
7. `trace_id` + `request_id`
8. `occurred_at`

### 8.2 必须审计的动作

1. 审核配置变更
2. 复核批量处理
3. 递交发货
4. 验货开始/扫码/完成
5. 打包登记

### 8.3 操作日志与审计日志区别

1. 审计日志用于合规追责，不可篡改。
2. 操作日志用于业务追踪，可用于运营查询与行为分析。

## 9. 事件契约与最终一致性

### 9.1 建议事件

1. `ORDER_REVIEWED`
2. `ORDER_RECHECKED`
3. `ORDER_SUBMIT_SHIPMENT_REQUESTED`
4. `OUTBOUND_VERIFY_STARTED`
5. `OUTBOUND_VERIFY_SCANNED`
6. `OUTBOUND_VERIFIED`
7. `PACK_REGISTERED`
8. `OUTBOUND_SHIPPED`

### 9.2 事件投递原则

1. 业务事务提交后写 Outbox，再异步投递。
2. 消费端按事件 `event_id` 幂等消费。
3. 投递失败进入 DLQ，支持重放与人工处理。

## 10. 错误码与接口语义（统一化）

### 10.1 建议错误码

1. `INVALID_ARGUMENT`（400）
2. `UNAUTHORIZED`（401）
3. `FORBIDDEN`（403）
4. `NOT_FOUND`（404）
5. `STATE_VERSION_CONFLICT`（409）
6. `IDEMPOTENCY_CONFLICT`（409）
7. `PRECONDITION_FAILED`（412）
8. `INTERNAL_ERROR`（500）

### 10.2 统一错误响应结构

```json
{
  "error_code": "STATE_VERSION_CONFLICT",
  "message": "order state version conflict",
  "request_id": "req_xxx",
  "trace_id": "trace_xxx"
}
```

## 11. 可观测性、SLO 与告警

### 11.1 指标（Metrics）

1. `order_review_requests_total`（按动作和结果分维度）
2. `recheck_batch_items_total`（成功/失败）
3. `wms_verify_scan_total`（扫码成功/失败）
4. `api_latency_ms`（P50/P95/P99）
5. `idempotency_replay_total`
6. `outbox_backlog_total`
7. `dlq_messages_total`

### 11.2 SLO（建议初始值）

1. 订单审核接口可用性：99.9%
2. 递交发货接口可用性：99.9%
3. 验货扫码接口可用性：99.95%
4. P95 响应时间：< 300ms（内部依赖场景）
5. 审计日志落库成功率：>= 99.99%

### 11.3 告警策略

1. 5 分钟错误率 > 2% 触发 P2 告警。
2. 15 分钟错误率 > 5% 触发 P1 告警。
3. DLQ 累积 > 阈值（按租户）触发人工处置。
4. 审计落库失败连续 N 次触发 P1 告警。

## 12. 异常补偿与 Runbook

### 12.1 典型异常

1. 递交发货成功写库但调用 WMS 超时。
2. WMS 回传重复或乱序。
3. 验货扫码中断（网络波动/终端掉线）。
4. 批量复核部分失败。

### 12.2 补偿策略

1. 使用补偿任务表记录失败请求，按指数退避重试。
2. 所有补偿任务必须可人工重放并带操作审计。
3. 乱序回传按状态优先级丢弃或挂起，不直接覆盖主状态。

### 12.3 值班 Runbook 要求

1. 明确排障入口：日志检索、Trace、审计、DLQ、补偿任务。
2. 明确恢复步骤：冻结流量、重放事件、人工补单、状态修复。
3. 明确升级路径：产品 -> 开发 -> SRE 的升级条件与 SLA。

## 13. 测试与质量闸门

### 13.1 测试分层

1. 单元测试：参数校验、权限校验、状态守卫。
2. 仓储测试：幂等记录、版本冲突、事务一致性。
3. 集成测试：OMS -> WMS 命令与回传闭环。
4. 合约测试：OpenAPI 与 handler 响应字段一致性。
5. 压测：扫码峰值与批量复核吞吐。

### 13.2 质量门槛

1. P0 相关模块覆盖率建议 >= 80%（语句/分支）。
2. 关键状态转换必须有“正向 + 逆向 + 并发冲突”测试。
3. 新接口必须包含 403 权限拒绝测试与审计断言。
4. 上线前必须完成一次演练：乱序回传 + 幂等重放 + DLQ 重试。

## 14. 发布与回滚策略

### 14.1 发布阶段

1. 阶段 A：仅发布契约与骨架，关闭入口开关（已完成）。
2. 阶段 B：灰度租户启用持久化逻辑，观察指标与告警。
3. 阶段 C：全量启用，保留旧接口并行一段时间。
4. 阶段 D：旧接口下线评估与迁移收口。

### 14.2 灰度与开关

1. 审核配置、复核队列、验货会话分别独立 feature flag。
2. 支持按租户、仓库、渠道维度灰度。
3. 回滚优先通过开关降级，不优先回滚二进制。

## 15. 分期实施计划（建议）

### 15.1 P0（当前已完成骨架）

1. 审核总配置接口
2. 复核队列查询 + 批量复核接口
3. 显式递交发货接口
4. 验货扫码三段式接口
5. OpenAPI 契约同步、权限校验、审计日志落点

### 15.2 P0+（下一阶段必须补齐）

1. 复核批处理真实状态推进与失败明细持久化
2. 验货会话与扫码明细持久化（含一单多包）
3. `Idempotency-Key` 完整落库与响应回放
4. 乐观锁冲突返回 409 与重试策略
5. 关键链路集成测试与审计断言

### 15.3 P1

1. 打包登记增强（包材、重量、作业人维度）
2. 规则试算接口（运营上线前验证）
3. 波次汇总单打印
4. 异常补偿作业可视化（运营后台）

## 16. 与现有接口兼容策略

1. 保持 `orders/submit`、`wms/outbounds/*` 的向后兼容，不破坏旧调用。
2. 新旧接口并行发布，按租户逐步切换。
3. 旧 `operator-register` 维持基础能力，新流程优先 `pack/register`。
4. 迁移期统一输出废弃公告与切换时间线，避免无通知下线。
