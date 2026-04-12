# 订单流程接口待处理清单
> 基于订单下载 → 审核 → 仓储履约 → 发货 → 结算的闭环梳理接口缺口；已补齐的接口标记为“已完成”。

| 编号 | 接口能力 | 主要用途 | 主要路径 | 状态 |
| --- | --- | --- | --- | --- |
| OF-API-001 | 拉单触发/补偿 | 手动或任务触发平台拉单 | `POST /v1/tenants/{tenant_id}/shops/{shop_id}/sync-jobs/run` | 已完成 |
| OF-API-002 | 自动投审配置 | 下载后自动进入审核 | `GET/PUT /v1/tenants/{tenant_id}/sales/shops/{shop_id}` | 已完成 |
| OF-API-003 | 订单审核处理 | 审核通过/驳回/标记 | `POST /v1/tenants/{tenant_id}/orders/{order_id}/review` | 已完成 |
| OF-API-004 | 回退到审核 | 已完成订单回退并取消单号预约 | `POST /v1/tenants/{tenant_id}/orders/{order_id}/rollback-review` | 已完成 |
| OF-API-005 | 波次生成/运行 | 多发货单按规则生成波次 | `POST /v1/tenants/{tenant_id}/wms/waves`、`POST /v1/tenants/{tenant_id}/wms/waves/run` | 已完成 |
| OF-API-006 | 出库扩展操作 | 预约单号、补录物流、加包、取消 | `POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/*` | 已完成 |
| OF-API-007 | 打包员记录 | 打包计件统计入口 | `POST /v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/pack` | 已完成 |
| OF-API-008 | 单号池管理 | 单号导入/预约/释放/作废 | `POST /v1/tenants/{tenant_id}/waybills/*` | 已完成 |
| OF-API-009 | 利润/结算查询 | 结算与利润快照查询 | `GET /v1/tenants/{tenant_id}/fin/profit` | 已完成 |
