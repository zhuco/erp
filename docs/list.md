# ERP 文档统一入口（合并版）

> 更新日期：2026-04-10  
> 目标：把“功能目标、交付状态、实现规范、历史记录”拆层，并给出单一裁决口径，避免同主题多文档冲突。

## 1. 先看这里（推荐阅读顺序）

1. `README.md`：项目启动与总览。
2. `docs/list.md`（本文）：文档分层、冲突裁决、主题归并。
3. `docs/ops/ENGINEERING_STANDARDS.md`：工程命令与门禁统一入口。
4. `docs/ops/TASK_COMPLETION_MATRIX.md` + `docs/ops/RELEASE_GATE.md`：当前交付状态与发布门禁。
5. `docs/spec/*`：功能/数据/API 规范细节。

## 2. 文档分层与权威优先级

同一主题内容冲突时，按以下顺序裁决（从高到低）：

1. **实现与门禁事实层**
- `openapi/erp_api_v1.yaml`
- `migrations/*`
- `crates/*` 与 `apps/erp-web/*`
- `docs/ops/ENGINEERING_STANDARDS.md`
- `docs/ops/RELEASE_GATE.md`
- `docs/ops/TASK_COMPLETION_MATRIX.md`

2. **当前规范层（可执行）**
- `docs/spec/*_updates_YYYY-MM-DD.md`（带日期的增量规范）
- `docs/spec/12~17` 及其他仍在维护的领域规范
- `docs/web/*`
- `docs/ops/*`（除历史说明外）

3. **设计与计划层（过程留痕）**
- `docs/superpowers/specs/*`
- `docs/superpowers/plans/*`

4. **历史评估层（只做参考）**
- `docs/reports/*` 中的阶段性报告/快照
- 旧版状态机草案、旧版缺口矩阵

## 3. 同主题归并矩阵（冲突已裁决）

| 主题 | 当前权威文档 | 已归并/替代来源 | 裁决说明 |
| --- | --- | --- | --- |
| 项目目标与当前交付状态 | `docs/ops/TASK_COMPLETION_MATRIX.md` | `docs/reports/archive/2026-q1-gap-blocker-platform-summary.md`、`docs/reports/archive/feature_gap_matrix.md`、`docs/reports/archive/task_list.md`、`docs/reports/archive/launch_blockers.md` | 以 2026-04-09/10 的门禁与完成矩阵为准，旧报告保留作历史记录。 |
| 发布门禁与上线标准 | `docs/ops/RELEASE_GATE.md`、`docs/ops/RELEASE_GATE_CHECKLIST.md` | `docs/ops/GO_NO_GO_CHECKLIST.md`、`docs/ops/ACCEPTANCE.md` | `RELEASE_GATE*` 为当前执行面；Go/No-Go 与 Acceptance 作为补充。 |
| 工程命令入口 | `docs/ops/ENGINEERING_STANDARDS.md` | `docs/ops/progress_check.md`、脚本兼容层说明 | 所有验收命令以根级 `package.json` scripts 为准。 |
| 订单状态机与动作 | `docs/spec/12_sales_order_status_actions.md`、`docs/spec/14_action_permission_matrix.md`、`docs/spec/order_workflow_api_updates_2026-04-07.md` | `docs/spec/04_order_status_machine.md`、`docs/spec/SALES_ORDER_STATE_MACHINE.md` | 旧状态机文档视为早期草案，增量 API 更新优先。 |
| WMS 主链路 | `docs/spec/wms_workflow_api_updates_2026-04-07.md`、`docs/spec/wms_inbound_inventory_workflow_api_updates_2026-04-08.md` | `docs/spec/erp-process-flows/02-出入库流程.md` 等流程说明 | 带日期的 API 增量规范优先于流程描述文档。 |
| 平台同步与映射 | `docs/spec/platform_sync_and_mapping_api_updates_2026-04-08.md` | `docs/spec/03_order_field_mapping.md`、部分 reports 描述 | 同主题以最新日期文档为准。 |
| 售后聚合模型 | `docs/spec/aftersales_case_schema.md`、`docs/spec/05_after_sale_case_aggregation.md` | `docs/reports/*` 中售后描述 | 两篇规范互补，接口行为以 OpenAPI 与代码实现为准。 |
| 功能边界（业务域） | `docs/spec/erp-feature-catalog/*` | `docs/reports/archive/2026-q1-q2-delivery-readiness-summary.md`、`docs/reports/archive/functional_boundary_basic_delivery.md` | catalog 是长期维护版本，report 为阶段快照。 |
| 流程视角（跨部门） | `docs/spec/erp-process-flows/*` | `docs/reports/archive/task_list.md` 中流程片段 | 流程文档负责“怎么跑”，任务卡负责“做什么”。 |
| Web UI 开发与验收 | `docs/web/TESTING.md`、`docs/web/UI_MODULES.md` | 早期 web readiness 报告 | 验收与测试以 `TESTING.md` 当前门禁命令为准。 |
| 运维运行手册 | `docs/ops/STAGING_RUNBOOK.md`、`docs/ops/WEB_DEPLOYMENT.md`、`docs/ops/SSO_RUNBOOK.md`、`docs/ops/DR_RUNBOOK.md` | 历史部署笔记 | Runbook 系列为操作主入口。 |
| ErpDataGrid 专题 | `docs/ErpDataGrid/README.md`、`docs/ErpDataGrid/ErpDataGridEcommercePlan.md` | `docs/ErpDataGrid/AI-*.md` 子任务文档 | README + 总计划是入口，AI 分工文档作为实现拆分记录。 |

