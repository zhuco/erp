# 销售/履约/售后动作与权限矩阵

> 目标：统一动作码、权限与幂等规则，避免前后端散落判断。状态机对齐 `docs/spec/SALES_ORDER_STATE_MACHINE.md`，OMS 动作参考 `docs/spec/12_sales_order_status_actions.md`。

## 1. 通用规则
- `action_code` 为唯一业务动作标识，必须写入审计日志与 `order_action_log`。
- 动作请求统一携带 `version`（乐观锁）与 `idempotency_key`（幂等键）。
- 需要原因的动作（驳回/关闭/作废/退款）必须记录 `reason`。
- 权限以 `openapi/erp_api_v1.yaml` 的 `x-permissions` 为准，文中为建议值。

## 2. 销售单动作
| action_code | 入口状态 | 权限 | 幂等键建议 | 说明 |
| --- | --- | --- | --- | --- |
| sales_submit | DRAFT | `orders.write` | sales_order_id+version | 提交审批 |
| sales_approve | PENDING_APPROVAL | `orders.write` | sales_order_id+version | 审批通过 |
| sales_reject | PENDING_APPROVAL | `orders.write` | sales_order_id+version | 审批驳回 |
| sales_resubmit | REJECTED | `orders.write` | sales_order_id+version | 修改后再提交 |
| sales_confirm | APPROVED | `orders.write`/`shipping.write` | sales_order_id+version | 下发 WMS 出库 |
| sales_invoice | IN_PROGRESS | `finance.write` | sales_order_id+invoice_no | 触发开票 |
| sales_receive_payment | IN_PROGRESS | `finance.write` | sales_order_id+payment_id | 收款确认 |
| sales_refund | IN_PROGRESS/COMPLETED | `finance.write` | sales_order_id+refund_id | 退款/部分退款 |
| sales_close | APPROVED/IN_PROGRESS/COMPLETED | `orders.write` | sales_order_id+version | 关单/撤销履约 |
| sales_void | DRAFT/REJECTED | `orders.write` | sales_order_id+version | 作废 |
| sales_split | DRAFT/APPROVED/IN_PROGRESS | `orders.write` | sales_order_id+version | 拆单 |
| sales_merge | DRAFT/APPROVED | `orders.write` | sales_order_id+version | 合单 |
| sales_reopen | CLOSED | `orders.admin` | sales_order_id+version | 重开 |

## 3. WMS 回传动作（系统回调）
| event_code | 来源 | 影响字段 | 幂等键建议 | 说明 |
| --- | --- | --- | --- | --- |
| wms_outbound_created | WMS | `wms_outbound_status` | outbound_id+status | 出库单创建成功 |
| wms_wave_created | WMS | `wms_wave_status` | wave_id+status | 波次生成 |
| wms_wave_released | WMS | `wms_wave_status` | wave_id+status | 波次下发 |
| wms_pick_started | WMS | `wms_pick_status` | outbound_id+status | 拣货开始 |
| wms_pack_completed | WMS | `wms_pack_status` | outbound_id+status | 打包完成 |
| wms_shipped | WMS | `delivery_status`/`wms_ship_status` | package_id+status | 发货完成 |
| wms_ship_failed | WMS | `wms_ship_status` | package_id+status | 发货失败 |
| wms_outbound_closed | WMS | `wms_outbound_status` | outbound_id+status | 出库单取消 |

> WMS 回调不走权限校验，改为签名验真 + 幂等校验 + 状态不回退。

## 4. 售后动作
| action_code | 入口状态 | 权限 | 幂等键建议 | 说明 |
| --- | --- | --- | --- | --- |
| after_sales_apply | SUBMITTED | `aftersales.write` | case_id+version | 发起售后 |
| after_sales_approve | PENDING_REVIEW | `aftersales.write` | case_id+version | 审核通过 |
| after_sales_reject | PENDING_REVIEW | `aftersales.write` | case_id+version | 审核驳回 |
| after_sales_receive | WAITING_RECEIPT | `aftersales.write` | return_id+version | 收货确认 |
| after_sales_refund | PENDING_REFUND | `aftersales.write`/`finance.write` | refund_id | 执行退款 |
| after_sales_close | CLOSED | `aftersales.write` | case_id+version | 结案 |

## 5. 权限对齐建议
- `orders.write`/`orders.force_review`：销售单与审核动作。
- `shipping.write`：履约/发货相关动作。
- `aftersales.read/write`：售后查询与处理动作。
- `finance.write`：收款、退款、开票与对账动作（如未存在可新增）。
