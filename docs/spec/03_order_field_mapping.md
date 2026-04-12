# 订单字段映射规范（平台 → OrderProjection）

> 目标：SaaS 多店铺、日百万单级别；以**平台订单**为准，只做“字段映射 + 状态映射 + 原始报文留存(raw)”。

---

## 1. 统一领域字段（OrderProjection）

统一投影字段（见 `crates/erp-domain` / `crates/erp-types`）：

- `platform_order_id`：平台主订单号（字符串）
- `status`：统一订单状态（`OrderStatus`）
- `total_amount_cent`：订单实付金额（分，CNY）
- `goods_amount_cent`：商品金额（分，可空）
- `postage_amount_cent`：运费金额（分，可空）
- `discount_amount_cent`：优惠总额（分，可空）
- `platform_discount_cent`：平台优惠（分，可空）
- `seller_discount_cent`：商家优惠（分，可空）
- `coupon_discount_cent`：券/红包优惠（分，可空）
- `currency`：默认 `CNY`
- `buyer_name`：买家标识（平台 open_id / buyer_id / nickname 等，按可得字段）
- `receiver_name / receiver_phone / receiver_address`：收货信息（优先“已解密”，否则使用“脱敏字段”）
- `pay_at / ship_at / finish_at / cancel_at`：关键时间（UTC，可空）
- `buyer_message`：买家留言（可空）
- `seller_remark`：商家备注（可空）
- `logistics_company / tracking_no`：物流公司/运单号（可空；多包裹场景建议在 raw 中保留完整结构）
- `order_created_at`：下单时间（UTC）
- `platform_updated_at`：平台侧更新时间（UTC，可空）
- `items[]`：订单明细行（至少 1 行，稳定 `line_id`）
- `raw`：平台原始订单对象（JSON，原样存储，便于追溯/回放）

统一明细行字段（OrderItemProjection）：

- `pay_amount_cent`：子单实付（分，可空）
- `discount_cent`：子单优惠（分，可空）
- `refund_state`：子单退款状态（`RefundState`）
- `after_sale_state`：子单售后状态（`AfterSaleState`）
- `refund_amount_cent`：子单退款金额（分，可空）
- `after_sale_type`：售后类型/原因（平台原值字符串，可空）
- `after_sale_id`：售后单号/售后ID（平台原值字符串，可空）

---

## 2. 拼多多 PDD

### 2.1 对齐的官方接口形态

- 详情：`pdd.order.information.get`（以返回的 `order_info` 为准）
- 列表/增量：`pdd.order.basic.list.get`（以返回的 `order_list` 为准）

> 两个接口结构略有差异，但核心字段名（`order_sn`、`order_status`、`refund_status`、`pay_amount`、`item_list`）一致或可兼容。

### 2.2 主单字段映射

| OrderProjection 字段 | PDD 字段 | 类型/单位 | 规则/备注 |
|---|---|---:|---|
| `platform_order_id` | `order_sn` | string | 必填 |
| `order_created_at` | `created_time`（或 `order_create_time`） | string/seconds | `parse_platform_time`（支持 `yyyy-MM-dd HH:mm:ss` 或 unix 秒） |
| `platform_updated_at` | `updated_at`（或 `updated_time` / `update_time`） | string/seconds | 可空 |
| `total_amount_cent` | `pay_amount`（或 `order_amount` / `order_pay_amount`） | **元 (DOUBLE)** | **元→分：×100 并四舍五入** |
| `goods_amount_cent` | `goods_amount`（或 `goods_amount_total` / `goods_sum_amount`） | **元 (DOUBLE)** | 可空，元→分 |
| `postage_amount_cent` | `postage`（或 `postage_amount` / `post_amount`） | **元 (DOUBLE)** | 可空，元→分 |
| `discount_amount_cent` | `discount_amount`（或 `promotion_amount`） | **元 (DOUBLE)** | 可空，元→分 |
| `platform_discount_cent` | `platform_discount`（或 `platform_discount_amount`） | **元 (DOUBLE)** | 可空，元→分 |
| `seller_discount_cent` | `seller_discount`（或 `seller_discount_amount` / `shop_discount_amount`） | **元 (DOUBLE)** | 可空，元→分 |
| `coupon_discount_cent` | `coupon_discount`（或 `coupon_discount_amount`） | **元 (DOUBLE)** | 可空，元→分 |
| `pay_at` | `pay_time`（或 `payment_time`） | string/seconds | 可空 |
| `ship_at` | `shipping_time`（或 `delivery_time` / `consign_time`） | string/seconds | 可空 |
| `finish_at` | `confirm_time`（或 `receive_time` / `success_time`） | string/seconds | 可空 |
| `cancel_at` | `cancel_time`（或 `canceled_time`） | string/seconds | 可空 |
| `buyer_message` | `buyer_message`（或 `buyer_memo`） | string | 可空 |
| `seller_remark` | `seller_remark`（或 `seller_memo`） | string | 可空 |
| `logistics_company` | `shipping_company`（或 `logistics_company`） | string | 可空 |
| `tracking_no` | `tracking_number`（或 `tracking_no` / `express_no`） | string | 可空 |
| `buyer_name` | `buyer_name` / `nickname` | string | 可空 |
| `receiver_name` | `receiver_name` → `receiver_name_mask` | string | 先取解密字段，空则取脱敏字段 |
| `receiver_phone` | `receiver_phone` → `receiver_phone_mask` | string | 同上 |
| `receiver_address` | `province` + `city` + `town` + (`receiver_address` → `receiver_address_mask` → `address_mask`) | string | 拼接成可展示地址 |
| `items[]` | `item_list` | array | 见下节 |