## 4. 当前“功能目标”口径（合并结果）

当前项目目标按两层理解：

1. **交付口径**  
以 `docs/ops/TASK_COMPLETION_MATRIX.md` 为主，Task1~Task9 已给出“最小闭环已完成”与验证命令。

2. **持续演进口径**  
以 `docs/spec/*` 与 `docs/ops/*` 为主，持续迭代 API、权限、门禁、性能、可观测性等细项。

若你看到任一报告文档与上述口径冲突，优先采信 `TASK_COMPLETION_MATRIX.md`、`RELEASE_GATE.md`、OpenAPI 与代码实现。

## 5. 历史文档保留策略

以下文档不删除，但默认视为历史参考：

- `docs/reports/archive/README.md`
- `docs/reports/archive/2026-q1-gap-blocker-platform-summary.md`
- `docs/reports/archive/2026-q1-q2-delivery-readiness-summary.md`
- `docs/reports/archive/progress_check_*.md`
- `docs/reports/archive/feature_gap_matrix.md`
- `docs/reports/archive/task_list.md`
- `docs/reports/archive/launch_blockers.md`
- `docs/reports/archive/platform_integration_report.md`
- `docs/reports/archive/functional_boundary_basic_delivery.md`
- `docs/reports/archive/implementation_and_e2e_plan_202602.md`
- `docs/reports/archive/web_readiness_report_20260108.md`
- `docs/reports/archive/api_gap_completion_checklist_20260305.md`
- 旧版状态机草案文档（如 `docs/spec/SALES_ORDER_STATE_MACHINE.md`）

保留原因：

- 需要审计“当时如何判断缺口与决策”。
- 需要追溯多轮改造的背景与风险变化。

## 6. 维护规则（避免再次失控）

新增或更新文档时，遵循以下规则：

1. 同主题已有文档时，不新开平行“总览”文档，优先在原文追加“更新章节 + 日期”。
2. 必须在本文“同主题归并矩阵”补一行映射（新增/替代关系）。
3. 若内容只在某个阶段有效，标题需带日期后缀或在开头明确“阶段性文档”。
4. 任何与门禁、上线、完成状态有关的结论，必须回写 `docs/ops/TASK_COMPLETION_MATRIX.md` 或 `docs/ops/RELEASE_GATE*.md`。
5. `README.md` 只保留导航，不承载长篇状态结论，避免重复维护。
