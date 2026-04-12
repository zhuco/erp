# 功能 API 补齐清单（含测试状态）

更新时间：2026-03-05  
适用范围：CRM 高阶能力 + Stats 统计接口

## 1. 依据与目标

本清单依据以下文档与现状差异生成，并已执行补齐：

- `docs/reports/archive/functional_boundary_basic_delivery.md`
- `docs/reports/archive/feature_gap_matrix.md`
- 前端实际调用：
  - `apps/erp-web/src/api/customerSegments.ts`
  - `apps/erp-web/src/api/customerLifecycle.ts`
  - `apps/erp-web/src/api/customerServiceTickets.ts`
  - `apps/erp-web/src/api/customerLoyalty.ts`
  - `apps/erp-web/src/api/stats.ts`

目标：形成“接口清单 + 每个接口测试用例 + 通过状态 + E2E 状态”的可交付文档。

## 2. API 补齐清单（执行结果）

| 模块 | API | 后端实现 | OpenAPI | 数据迁移 | 补齐状态 |
| --- | --- | --- | --- | --- | --- |
| 客户分群 | `GET/POST /v1/tenants/{tenant_id}/crm/customer-segments`<br>`PUT/DELETE /v1/tenants/{tenant_id}/crm/customer-segments/{segment_id}` | 已补齐 | 已补齐 | 已补齐 | 已完成 |
| 生命周期任务 | `GET/POST /v1/tenants/{tenant_id}/crm/lifecycle-tasks`<br>`PUT/DELETE /v1/tenants/{tenant_id}/crm/lifecycle-tasks/{task_id}` | 已补齐 | 已补齐 | 已补齐 | 已完成 |
| 客服工单 | `GET/POST /v1/tenants/{tenant_id}/crm/service-tickets`<br>`PUT/DELETE /v1/tenants/{tenant_id}/crm/service-tickets/{ticket_id}` | 已补齐 | 已补齐 | 已补齐 | 已完成 |
| 积分流水 | `GET /v1/tenants/{tenant_id}/crm/points-ledgers` | 已补齐 | 已补齐 | 已补齐 | 已完成 |
| 储值流水 | `GET /v1/tenants/{tenant_id}/crm/stored-value-ledgers` | 已补齐 | 已补齐 | 已补齐 | 已完成 |
| 统计看板 | `GET /v1/tenants/{tenant_id}/stats/{stats_type}` | 已补齐 | 已补齐 | 不涉及 | 已完成 |

核心证据：

- 路由与处理：`crates/erp-api/src/lib.rs`
- 迁移：`migrations/0086_crm_and_stats_api.sql`
- OpenAPI：`openapi/erp_api_v1.yaml`

## 3. 接口测试用例清单（API 集成）

测试文件：`crates/erp-api/tests/crm_and_stats_api.rs`  
执行命令：`cargo test -p erp-api --test crm_and_stats_api -- --nocapture`  
执行结果：`16 passed; 0 failed`

