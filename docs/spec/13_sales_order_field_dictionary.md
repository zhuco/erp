# 销售单字段字典（Sales Order Schema）

> 目标：统一销售单（内部业务单）字段口径，作为状态机与接口对齐依据。平台订单字段映射见 `docs/spec/03_order_field_mapping.md`。

## 1. 主表字段（sales_orders）
| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| sales_order_id | uuid | 是 | 销售单主键 |
| tenant_id | uuid | 是 | 租户 |
| shop_id | uuid | 否 | 店铺/业务主体 |
| sales_order_no | string | 是 | 业务单号 |
| source_type | enum | 是 | 来源：MANUAL/PLATFORM |
| source_order_ids | json | 否 | 关联平台订单ID |
| customer_id | uuid | 否 | 客户主体 |
| currency_code | string | 是 | 币种（如 CNY） |
| tax_mode | enum | 否 | TAX_INCLUDED/TAX_EXCLUDED |
| sales_order_status | enum | 是 | 见 `docs/spec/SALES_ORDER_STATE_MACHINE.md` |
| delivery_status | enum | 是 | NOT_SHIPPED/PARTIALLY_SHIPPED/SHIPPED |
| invoice_status | enum | 是 | NOT_INVOICED/PARTIALLY_INVOICED/INVOICED |
| payment_status | enum | 是 | UNPAID/PARTIALLY_PAID/PAID |
| refund_status | enum | 是 | NONE/PARTIAL_REFUND/REFUNDED |
| return_status | enum | 否 | NONE/PARTIAL_RETURN/RETURNED |
| stock_status | enum | 否 | NOT_RESERVED/RESERVED/RELEASED |
| total_amount_cent | int64 | 是 | 含税总额 |
| discount_amount_cent | int64 | 否 | 折扣金额 |
| shipping_amount_cent | int64 | 否 | 运费 |
| receivable_amount_cent | int64 | 是 | 应收金额 |
| paid_amount_cent | int64 | 是 | 已收金额 |
| refunded_amount_cent | int64 | 是 | 已退款金额 |
| invoice_amount_cent | int64 | 否 | 已开票金额 |
| status_flags | json | 否 | 状态标记集合 |
| remark | string | 否 | 业务备注 |
| version | int32 | 是 | 乐观锁版本 |
| created_at | timestamp | 是 | 创建时间 |
| updated_at | timestamp | 是 | 更新时间 |
| created_by | uuid | 否 | 创建人 |
| updated_by | uuid | 否 | 更新人 |

## 2. 明细字段（sales_order_items）
| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| line_id | uuid | 是 | 行项目ID |
| sales_order_id | uuid | 是 | 关联销售单 |
| sku_id | uuid | 是 | SKU |
| sku_code | string | 否 | SKU 编码 |
| sku_name | string | 否 | SKU 名称 |
| qty | int32 | 是 | 数量 |
| price_cent | int64 | 是 | 含税单价 |
| tax_rate | decimal | 否 | 税率 |
| discount_cent | int64 | 否 | 行折扣 |
| amount_cent | int64 | 是 | 行小计 |
| warehouse_id | uuid | 否 | 指定发货仓 |
| delivery_deadline | timestamp | 否 | 发货期望时间 |
| extra | json | 否 | 扩展字段 |

## 3. 收货与物流字段（sales_orders）
| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| receiver_name | string | 是 | 收件人 |
| receiver_phone | string | 是 | 收件电话 |
| receiver_address | string | 是 | 详细地址 |
| receiver_province | string | 否 | 省 |
| receiver_city | string | 否 | 市 |
| receiver_district | string | 否 | 区县 |
| receiver_postcode | string | 否 | 邮编 |
| carrier_code | string | 否 | 承运商编码 |
| logistics_service | string | 否 | 物流服务 |
| expected_ship_at | timestamp | 否 | 期望发货时间 |

## 4. WMS 关联字段（sales_orders）
| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| wms_outbound_id | uuid | 否 | WMS 出库单ID |
| wms_outbound_status | enum | 否 | `OutboundStatus` |
| wms_wave_id | uuid | 否 | 波次ID |
| wms_wave_status | enum | 否 | `WaveStatus` |
| wms_ship_status | enum | 否 | `ShipStatus` |
| package_count | int32 | 否 | 包裹数量 |
| waybill_nos | json | 否 | 运单号集合 |

## 5. 审核与异常字段（可选）
| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| review_status | enum | 否 | 审核子状态（`OrderReviewStatus`） |
| fulfillment_status | enum | 否 | 履约子状态（`OrderFulfillmentStatus`） |
| exception_status | enum | 否 | 异常子状态 |
| freeze_reason | string | 否 | 冻结原因 |
| exception_flags | json | 否 | 异常标记集合 |
