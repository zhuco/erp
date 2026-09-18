# 工业级工作台模板 95% 完成度 Implementation Plan

> 历史计划。自 2026-09-07 起，本文的 AG Grid 技术路线及旧表格接入要求全部失效；当前约束见 [自研组件库规范](../../UI_COMPONENT_LIBRARY.md)，当前执行队列见 [路线图](../../ROADMAP.md)。不得据此重新引入 AG Grid。

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将当前工业级 UI 工作台模板从“核心可用、系统迁移未完成”提升到 95% 以上：模板抽象业务无关、核心页面统一接入、旧模板基本清零、模板治理门禁稳定通过。

**Architecture:** 先把 `WorkbenchPageTemplate` 从订单复核旧模板中解耦，形成真正业务无关的工业工作台模板；再用模板契约和治理脚本驱动页面迁移；最后用单测、E2E、lint/typecheck/build 和覆盖率报告锁定 95% 目标。页面层只能替换数据源、列定义、搜索、按钮和面板内容，不允许复制布局骨架。

**Tech Stack:** React 18、TypeScript、Vite、Ant Design、AG Grid、Vitest、Playwright、pnpm、现有 `WorkbenchLayout` / `WorkbenchPageTemplate` / `SettingsPageTemplate` / `UiModule*`。

---

## 0. 95% 完成度定义

### 必须达成的量化门槛

- `pnpm --dir apps/erp-web run lint:page-templates` 退出码为 0。
- 适用路由页面模板接入率 `>= 95%`，由 `apps/erp-web/scripts/check-page-template-usage.mjs` 输出并校验。
- `apps/erp-web/src/pages/**` 中业务页面不再直接导入旧 `OrderReviewWorkbenchTemplate` / `OrderReviewWorkbenchToolbarTemplate`，允许保留的兼容文件必须集中在 `apps/erp-web/src/components/workbench/templates/` 内。
- 强约束页面全部通过：订单复核、订单查询、订单重审、销售查询、系统设置、销售设置、渠道/仓库/客户/标签/WMS 基础资料工作台。
- 新增页面必须使用 `WorkbenchPageTemplate`、`SettingsPageTemplate` 或显式登记为非工作台豁免。
- 核心验证命令全部通过：
  - `pnpm --dir apps/erp-web run lint:page-templates`
  - `pnpm --dir apps/erp-web exec vitest --run src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts src/app/layout/WorkbenchLayout.interactions.test.ts src/app/layout/WorkbenchLayout.splitter-a11y.test.ts src/components/page-templates/workbench/workbenchContracts.test.ts src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`
  - `pnpm --filter erp-web typecheck`
  - `pnpm --filter erp-web build`
  - `pnpm --filter erp-web test:e2e -- --project=chromium e2e/workbench-a11y.spec.ts e2e/workbench-layout-presets.spec.ts e2e/order-workbench-pages.spec.ts e2e/order-review-interactions.spec.ts`

### 完成度评分口径

- 模板内核：`30%`，业务无关模板、面板、工具栏、快捷键、可访问性、状态持久化。
- 页面迁移：`35%`，适用路由页面接入率、旧模板清理、强约束页面通过。
- 操作一致性：`20%`，工具栏位置、搜索入口、筛选/详情/底部面板、表格行为和快捷键一致。
- 测试治理：`15%`，单测/E2E/脚本门禁/覆盖率报告稳定通过。

---

## 1. 目标文件结构

### 新增文件

- `apps/erp-web/src/components/page-templates/workbench/IndustrialWorkbenchTemplate.tsx`：真正业务无关的工作台页面骨架。
- `apps/erp-web/src/components/page-templates/workbench/industrialWorkbenchTemplate.css`：业务无关工作台模板样式，逐步替代 `order-review-*` 语义样式。
- `apps/erp-web/src/components/page-templates/workbench/IndustrialWorkbenchTemplate.test.ts`：模板契约和业务语义隔离测试。
- `apps/erp-web/src/components/page-templates/workbench/workbenchTemplateMetrics.ts`：页面模板覆盖率统计和 95% 阈值计算逻辑。
- `apps/erp-web/src/components/page-templates/workbench/workbenchTemplateMetrics.test.ts`：覆盖率统计单测。
- `apps/erp-web/e2e/workbench-template-consistency.spec.ts`：跨模块工具栏、搜索、面板、主表一致性 E2E。

### 重点修改文件

- `apps/erp-web/src/components/page-templates/workbench/WorkbenchPageTemplate.tsx`：从薄包装改为指向 `IndustrialWorkbenchTemplate`，保留外部 API 兼容。
- `apps/erp-web/src/components/page-templates/workbench/WorkbenchToolbar.tsx`：继续兼容现有 toolbar props，补充统一工具栏契约。
- `apps/erp-web/src/components/page-templates/workbench/index.ts`：导出新模板和覆盖率工具。
- `apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx`：仅作为兼容层保留，不再被页面直接使用。
- `apps/erp-web/scripts/check-page-template-usage.mjs`：从“字符串扫描+基线”升级为“适用页面分类+覆盖率阈值+旧模板强约束”。
- `apps/erp-web/scripts/page-template-baseline.json`：改造为明确的豁免清单；每项必须包含原因和到期策略。
- `apps/erp-web/src/pages/orders/OrderQueryPage.tsx`：从旧订单复核模板迁移到 `WorkbenchPageTemplate`。
- `apps/erp-web/src/pages/system/SystemSettingsPage.tsx`：从 `UiModule07SettingsLayout` 迁移到 `SettingsPageTemplate`。
- `apps/erp-web/src/pages/inbounds/*.tsx`：入库相关工作台批量迁移。
- `apps/erp-web/src/pages/skus/SkuPage.tsx`、`apps/erp-web/src/pages/skus/SkuImportPage.tsx`：SKU 页面迁移。
- `apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`：WMS 出库确认页面迁移。
- `apps/erp-web/src/pages/logistics/*.tsx`、`apps/erp-web/src/pages/system/*.tsx`：旧设置布局迁移。
- `apps/erp-web/src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`：扩大统一模板接入断言。

