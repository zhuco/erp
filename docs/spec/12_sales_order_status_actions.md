# 销售单状态聚合与动作返回规范
> 基于现有 `OmsOrderStatus`、`OrderReviewStatus`、`exception_flags` 聚合主状态，统一驱动前端动作展示。
> 注意：此处 `sales_status` 为 OMS 聚合状态，销售单状态机见 `docs/spec/SALES_ORDER_STATE_MACHINE.md`。

## 1. 目标与范围
- **目标**：为销售单提供稳定的主状态 `sales_status` 与 `available_actions`，避免前端散落判断。
- **范围**：订单审核、OMS 履约、异常处理、拆单/包裹相关动作。
- **前提**：允许反审核、先发后收、支持部分发货/拆单，作废/关闭仅依赖权限控制。

## 2. 字段定义
### 2.1 主状态 `sales_status`
| 状态 | 含义 |
| --- | --- |
| `NEW` | 新建/待进入审单 |
| `REVIEW_PENDING` | 待审核 |
| `REVIEW_REJECTED` | 审核驳回 |
| `ON_HOLD` | 异常/冻结 |
| `READY_TO_SHIP` | 已审待发 |
| `FULFILLING` | 履约中（分配/拣货/打包/部分发货） |
| `SHIPPED` | 已发货 |
| `DONE` | 已完成 |
| `CANCELLED` | 已取消 |
| `UNKNOWN` | 未识别兜底 |

### 2.2 状态标记 `status_flags`
用于补充“主状态无法表达的细节”，不参与主状态判定。
| 标记 | 说明 | 典型来源 |
| --- | --- | --- |
| `HAS_EXCEPTION` | 有异常命中 | `exception_flags` 非空 |
| `IS_FROZEN` | 订单冻结 | `freeze_reason` 非空 |
| `HAS_SPLIT` | 存在拆单 | 拆单记录/子单数 |
| `PARTIAL_SHIPPED` | 部分发货 | 包裹状态汇总 |

### 2.3 可用动作 `available_actions`
后端返回动作列表，前端只依赖动作渲染按钮。
```json
{
  "sales_status": "READY_TO_SHIP",
  "status_flags": ["HAS_EXCEPTION"],
  "available_actions": [
    { "code": "review_pass", "label": "审核通过", "permission": "orders.write" },
    { "code": "order_cancel", "label": "作废", "permission": "orders.write", "need_reason": true }
  ]
}
```

## 3. 主状态聚合规则（优先级）
按优先级从上到下命中：
1. `oms_status == CANCELLED` → `CANCELLED`
2. `review_status == REJECTED` → `REVIEW_REJECTED`
3. `exception_flags 非空` 或 `freeze_reason 非空` → `ON_HOLD`
4. `review_status == PENDING` 或 `oms_status == REVIEW_PENDING` → `REVIEW_PENDING`
5. `oms_status == NEW` → `NEW`
6. `oms_status == PENDING_SHIPMENT` → `READY_TO_SHIP`
7. `oms_status in [ALLOCATED,PICKING,PACKED]` → `FULFILLING`
8. `oms_status == SHIPPED` → `SHIPPED`
9. `oms_status == DONE` → `DONE`
10. 其它 → `UNKNOWN`

**部分发货处理**
- 若存在包裹明细，出现 `PARTIALLY_SHIPPED` 则强制为 `FULFILLING`。
- 所有包裹均为 `SHIPPED` 时返回 `SHIPPED`。

## 4. 动作与接口映射（建议）
| action code | 说明 | 典型状态 | 权限 | 接口 |
| --- | --- | --- | --- | --- |
| `review_pass` | 审核通过 | `REVIEW_PENDING` | `orders.write` | `POST /orders/{id}/review` |
| `review_reject` | 审核驳回 | `REVIEW_PENDING` | `orders.write` | `POST /orders/{id}/review` |
| `review_flag` | 标记异常 | `REVIEW_PENDING` | `orders.write` | `POST /orders/{id}/review` |
| `review_force` | 强制审核 | `REVIEW_PENDING` | `orders.force_review` | `POST /orders/{id}/force-review` |
| `review_return` | 退回拉单 | `REVIEW_PENDING` | `orders.write` | `POST /orders/{id}/return-to-download` |
| `review_requeue` | 重新审单 | `REVIEW_PENDING` | `orders.write` | `POST /orders/{id}/review/requeue` |
| `order_cancel` | 作废 | 全部 | `orders.write` | `POST /orders/{id}/cancel` |
| `shipment_allocate` | 分配库存 | `READY_TO_SHIP` | `shipping.write` | `POST /oms/orders/{id}/allocate` |
| `shipment_create` | 生成包裹 | `READY_TO_SHIP` | `shipping.write` | `POST /oms/orders/{id}/shipments` |
| `shipment_release` | 释放库存 | `READY_TO_SHIP` | `shipping.write` | `POST /oms/orders/{id}/release` |
| `shipment_ship` | 包裹发货 | `FULFILLING` | `shipping.write` | `POST /oms/shipments/{id}/ship` |
| `shipment_resend` | 补发 | `FULFILLING` | `shipping.write` | `POST /oms/orders/{id}/resend` |
| `shipment_cancel_request` | 撤销包裹 | `FULFILLING` | `shipping.write` | `POST /oms/shipments/{id}/cancel-request` |
| `exception_handle` | 异常处理 | `ON_HOLD` | `orders.write` | `POST /orders/{id}/exceptions/handle` |

## 5. 先发后收说明
- `payment_status` 不影响主状态流转，仅作为财务子状态展示。
- 结案/完成由 `oms_status` 与履约回传驱动，不等待收款。

## 6. 前端接入规则
- 前端仅根据 `available_actions` 决定按钮可见/可用。
- 标签/状态展示优先使用 `sales_status`，避免混用 `oms_status`。
