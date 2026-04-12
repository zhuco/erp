# OMS-WMS 主链一期落地 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于现有 `OrderReviewPage`、`WmsWavesPage`、`WmsPickTasksPage`、`WmsPackPage`、`WmsOutboundConfirmPage` 与纯开源 `ErpDataGridNext`，落地 `订单审核 -> 订单复核 -> 提交发货 -> 波次生成 -> 拣货 / 复核 / 打包 -> 出库交接` 一期主链，并收口无用调试页。

**Architecture:** 方案遵循“主链优先的双中台轻重构”。OMS 侧保留 `OrderReviewPage` 作为主工作台，新增 `OrderExceptionsPage` 承接异常复核闭环；WMS 侧保留波次、拣货、打包页面并按主链语义重构，新增 `WmsHandoverPage` 承接“出库交接”，原 `WmsShipPage` 退化为兼容壳层。共享状态、阻断规则、权限别名和特性开关统一收口到 `src/features/oms-wms-main-chain/`，避免页面继续散落写死状态值。

**Tech Stack:** React、TypeScript、React Query、Ant Design、TanStack-based `ErpDataGridNext`、Vitest、Playwright、前端 mock API

---

## 前置说明

- 当前工作目录 `F:\erp` 不是 Git 仓库；本计划中的 `git commit` 步骤默认在真实 Git 工作树或后续 worktree 中执行。
- 本计划只覆盖一期主链与相关支撑约束，不包含售后、财务、采购、物流资费回写等二期联动。
- 本计划不替换现有 grid 技术路线，只复用并补强当前 `ErpDataGridNext` 的共享交互约束。
- `库存预检` 与 `库存预占` 在实施中必须明确区分：
  - `库存预检`：提交发货前的可售/规则校验，只返回阻断原因，不写 `reserve_status`
  - `库存预占`：提交发货动作内执行；仅成功后写 `reserve_status = RESERVED`
- 一期实际落地的 feature flag 只收口以下 5 项，其余 flag 不纳入本轮关键路径：
  - `wms.enable_wave`
  - `wms.enable_outbound_inspection`
  - `wms.enable_pack_performance`
  - `wms.enable_shipment_cancel`
  - `flows.pick_recommend`
- 所有新增主表页面必须直接复用现有 `ErpDataGridNext` 的紧凑规格：
  - 数据行高保持 `26px`
  - 表头保持紧凑高度
  - 配色跟随现有菜单主题，不允许单页另起独立色板

## 文件结构

### 共享主链契约

- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/statuses.ts`
- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/guards.ts`
- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/statuses.test.ts`
- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/guards.test.ts`
- Modify: `F:/erp/apps/erp-web/src/features/featureFlags.ts`
- Modify: `F:/erp/apps/erp-web/src/features/rbac.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/auth.ts`

职责说明：

- `statuses.ts`：定义一期 `audit_status / exception_status / fulfillment_status / reserve_status / wave_status / pick_status / pack_status / handover_status` 的前端常量、标签、派生字段与显示口径。
- `guards.ts`：实现提交发货、建波次、开始拣货、复核称重、出库交接、物流回传重试等动作的统一阻断判断与阻断原因生成。
- `featureFlags.ts`：只保留并对齐本期实际落地的 flag；把当前 `WmsPackPage` 上的 `flows.pack_performance` 使用口径统一到 `wms.enable_pack_performance`。
- `rbac.tsx` / `auth.ts`：补充主链动作粒度权限别名，并让 mock 登录态默认具备本期所需权限。

### OMS 页面与 API/mock

- Create: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/orders/orderExceptionsPage.css`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/orderReviewPage.css`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/ShipOrderDrawer.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewToolbar.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewPanels.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewTableSection.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/orders.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/orders.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/data.ts`
- Create: `F:/erp/apps/erp-web/src/api/orders.mainChain.test.ts`

职责说明：

- `orders.ts` / `mocks/orders.ts`：补齐异常池列表/详情/领取/分派/升级/通过/挂起/驳回/回退、提交发货预检与提交发货接口。
- `OrderExceptionsPage.tsx`：承接 OMS 异常池闭环，支持 SLA 卡片、责任人流转、复核动作与批量阻断说明。
- `OrderReviewPage.tsx`：只保留“订单审核 + 提交发货入口”职责，不再承担异常池分派/升级。
- `ShipOrderDrawer.tsx`：改造成“提交发货确认层”，展示预检结果、阻断原因、预占结果和将生成的出库单数量。

### WMS 页面与 API/mock

- Create: `F:/erp/apps/erp-web/src/pages/wms/WmsHandoverPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/wms/wmsHandoverPage.css`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsPickTasksPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsPackPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/wms.ts`
- Modify: `F:/erp/apps/erp-web/src/api/shipments.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/wms.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/shipments.ts`
- Create: `F:/erp/apps/erp-web/src/api/wms.mainChain.test.ts`

