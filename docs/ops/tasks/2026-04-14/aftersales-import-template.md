---
name: 售后导入模板落地
goal: 将批量导入页的模板下载从占位提示实现为真实可下载 CSV 模板并补齐测试
background: 补齐售后批量导入的工业级可操作性，降低人工模板传递成本
owner: integrator-01
branch: codex/aftersales-import-template-20260414
worktree: .worktrees/aftersales-import-template-20260414
allowed_files:
  - apps/erp-web/src/pages/afterSales/afterSalesImportTemplate.ts
  - apps/erp-web/src/pages/afterSales/afterSalesImportTemplate.test.ts
  - apps/erp-web/src/pages/afterSales/AfterSalesBulkEditPage.tsx
disallowed_files:
  - apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx
dependencies:
  - 不得改动列表页与详情页动作逻辑
dod:
  - 下载模板按钮触发真实模板下载
  - CSV 模板字段覆盖售后导入核心列
  - 补齐 vitest 测试
test_commands:
  - pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesImportTemplate.test.ts
delivery: Markdown + 相关资源
---

## 目标
将批量导入页的模板下载从占位提示实现为真实可下载 CSV 模板并补齐测试

## 业务背景
补齐售后批量导入的工业级可操作性，降低人工模板传递成本

## DoD
- 下载模板按钮触发真实模板下载
- CSV 模板字段覆盖售后导入核心列
- 补齐 vitest 测试

## 依赖前提
- 不得改动列表页与详情页动作逻辑

## 测试命令
- `pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesImportTemplate.test.ts`

## 交付格式
Markdown + 相关资源
