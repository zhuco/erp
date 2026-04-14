---
name: 售后收货页交互增强
goal: 增强售后收货页的仓库默认选择和空仓库提示并补齐测试
background: 提高退货收货场景的完成率，减少仓库选择错误和空数据卡死
owner: integrator-01
branch: codex/aftersales-receive-model-20260414
worktree: .worktrees/aftersales-receive-model-20260414
allowed_files:
  - apps/erp-web/src/pages/afterSales/afterSalesReceiveModel.ts
  - apps/erp-web/src/pages/afterSales/afterSalesReceiveModel.test.ts
  - apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx
disallowed_files:
  - apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesDetailPage.tsx
dependencies:
  - 不得修改列表页和详情页动作逻辑
dod:
  - 默认仓库选择规则可复用
  - 无仓库时页面有明确提示并阻止提交
  - 补齐 vitest 测试
test_commands:
  - pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesReceiveModel.test.ts
delivery: Markdown + 相关资源
---

## 目标
增强售后收货页的仓库默认选择和空仓库提示并补齐测试

## 业务背景
提高退货收货场景的完成率，减少仓库选择错误和空数据卡死

## DoD
- 默认仓库选择规则可复用
- 无仓库时页面有明确提示并阻止提交
- 补齐 vitest 测试

## 依赖前提
- 不得修改列表页和详情页动作逻辑

## 测试命令
- `pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesReceiveModel.test.ts`

## 交付格式
Markdown + 相关资源
