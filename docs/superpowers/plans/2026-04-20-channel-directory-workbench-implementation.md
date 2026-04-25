# 渠道目录一体化工作台 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `/channels` 重构为真正的渠道目录二级目录页，接入一体化数据作业台，并补齐渠道主数据字段、预留服务页路由与统一菜单结构。

**Architecture:** 保留统一工作台外壳，重做渠道域自己的数据模型、主表列、左侧筛选与右侧详情。导航层改成“目录 -> 渠道目录”，页面入口与文件组织对齐其它二级目录页规范。

**Tech Stack:** React、TypeScript、Ant Design、React Query、Vitest、统一工作台模板

---

### Task 1: 锁定导航与路由变更

**Files:**
- Modify: `apps/erp-web/src/app/layout/menu.test.ts`
- Modify: `apps/erp-web/src/app/layout/headerMenuModel.test.ts`
- Modify: `apps/erp-web/src/app/layout/moduleSearch.test.ts`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts`
- Modify: `apps/erp-web/src/app/router.tsx`
- Modify: `apps/erp-web/src/app/layout/menu.tsx`

- [ ] **Step 1: 写失败测试，表达目录二级化与新路由白名单**
- [ ] **Step 2: 运行相关测试，确认因目录结构/路由未实现而失败**
- [ ] **Step 3: 最小实现菜单、头部模块、搜索索引与路由白名单变更**
- [ ] **Step 4: 重新运行相关测试，确认通过**

### Task 2: 扩展渠道主数据模型与 mock

**Files:**
- Modify: `apps/erp-web/src/api/channels.ts`
- Modify: `apps/erp-web/src/api/mocks/data.ts`
- Modify: `apps/erp-web/src/api/mocks/masterData.ts`
- Test: `apps/erp-web/src/pages/channels/channelDirectoryModel.test.ts`

- [ ] **Step 1: 写失败测试，锁定渠道字段扩展与线上/线下规则**
- [ ] **Step 2: 运行测试，确认失败原因正确**
- [ ] **Step 3: 最小实现 `Channel` 类型、payload、mock 数据与 mock CRUD 扩展**
- [ ] **Step 4: 重新运行模型测试，确认通过**

### Task 3: 新建渠道目录页面结构

**Files:**
- Create: `apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`
- Create: `apps/erp-web/src/pages/channels/ChannelDirectoryPanels.tsx`
- Create: `apps/erp-web/src/pages/channels/channelDirectoryModel.ts`
- Create: `apps/erp-web/src/pages/channels/channelDirectoryColumns.tsx`
- Create: `apps/erp-web/src/pages/channels/channelDirectoryPage.css`
- Test: `apps/erp-web/src/pages/channels/channelDirectoryPage.test.tsx`

- [ ] **Step 1: 写失败测试，锁定页面必须启用左侧/右侧、关闭底部、无记号**
- [ ] **Step 2: 运行测试，确认失败**
- [ ] **Step 3: 最小实现渠道目录页面骨架与统一工作台接入**
- [ ] **Step 4: 重新运行页面结构测试，确认通过**

### Task 4: 实现渠道目录业务交互

**Files:**
- Modify: `apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`
- Modify: `apps/erp-web/src/pages/channels/ChannelDirectoryPanels.tsx`
- Modify: `apps/erp-web/src/pages/channels/channelDirectoryModel.ts`
- Modify: `apps/erp-web/src/pages/channels/channelDirectoryColumns.tsx`
- Test: `apps/erp-web/src/pages/channels/channelDirectoryPage.test.tsx`

- [ ] **Step 1: 写失败测试，覆盖搜索、筛选、右侧详情与新建/修改校验**
- [ ] **Step 2: 运行测试，确认失败**
- [ ] **Step 3: 最小实现主表、工具栏动作、抽屉表单、右侧详情与服务动作规则**
- [ ] **Step 4: 重新运行页面测试，确认通过**

### Task 5: 接入预留服务页与兼容入口

**Files:**
- Modify: `apps/erp-web/src/app/router.tsx`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts`
- Modify: `apps/erp-web/src/pages/Placeholders.tsx`

- [ ] **Step 1: 写失败测试，锁定 `/channels/service-purchase` 与 `/channels/authorize` 可访问**
- [ ] **Step 2: 运行测试，确认失败**
- [ ] **Step 3: 最小实现预留页与 reduced UI 白名单**
- [ ] **Step 4: 重新运行测试，确认通过**

### Task 6: 清理旧入口并完成验证

**Files:**
- Modify: `apps/erp-web/src/pages/channels/ChannelsPage.tsx`
- Modify: `apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`
- Test: 相关测试集合

- [ ] **Step 1: 将旧 `ChannelsPage` 退为兼容层或停止作为主入口**
- [ ] **Step 2: 运行目标测试集**
- [ ] **Step 3: 运行构建或至少运行受影响模块测试**
- [ ] **Step 4: 记录未覆盖风险并准备交付说明**