### 原则上不修改

- `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`：全局壳层已相对成熟，除非新模板确实需要通用能力，不在本计划中重构。
- `apps/erp-web/src/components/ui-modules/UiModule03MainTableSection.tsx`：只补契约和测试，不做大规模表格重写。
- 后端 API 和数据模型：本计划只做 UI 模板统一，不改业务语义。

---

## 2. 执行策略

### 推荐批次

1. **模板抽象批次**：先创建 `IndustrialWorkbenchTemplate`，让新旧模板并存且测试通过。
2. **治理门禁批次**：让脚本能准确计算适用页面覆盖率，避免误伤组件/抽屉/详情子组件。
3. **强约束修复批次**：先修 `OrderQueryPage` 和 `SystemSettingsPage`，让当前 strict rule 失败项清零。
4. **旧模板清零批次**：迁移入库、SKU、WMS、商品差异等仍直接导入旧模板的页面。
5. **旧设置布局清理批次**：迁移 `UiModule07SettingsLayout` / `UiModule07SettingsSection` 残留页面。
6. **跨模块一致性批次**：补 E2E 和视觉/交互一致性检查，确保用户学习成本下降。
7. **95% 发布门禁批次**：跑完整验证，更新报告，冻结新增页面规则。

### 工作方式

- 每个批次先写失败测试，再做最小实现，再跑目标测试。
- 每个批次独立提交；不要把模板抽象、页面迁移、E2E 修复混在一个提交里。
- 每次迁移一个页面时，只替换骨架与模板入口，不顺手改业务逻辑。
- 页面显隐必须通过模板 props / `useWorkbenchPanels()` / 页面适配配置控制，不复制布局。

---

## 3. 详细任务

### Task 1: 固化 95% 覆盖率指标

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/workbenchTemplateMetrics.ts`
- Create: `apps/erp-web/src/components/page-templates/workbench/workbenchTemplateMetrics.test.ts`
- Modify: `apps/erp-web/src/components/page-templates/workbench/index.ts`

- [ ] **Step 1: 写覆盖率统计失败测试**

在 `workbenchTemplateMetrics.test.ts` 中覆盖：

```ts
import { describe, expect, it } from 'vitest';

import {
  calculateTemplateAdoptionRate,
  classifyTemplateUsage,
  isTemplateAdoptionPassing,
} from './workbenchTemplateMetrics';