职责说明：

- `wms.ts` / `mocks/wms.ts`：补齐待波次池、波次取消边界、拣货/短拣/复核差异/称重异常、交接批次、交接回滚与出库交接状态推进。
- `shipments.ts` / `mocks/shipments.ts`：承接物流推送结果、失败重试与交接后 `SHIPPED` 推进，不引入 OMS 侧大改。
- `WmsWavesPage.tsx`：重构为“待波次池 + 波次中心”双态页面。
- `WmsPickTasksPage.tsx` / `WmsPackPage.tsx`：围绕仓内执行状态和仓内异常记录收口，不回退 OMS 审核态。
- `WmsHandoverPage.tsx`：新增正式“出库交接”页。
- `WmsShipPage.tsx`：只保留兼容跳转/包装，不再承载真实“发货页”语义。
- `WmsOutboundConfirmPage.tsx`：下沉为扫码快操作台，只做单号/包裹/波次的快速确认与异常回显。

### 路由、菜单与回归测试

- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Create: `F:/erp/apps/erp-web/src/app/layout/menu.test.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionContract.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionContract.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useSelectionController.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/TanStackGridAdapter.tsx`
- Delete: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridDebugPage.tsx`
- Delete: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridReviewSandboxPage.tsx`
- Delete: `F:/erp/apps/erp-web/src/pages/demo/WorkbenchListDemoPage.tsx`
- Delete: `F:/erp/apps/erp-web/e2e/erp-data-grid-debug.spec.ts`
- Create: `F:/erp/apps/erp-web/e2e/order-exceptions.spec.ts`
- Create: `F:/erp/apps/erp-web/e2e/order-review-main-chain.spec.ts`
- Create: `F:/erp/apps/erp-web/e2e/wms-wave-main-chain.spec.ts`
- Create: `F:/erp/apps/erp-web/e2e/wms-handover.spec.ts`
- Create: `F:/erp/apps/erp-web/e2e/oms-wms-main-chain.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-actions.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/outbound-flow.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts`

职责说明：

- `router.tsx` / `menu.tsx`：删除旧调试页入口，新增异常池与出库交接正式入口。
- `selection*` 与 `useSelectionController.ts`：把“活动行 / 钩选 / Shift 逐行反选 / Space 钩选 / 左上角当前可见行全选与反选”固化为跨页面共享约束。
- Playwright 用例：覆盖 OMS 主链、WMS 主链、全链路闭环与唯一保留的 `ErpDataGridNext` 回归页。

---

### Task 1: 固化主链共享状态、阻断规则、权限与特性开关

**Files:**
- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/statuses.ts`
- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/guards.ts`
- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/statuses.test.ts`
- Create: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/guards.test.ts`
- Modify: `F:/erp/apps/erp-web/src/features/featureFlags.ts`
- Modify: `F:/erp/apps/erp-web/src/features/rbac.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/auth.ts`
- Test: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/statuses.test.ts`
- Test: `F:/erp/apps/erp-web/src/features/oms-wms-main-chain/guards.test.ts`

- [ ] **Step 1: 先写失败的状态与阻断规则单测**

```ts
import { describe, expect, it } from 'vitest';

import {
  canCreateWave,
  canSubmitForFulfillment,
} from './guards';
import { deriveReviewStatusLabel } from './statuses';

