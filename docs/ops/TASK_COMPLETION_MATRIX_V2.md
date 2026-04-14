# 工业级电商 ERP 任务完成矩阵（V2）

## 目标
面向“订单处理、商品匹配、订单修改、发货单发货、波次发货”五大主链路，给出发布前可审计的完成标准。

## 完成矩阵

| 业务域 | 最小后端闭环 | 契约门禁 | 落库证据（必选其一） | 验收命令 |
| --- | --- | --- | --- | --- |
| 订单处理 | `oms_orders.rs` | `/oms/orders/*` 状态码+权限齐全 | `order_action_log` / `order_status_history` | `cargo test -p erp-api --test oms_orders -- --nocapture` |
| 商品匹配 | `platform_sync_and_mapping_flow_integration.rs` | `/shop-products/*` 主链路齐全 | `shop_product_tasks` / `channel_sku_mappings` / `operation_logs` | `cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture` |
| 订单修改 | `order_match_ship_wave_persistence_guard.rs` | 写接口声明 `x-persistence-required` | `order_action_log` / `operation_logs` | `cargo test -p erp-api --test order_match_ship_wave_persistence_guard -- --nocapture` |
| 发货单发货 | `shipment_confirm.rs` | `/shipments/*` 权限与状态码完整 | `shipments` / `operation_logs` | `cargo test -p erp-api --test shipment_confirm -- --nocapture` |
| 波次发货 | 现有 WMS 集成流 | `/wms/waves/*` 契约完整 | `operation_logs` / `audit_logs` | `cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture` |

## 统一阻断规则（P0）
- 关键测试出现 `skipping ... DATABASE_URL` 或 `pool timed out`：**NO-GO**
- 缺失主链路 OpenAPI 契约（路径/权限/状态码）：**NO-GO**
- 写操作缺失持久化声明或无实际落库证据：**NO-GO**

## 建议发布顺序
1. `pnpm run repo:verify`
2. `pnpm run test:backend:release-gate`
3. `pnpm --dir apps/erp-web run generate:api`
4. `pnpm --dir apps/erp-web build`
5. `pnpm run test:e2e:release-gate`