describe('workbench template metrics', () => {
  it('calculates adoption rate against applicable pages only', () => {
    const result = calculateTemplateAdoptionRate([
      { file: 'src/pages/orders/OrderReviewPage.tsx', applicable: true, hasTemplate: true },
      { file: 'src/pages/orders/OrderQueryPage.tsx', applicable: true, hasTemplate: false },
      { file: 'src/pages/orders/components/OrderReviewPanels.tsx', applicable: false, hasTemplate: false },
    ]);

    expect(result.applicableTotal).toBe(2);
    expect(result.templateTotal).toBe(1);
    expect(result.rate).toBe(0.5);
  });

  it('passes only when adoption is at least 95 percent', () => {
    expect(isTemplateAdoptionPassing(0.95)).toBe(true);
    expect(isTemplateAdoptionPassing(0.949)).toBe(false);
  });

  it('classifies allowed templates and legacy tokens', () => {
    expect(classifyTemplateUsage('<WorkbenchPageTemplate />')).toMatchObject({
      hasTemplate: true,
      hasLegacyTemplate: false,
    });
    expect(classifyTemplateUsage('<OrderReviewWorkbenchTemplate />')).toMatchObject({
      hasTemplate: false,
      hasLegacyTemplate: true,
    });
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm --dir apps/erp-web exec vitest --run src/components/page-templates/workbench/workbenchTemplateMetrics.test.ts`

Expected: FAIL，提示模块不存在或函数不存在。

- [ ] **Step 3: 实现最小统计工具**

在 `workbenchTemplateMetrics.ts` 中实现纯函数：

```ts
export type TemplatePageUsage = {
  file: string;
  applicable: boolean;
  hasTemplate: boolean;
};

export type TemplateAdoptionRate = {
  applicableTotal: number;
  templateTotal: number;
  rate: number;
};

const acceptedTemplateTokens = [
  'WorkbenchPageTemplate',
  'SettingsPageTemplate',
  'UiModule03MainTableSection',
] as const;

const legacyTemplateTokens = [
  'OrderReviewWorkbenchTemplate',
  'OrderReviewWorkbenchToolbarTemplate',
  'UiModule07SettingsLayout',
  'UiModule07SettingsSection',
] as const;

export const calculateTemplateAdoptionRate = (pages: TemplatePageUsage[]): TemplateAdoptionRate => {
  const applicablePages = pages.filter((page) => page.applicable);
  const templateTotal = applicablePages.filter((page) => page.hasTemplate).length;

  return {
    applicableTotal: applicablePages.length,
    templateTotal,
    rate: applicablePages.length === 0 ? 1 : templateTotal / applicablePages.length,
  };
};

export const isTemplateAdoptionPassing = (rate: number) => rate >= 0.95;

export const classifyTemplateUsage = (source: string) => ({
  hasTemplate: acceptedTemplateTokens.some((token) => source.includes(token)),
  hasLegacyTemplate: legacyTemplateTokens.some((token) => source.includes(token)),
});
```

- [ ] **Step 4: 导出工具**

在 `apps/erp-web/src/components/page-templates/workbench/index.ts` 中导出：

```ts
export * from './workbenchTemplateMetrics';
```

- [ ] **Step 5: 运行测试确认通过**

Run: `pnpm --dir apps/erp-web exec vitest --run src/components/page-templates/workbench/workbenchTemplateMetrics.test.ts`

Expected: PASS。

---

### Task 2: 建立业务无关 `IndustrialWorkbenchTemplate`

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/IndustrialWorkbenchTemplate.tsx`
- Create: `apps/erp-web/src/components/page-templates/workbench/industrialWorkbenchTemplate.css`
- Create: `apps/erp-web/src/components/page-templates/workbench/IndustrialWorkbenchTemplate.test.ts`
- Modify: `apps/erp-web/src/components/page-templates/workbench/WorkbenchPageTemplate.tsx`
- Modify: `apps/erp-web/src/components/page-templates/workbench/index.ts`

- [ ] **Step 1: 写业务语义隔离测试**

在 `IndustrialWorkbenchTemplate.test.ts` 中用源码断言防回退：

```ts
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('IndustrialWorkbenchTemplate', () => {
  it('does not expose order-review defaults in the generic template', () => {
    const source = readFileSync(resolve(__dirname, 'IndustrialWorkbenchTemplate.tsx'), 'utf8');

    expect(source).toContain('industrial-workbench-page');
    expect(source).not.toContain('order-review-page');
    expect(source).not.toContain('OrderReviewWorkbenchTemplate');
  });

  it('keeps WorkbenchPageTemplate backed by the generic industrial template', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchPageTemplate.tsx'), 'utf8');

    expect(source).toContain('IndustrialWorkbenchTemplate');
    expect(source).not.toContain('OrderReviewWorkbenchTemplate');
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm --dir apps/erp-web exec vitest --run src/components/page-templates/workbench/IndustrialWorkbenchTemplate.test.ts`

Expected: FAIL，因为新模板不存在且 `WorkbenchPageTemplate` 仍包装旧订单模板。

- [ ] **Step 3: 创建模板 props**

在 `IndustrialWorkbenchTemplate.tsx` 中定义与现有 `OrderReviewWorkbenchTemplateProps` 兼容的 props：

```ts
import type { TablePaginationConfig } from 'antd';
import type { ReactNode, Ref } from 'react';

export type IndustrialWorkbenchTemplateProps = {
  pageTestId?: string;
  className?: string;
  toolbar?: ReactNode;
  header?: ReactNode;
  actionBar?: ReactNode;
  leftPanel?: ReactNode;
  leftPanelVisible?: boolean;
  leftPanelSyncKey?: string | number;
  tableContainerRef?: Ref<HTMLDivElement>;
  tableClassName?: string;
  tableContent: ReactNode;
  tableFooter?: ReactNode;
  footerSummary?: ReactNode;
  footerSyncKey?: string | number;
  total?: number;
  pagination?: TablePaginationConfig;
  onPaginationChange?: (pagination: TablePaginationConfig) => void;
  showFooterScrollbarRow?: boolean;
  rightPanel?: ReactNode;
  rightPanelVisible?: boolean;
  rightPanelSyncKey?: string | number;
  bottomPanel?: ReactNode;
  bottomPanelVisible?: boolean;
  bottomPanelSyncKey?: string | number;
  leftPanelWidth?: number;
  rightPanelWidth?: number;
  bottomPanelHeight?: number;
  overlays?: ReactNode;
  stateCacheKey?: string;
};
```

- [ ] **Step 4: 迁移旧模板通用逻辑**

从 `apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx` 复制通用逻辑到新模板，但替换默认值：

```ts
pageTestId = 'industrial-workbench-page'
className = 'industrial-workbench-page'
tableClassName = 'industrial-workbench-table industrial-workbench-table-module-03 industrial-workbench-table-external-scrollbar'
```

要求：

- 继续使用 `useWorkbenchLayoutController()` 挂载左/右/底部面板。
- 继续使用 `UiModule03TableFooter` 统一底部分页与滚动条。
- 保留 `stateCacheKey` 的滚动位置读写。
- 新文件中不出现 `order-review-page`、`OrderReviewWorkbenchTemplate`。

- [ ] **Step 5: 替换 `WorkbenchPageTemplate` 底层实现**

把 `WorkbenchPageTemplate.tsx` 改为：

```tsx
import { isValidElement, type ReactElement } from 'react';

import {
  IndustrialWorkbenchTemplate,
  type IndustrialWorkbenchTemplateProps,
} from './IndustrialWorkbenchTemplate';
import { WorkbenchToolbar, type WorkbenchToolbarProps } from './WorkbenchToolbar';

type WorkbenchToolbarElement = ReactElement<WorkbenchToolbarProps, typeof WorkbenchToolbar>;

export type WorkbenchPageTemplateProps = Omit<IndustrialWorkbenchTemplateProps, 'toolbar'> & {
  toolbar?: WorkbenchToolbarElement | null;
};

const isWorkbenchToolbarElement = (toolbar: WorkbenchPageTemplateProps['toolbar']) =>
  toolbar == null || (isValidElement(toolbar) && toolbar.type === WorkbenchToolbar);

export const WorkbenchPageTemplate = ({ toolbar, ...rest }: WorkbenchPageTemplateProps) => {
  if (!isWorkbenchToolbarElement(toolbar)) {
    throw new Error('WorkbenchPageTemplate 的 toolbar 必须使用 WorkbenchToolbar，以保持工作台操作区一致。');
  }

  return <IndustrialWorkbenchTemplate {...rest} toolbar={toolbar} />;
};
```

- [ ] **Step 6: 新增业务无关 CSS**

在 `industrialWorkbenchTemplate.css` 中先复用现有布局结构，提供业务无关类名：

```css
.industrial-workbench-page {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  width: 100%;
  max-width: 100%;
  height: 100%;
}

.industrial-workbench-table {
  min-height: 0;
}

.industrial-workbench-bottom-panel {
  height: 100%;
  min-height: 0;
}
```

如果发现旧 `orderReviewPage.css` 中有必须复用的通用类，不要复制全部样式；只迁移和模板骨架相关的最小规则。

- [ ] **Step 7: 导出新模板**

在 `apps/erp-web/src/components/page-templates/workbench/index.ts` 中导出：

```ts
export * from './IndustrialWorkbenchTemplate';
```

- [ ] **Step 8: 运行模板测试**

Run: `pnpm --dir apps/erp-web exec vitest --run src/components/page-templates/workbench/IndustrialWorkbenchTemplate.test.ts src/components/page-templates/workbench/workbenchContracts.test.ts src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`

Expected: PASS。

---

### Task 3: 升级模板治理脚本到 95% 门禁

**Files:**
- Modify: `apps/erp-web/scripts/check-page-template-usage.mjs`
- Modify: `apps/erp-web/scripts/page-template-baseline.json`
- Create/Modify: `apps/erp-web/scripts/check-page-template-usage.test.mjs`，如果当前脚本测试体系已有约定则放入同目录现有测试文件。

- [ ] **Step 1: 定义适用页面分类规则**

在脚本中新增分类：

- `route-page`：路由直接渲染的页面，必须计入覆盖率。
- `workbench-page`：列表/工作台/查询/审核/基础资料页面，必须使用 `WorkbenchPageTemplate` 或同级模板。
- `settings-page`：配置/设置/详情表单页面，必须使用 `SettingsPageTemplate`。
- `component-only`：`components/`、`shared/`、`review-native/`、`*-form/` 内部组件，不计入覆盖率。
- `explicit-exemption`：确有原因暂不迁移的页面，必须在 baseline 中写明原因、负责人、计划迁移批次。

- [ ] **Step 2: 改造 baseline 格式**

把 `page-template-baseline.json` 改为对象数组，例如：

```json
{
  "exemptions": [
    {
      "file": "src/pages/orders/OrderDetailPage.tsx",
      "reason": "详情页正在等待详情模板定稿，暂不计入工作台覆盖率",
      "expiresAfter": "industrial-workbench-95-phase-5"
    }
  ]
}
```

要求：不能再用“无说明路径列表”作为豁免。

- [ ] **Step 3: 接入覆盖率统计函数**

脚本应输出：

```text
[page-template] applicable pages: 120
[page-template] template pages: 115
[page-template] adoption rate: 95.83%
```

当 `< 95%` 时失败：

```text
[page-template] adoption rate below required 95.00%
```

- [ ] **Step 4: 保留强约束规则**

继续保留当前 strict rules，并新增：

- `src/pages/orders/OrderQueryPage.tsx` 必须使用 `WorkbenchPageTemplate`、`WorkbenchToolbar`、`OrderWorkbenchMainGridEntry`、`variant="query"`。
- `src/pages/system/SystemSettingsPage.tsx` 必须使用 `SettingsPageTemplate`、`SettingsPageSection`。
- `src/pages/**` 不允许直接从 `@/components/workbench/templates` 导入旧模板。

- [ ] **Step 5: 运行脚本确认仍失败但输出更可执行**

Run: `pnpm --dir apps/erp-web run lint:page-templates`

Expected: FAIL，但必须出现覆盖率数字、缺口清单、旧模板清单、严格规则清单。

---

### Task 4: 修复当前 strict rule 失败项

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderQueryPage.tsx`
- Modify: `apps/erp-web/src/pages/system/SystemSettingsPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`
- Add/Modify: `apps/erp-web/src/pages/system/SystemSettingsPage.test.ts` 或现有系统设置相关测试。

- [ ] **Step 1: 写订单查询模板接入测试**

在 `workbenchUnifiedTemplateAdoption.test.ts` 增加：

```ts
it('keeps order query aligned with the unified industrial workbench template', () => {
  const source = readFileSync(resolve(__dirname, '../OrderQueryPage.tsx'), 'utf8');

  expect(source).toContain('WorkbenchPageTemplate');
  expect(source).toContain('WorkbenchToolbar');
  expect(source).toContain('OrderWorkbenchMainGridEntry');
  expect(source).toContain('variant="query"');
  expect(source).not.toContain('OrderReviewWorkbenchTemplate');
  expect(source).not.toContain('OrderReviewWorkbenchToolbarTemplate');
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm --dir apps/erp-web exec vitest --run src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`

Expected: FAIL。

- [ ] **Step 3: 迁移 `OrderQueryPage.tsx`**

按订单复核页的当前写法迁移：

- 导入 `WorkbenchPageTemplate`、`WorkbenchToolbar`。
- 保留当前业务数据源、列定义、分页、搜索、筛选逻辑。
- 主表使用 `OrderWorkbenchMainGridEntry variant="query"`。
- 左/右/底部面板通过 `leftPanel`、`rightPanel`、`bottomPanel` props 挂载。
- 移除旧模板导入。

- [ ] **Step 4: 写系统设置模板接入测试**

新增或修改系统设置测试：

```ts
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('SystemSettingsPage template adoption', () => {
  it('uses the unified settings page template', () => {
    const source = readFileSync(resolve(__dirname, 'SystemSettingsPage.tsx'), 'utf8');

    expect(source).toContain('SettingsPageTemplate');
    expect(source).toContain('SettingsPageSection');
    expect(source).not.toContain('UiModule07SettingsLayout');
    expect(source).not.toContain('UiModule07SettingsSection');
  });
});
```

- [ ] **Step 5: 迁移 `SystemSettingsPage.tsx`**

- 用 `SettingsPageTemplate` 包裹页面。
- 用 `SettingsPageSection` 替换旧设置区块。
- 保留表单字段、校验、保存逻辑和中文文案。
- 不改系统设置业务逻辑。

- [ ] **Step 6: 验证 strict rule 清零**

Run: `pnpm --dir apps/erp-web run lint:page-templates`

Expected: 不再出现 `OrderQueryPage.tsx` 和 `SystemSettingsPage.tsx` 的 strict rule violation；如果覆盖率还未达 95%，允许仍因后续页面迁移失败。

---

### Task 5: 清理直接使用旧订单工作台模板的页面

**Files:**
- Modify: `apps/erp-web/src/pages/inbounds/InboundDetailPage.tsx`
- Modify: `apps/erp-web/src/pages/inbounds/InboundInspectPage.tsx`
- Modify: `apps/erp-web/src/pages/inbounds/InboundPutawayPage.tsx`
- Modify: `apps/erp-web/src/pages/inbounds/InboundQuickReceivePage.tsx`
- Modify: `apps/erp-web/src/pages/inbounds/InboundReceivePage.tsx`
- Modify: `apps/erp-web/src/pages/kits/KitEditorPage.tsx`
- Modify: `apps/erp-web/src/pages/products/ProductDiffPage.tsx`
- Modify: `apps/erp-web/src/pages/skus/SkuImportPage.tsx`
- Modify: `apps/erp-web/src/pages/skus/SkuPage.tsx`
- Modify: `apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- Modify: related tests under the same page folders, if present。

- [ ] **Step 1: 写旧模板清零测试**

新增到合适的 adoption test：

```ts
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const legacyTemplatePages = [
  'src/pages/inbounds/InboundDetailPage.tsx',
  'src/pages/inbounds/InboundInspectPage.tsx',
  'src/pages/inbounds/InboundPutawayPage.tsx',
  'src/pages/inbounds/InboundQuickReceivePage.tsx',
  'src/pages/inbounds/InboundReceivePage.tsx',
  'src/pages/kits/KitEditorPage.tsx',
  'src/pages/products/ProductDiffPage.tsx',
  'src/pages/skus/SkuImportPage.tsx',
  'src/pages/skus/SkuPage.tsx',
  'src/pages/wms/WmsOutboundConfirmPage.tsx',
];

it.each(legacyTemplatePages)('removes direct legacy workbench template usage from %s', (file) => {
  const source = readFileSync(resolve(process.cwd(), file), 'utf8');

  expect(source).not.toContain("@/components/workbench/templates");
  expect(source).not.toContain('OrderReviewWorkbenchTemplate');
  expect(source).not.toContain('OrderReviewWorkbenchToolbarTemplate');
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm --dir apps/erp-web exec vitest --run src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`

Expected: FAIL。

- [ ] **Step 3: 按业务域分批迁移入库页面**

对 `apps/erp-web/src/pages/inbounds/*.tsx`：

- 用 `WorkbenchPageTemplate` 替换旧模板。
- 用 `WorkbenchToolbar` 替换旧 toolbar。
- 若页面是详情/作业确认型页面，仍保持工作台三段式：顶部工具栏、主作业区、右侧详情/底部明细。
- 若确实不是工作台页面，登记为 `SettingsPageTemplate` 或 baseline 豁免，不能继续使用旧订单模板。

- [ ] **Step 4: 迁移 SKU / 商品差异 / 套装页面**

- 列表/导入进度/差异核对页面使用 `WorkbenchPageTemplate`。
- 表单编辑页面如果不适合工作台，使用 `SettingsPageTemplate`。
- 不继续从 `@/components/workbench/templates` 导入。

- [ ] **Step 5: 迁移 WMS 出库确认页面**

- 保留 WMS 业务按钮和扫描/确认流程。
- 工具栏位置统一到 `WorkbenchToolbar` 左侧业务动作、中间搜索/扫描入口、右侧通用操作。
- 主表/明细/日志不要复制自定义布局。

- [ ] **Step 6: 验证旧模板清零**

Run: `git grep -n -I -E "from '@/components/workbench/templates|OrderReviewWorkbenchTemplate|OrderReviewWorkbenchToolbarTemplate" -- apps/erp-web/src/pages`

Expected: 无页面直接命中；如果测试文件中有历史断言，需要同步改为新模板断言。

---

### Task 6: 清理旧设置布局残留

**Files:**
- Modify: `apps/erp-web/src/pages/afterSales/AfterSaleCaseDetailPage.tsx`
- Modify: `apps/erp-web/src/pages/afterSales/AfterSalesDetailPage.tsx`
- Modify: `apps/erp-web/src/pages/customers/CustomerDetailPage.tsx`
- Modify: `apps/erp-web/src/pages/customers/CustomerLoyaltyPage.tsx`
- Modify: `apps/erp-web/src/pages/finance/TaxManagementPage.tsx`
- Modify: `apps/erp-web/src/pages/logistics/LogisticsInstallationsPage.tsx`
- Modify: `apps/erp-web/src/pages/logistics/LogisticsSettlementsPage.tsx`
- Modify: `apps/erp-web/src/pages/logistics/WarehouseOptimizationPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/OrderDetailPage.tsx`
- Modify: `apps/erp-web/src/pages/procurement/PurchaseOrderDetailPage.tsx`
- Modify: `apps/erp-web/src/pages/reports/ReportsPage.tsx`
- Modify: `apps/erp-web/src/pages/system/DataScopesPage.tsx`
- Modify: `apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- Modify: `apps/erp-web/src/pages/system/SystemAuditReportsPage.tsx`
- Modify: `apps/erp-web/src/pages/system/SystemCustomFieldsPage.tsx`
- Modify: `apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- Modify: `apps/erp-web/src/pages/system/SystemSettingsPage.tsx`

- [ ] **Step 1: 写旧设置布局清零测试**

在页面模板采用测试中增加：

```ts
it('keeps pages free from legacy settings layout tokens', () => {
  const files = [
    'src/pages/system/DataScopesPage.tsx',
    'src/pages/system/ObservabilityPage.tsx',
    'src/pages/system/SystemAuditReportsPage.tsx',
    'src/pages/system/SystemCustomFieldsPage.tsx',
    'src/pages/system/SystemIntegrationsPage.tsx',
    'src/pages/system/SystemSettingsPage.tsx',
  ];

  for (const file of files) {
    const source = readFileSync(resolve(process.cwd(), file), 'utf8');
    expect(source).not.toContain('UiModule07SettingsLayout');
    expect(source).not.toContain('UiModule07SettingsSection');
  }
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm --dir apps/erp-web exec vitest --run src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`

Expected: FAIL。

- [ ] **Step 3: 迁移系统设置类页面**

- 使用 `SettingsPageTemplate title="..."`。
- 每个卡片/表单区改为 `SettingsPageSection`。
- 保留 `Form`、`Card`、`Table` 的业务实现，只移除旧布局壳。

- [ ] **Step 4: 迁移详情类页面**

- 如果是订单/客户/采购详情：优先使用 `SettingsPageTemplate` 作为详情页统一骨架。
- 如果详情页需要左筛选/右详情/底部日志：使用 `WorkbenchPageTemplate`。
- 迁移时不要改字段、接口、权限判断。

- [ ] **Step 5: 验证旧设置布局清零**

Run: `git grep -n -I -E "UiModule07SettingsLayout|UiModule07SettingsSection" -- apps/erp-web/src/pages`

Expected: 只允许测试说明或兼容模块内部命中；页面不应继续使用。

---

### Task 7: 统一工具栏和搜索契约

**Files:**
- Modify: `apps/erp-web/src/components/page-templates/workbench/WorkbenchToolbar.tsx`
- Modify: `apps/erp-web/src/components/page-templates/workbench/contracts.ts`
- Create/Modify: `apps/erp-web/src/components/page-templates/workbench/WorkbenchToolbar.contract.test.ts`
- Modify: migrated pages from Tasks 4-6 as needed。

- [ ] **Step 1: 写工具栏契约测试**

要求 `WorkbenchToolbar` 固定三段：

- `primaryActions`：左侧高频动作。
- `search` / `searchValue` / `onSearch`：中间业务搜索或扫描入口。
- `secondaryActions`：右侧通用操作。

测试示例：

```ts
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchToolbar contract', () => {
  it('keeps left actions, center search, and right actions explicit', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchToolbar.tsx'), 'utf8');

    expect(source).toContain('primaryActions');
    expect(source).toContain('secondaryActions');
    expect(source).toMatch(/search|Search/);
  });
});
```

- [ ] **Step 2: 补充契约类型**

在 `contracts.ts` 增加或完善：

```ts
export type WorkbenchToolbarRegion = 'primary-actions' | 'business-search' | 'secondary-actions';
```

- [ ] **Step 3: 统一迁移页面 toolbar 参数命名**

页面迁移时遵守：

- 左侧只放业务高频动作：新增、审核、作废、导入、导出、确认。
- 中间只放业务搜索/扫描/单号定位。
- 右侧只放刷新、列设置、视图、密度、帮助等通用动作。

- [ ] **Step 4: 验证工具栏相关测试**

Run: `pnpm --dir apps/erp-web exec vitest --run src/components/page-templates/workbench/WorkbenchToolbar.contract.test.ts src/components/ui-modules/UiModule02ActionSearchToolbar.test.ts`

Expected: PASS。

---

### Task 8: 补齐跨模块一致性 E2E

**Files:**
- Create: `apps/erp-web/e2e/workbench-template-consistency.spec.ts`
- Modify: `apps/erp-web/e2e/fixtures.ts` only if helper reuse is needed。

- [ ] **Step 1: 新增 E2E 测试文件**

覆盖 4 类页面：

- 订单复核：`/orders/review`
- 订单查询：`/sales/order-query` 或实际订单查询路由
- 基础资料：如渠道、仓库、客户分类
- 系统设置：`/system/settings`

测试结构：

```ts
import { expect, test } from './fixtures';

const workbenchRoutes = [
  { path: '/orders/review', pageTestId: 'order-review-page' },
  { path: '/sales/order-query', pageTestId: 'sales-order-query-page' },
  { path: '/channels', pageTestId: 'channels-workbench-page' },
];

test.describe('industrial workbench template consistency', () => {
  for (const route of workbenchRoutes) {
    test(`${route.path} keeps the unified workbench structure`, async ({ page }) => {
      await page.goto(route.path);

      await expect(page.locator('.workbench-shell')).toBeVisible();
      await expect(page.getByTestId(route.pageTestId)).toBeVisible();
      await expect(page.locator('.industrial-workbench-page, .order-review-page')).toBeVisible();
      await expect(page.locator('.workbench-panel-left, .workbench-panel-right, .workbench-main-bottom').first()).toBeAttached();
    });
  }
});
```

迁移完成后把 `.order-review-page` 兼容选择器去掉，只保留业务无关类。

- [ ] **Step 2: 运行 E2E 确认失败或暴露缺口**

Run: `pnpm --dir apps/erp-web exec playwright test e2e/workbench-template-consistency.spec.ts --project=chromium`

Expected: 初始可能 FAIL，用来驱动页面 `data-testid` 和模板类名补齐。

- [ ] **Step 3: 补齐页面 `data-testid`**

迁移页面必须提供稳定 `pageTestId`，命名规则：

- `order-review-page`
- `order-query-page`
- `inbound-receive-page`
- `sku-page`
- `wms-outbound-confirm-page`
- `system-settings-page`

- [ ] **Step 4: 运行 E2E 通过**

Run: `pnpm --dir apps/erp-web exec playwright test e2e/workbench-template-consistency.spec.ts --project=chromium`

Expected: PASS。

---

### Task 9: 页面覆盖率冲刺到 95%

**Files:**
- Modify: all files reported by `pnpm --dir apps/erp-web run lint:page-templates`。
- Modify: `apps/erp-web/scripts/page-template-baseline.json` only for justified temporary exemptions。

- [ ] **Step 1: 生成缺口清单**

Run: `pnpm --dir apps/erp-web run lint:page-templates`

Expected: 输出当前覆盖率和未接入页面。

- [ ] **Step 2: 将缺口分三类**

分类规则：

- `A 类：必须迁移`：列表、查询、审核、作业台、设置页。
- `B 类：应迁移但可排后`：详情页、导入向导、复杂表单。
- `C 类：可豁免`：非路由组件、抽屉内部、测试辅助页面、临时实验页。

- [ ] **Step 3: 先迁移 A 类页面**

每个页面按固定顺序：

1. 写源码断言测试。
2. 替换模板入口。
3. 保留业务逻辑。
4. 跑页面相关单测。
5. 跑 `lint:page-templates` 看覆盖率变化。

- [ ] **Step 4: 处理 B 类页面**

能快速迁移的详情页使用 `SettingsPageTemplate`；需要设计的新详情模板先豁免，但必须登记原因和到期批次。

- [ ] **Step 5: 严格控制 C 类豁免**

`page-template-baseline.json` 中豁免项不得超过适用页面的 `5%`，否则 95% 门禁失败。

- [ ] **Step 6: 达成 95%**

Run: `pnpm --dir apps/erp-web run lint:page-templates`

Expected: PASS，并输出 `adoption rate >= 95.00%`。

---

### Task 10: 完整验证与发布门禁

**Files:**
- Modify: `scripts/assess_industrial_progress.mjs`
- Modify: `tmp/industrial-progress/industrial-progress-latest.json` 不纳入提交，除非项目当前要求提交生成报告。
- Modify: `README.md` or `docs/` only if current docs mention old template usage。

- [ ] **Step 1: 扩展工业化进度报告**

在 `scripts/assess_industrial_progress.mjs` 中增加：

- `template_adoption_rate`
- `applicable_template_pages`
- `template_compliant_pages`
- `legacy_workbench_template_mentions`
- `legacy_settings_layout_mentions`
- `page_template_gate_status`

- [ ] **Step 2: 运行报告**

Run: `pnpm run assess:industrial-progress`

Expected: 生成报告，并包含模板 95% 指标。

- [ ] **Step 3: 运行页面模板门禁**

Run: `pnpm --dir apps/erp-web run lint:page-templates`

Expected: PASS。

- [ ] **Step 4: 运行核心单测**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run \
  src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts \
  src/app/layout/WorkbenchLayout.interactions.test.ts \
  src/app/layout/WorkbenchLayout.splitter-a11y.test.ts \
  src/components/page-templates/workbench/IndustrialWorkbenchTemplate.test.ts \
  src/components/page-templates/workbench/workbenchContracts.test.ts \
  src/components/page-templates/workbench/workbenchTemplateMetrics.test.ts \
  src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts
```

Expected: PASS。

- [ ] **Step 5: 运行类型检查和构建**

Run: `pnpm --filter erp-web typecheck`

Expected: PASS。

Run: `pnpm --filter erp-web build`

Expected: PASS。

- [ ] **Step 6: 运行关键 E2E**

Run:

```bash
pnpm --dir apps/erp-web exec playwright test \
  e2e/workbench-a11y.spec.ts \
  e2e/workbench-layout-presets.spec.ts \
  e2e/workbench-template-consistency.spec.ts \
  e2e/order-workbench-pages.spec.ts \
  e2e/order-review-interactions.spec.ts \
  --project=chromium
```

Expected: PASS。

- [ ] **Step 7: 最终全量 Web 验收**

Run: `pnpm run accept:web`

Expected: PASS。

- [ ] **Step 8: 完成提交前自检**

Run: `git status --short`

Expected: 只包含本计划范围内的模板、页面迁移、测试和文档改动；不得混入业务接口或无关重构。

---

## 4. 页面迁移检查清单

每迁移一个页面必须逐项确认：

- [ ] 页面只使用 `WorkbenchPageTemplate` / `SettingsPageTemplate` / 明确豁免之一。
- [ ] 工具栏左侧是业务高频动作，中间是搜索/扫描，右侧是通用动作。
- [ ] 主表使用统一主表入口或 `UiModule03MainTableSection` / AG Grid 工作台结构列。
- [ ] 左侧筛选、右侧详情、底部明细通过模板 props 或 `useWorkbenchPanels()` 挂载。
- [ ] 页面有稳定 `pageTestId`。
- [ ] 不新增硬编码 margin；按钮组使用 `Space` 或模板内置布局。
- [ ] 中文文案保持一致，新增文案使用 UTF-8。
- [ ] 页面相关 E2E 优先使用 `data-testid`。
- [ ] 不改业务 API、不改业务状态机、不顺手重构无关代码。

---

## 5. 风险与应对

### 风险 1：旧订单模板样式被多个页面隐式依赖

**应对：** 先让 `IndustrialWorkbenchTemplate` 兼容布局，再逐步迁移类名；E2E 允许短期双选择器，最终批次清掉 `order-review-*` 泛用依赖。

### 风险 2：页面扫描误把组件当页面导致覆盖率失真

**应对：** `check-page-template-usage.mjs` 必须引入适用页面分类，优先从路由注册表识别路由页面，组件目录默认不计入覆盖率。

### 风险 3：一次迁移页面太多导致回归难定位

**应对：** 按业务域小批次提交，每批只迁移 3-5 个页面，跑目标测试后再继续。

### 风险 4：为了达成覆盖率滥用 baseline 豁免

**应对：** baseline 豁免本身计入报告，并设上限；超过适用页面 5% 直接失败。

### 风险 5：统一模板降低特殊业务效率

**应对：** 特殊业务只能通过模板扩展点解决，例如 `toolbar`、`actionBar`、`leftPanel`、`rightPanel`、`bottomPanel`、`overlays`，不能复制布局。

---

## 6. 完成标准

- [ ] `WorkbenchPageTemplate` 不再包装 `OrderReviewWorkbenchTemplate`。
- [ ] 页面层不再直接导入旧 `@/components/workbench/templates`。
- [ ] `lint:page-templates` 通过，并输出模板接入率 `>= 95%`。
- [ ] 当前 strict rule violation 全部清零。
- [ ] 核心工作台单测通过。
- [ ] 关键工作台 E2E 通过。
- [ ] `pnpm --filter erp-web typecheck` 通过。
- [ ] `pnpm --filter erp-web build` 通过。
- [ ] 工业化进度报告能展示 95% 指标。
- [ ] 新页面开发规则明确：未接模板不得合入。

---

## 7. 建议提交顺序

1. `test: add workbench template adoption metrics`
2. `feat: add generic industrial workbench template`
3. `chore: enforce 95 percent page template gate`
4. `refactor: migrate order query and system settings templates`
5. `refactor: remove legacy order workbench template imports from pages`
6. `refactor: migrate legacy settings layout pages`
7. `test: add cross-module workbench consistency e2e`
8. `chore: report industrial workbench adoption progress`

---

## 8. 开始执行前的确认

执行前请先确认当前工作区已有未提交改动是否属于同一目标。如果不是，请先隔离工作区或创建独立分支/工作树，避免模板迁移和其他业务改动互相污染。
