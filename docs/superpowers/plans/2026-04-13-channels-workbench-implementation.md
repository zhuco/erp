# 渠道目录工作台重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `/channels` 重构为基于最新工作区模板的渠道工作台，并补齐系统自动生成的授权回调地址展示与回跳识别能力。

**Architecture:** 保留现有主表工业级交互，新增纯逻辑模型层负责回调地址生成与回跳解析，再将左侧筛选面板和右侧详情面板拆成页面专属展示组件。页面继续通过工作台模板容器接入左右面板，避免把本轮改造成接口重写工程。

**Tech Stack:** React 18、TypeScript、Ant Design 6、TanStack Query、Vitest、Vite

---

## 文件结构

### 新建文件

- `apps/erp-web/src/pages/channels/channelWorkbenchModel.ts`
  - 回调地址生成、回跳参数解析、统计摘要纯逻辑
- `apps/erp-web/src/pages/channels/channelWorkbenchModel.test.ts`
  - 纯逻辑测试
- `apps/erp-web/src/pages/channels/ChannelWorkbenchPanels.tsx`
  - 左侧筛选面板、右侧详情面板、页面概览组件
- `apps/erp-web/src/pages/channels/ChannelWorkbenchPanels.test.ts`
  - 面板渲染测试

### 修改文件

- `apps/erp-web/src/pages/channels/ChannelsPage.tsx`
  - 接入工作台面板、回调识别、复制动作和新头部
- `apps/erp-web/src/pages/channels/channelsPage.css`
  - 新工作台样式

### 参考文件

- `apps/erp-web/src/components/workbench/templates/WorkbenchMainTableTemplate.tsx`
- `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.tsx`
- `apps/erp-web/src/pages/sales/SalesShopConfigPage.tsx`
- `docs/superpowers/specs/2026-04-13-channels-workbench-design.md`

---

### Task 1: 为渠道工作台纯逻辑建立失败测试

**Files:**
- Create: `apps/erp-web/src/pages/channels/channelWorkbenchModel.test.ts`
- Create: `apps/erp-web/src/pages/channels/channelWorkbenchModel.ts`
- Test: `apps/erp-web/src/pages/channels/channelWorkbenchModel.test.ts`

- [ ] **Step 1: 写失败测试，覆盖回调地址生成和回跳解析**

```ts
expect(
  buildChannelAuthCallbackUrl({
    origin: 'https://erp.example.com',
    routerMode: 'browser',
    routerBasePath: '/',
    channelId: 'CH-1',
    platform: 'taobao',
  }),
).toBe('https://erp.example.com/channels?auth_callback=1&channel_id=CH-1&platform=taobao');
```

```ts
expect(
  parseChannelAuthCallbackSearch('?auth_callback=1&channel_id=CH-1&platform=taobao&code=abc'),
).toMatchObject({ channelId: 'CH-1', platform: 'taobao', hasCode: true });
```

- [ ] **Step 2: 运行失败测试**

Run: `pnpm --dir apps/erp-web vitest run src/pages/channels/channelWorkbenchModel.test.ts`
Expected: FAIL，原因是模型文件或导出尚不存在

- [ ] **Step 3: 写最小实现**

实现：

- `buildChannelAuthCallbackUrl`
- `parseChannelAuthCallbackSearch`
- `summarizeChannels`
- `buildChannelQuickFilters`

- [ ] **Step 4: 再次运行测试**

Run: `pnpm --dir apps/erp-web vitest run src/pages/channels/channelWorkbenchModel.test.ts`
Expected: PASS

---

### Task 2: 为左右工作台面板建立失败测试

**Files:**
- Create: `apps/erp-web/src/pages/channels/ChannelWorkbenchPanels.test.ts`
- Create: `apps/erp-web/src/pages/channels/ChannelWorkbenchPanels.tsx`
- Test: `apps/erp-web/src/pages/channels/ChannelWorkbenchPanels.test.ts`

- [ ] **Step 1: 写失败测试，覆盖右侧授权信息和左侧筛选视图**

```ts
expect(markup).toContain('data-testid="channels-auth-callback-url"');
expect(markup).toContain('data-testid="channels-left-quick-filter-ACTIVE"');
expect(markup).toContain('去店铺接入');
```

- [ ] **Step 2: 运行失败测试**

Run: `pnpm --dir apps/erp-web vitest run src/pages/channels/ChannelWorkbenchPanels.test.ts`
Expected: FAIL，原因是面板组件尚不存在

- [ ] **Step 3: 写最小实现**

实现：

- `ChannelsWorkbenchOverview`
- `ChannelsWorkbenchLeftPanel`
- `ChannelsWorkbenchRightPanel`

要求：

- 右侧展示渠道资料、授权信息、联动入口
- 授权区展示系统生成的回调地址
- 左侧展示预置视图与筛选摘要

- [ ] **Step 4: 再次运行测试**

Run: `pnpm --dir apps/erp-web vitest run src/pages/channels/ChannelWorkbenchPanels.test.ts`
Expected: PASS

---

### Task 3: 将渠道页接入新工作台 UI

**Files:**
- Modify: `apps/erp-web/src/pages/channels/ChannelsPage.tsx`
- Modify: `apps/erp-web/src/pages/channels/channelsPage.css`

- [ ] **Step 1: 接入回调模型与面板组件前，先写/补失败断言**

本轮以单元测试为主，不新增昂贵的页面级 E2E；确保 Task 1 和 Task 2 的测试已经红灯。

- [ ] **Step 2: 在 `ChannelsPage.tsx` 中接入新结构**

实现：

- 使用模型层生成授权回调地址
- 用 `useLocation` 解析回跳参数
- 自动选中回跳目标渠道
- 用页面专属左/右面板替换旧 `UiModule01QuickFiltersPanel` 和 `UiModule05DetailPanel`
- 头部增加概览卡片和回跳提示
- 工具栏右侧改为页面专属操作

- [ ] **Step 3: 更新样式**

实现：

- 概览卡片
- 左侧筛选面板
- 右侧授权地址块
- 当前筛选摘要

- [ ] **Step 4: 运行相关单元测试**

Run: `pnpm --dir apps/erp-web vitest run src/pages/channels/channelWorkbenchModel.test.ts src/pages/channels/ChannelWorkbenchPanels.test.ts`
Expected: PASS

---

### Task 4: 验证类型与页面回归

**Files:**
- Modify: `apps/erp-web/src/pages/channels/ChannelsPage.tsx`
- Modify: `apps/erp-web/src/pages/channels/channelsPage.css`

- [ ] **Step 1: 运行类型检查**

Run: `pnpm --dir apps/erp-web typecheck`
Expected: PASS

- [ ] **Step 2: 如有必要修正类型或导入错误**

- [ ] **Step 3: 重新运行受影响单元测试**

Run: `pnpm --dir apps/erp-web vitest run src/pages/channels/channelWorkbenchModel.test.ts src/pages/channels/ChannelWorkbenchPanels.test.ts`
Expected: PASS

---

## 执行备注

- 当前工作区已有用户未提交修改，本轮只操作 `/channels` 相关文件与本次新增文档。
- 由于本会话未获得用户对 subagent delegation 的授权，本轮默认采用 Inline Execution。
- 本轮不自动创建 commit；如用户需要，我可以在实现完成并验证后再整理提交。