describe('主链状态与阻断规则', () => {
  it('AUTO_PASSED 订单显示为“无需复核”且允许提交发货', () => {
    expect(deriveReviewStatusLabel({ audit_status: 'AUTO_PASSED' })).toBe('无需复核');
    expect(
      canSubmitForFulfillment({
        audit_status: 'AUTO_PASSED',
        exception_status: 'NONE',
        reserve_precheck_passed: true,
      }).allowed,
    ).toBe(true);
  });

  it('未预占成功的单据禁止建波次', () => {
    expect(
      canCreateWave({
        fulfillment_status: 'WAVE_PENDING',
        reserve_status: 'RESERVE_FAILED',
      }).allowed,
    ).toBe(false);
  });
});
```

- [ ] **Step 2: 运行单测确认当前缺少共享主链契约**

Run: `pnpm --dir apps/erp-web test:unit -- src/features/oms-wms-main-chain/statuses.test.ts src/features/oms-wms-main-chain/guards.test.ts`

Expected: FAIL，提示 `statuses.ts` / `guards.ts` 不存在或断言失败

- [ ] **Step 3: 写最小主链契约实现，并对齐权限与 feature flag**

`statuses.ts` 最小骨架：

```ts
export const auditPassedStatuses = ['AUTO_PASSED', 'REVIEW_PASSED'] as const;

export const deriveReviewStatusLabel = (input: { audit_status: string }) => {
  if (input.audit_status === 'AUTO_PASSED') return '无需复核';
  if (input.audit_status === 'REVIEW_PASSED') return '复核通过';
  return '待复核';
};
```

`guards.ts` 最小骨架：

```ts
export const canSubmitForFulfillment = (input: {
  audit_status: string;
  exception_status: string;
  reserve_precheck_passed: boolean;
}) => {
  const allowed =
    ['AUTO_PASSED', 'REVIEW_PASSED'].includes(input.audit_status) &&
    ['NONE', 'RESOLVED', 'CLOSED'].includes(input.exception_status) &&
    input.reserve_precheck_passed;
  return { allowed, reasons: allowed ? [] : ['前置条件未满足'] };
};
```

要求：
- `featureFlags.ts` 中把 `WmsPackPage` 的性能开关使用口径统一到 `wms.enable_pack_performance`
- `rbac.tsx` 里新增一期动作粒度权限别名，但保留现有粗粒度权限用于兼容
- `auth.ts` 的 mock 登录态默认带上本期新增权限，保证 Playwright 不被权限阻断

- [ ] **Step 4: 重新运行主链契约单测**

Run: `pnpm --dir apps/erp-web test:unit -- src/features/oms-wms-main-chain/statuses.test.ts src/features/oms-wms-main-chain/guards.test.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/features/oms-wms-main-chain apps/erp-web/src/features/featureFlags.ts apps/erp-web/src/features/rbac.tsx apps/erp-web/src/api/auth.ts
git commit -m "feat: add oms wms main chain shared contracts"
```

### Task 2: 清理无用调试页并收口菜单、路由与 grid 共享交互约束

**Files:**
- Delete: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridDebugPage.tsx`
- Delete: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridReviewSandboxPage.tsx`
- Delete: `F:/erp/apps/erp-web/src/pages/demo/WorkbenchListDemoPage.tsx`
- Delete: `F:/erp/apps/erp-web/e2e/erp-data-grid-debug.spec.ts`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Create: `F:/erp/apps/erp-web/src/app/layout/menu.test.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionContract.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionContract.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useSelectionController.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/TanStackGridAdapter.tsx`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts`
- Test: `F:/erp/apps/erp-web/src/app/layout/menu.test.tsx`
- Test: `F:/erp/apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts`

- [ ] **Step 1: 先写失败的菜单与共享交互约束测试**

```ts
import { describe, expect, it } from 'vitest';
import { flattenMenuItems, appMenuItems } from './menu';

describe('主链菜单收口', () => {
  it('只保留 ErpDataGridNext 调试页', () => {
    const paths = flattenMenuItems(appMenuItems).map((item) => item.path);
    expect(paths).toContain('/system/erp-data-grid-next');
    expect(paths).not.toContain('/system/erp-data-grid');
    expect(paths).not.toContain('/system/erp-data-grid-review');
  });
});
```

