# WMS 出入库与发货链路 API/权限同步说明（2026-04-07）

## 1. 目标

本次同步面向工业级电商 ERP 的 WMS 关键执行链路，补齐以下业务能力的后端接口与权限门禁，确保可直接进入企业级上线验收：

1. 等待发货队列
2. 生成波次后的批量发货（波次发货）
3. 直接发货（跳过部分流程的受控能力）
4. 打包/配货员登记
5. 与既有入库、出库、打印接口形成完整闭环

## 2. WMS 链路规划（企业级）

### 2.1 入库链路（既有能力）

1. 创建入库单 `POST /wms/inbounds`
2. 入库上架与明细确认
3. 入库单打印 `POST /wms/inbounds/{inbound_doc_id}/print`

### 2.2 出库与待发货链路

1. 创建/同步出库单 `POST /wms/outbounds`
2. 审核、拣货、打包、验货、称重（既有）
3. 等待发货队列查询（新增）`GET /wms/outbounds/waiting-ship`

### 2.3 波次链路

1. 波次规则管理（既有）`/wms/wave-rules`
2. 生成波次（既有）`POST /wms/waves/run`
3. 波次详情查看（既有）`GET /wms/waves/{wave_id}`
4. 波次发货（新增）`POST /wms/waves/{wave_id}/ship`

### 2.4 直接发货与人员登记链路

1. 直接发货（新增）`POST /wms/outbounds/{outbound_doc_id}/direct-ship`
2. 打包/配货员登记（新增）`POST /wms/outbounds/{outbound_doc_id}/operator-register`
3. 单据打印（既有）`POST /wms/outbounds/{outbound_doc_id}/print`、`POST /wms/waves/{wave_id}/print`

## 3. 新增 API 列表

### 3.1 等待发货队列

- 路径：`GET /v1/tenants/{tenant_id}/wms/outbounds/waiting-ship`
- 权限：`wms.waiting_ship.read`
- 说明：返回满足 `outbound_status=PENDING_SHIP` 且 `ship_status=NOT_SHIPPED` 的出库单列表
- 关键查询参数：
  - `warehouse_id`
  - `shipment_id`
  - `shop_id`
  - `carrier_code`
  - `tracking_no`
  - `doc_no_keyword`
  - `offset`
  - `limit`
- 响应：`WmsOutboundListResponse`

### 3.2 波次发货

- 路径：`POST /v1/tenants/{tenant_id}/wms/waves/{wave_id}/ship`
- 权限：`wms.wave_ship`
- 条件权限：当 `direct_ship=true` 时，额外要求 `wms.direct_ship`
- 请求：`WmsWaveShipRequest`
- 响应：`WmsWaveShipResponse`
- 说明：
  - 支持 `continue_on_error`（失败后是否继续后续单据）
  - 输出每个出库单的处理结果（成功/失败 + 失败原因）

### 3.3 直接发货

- 路径：`POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/direct-ship`
- 权限：`wms.direct_ship`
- 请求：`WmsDirectShipRequest`
- 响应：`204 No Content`
- 说明：
  - 支持传入串号 `serials`
  - 在存在串号管控商品时会做发货前串号完整性校验

### 3.4 打包/配货员登记

- 路径：`POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/operator-register`
- 权限：`wms.operator.register`
- 请求：`WmsOutboundOperatorRegisterRequest`
- 响应：`WmsOutboundOperatorRegisterResponse`
- 说明：用于登记配货员/打包员与备注，便于追溯和绩效统计

## 4. 权限矩阵（本次新增）

| 能力 | 权限码 | 风险级别 | 说明 |
| --- | --- | --- | --- |
| 等待发货队列查询 | `wms.waiting_ship.read` | 低 | 仅查询 |
| 波次发货 | `wms.wave_ship` | 高 | 批量推进出库单到发货态 |
| 直接发货 | `wms.direct_ship` | 高 | 可跳过常规流程节点 |
| 打包/配货员登记 | `wms.operator.register` | 中 | 影响履约追溯数据 |

说明：新增权限保持细粒度，不与 `wms.write` 自动等价，避免高风险动作被宽权限放开。

## 5. 状态与业务一致性

1. 等待发货队列通过固定状态组合筛选，避免前端重复拼装状态条件。
2. 波次发货与直接发货均会写入操作日志与审计日志。
3. 发货成功后同步销售单履约相关状态，保持 OMS/WMS 状态一致。
4. 串号商品在直接发货场景下必须满足串号绑定数量要求。

## 6. OpenAPI 同步项

已同步到 `openapi/erp_api_v1.yaml`：

1. 新增 4 条路径
2. 新增 6 个 schema
   - `WmsWaveShipRequest`
   - `WmsWaveShipDocResult`
   - `WmsWaveShipResponse`
   - `WmsDirectShipRequest`
   - `WmsOutboundOperatorRegisterRequest`
   - `WmsOutboundOperatorRegisterResponse`
3. 为路径补充 `x-permissions` 约束

## 7. 回归与验收建议

建议上线前至少覆盖以下自动化验证：

1. 权限门禁测试：缺权限必须返回 `403`
2. 串号约束测试：串号商品无完整绑定时禁止发货
3. 波次批量异常策略：`continue_on_error` 两种模式都需要验证
4. 审计日志完整性：关键动作应包含操作人、单据、时间和结果
