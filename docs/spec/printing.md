# 打印模块

## print_devices
- 字段: `device_id` uuid pk, `tenant_id` uuid, `device_fingerprint` text, `operator_user_id` uuid, `default_printer_name` text, `created_at` timestamptz, `updated_at` timestamptz
- 唯一约束: `unique (device_fingerprint)`（终端指纹全局唯一）

## print_templates
- 字段: `template_id` uuid pk, `tenant_id` uuid, `shop_id` uuid, `platform` text, `template_type` text, `platform_template_id` text, `version` int, `content` jsonb, `is_active` bool, `created_at` timestamptz, `updated_at` timestamptz
- 模板类型: `SHIPPING_LABEL` / `PICK_LIST` / `INVOICE`
- 唯一约束: `unique (tenant_id, shop_id, template_type, version)`
- 建议: `is_active=true` 的版本用于默认渲染，历史版本只读保留

## print_jobs
- 字段: `job_id` uuid pk, `tenant_id` uuid, `shop_id` uuid, `type` text, `ref_type` text, `ref_id` uuid, `payload` jsonb, `status` text, `target_printer_name` text, `created_at` timestamptz, `updated_at` timestamptz, `printed_at` timestamptz
- 任务类型: `type` 复用模板类型（`SHIPPING_LABEL` / `PICK_LIST` / `INVOICE`）
- 关联类型: `ref_type` = `ORDER` / `SHIPMENT` / `WAYBILL`
- 状态: `QUEUED` / `PRINTING` / `SUCCESS` / `FAILED`
- 索引: `(tenant_id, status, created_at)`, `(tenant_id, ref_type, ref_id)`

## 审计建议
- 每次打印请求都写入 `print_jobs`，并在 `payload` 中保存 `template_id`、`template_version`、`device_id`、`operator_user_id`、`request_id`、客户端来源等信息。
- `printed_at` 只在成功完成时写入，失败原因放在 `payload.error`，方便追踪失败与补打。
- `print_devices.operator_user_id` 用于绑定当前操作员，切换账号时更新并记录在审计日志中。

## 重打规则建议
- 重打不覆盖旧记录，新建 `print_jobs`，在 `payload.reprint_of` 保存被重打的 `job_id`。
- 当原任务 `status=PRINTING` 时拒绝重打；`SUCCESS/FAILED` 时允许重打。
- 可按业务限制重打次数与时间窗口，例如发货后 7 天内允许重打，超过则需要管理员权限。