```ts
test('ErpDataGridNext 保持活动行、Shift 逐行反选和表头当前可见行反选', async ({ page }) => {
  await page.goto('/system/erp-data-grid-next');
  await page.getByTestId('erp-grid-next-row-r2').click();
  await page.keyboard.down('Shift');
  await page.getByTestId('erp-grid-next-row-checkbox-r5').click();
  await page.keyboard.up('Shift');
  await page.getByTestId('erp-grid-next-header-checkbox-menu').click();
  await page.getByRole('menuitem', { name: '反选当前可见行' }).click();
});
```

- [ ] **Step 2: 运行菜单单测和 grid 回归用例，确认先失败**

Run: `pnpm --dir apps/erp-web test:unit -- src/app/layout/menu.test.tsx src/components/erp-data-grid-next/core/selectionModel.test.ts src/components/erp-data-grid-next/core/selectionContract.test.ts`

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/erp-data-grid-next-debug.spec.ts`

Expected: 至少一项 FAIL，表明旧入口仍存在或当前交互语义未完全固化

- [ ] **Step 3: 删除旧调试页并把共享交互约束沉淀到 grid 核心**

要求：
- `router.tsx` / `menu.tsx` 删除 `ErpDataGridDebugPage`、`ErpDataGridReviewSandboxPage`、`WorkbenchListDemoPage` 入口
- `ErpDataGridNext` 只保留一个左上角复选框入口，不再出现额外“反”字列
- `selectionModel.ts` / `selectionContract.ts` / `useSelectionController.ts` 必须明确以下约束：
  - 左键单击只切换活动行，不直接改钩选
  - `Ctrl / Meta` 切换单行钩选
  - `Shift` 以当前活动行 / 上一次选中行为锚点，对区间逐行反选
  - `Space` 切换当前活动行钩选
  - 左上角只对当前可见行执行“全选 / 反选”

- [ ] **Step 4: 重新运行菜单单测与 grid 回归**

Run: `pnpm --dir apps/erp-web test:unit -- src/app/layout/menu.test.tsx src/components/erp-data-grid-next/core/selectionModel.test.ts src/components/erp-data-grid-next/core/selectionContract.test.ts`

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/erp-data-grid-next-debug.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/app/router.tsx apps/erp-web/src/app/layout/menu.tsx apps/erp-web/src/app/layout/menu.test.tsx apps/erp-web/src/components/erp-data-grid-next apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts apps/erp-web/e2e/erp-data-grid-debug.spec.ts
git add -u apps/erp-web/src/pages/system apps/erp-web/src/pages/demo
git commit -m "chore: remove obsolete debug pages and lock grid interaction contract"
```

### Task 3: 扩展 OMS API 与 mock，落地主链状态、异常池动作与提交发货

**Files:**
- Modify: `F:/erp/apps/erp-web/src/api/orders.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/orders.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/data.ts`
- Create: `F:/erp/apps/erp-web/src/api/orders.mainChain.test.ts`
- Test: `F:/erp/apps/erp-web/src/api/orders.mainChain.test.ts`

- [ ] **Step 1: 先写失败的 OMS 主链 API/mock 单测**

```ts
import { describe, expect, it } from 'vitest';
import {
  claimMockOrderException,
  listMockOrderExceptions,
  submitMockOrderFulfillment,
} from '@/api/mocks';

describe('OMS 主链 mock', () => {
  it('异常池只返回 fulfillment_status = NOT_SUBMITTED 的异常单', () => {
    const result = listMockOrderExceptions({ status: 'OPEN' });
    expect(result.items.every((item) => item.fulfillment_status === 'NOT_SUBMITTED')).toBe(true);
  });

  it('提交发货成功后写入 RESERVED 和 WAVE_PENDING', () => {
    const result = submitMockOrderFulfillment('TEST-ORDER-001');
    expect(result.reserve_status).toBe('RESERVED');
    expect(result.fulfillment_status).toBe('WAVE_PENDING');
  });
});
```

- [ ] **Step 2: 运行 OMS 主链 API/mock 单测，确认当前能力不足**

Run: `pnpm --dir apps/erp-web test:unit -- src/api/orders.mainChain.test.ts`

Expected: FAIL，提示异常池或提交发货相关 mock / 类型不存在

- [ ] **Step 3: 在 `orders.ts` 与 `mocks/orders.ts` 中补齐一期接口与状态推进**

