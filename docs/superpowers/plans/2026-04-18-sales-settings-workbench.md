# 销售设置工作台 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将销售基础设置与策略设置整合为一个统一的销售设置工作台，并保持旧路由兼容。

**Architecture:** 通过新增统一工作台页面承载总览、分组导航和基础设置表单，并将现有销售策略页抽取为可嵌入的策略管理面板。旧的 `sales/settings` 与 `sales/strategy-settings` 路由均指向同一工作台，只是初始定位分组不同。

**Tech Stack:** React 18、TypeScript、Ant Design、React Query、Vitest、现有 UiModule 组件

---

### Task 1: 建立工作台摘要模型

**Files:**
- Create: `apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.ts`
- Create: `apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.test.ts`

- [ ] **Step 1: 写失败测试**
- [ ] **Step 2: 运行测试并确认失败**
- [ ] **Step 3: 实现销售设置摘要、分组状态和策略分类汇总模型**
- [ ] **Step 4: 运行单测并确认通过**

### Task 2: 抽取策略管理面板

**Files:**
- Create: `apps/erp-web/src/pages/sales/SalesStrategyManagementPanel.tsx`
- Modify: `apps/erp-web/src/pages/sales/salesStrategySettingsPage.css`
- Modify: `apps/erp-web/src/pages/sales/SalesStrategySettingsPage.tsx`

- [ ] **Step 1: 复制当前策略管理逻辑到可复用面板组件**
- [ ] **Step 2: 让原路由页面改为统一工作台入口**
- [ ] **Step 3: 保持策略操作、新建、修改、启停逻辑不回退**

### Task 3: 实现统一销售设置工作台

**Files:**
- Create: `apps/erp-web/src/pages/sales/SalesSettingsWorkbenchPage.tsx`
- Create: `apps/erp-web/src/pages/sales/salesSettingsWorkbenchPage.css`
- Modify: `apps/erp-web/src/pages/sales/SalesSettingsPage.tsx`

- [ ] **Step 1: 组装头部、总览卡、左侧分组导航**
- [ ] **Step 2: 将基础设置重构为分组摘要 + 表单编辑**
- [ ] **Step 3: 将地址和备注同步合并为统一治理分组**
- [ ] **Step 4: 将策略中心嵌入工作台**

### Task 4: 路由与入口兼容

**Files:**
- Modify: `apps/erp-web/src/app/router.tsx`
- Modify: `apps/erp-web/src/app/layout/menu.tsx`

- [ ] **Step 1: 保持两个旧路由可访问**
- [ ] **Step 2: 让策略入口默认定位到策略中心**
- [ ] **Step 3: 校验菜单文案与入口不冲突**

### Task 5: 验证

**Files:**
- Verify only

- [ ] **Step 1: 运行工作台模型单测**
- [ ] **Step 2: 运行针对销售设置页面的类型检查或相关测试**
- [ ] **Step 3: 运行 lint 或最小范围验证命令**
- [ ] **Step 4: 记录未执行项与残余风险**