### 2.3 明细行 `item_list` 映射

| OrderItemProjection 字段 | PDD 字段 | 类型/单位 | 规则/备注 |
|---|---|---:|---|
| `line_id` | `outer_id` → (`goods_id`:`sku_id`) → fallback | string | **要求稳定且唯一**：优先 `outer_id`（SKU 外部编码） |
| `sku_id` | `sku_id` | string | LONG/string 均可 |
| `title` | `goods_name` | string | 可空 |
| `qty` | `goods_count` | int | 默认 1 |
| `price_cent` | `goods_price` | **元 (DOUBLE)** | 元→分 |
| `amount_cent` | - | 分 | 计算：`price_cent * qty`（PDD 明细通常不直接给行金额） |
| `pay_amount_cent` | `pay_amount`（或 `goods_amount` / `item_amount`） | **元 (DOUBLE)** | 可空，元→分；缺失时可由主单与行金额推导 |
| `discount_cent` | `discount_amount`（或 `promotion_amount`） | **元 (DOUBLE)** | 可空，元→分；缺失时可由 `amount_cent - pay_amount_cent` 推导 |
| `refund_state` | `refund_status`（子单优先；缺失回退主单） | int | 统一映射见 4.2 |
| `after_sale_state` | `after_sales_status`（子单优先；缺失回退主单） | int | 统一映射见 4.2 |
| `refund_amount_cent` | `refund_amount`（或 `after_sale_amount`） | **元 (DOUBLE)** | 可空，元→分 |
| `after_sale_type` | `after_sales_type`（或 `after_sale_type`） | string/int | 可空，统一转 string |
| `after_sale_id` | `after_sales_id`（或 `after_sale_id` / `after_sales_sn`） | string/int | 可空，统一转 string |

### 2.4 状态映射（逐字段对齐）

PDD 订单最终状态由多个字段共同决定：

- `confirm_status`（成交状态）：`0 未成交 / 1 已成交 / 2 已取消`
- `order_status`（发货状态）：`1 待发货 / 2 已发货待签收 / 3 已签收`
- `refund_status`（售后状态）：`1 无售后或售后关闭 / 2 售后处理中 / 3 退款中 / 4 退款成功`
- `after_sales_status`（更细粒度售后状态，含退款/换货补寄/维修等枚举）
- `risk_control_status`（订单审核/风控）：`0 正常 / 1 审核中`

**统一状态映射（本仓库实现的优先级）**：

1. `refund_status == 4` 或 `after_sales_status == 10` → `REFUNDED`
2. `refund_status in {2,3}` 或 `after_sales_status` 属于“退款/纠纷处理中” → `REFUNDING`
3. `after_sales_status` 属于“换货补寄/维修处理中” 或 `risk_control_status == 1` → `ON_HOLD`
4. `confirm_status == 2` → `CANCELLED`
5. `confirm_status == 0` → `PENDING_PAYMENT`
6. `confirm_status == 1` 时按 `order_status`：
   - `1` → `PENDING_SHIPMENT`
   - `2` → `SHIPPED`
   - `3` → `COMPLETED`

