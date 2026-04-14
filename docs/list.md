# ERP 文档统一入口（2026-04-13）

> 目标：把“项目目标、完整功能清单、当前成熟度、实现细节、历史记录”分层，避免同主题多份文档并行失控。

## 1. 先看这 4 份

1. `README.md`
   项目启动、仓库结构与开发入口。

2. `docs/spec/erp-industrial-product-blueprint.md`
   项目产品目标蓝图的权威入口。这里定义项目到底要做成什么产品、菜单应该如何组织、核心闭环是什么。

3. `docs/spec/erp-launch-full-feature-matrix.md`
   计划上线完整功能实现表的权威入口。这里定义工业级电商 ERP 应具备哪些能力、各能力当前处于什么状态、哪些能力属于 P0 / P1 / P2。

4. `docs/reports/erp-industrial-maturity-assessment-2026-04-13.md`
   当前项目成熟度评估的权威入口。这里用于看“我们离成熟产品还有多远”。

## 2. 文档权威层级

同一主题发生冲突时，按以下顺序裁决：

1. 真实代码与契约层
   - `openapi/erp_api_v1.yaml`
   - `migrations/*`
   - `crates/*`
   - `apps/erp-web/*`

2. 上线与门禁层
   - `docs/ops/RELEASE_GATE.md`
   - `docs/ops/TASK_COMPLETION_MATRIX.md`
   - `docs/ops/SECURITY_GATE.md`
   - `docs/ops/*runbook*.md`

3. 产品目标与功能规划层
   - `docs/spec/erp-industrial-product-blueprint.md`
   - `docs/spec/erp-launch-full-feature-matrix.md`

4. 当前实现评估层
   - `docs/reports/erp-industrial-maturity-assessment-2026-04-13.md`

5. 领域拆分文档层
   - `docs/spec/erp-feature-catalog/*`
   - `docs/spec/erp-process-flows/*`
   - `docs/spec/*_updates_*.md`

6. 历史记录层
   - `docs/reports/archive/*`
   - `docs/superpowers/specs/*`
   - `docs/superpowers/plans/*`

## 3. 合并后的阅读路径

### 3.1 看项目目标

读：

- `docs/spec/erp-industrial-product-blueprint.md`

### 3.2 看完整功能范围与上线清单

读：

- `docs/spec/erp-launch-full-feature-matrix.md`

### 3.3 看当前完成度与上线风险

读：

- `docs/reports/erp-industrial-maturity-assessment-2026-04-13.md`
- `docs/ops/TASK_COMPLETION_MATRIX.md`
- `docs/ops/RELEASE_GATE.md`

### 3.4 看具体领域实现细节

按主题进入：

- 销售与订单：`docs/spec/01_architecture.md`、`docs/spec/03_order_field_mapping.md`、`docs/spec/12_sales_order_status_actions.md`
- 仓储与库存：`docs/spec/inventory_schema.md`、`docs/spec/wms_workflow_api_updates_2026-04-07.md`
- 采购与入库：`docs/spec/wms_inbound_inventory_workflow_api_updates_2026-04-08.md`
- 平台同步与映射：`docs/spec/platform_sync_and_mapping_api_updates_2026-04-08.md`
- 售后与逆向：`docs/spec/aftersales_case_schema.md`
- 安全与审计：`docs/spec/security_and_audit.md`

## 4. 当前拆分文档的定位

以下文档仍然保留，但不再单独承担“项目总目标”职责：

- `docs/spec/erp-feature-catalog/*`
  作用：保留各业务域的详细拆分说明，是产品蓝图的来源文档。

- `docs/spec/erp-process-flows/*`
  作用：保留跨部门流程说明，是流程蓝图的来源文档。

- `docs/spec/*_updates_*.md`
  作用：保留某次 API 或工作流增量变更说明。

- `docs/reports/archive/*`
  作用：保留历史阶段判断，不再作为当前状态唯一依据。

## 5. 当前最值得注意的裁决规则

- 关于“项目目标是什么”，以 `docs/spec/erp-industrial-product-blueprint.md` 为准。
- 关于“完整功能清单和上线计划怎么排”，以 `docs/spec/erp-launch-full-feature-matrix.md` 为准。
- 关于“目前做到什么程度、还能不能上线”，以 `docs/reports/erp-industrial-maturity-assessment-2026-04-13.md` 为准。
- 关于“某个接口或字段到底有没有实现”，以 `openapi/erp_api_v1.yaml` 和真实代码为准。
- 关于“是否允许发布”，以 `docs/ops/RELEASE_GATE.md` 与相关门禁命令为准。

## 6. 后续维护规则

1. 新增总览性文档前，先判断是否应该直接更新上述三份权威文档。
2. 如果只是某个领域的增量实现，不新增新的总目标文档，优先更新对应领域拆分文档。
3. 新增阶段性评估报告时，应放入 `docs/reports/`，并在标题中带日期。
4. 新的功能结论、成熟度结论、上线结论必须同步回写到权威入口，避免再次出现“有文档但没有统一口径”。
