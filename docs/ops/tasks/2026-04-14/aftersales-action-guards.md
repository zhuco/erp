---
name: 售后动作规则统一
goal: 统一售后列表页和详情页的动作可用规则并补齐测试
background: 避免同一售后状态在不同页面出现不一致操作，提升工业级操作一致性
owner: integrator-01
branch: codex/aftersales-action-guards-20260414
worktree: .worktrees/aftersales-action-guards-20260414
allowed_files:
  - apps/erp-web/src/pages/afterSales/afterSalesActionGuards.ts
  - apps/erp-web/src/pages/afterSales/afterSalesActionGuards.test.ts
  - apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesDetailPage.tsx
disallowed_files:
  - apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx
dependencies:
  - 不得改动 afterSales import / receive 相关文件
dod:
  - 列表页与详情页使用同一套动作守卫
  - 补齐 vitest 规则测试
  - 不改变现有已通过的售后主链路行为
test_commands:
  - pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesActionGuards.test.ts
delivery: Markdown + 相关资源
---

## 目标
统一售后列表页和详情页的动作可用规则并补齐测试

## 业务背景
避免同一售后状态在不同页面出现不一致操作，提升工业级操作一致性

## DoD
- 列表页与详情页使用同一套动作守卫
- 补齐 vitest 规则测试
- 不改变现有已通过的售后主链路行为

## 依赖前提
- 不得改动 afterSales import / receive 相关文件

## 测试命令
- `pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesActionGuards.test.ts`

## 交付格式
Markdown + 相关资源