> 说明：PDD 的“成交/发货/售后”是多维字段；ERP 展示层通常需要“一个主状态”，因此采用以上优先级。

---

## 3. 抖店 Douyin（抖音电商开放平台）

### 3.1 对齐的官方接口形态

- 详情：`order.orderDetail`（HTTP: `POST /order/orderDetail`）
  - 订单对象常位于 `data.shop_order_detail`

### 3.2 主单字段映射

| OrderProjection 字段 | 抖店字段（shop_order_detail） | 类型/单位 | 规则/备注 |
|---|---|---:|---|
| `platform_order_id` | `order_id` | string/int | 必填 |
| `order_created_at` | `create_time` | seconds | unix 秒 |
| `platform_updated_at` | `update_time` | seconds | 可空 |
| `total_amount_cent` | `pay_amount`（或 `order_amount`） | **分 (INT)** | 直接写入 |
| `goods_amount_cent` | `order_amount`（或 `goods_amount` / `total_amount`） | 分 | 可空 |
| `postage_amount_cent` | `post_amount`（或 `postage` / `shipping_amount`） | 分 | 可空 |
| `discount_amount_cent` | `promotion_amount`（或 `discount_amount` / `coupon_amount`） | 分 | 可空 |
| `platform_discount_cent` | `platform_discount_amount`（或 `platform_discount`） | 分 | 可空 |
| `seller_discount_cent` | `shop_discount_amount`（或 `seller_discount_amount` / `seller_discount`） | 分 | 可空 |
| `coupon_discount_cent` | `coupon_amount`（或 `coupon_discount_amount`） | 分 | 可空 |
| `pay_at` | `pay_time`（或 `pay_time_sec` / `payment_time`） | seconds | 可空 |
| `ship_at` | `ship_time`（或 `shipping_time` / `consign_time`） | seconds | 可空 |
| `finish_at` | `finish_time`（或 `confirm_time` / `receive_time`） | seconds | 可空 |
| `cancel_at` | `cancel_time`（或 `canceled_time`） | seconds | 可空 |
| `buyer_message` | `buyer_words`（或 `buyer_message`） | string | 可空 |
| `seller_remark` | `seller_words`（或 `seller_remark`） | string | 可空 |
| `logistics_company` | `logistics_info[0].company`（fallback `logistics_company`） | string | 可空 |
| `tracking_no` | `logistics_info[0].tracking_no`（fallback `tracking_no`） | string | 可空 |
| `buyer_name` | `open_id` / `buyer_id` / `user_id` | string/int | 可空 |
| `receiver_name` | `mask_post_receiver`（或 `post_receiver`） | string | 默认用脱敏字段（解密需额度/权限） |
| `receiver_phone` | `mask_post_tel`（或 `post_tel`） | string | 同上 |
| `receiver_address` | `mask_post_addr.{province,city,district,street,detail}` | string | 拼接成可展示地址 |
| `items[]` | `sku_order_list` | array | 见下节 |

### 3.3 明细行 `sku_order_list` 映射

| OrderItemProjection 字段 | 抖店字段 | 类型/单位 | 规则/备注 |
|---|---|---:|---|
| `line_id` | `sku_order_id` → `out_sku_id` → `sku_id` → fallback | string | **要求稳定且唯一** |
| `sku_id` | `sku_id` | string/int | 统一转 string |
| `title` | `product_name` | string | 可空 |
| `qty` | `item_num` | int | 默认 1 |
| `price_cent` | `goods_price` | 分 | 直接写入 |
| `amount_cent` | `order_amount` | 分 | 缺失时退化为 `price_cent*qty` |
| `pay_amount_cent` | `pay_amount`（或 `real_pay_amount` / `total_pay_amount`） | 分 | 可空 |
| `discount_cent` | `promotion_amount`（或 `discount_amount` / `coupon_amount`） | 分 | 可空；缺失时可由 `amount_cent - pay_amount_cent` 推导 |
| `refund_state` | `after_sale_info.refund_status`（fallback `refund_status`） | int | 统一映射见 4.3 |
| `after_sale_state` | `after_sale_info.after_sale_status`（fallback `after_sale_status`） | int | 统一映射见 4.3 |
| `refund_amount_cent` | `after_sale_info.refund_amount`（fallback `refund_amount`） | 分 | 可空 |
| `after_sale_type` | `after_sale_info.after_sale_type_desc`（或 `after_sale_type`） | string/int | 可空，统一转 string |
| `after_sale_id` | `after_sale_info.after_sale_id`（或 `aftersale_id`） | string/int | 可空，统一转 string |

