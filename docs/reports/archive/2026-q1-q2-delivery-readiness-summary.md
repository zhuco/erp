# 2026 Q1-Q2 交付与就绪历史总览

> 合并日期：2026-04-10  
> 合并范围（历史文档）：  
> `docs/reports/archive/functional_boundary_basic_delivery.md`  
> `docs/reports/archive/implementation_and_e2e_plan_202602.md`  
> `docs/reports/archive/web_readiness_report_20260108.md`  
> `docs/reports/archive/api_gap_completion_checklist_20260305.md`

## 1. 文档定位

本文件将 Q1-Q2 阶段内“交付边界、Web 就绪检查、营销实现计划、CRM/Stats API 补齐清单”做统一归纳，减少重复阅读成本。

本文件仅用于历史追溯，不作为当前发布依据。当前口径请优先参考：

- `docs/ops/TASK_COMPLETION_MATRIX.md`
- `docs/ops/RELEASE_GATE.md`
- `docs/ops/RELEASE_GATE_CHECKLIST.md`
- `openapi/erp_api_v1.yaml` 与代码实现

## 2. 历史结论（合并后）

### 2.1 交付边界层面

1. 当时已形成 ERP 主链路“基础可交付版”边界定义，强调订单、仓储、库存、售后、采购财务、报表导出与系统治理闭环。
2. 同期明确将“计划补货、专题统计驾驶舱、系统扩展页生产化”等能力列为阶段外范围。

### 2.2 Web 生产就绪层面

1. 历史检查给出 “多数项 PASS，少数环境项 WEAK” 结论（例如本地 pnpm 环境前置）。
2. 认证权限、多平台支持、错误空态、表格能力、打印、E2E 与部署链路在当时已有完整检查记录。

### 2.3 营销与 API 补齐层面

1. 营销模块在当时形成了“OpenAPI 契约补齐 + 前端 Mock + 自动化测试”的执行计划与落地记录。
2. CRM/Stats 相关接口完成了 API 级补齐与集成测试覆盖，E2E 层存在个别选择器稳定性问题并已留痕。

## 3. 与当前文档体系的关系

1. 边界定义：当前以 `docs/spec/erp-feature-catalog/*` 与 `docs/spec/erp-process-flows/*` 为主。
2. 就绪判断：当前以 `docs/ops/TASK_COMPLETION_MATRIX.md` 与 `docs/ops/RELEASE_GATE*.md` 为主。
3. API 事实：当前以 OpenAPI 与代码测试结果为主。

## 4. 使用建议

1. 需要复盘 2026 年上半年的决策背景时，先读本文件，再按需打开对应 archive 原文。
2. 需要做当前发布评估时，跳过本文件，直接使用 `docs/ops/*` 最新门禁文档。