要求：
- `orders.ts` 补齐以下类型与方法：
  - `OrderExceptionSummary`
  - `OrderExceptionActionPayload`
  - `FulfillmentPrecheckResult`
  - `submitOrdersForFulfillment`
  - `listOrderExceptions`
  - `getOrderExceptionDetail`
  - `claimOrderException`
  - `assignOrderException`
  - `escalateOrderException`
  - `resolveOrderException`
  - `holdOrderException`
  - `rejectOrderException`
  - `requeueOrderAudit`
- `mocks/orders.ts` 明确区分：
  - `库存预检通过`：只返回前置校验结果
  - `提交发货成功`：写 `reserve_status = RESERVED`、`fulfillment_status = WAVE_PENDING`
  - `提交发货失败`：写 `reserve_status = RESERVE_FAILED`，并保持 `NOT_SUBMITTED`
- `mocks/data.ts` 增加足够的异常池样本、挂起样本、待提交发货样本、预占失败样本，供页面和 E2E 稳定复现

- [ ] **Step 4: 重新运行 OMS 主链 API/mock 单测**

Run: `pnpm --dir apps/erp-web test:unit -- src/api/orders.mainChain.test.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/api/orders.ts apps/erp-web/src/api/mocks/orders.ts apps/erp-web/src/api/mocks/data.ts apps/erp-web/src/api/orders.mainChain.test.ts
git commit -m "feat: add oms main chain api and mock transitions"
```

### Task 4: 新增订单异常池页，承接 OMS 复核闭环

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/orders/orderExceptionsPage.css`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Create: `F:/erp/apps/erp-web/e2e/order-exceptions.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/order-exceptions.spec.ts`

- [ ] **Step 1: 先写失败的异常池 E2E**

```ts
import { expect, test } from '@playwright/test';

test('订单异常池支持领取、升级、复核通过和回退待审核', async ({ page }) => {
  await page.goto('/orders/exceptions');
  await expect(page.getByTestId('order-exceptions-title')).toBeVisible();
  await page.getByTestId('order-exceptions-row-checkbox').first().click();
  await page.getByTestId('order-exceptions-claim-action').click();
  await page.getByTestId('order-exceptions-resolve-action').click();
  await expect(page.getByTestId('order-exceptions-sla-board')).toBeVisible();
});
```

- [ ] **Step 2: 运行异常池 E2E，确认页面尚未存在**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/order-exceptions.spec.ts`

Expected: FAIL，提示 `/orders/exceptions` 路由或选择器不存在

- [ ] **Step 3: 实现异常池页并接入共享 grid 语义**

要求：
- 页面只处理 `fulfillment_status = NOT_SUBMITTED` 的异常订单
- 顶部必须有 SLA 看板：待处理数、处理中数、超时数、我负责数
- 主表使用 `ErpDataGridNext`，沿用 Task 2 固化的活动行 / 钩选语义
- 行动作和批量动作至少包含：
  - 领取
  - 分派
  - 转派
  - 升级
  - 复核通过
  - 退回待补充
  - 驳回关闭
  - 回退到待审核
- 页面内必须直接展示阻断原因和最近处理记录，避免用户回跳主审核页找上下文

- [ ] **Step 4: 重新运行异常池 E2E**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/order-exceptions.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx apps/erp-web/src/pages/orders/orderExceptionsPage.css apps/erp-web/src/app/router.tsx apps/erp-web/src/app/layout/menu.tsx apps/erp-web/e2e/order-exceptions.spec.ts
git commit -m "feat: add order exceptions page"
```

### Task 5: 重构订单审核页与提交发货确认层

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/orderReviewPage.css`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/ShipOrderDrawer.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewToolbar.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewPanels.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewTableSection.tsx`
- Create: `F:/erp/apps/erp-web/e2e/order-review-main-chain.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/order-review-main-chain.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`

- [ ] **Step 1: 先写失败的订单审核主链 E2E**

```ts
import { expect, test } from '@playwright/test';

