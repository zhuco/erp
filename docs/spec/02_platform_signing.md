# 平台对接：验签与签名规范（淘宝 / 京东 / 拼多多 / 抖音）

> 目标：实现**生产可用**的平台对接安全链路：
> - 入站（平台 -> 我们）Webhook/事件推送的**正式验签**
> - 出站（我们 -> 平台）API 调用的**请求签名**

本文档面向 Codex/AI：所有规则都用结构化要点描述，并给出关键字段、算法、边界条件。

---

## 1. 通用约定

### 1.1 术语
- `system params`：平台网关通用参数（method/app_key/timestamp/sign_method 等）
- `biz params`：具体接口业务参数（如 tid/order_id/order_sn 等）
- `sign`：请求签名（字符串）

### 1.2 编码与排序
- **排序**：除特别说明外，均按参数名 ASCII/字典序升序排序。
- **编码**：参与签名的参数值，一律采用 UTF-8 的字符序列；URL 表单入站验签时要先 percent-decode。
- **排除项**：计算签名时必须排除 `sign` 本身；对空值参数通常跳过。

### 1.3 Hex 大小写
- 淘宝 / 京东 / 拼多多：历史上普遍要求 **HEX 大写**
- 抖店（抖音电商开放平台）：示例中普遍为 **HEX 小写**（本仓库实现默认小写）

> 注意：不同环境/网关可能接受大小写不敏感对比，但我们建议：**按文档要求输出**，验签时用 case-insensitive 比较。

---

## 2. 淘宝（Taobao/TOP）

### 2.1 必需凭据（shop.credentials JSON）
```json
{
  "app_key": "...",
  "app_secret": "...",
  "session": "...", 
  "sign_method": "md5", 
  "gateway_url": "https://eco.taobao.com/router/rest"
}
```

### 2.2 出站请求签名
- `sign_method = md5`
  - `string_to_sign = secret + concat(sorted(k+v)) + secret`
  - `sign = MD5(string_to_sign).hex_upper`
- `sign_method = hmac`
  - `string_to_sign = concat(sorted(k+v))`
  - `sign = HMAC_MD5(secret, string_to_sign).hex_upper`
- `sign_method = hmac_sha256`
  - `string_to_sign = concat(sorted(k+v))`
  - `sign = HMAC_SHA256(secret, string_to_sign).hex_upper`

### 2.3 入站验签（适配 form-urlencoded）
- 适配 QiMen/TOP 的 HTTP 回调：body 常见为 `application/x-www-form-urlencoded`
- 从 body 中解析全部参数，取出 `sign` / `sign_method`，按 2.2 规则重算并对比。

---

## 3. 京东（JOS）

### 3.1 必需凭据（shop.credentials JSON）
```json
{
  "app_key": "...",
  "app_secret": "...",
  "access_token": "...",
  "gateway_url": "https://api.jd.com/routerjson",
  "order_detail_method": "jingdong.pop.order.get"
}
```

### 3.2 出站请求签名（MD5）
- `string_to_sign = secret + concat(sorted(k+v)) + secret`
- `sign = MD5(string_to_sign).hex_upper`

### 3.3 入站验签（若推送包含 sign）
- 若 webhook body 为 form-urlencoded 且包含 `sign`：按 3.2 重算并对比。
- 若未包含 sign：当前实现放行（由上层租户/网络边界保证），建议在正式环境补充 JD 官方推送验签细则。

---

## 4. 拼多多（PDD）

> 说明：PDD 官方文档在部分网络环境下不可直连；本仓库实现采用业内通用规则（client_secret 前后包裹 + 参数排序 + MD5 大写）。上线前请用官方/控制台的签名工具或示例向量做一次对齐校验。

### 4.1 必需凭据（shop.credentials JSON）
```json
{
  "client_id": "...",
  "client_secret": "...",
  "access_token": "...",
  "gateway_url": "https://gw-api.pinduoduo.com/api/router"
}
```

### 4.2 出站请求签名（MD5）
- `string_to_sign = client_secret + concat(sorted(k+v)) + client_secret`
- `sign = MD5(string_to_sign).hex_upper`

### 4.3 入站验签
- 若推送中包含 `sign`：按 4.2 重算并对比。

---

## 5. 抖店（抖音电商开放平台 / Jinritemai）

### 5.1 必需凭据（shop.credentials JSON）
```json
{
  "app_key": "...",
  "app_secret": "...",
  "access_token": "...",

  "gateway_base": "https://openapi.jinritemai.com",
  "order_detail_path": "/order/orderDetail",

  "sign_method": "md5",

  "webhook_secret": "..." 
}
```

### 5.2 出站请求签名（md5 / hmac-sha256，小写 hex）
- 统一：`concat = concat(sorted(k+v))`（排除 `sign`）
- `md5`
  - `string_to_sign = secret + concat + secret`
  - `sign = MD5(string_to_sign).hex_lower`
- `hmac-sha256`
  - `sign = HMAC_SHA256(secret, concat).hex_lower`

### 5.3 入站验签（事件推送）
- Headers:
  - `app-id`
  - `event-sign`
- 计算：`event_sign = MD5(app_id + raw_body + webhook_secret).hex_lower`
- 对比：case-insensitive

---

## 6. 验签失败策略（建议）

- 失败返回 HTTP 401/403（由 channel 层决定）；当前仓库在 worker 层校验失败会记录错误并停止处理该事件。
- 记录审计字段：shop_id、platform、签名算法、对比差异（不要记录 secret）。
- 支持灰度：允许为每店铺配置 `webhook_verification = strict | permissive`。