### 3.4 状态映射（逐字段对齐）

抖店以 `order_status` 为主（官方枚举）：

- `1`：待确认/待支付
- `103`：部分支付
- `105`：已支付
- `2`：备货中（待发货）
- `101`：部分发货
- `3`：已发货（全部发货）
- `4`：已取消
- `5`：已完成（已收货）
- `21`：发货前退款完结
- `22`：发货后退款完结
- `39`：收货后退款完结

**统一状态映射（本仓库实现）**：

- `1/103` → `PENDING_PAYMENT`
- `105/2` → `PENDING_SHIPMENT`
- `101` → `PARTIALLY_SHIPPED`
- `3` → `SHIPPED`
- `4` → `CANCELLED`
- `5` → `COMPLETED`
- `21/22/39` → `REFUNDED`

**风控覆盖（重要）**：

- 当订单处于风控中，查询接口会返回 `sku_order_tag_ui.key = risk_processing`（可能出现在主单或子单上）。
- 本仓库把该标记视为 `ON_HOLD`（不改变 `order_status` 原始值，仍会保存在 `raw`）。

---

## 4. 退款/售后 子单级建模（关键）

### 4.1 为什么要“子单级”

平台售后多数以**子单/SKU 维度**发生：同一笔主订单内，不同商品可能分别退款、仅部分退款、换货或补寄。

因此本仓库把退款/售后状态建模在 `OrderItemProjection` 上：

- `refund_state`：退款维度（`NONE/PROCESSING/SUCCESS/CLOSED/UNKNOWN`）
- `after_sale_state`：售后维度（`NONE/PROCESSING/SUCCESS/CLOSED/UNKNOWN`）
- `refund_amount_cent`：子单退款金额（可空）
- `after_sale_type` / `after_sale_id`：平台原值（可空，便于追溯）

> 说明：主单 `status=REFUNDING/REFUNDED` 仍然保留，用于“主订单列表”快速过滤；
> 但运营/客服实际处理通常以子单为单位。

### 4.2 PDD 统一映射（由 `refund_status` + `after_sales_status` 推导）

本仓库 adapter（`crates/adapter-pdd`）使用以下**粗粒度规则**：

- `refund_state`：
  - `refund_status == 4` 或 `after_sales_status == 10` → `SUCCESS`
  - `refund_status in {2,3}` 或 `after_sales_status in {2,3,4,5,6,7,8,31,33}` → `PROCESSING`
  - `refund_status == 1` 且未命中上面规则 → `NONE`
  - 其它 → `UNKNOWN`

- `after_sale_state`：
  - `after_sales_status == 0/缺失` → `NONE`
  - `after_sales_status == 10` → `SUCCESS`
  - `after_sales_status in {9,11,12,13,16,17,24,25}` → `CLOSED`（关闭/取消/失败/完结等）
  - 其它任意非 0 枚举 → `PROCESSING`

> 注意：PDD 官方枚举可能扩展；如果新增/变更，请同时补充：
> 1) 枚举列表；2) 单测向量；3) 本文档。

### 4.3 抖店 统一映射（优先 `after_sale_info`）

抖店子单售后常见在 `sku_order_list[].after_sale_info`（或同名变体字段）内：

- `refund_state`：优先 `after_sale_info.refund_status`；缺失时若主单 `order_status in {21,22,39}` 视为 `SUCCESS`
- `after_sale_state`：优先 `after_sale_info.after_sale_status`

由于不同账号权限/接口版本可能返回字段不同，本仓库采取：

- 统一映射只做粗粒度：`NONE/PROCESSING/SUCCESS/CLOSED/UNKNOWN`
- 平台原始结构完整保存在 `raw` 中，前端展示可优先读 `raw.*_desc` 等文本字段

---

## 5. 兼容性与演进策略

- 字段缺失：允许部分字段为空（如 `buyer_name`、`receiver_*`），但 `platform_order_id/status/total_amount_cent/order_created_at/items` 必须可落库。
- 解密字段：默认不依赖解密；如后续接入“隐私中心解密”，可在 adapter 层追加 `decrypt_xxx` 并覆盖脱敏字段。
- 状态扩展：如平台新增枚举，必须：
  1) 补充枚举列表；
  2) 补充单测向量；
  3) 更新本文档。
