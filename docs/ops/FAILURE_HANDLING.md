# 失败处理与重试

## 总体策略
- 所有失败必须落库：`status = FAILED` + `error/last_error` 字段记录原因。
- 失败分类为“可自动重试”和“需人工介入”，统一由可追踪的记录驱动重放。
- 重试入口统一走仓储方法（或 API 包装），保证幂等：相同失败记录只会重新入队一次。

## 失败类型统一处理
| 失败类型 | 记录位置 | 处理策略 | 重试入口 |
| --- | --- | --- | --- |
| 外部接口失败 | `shipments`/`outbox_events` | 标记 `FAILED`，记录 `last_error`，可按退避重试 | `ShipmentsRepo::retry_failed_shipment` / `OutboxRepo::retry_failed_outbox` |
| 入站校验失败 | `inbox_events` | 标记 `FAILED`，修复配置后重放 | `InboxRepo::retry_failed_event` |
| 限流 | `outbox_events` | `mark_failed` 设置 `next_retry_at`，等待或人工加速 | `OutboxRepo::retry_failed_outbox` |
| 超时 | `outbox_events` | 视为可重试错误，记录错误详情 | `OutboxRepo::retry_failed_outbox` |
| 打印失败 | `print_jobs` | 标记 `FAILED`，保留原 payload | `PrintJobsRepo::retry_failed_job` |
| 发货失败 | `shipments` | 标记 `FAILED`，保留请求参数 | `ShipmentsRepo::retry_failed_shipment` |
| 库存不足 | `inventory_*` | 返回业务错误，等待补货或手动调整 | 重新触发发货/出库流程 |
| 映射缺失 | `outbox_events` (`MAPPING_MISSING`) | 修复 SKU 映射后重放事件 | `InboxRepo::retry_failed_event` |

## 失败记录可见性
- `inbox_events`：`status = FAILED`，`error` 字段记录原因。
- `outbox_events`：`status = FAILED/DEAD`，`last_error` 记录原因。
- `print_jobs`：`status = FAILED`。
- `shipments`：`status = FAILED`。

## 幂等重试约定
- `retry_failed_event`/`retry_failed_job`/`retry_failed_shipment` 仅当状态为 `FAILED` 才允许重放。
- 若已存在未消费的 outbox 事件（`PENDING/FAILED/DISPATCHING`），重试不会重复入队。
- `retry_failed_outbox` 仅更新同一条 outbox 记录，避免生成新事件。
