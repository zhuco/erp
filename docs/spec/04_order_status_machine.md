# 统一订单状态机（OrderStatus）

> 口径说明（2026-04-10）：本文件描述“平台订单映射与投影”基线。  
> 涉及销售单动作权限与 2026-04-07 之后的接口变更时，请与 `docs/spec/12_sales_order_status_actions.md`、`docs/spec/14_action_permission_matrix.md`、`docs/spec/order_workflow_api_updates_2026-04-07.md` 联合阅读；若冲突，以后者为准。

> 平台订单为准：我们不“推测”状态，只做**映射**与**投影**。

---

## 1. 状态定义

- `PENDING_PAYMENT`：订单已创建，未支付
- `PENDING_SHIPMENT`：已支付，待发货/备货
- `PARTIALLY_SHIPPED`：部分发货（多包裹/多子单）
- `SHIPPED`：已发货，待签收/确认收货
- `COMPLETED`：交易成功结束
- `CANCELLED`：取消（通常未完成交易闭环，可能未支付或支付前取消）
- `CLOSED`：关闭（平台系统关闭/超时关闭/退款关闭等）
- `REFUNDING`：退款中/售后处理中
- `REFUNDED`：退款完成（全额/主要金额退款完成）
- `ON_HOLD`：冻结/暂停/风控锁定
- `UNKNOWN`：无法识别的状态

> 注意：`CANCELLED` 与 `CLOSED` 在不同平台语义不同；本项目以“便于运营”角度做默认划分，上线前可按业务要求调整。

---

## 2. 统一状态转移（推荐）

> 这是“推荐状态机”，并不用于强约束写入；写入时以平台状态映射为准。

```
PENDING_PAYMENT
  ├──> CANCELLED
  └──> PENDING_SHIPMENT

PENDING_SHIPMENT
  ├──> PARTIALLY_SHIPPED
  ├──> SHIPPED
  ├──> REFUNDING
  └──> CLOSED

PARTIALLY_SHIPPED
  ├──> SHIPPED
  ├──> REFUNDING
  └──> CLOSED

SHIPPED
  ├──> COMPLETED
  ├──> REFUNDING
  └──> CLOSED

REFUNDING
  ├──> REFUNDED
  └──> (回退到 SHIPPED/PENDING_SHIPMENT 视平台而定，默认不回退)

REFUNDED
  └──> CLOSED (可选：有些平台最终状态显示为 CLOSED)

ON_HOLD
  ├──> PENDING_SHIPMENT / SHIPPED / CLOSED (视平台恢复/关闭)
  └──> (保持)
```

---

## 3. 事件驱动写模型建议

- 订单投影表（orders）只存当前快照：`status/amount/.../raw`。
- 历史轨迹建议追加：`order_events`（TODO）：记录每次平台状态变化、时间戳、触发源（webhook/poll/manual）。
- 对“退款中 -> 退款完成”的识别，建议优先读平台售后/逆向接口，而不是仅看订单状态。

---

## 4. 平台侧“多维状态”处理约定

### 4.1 拼多多 PDD

PDD 同时存在多组状态字段（成交/发货/售后/风控）。为了让 ERP 只展示一个“主状态”，本仓库采用：

1) 退款成功/退款中优先（`refund_status` / `after_sales_status`）
2) 风控/换货补寄/维修等“需人工关注”优先（映射为 `ON_HOLD`）
3) 再回落到成交状态（`confirm_status`）与发货状态（`order_status`）

详见：`docs/spec/03_order_field_mapping.md` 的 PDD 章节。

### 4.2 抖店 Douyin

抖店以 `order_status` 为主枚举；但风控订单需要额外读取 `sku_order_tag_ui.key` 是否为 `risk_processing`：

- 若风控中：映射为 `ON_HOLD`
- 否则：按 `order_status` 映射

详见：`docs/spec/03_order_field_mapping.md` 的 Douyin 章节。
