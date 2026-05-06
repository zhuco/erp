---
name: erp-ui
description: ERP Web UI 模块化开发与页面重构指导。适用于需求如“按 ERP Web UI 模块化开发说明完成新建订单 UI”“按 UI 开发文档完成客户档案/库存查询 UI”“需要模块 1-5 或模块化改造”等场景。
---

# erp-ui

## 概览

指导在 `apps/erp-web` 中基于模块化规范完成 ERP Web UI 的新建与重构，确保工作台布局与组件复用一致。

## 快速流程

1. 明确需求类型（新建/重构）与页面位置，确认需要的模块编号（如 1-5）。
2. 阅读 `docs/FRONTEND_GUIDE.md` 获取工作台布局、复用方式、表格决策与测试要求；用户提到“UI 开发文档”默认指该文件。
3. 选择入口页面（`apps/erp-web/src/pages/...`）并对照示例页面复用模块组件。
4. 按模块说明挂载 `WorkbenchLayout` 面板与 UI 组件，保持布局一致性。
5. 处理样式与交互细节，保持已有命名与结构，避免重复造轮子。

## 模块选型与复用要点

- 订单类新页面通常复用模块 1-5（左侧快速查询、三栏工具栏、主表、底部子表、右侧详情）。
- 策略列表类页面复用模块 6（策略表）。
- 设置类页面复用模块 7（设置页布局）。
- 需要工作台面板时，使用 `useWorkbenchPanels()` 挂载 left/right/bottom 面板。

## 参考入口

- 规范文档：`docs/FRONTEND_GUIDE.md`
- 模块组件目录：`apps/erp-web/src/components/ui-modules/`
- 参考页面：`apps/erp-web/src/pages/orders/OrderReviewPage.tsx`（模块 1-5）、`apps/erp-web/src/pages/sales/SalesStrategySettingsPage.tsx`（模块 6）、`apps/erp-web/src/pages/settings/SettingsPage.tsx`（模块 7）

## 测试准备（当前不强制）

当前需求不要求补 E2E 或单测，但页面交互应预留 `data-testid` 等测试标识，便于后续补充测试时快速接入（规范可参考 `docs/FRONTEND_GUIDE.md`）。