test('订单审核页只保留审核与提交发货职责，并展示提交发货阻断结果', async ({ page }) => {
  await page.goto('/orders/review');
  await expect(page.getByTestId('order-review-submit-shipping-action')).toBeVisible();
  await page.getByTestId('order-review-row-checkbox').first().click();
  await page.getByTestId('order-review-submit-shipping-action').click();
  await expect(page.getByTestId('ship-order-precheck-summary')).toBeVisible();
});
```

- [ ] **Step 2: 运行订单审核主链与选择交互 E2E**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/order-review-main-chain.spec.ts e2e/order-review-selection.spec.ts`

Expected: FAIL，至少一项提示提交发货确认层或阻断摘要未按主链语义实现

- [ ] **Step 3: 把 `OrderReviewPage` 收口为“审核 + 提交发货入口”**

要求：
- 页面看板最少展示：
  - 待审核数
  - 异常待复核数
  - 待提交发货数
  - 挂起数
  - 超时未处理数
- `ShipOrderDrawer.tsx` 改造成“提交发货确认层”，必须展示：
  - 订单数
  - 商品件数
  - 目标仓库
  - 预检通过数 / 阻断数
  - 预占结果
  - 将生成的出库单数
- `OrderReviewPage.tsx` 不再承担异常池分派/升级动作，只保留跳转入口
- `order-review-selection.spec.ts` 保持以下行为不回退：
  - 左键单击只切活动行
  - `Ctrl / Shift` 自动钩选
  - `Shift` 区间逐行反选
  - `Space` 切换当前活动行钩选

- [ ] **Step 4: 重新运行订单审核主链与选择交互 E2E**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/order-review-main-chain.spec.ts e2e/order-review-selection.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/pages/orders/OrderReviewPage.tsx apps/erp-web/src/pages/orders/orderReviewPage.css apps/erp-web/src/pages/orders/ShipOrderDrawer.tsx apps/erp-web/src/pages/orders/components/OrderReviewToolbar.tsx apps/erp-web/src/pages/orders/components/OrderReviewPanels.tsx apps/erp-web/src/pages/orders/components/OrderReviewTableSection.tsx apps/erp-web/e2e/order-review-main-chain.spec.ts apps/erp-web/e2e/order-review-selection.spec.ts
git commit -m "feat: refactor order review page for fulfillment submission"
```

### Task 6: 扩展 WMS API 与 mock，补齐待波次、仓内执行、交接与回传状态推进

**Files:**
- Modify: `F:/erp/apps/erp-web/src/api/wms.ts`
- Modify: `F:/erp/apps/erp-web/src/api/shipments.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/wms.ts`
- Modify: `F:/erp/apps/erp-web/src/api/mocks/shipments.ts`
- Create: `F:/erp/apps/erp-web/src/api/wms.mainChain.test.ts`
- Test: `F:/erp/apps/erp-web/src/api/wms.mainChain.test.ts`

- [ ] **Step 1: 先写失败的 WMS 主链 API/mock 单测**

```ts
import { describe, expect, it } from 'vitest';
import {
  cancelMockWmsWave,
  createMockWmsHandoverBatch,
  retryMockShipmentPush,
} from '@/api/mocks';

describe('WMS 主链 mock', () => {
  it('一期只允许取消 CREATED 且未开始拣货的波次', () => {
    expect(() => cancelMockWmsWave('WAVE-RELEASED')).toThrow();
  });

  it('交接成功后进入 HANDOVERED，物流回传成功后进入 SHIPPED', () => {
    const handover = createMockWmsHandoverBatch({ outbound_doc_ids: ['OB-1001'] });
    expect(handover.fulfillment_status).toBe('HANDOVERED');
    const pushed = retryMockShipmentPush(handover.shipment_id);
    expect(pushed.push_status).toBe('SUCCESS');
  });
});
```

- [ ] **Step 2: 运行 WMS 主链单测，确认当前 mock 转移不完整**

Run: `pnpm --dir apps/erp-web test:unit -- src/api/wms.mainChain.test.ts`

Expected: FAIL，提示波次取消边界、交接批次或回传重试能力不存在

- [ ] **Step 3: 在 `wms.ts` / `shipments.ts` 与对应 mock 中补齐一期能力**

要求：
- `wms.ts` / `mocks/wms.ts` 补齐：
  - 待波次池列表
  - 波次创建 / 下发 / 取消
  - 仓内异常记录
  - 交接批次创建 / 加入 / 移出 / 确认 / 回滚
- `shipments.ts` / `mocks/shipments.ts` 补齐：
  - 交接后物流推送结果
  - 推送失败重试
  - `HANDOVERED -> SHIPPED` 的最终推进
- 所有状态推进必须经过 Task 1 的共享 guard 与状态枚举，不允许页面里再写分散字符串

- [ ] **Step 4: 重新运行 WMS 主链单测**

Run: `pnpm --dir apps/erp-web test:unit -- src/api/wms.mainChain.test.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/api/wms.ts apps/erp-web/src/api/shipments.ts apps/erp-web/src/api/mocks/wms.ts apps/erp-web/src/api/mocks/shipments.ts apps/erp-web/src/api/wms.mainChain.test.ts
git commit -m "feat: add wms main chain api and mock transitions"
```

### Task 7: 重构波次页为“待波次池 + 波次中心”，并打通拣货、复核、打包、称重

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsPickTasksPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsPackPage.tsx`
- Create: `F:/erp/apps/erp-web/e2e/wms-wave-main-chain.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-actions.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/wms-wave-main-chain.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-actions.spec.ts`