| 接口 | 用例 | 覆盖点 | 结果 |
| --- | --- | --- | --- |
| `GET /crm/customer-segments` | `crm_customer_segments_get_requires_customers_read` | 权限门禁（缺少 `customers.read`） | 通过 |
| `POST /crm/customer-segments` | `crm_customer_segments_post_validates_name` | 参数校验（`name` 必填） | 通过 |
| `PUT /crm/customer-segments/{segment_id}` | `crm_customer_segments_put_requires_fields` | 参数校验（更新字段不能为空） | 通过 |
| `DELETE /crm/customer-segments/{segment_id}` | `crm_customer_segments_delete_rejects_bad_segment_id` | 路径参数校验（UUID） | 通过 |
| `GET /crm/lifecycle-tasks` | `crm_lifecycle_get_requires_customers_read` | 权限门禁（缺少 `customers.read`） | 通过 |
| `POST /crm/lifecycle-tasks` | `crm_lifecycle_post_validates_stage` | 参数校验（`stage` 枚举） | 通过 |
| `PUT /crm/lifecycle-tasks/{task_id}` | `crm_lifecycle_put_requires_fields` | 参数校验（更新字段不能为空） | 通过 |
| `DELETE /crm/lifecycle-tasks/{task_id}` | `crm_lifecycle_delete_rejects_bad_task_id` | 路径参数校验（UUID） | 通过 |
| `GET /crm/service-tickets` | `crm_service_tickets_get_validates_updated_from` | 查询参数校验（RFC3339） | 通过 |
| `POST /crm/service-tickets` | `crm_service_tickets_post_requires_customer_name` | 参数校验（`customer_name` 必填） | 通过 |
| `PUT /crm/service-tickets/{ticket_id}` | `crm_service_tickets_put_requires_fields` | 参数校验（更新字段不能为空） | 通过 |
| `DELETE /crm/service-tickets/{ticket_id}` | `crm_service_tickets_delete_rejects_bad_ticket_id` | 路径参数校验（UUID） | 通过 |
| `GET /crm/points-ledgers` | `crm_points_ledgers_get_validates_time_range` | 时间参数校验 | 通过 |
| `GET /crm/stored-value-ledgers` | `crm_stored_value_ledgers_get_validates_time_range` | 时间参数校验 | 通过 |
| `GET /stats/{stats_type}` | `stats_get_requires_stats_or_reports_permission` | 权限门禁（`stats.read` / `reports.read`） | 通过 |
| `GET /stats/{stats_type}` | `stats_get_validates_stats_type` | 路径参数校验（`stats_type` 枚举） | 通过 |

## 4. E2E 测试（必要场景）

### 4.1 已通过场景

执行命令：`pnpm playwright test e2e/crm-stats-api.spec.ts`  
结果：`3 passed; 0 failed`

| 场景 | 覆盖接口 | 结果 |
| --- | --- | --- |
| 客户分群创建+筛选 | `customer-segments`（GET/POST/筛选查询） | 通过 |
| 生命周期任务创建+筛选 | `lifecycle-tasks`（GET/POST/筛选查询） | 通过 |
| 积分/储值页加载 + 统计页加载 | `points-ledgers`、`stored-value-ledgers`、`stats/{stats_type}` | 通过 |

### 4.2 失败场景（已记录）

执行命令：`pnpm playwright test e2e/customer-service-tickets.spec.ts`  
结果：`1 failed`

| 场景 | 覆盖接口 | 结果 | 失败原因 |
| --- | --- | --- | --- |
| 客服工单创建+搜索 | `service-tickets`（GET/POST） | 失败 | 下拉选项选择器在当前 UI 渲染状态下不稳定（元素可见性/视口问题）导致超时 |

说明：客服工单接口的 API 集成测试已通过，当前失败仅在前端 E2E 选择器稳定性层面，不影响后端接口补齐结论。

## 5. 逐接口测试通过标记

| 接口 | API 集成测试 | E2E 测试 | 最终标记 |
| --- | --- | --- | --- |
| `customer-segments` | 通过 | 通过 | 通过 |
| `lifecycle-tasks` | 通过 | 通过 | 通过 |
| `service-tickets` | 通过 | 未通过 | 部分通过（接口通过，E2E 待修） |
| `points-ledgers` | 通过 | 通过 | 通过 |
| `stored-value-ledgers` | 通过 | 通过 | 通过 |
| `stats/{stats_type}` | 通过 | 通过 | 通过 |

## 6. 本次附加修复

为支持稳定 E2E，本次对页面增加了必要 `data-testid`（客户分群、生命周期页）并补充了 CRM/Stats E2E 用例：

- `apps/erp-web/src/pages/customers/CustomerSegmentsPage.tsx`
- `apps/erp-web/src/pages/customers/CustomerLifecyclePage.tsx`
- `apps/erp-web/e2e/crm-stats-api.spec.ts`

并修正了既有客服工单 E2E 选择器以尝试提升稳定性：

- `apps/erp-web/e2e/customer-service-tickets.spec.ts`