- [ ] **Step 1: 先写失败的波次与仓内执行 E2E**

```ts
import { expect, test } from '@playwright/test';

test('待波次池可建波次，仓内执行记录短拣与复核差异', async ({ page }) => {
  await page.goto('/wms/waves');
  await expect(page.getByTestId('wms-wave-pending-title')).toBeVisible();
  await page.getByTestId('wms-wave-pending-row-checkbox').first().click();
  await page.getByTestId('wms-wave-create-action').click();
  await expect(page.getByTestId('wms-wave-detail-drawer')).toBeVisible();
});
```

- [ ] **Step 2: 运行波次与仓内执行 E2E，确认当前页面语义仍偏旧**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-actions.spec.ts`

Expected: FAIL，至少一项提示待波次池、短拣异常、称重阻断或页面选择器不存在

- [ ] **Step 3: 在波次、拣货、打包页实现一期仓内执行主链**

要求：
- `WmsWavesPage.tsx`
  - 分成“待波次池”和“已生成波次”两块
  - 只允许 `reserve_status = RESERVED` 的订单进入待波次池
  - 只允许 `wave_status = CREATED` 且未开始拣货的波次取消
- `WmsPickTasksPage.tsx`
  - 增加短拣原因、缺货原因、容器绑定、改派原因
  - 短拣和异常必须写仓内异常记录，而不是只弹提示
- `WmsPackPage.tsx`
  - 强制顺序：打包 -> 复核 -> 称重 -> 封箱
  - 复核不通过或未称重时，禁止进入交接
  - 使用 `wms.enable_outbound_inspection` 和 `wms.enable_pack_performance` 作为唯一 flag 判断

- [ ] **Step 4: 重新运行波次与仓内执行 E2E**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-actions.spec.ts e2e/outbound-fulfillment-flow.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/pages/wms/WmsWavesPage.tsx apps/erp-web/src/pages/wms/WmsPickTasksPage.tsx apps/erp-web/src/pages/wms/WmsPackPage.tsx apps/erp-web/e2e/wms-wave-main-chain.spec.ts apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts apps/erp-web/e2e/outbound-fulfillment-actions.spec.ts
git commit -m "feat: refactor wave pick and pack pages for main chain"
```

### Task 8: 新建出库交接页，并把扫码确认页收口为终端快操作台

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/wms/WmsHandoverPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/wms/wmsHandoverPage.css`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Create: `F:/erp/apps/erp-web/e2e/wms-handover.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/outbound-flow.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/wms-handover.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/outbound-flow.spec.ts`

- [ ] **Step 1: 先写失败的交接与扫码终端 E2E**

```ts
import { expect, test } from '@playwright/test';

test('出库交接页支持建批次、确认交接和物流回传重试', async ({ page }) => {
  await page.goto('/wms/handover');
  await expect(page.getByTestId('wms-handover-title')).toBeVisible();
  await page.getByTestId('wms-handover-create-batch').click();
  await expect(page.getByTestId('wms-handover-batch-drawer')).toBeVisible();
});
```

- [ ] **Step 2: 运行交接与扫码终端 E2E**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/wms-handover.spec.ts e2e/outbound-flow.spec.ts`

Expected: FAIL，提示 `/wms/handover` 或交接批次选择器不存在

- [ ] **Step 3: 实现正式交接页并退化旧发货页**

要求：
- `WmsHandoverPage.tsx`
  - 支持交接批次创建、包裹/出库单加入移出、交接确认、交接回滚、交接清单打印、物流回传重试
  - 只有完成封箱和称重的包裹允许进入交接批次
  - 交接成功写 `handover_status = HANDOVERED`；物流回传成功后才推进到 `SHIPPED`
- `WmsShipPage.tsx`
  - 改成兼容包装层，内部直接复用 `WmsHandoverPage` 或重定向到 `/wms/handover`
- `WmsOutboundConfirmPage.tsx`
  - 只保留扫码单号 / 波次 / 包裹号的快操作能力
  - 不再承载规则维护、批量策略或审计主视图

- [ ] **Step 4: 重新运行交接与扫码终端 E2E**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/wms-handover.spec.ts e2e/outbound-flow.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/pages/wms/WmsHandoverPage.tsx apps/erp-web/src/pages/wms/wmsHandoverPage.css apps/erp-web/src/pages/wms/WmsShipPage.tsx apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx apps/erp-web/src/app/router.tsx apps/erp-web/src/app/layout/menu.tsx apps/erp-web/e2e/wms-handover.spec.ts apps/erp-web/e2e/outbound-flow.spec.ts
git commit -m "feat: add wms handover page and simplify outbound confirm terminal"
```

### Task 9: 跑通 OMS -> WMS 主链 E2E，并完成验收级回归

**Files:**
- Create: `F:/erp/apps/erp-web/e2e/oms-wms-main-chain.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/critical-paths.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/oms-wms-main-chain.spec.ts`

- [ ] **Step 1: 先写失败的全链路 E2E**

```ts
import { expect, test } from '@playwright/test';

test('OMS 到 WMS 主链闭环可在 mock 模式下走通', async ({ page }) => {
  await page.goto('/orders/review');
  await page.getByTestId('order-review-row-checkbox').first().click();
  await page.getByTestId('order-review-submit-shipping-action').click();
  await page.getByTestId('ship-order-submit').click();
  await page.goto('/wms/waves');
  await page.getByTestId('wms-wave-create-action').click();
  await page.goto('/wms/pick-tasks');
  await page.goto('/wms/pack');
  await page.goto('/wms/handover');
  await expect(page.getByTestId('wms-handover-title')).toBeVisible();
});
```

- [ ] **Step 2: 运行全链路 E2E，确认此前局部改动已经可串联**

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/oms-wms-main-chain.spec.ts`

Expected: FAIL 或半通过；至少暴露 1 处跨页状态推进缺口

- [ ] **Step 3: 补最后的跨页串联与查询失效处理**

要求：
- 订单提交发货后，待波次池必须可见
- 波次生成后，拣货页必须可见对应任务
- 打包、复核、称重完成后，交接页必须可见对应包裹/出库单
- 交接成功后，扫码终端与发货/回传状态必须能读到最终结果
- 所有跨页刷新必须通过 React Query `invalidateQueries` 或明确 `setQueryData` 串联，不允许依赖手工刷新页面

- [ ] **Step 4: 跑完整回归命令**

Run: `pnpm --dir apps/erp-web test:unit -- src/features/oms-wms-main-chain/statuses.test.ts src/features/oms-wms-main-chain/guards.test.ts src/api/orders.mainChain.test.ts src/api/wms.mainChain.test.ts src/components/erp-data-grid-next/core/selectionModel.test.ts src/components/erp-data-grid-next/core/selectionContract.test.ts`

Run: `pnpm --dir apps/erp-web test:e2e -- e2e/order-exceptions.spec.ts e2e/order-review-main-chain.spec.ts e2e/wms-wave-main-chain.spec.ts e2e/wms-handover.spec.ts e2e/oms-wms-main-chain.spec.ts`

Run: `pnpm --dir apps/erp-web typecheck`

Run: `pnpm --dir apps/erp-web build`

Expected: 全部 PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/e2e/oms-wms-main-chain.spec.ts apps/erp-web/e2e/critical-paths.spec.ts
git commit -m "test: cover oms wms main chain end to end"
```
