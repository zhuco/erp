import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

const repoRoot = path.resolve(__dirname, '../../../../../..');

const readRepoFile = (relativePath: string) =>
  readFileSync(path.resolve(repoRoot, relativePath), 'utf8');

describe('order workbench shared entry regression', () => {
  it('records the current grid technology decision in a dedicated document', () => {
    const decisionDocPath = path.resolve(
      repoRoot,
      'docs/ErpDataGrid/order-workbench-technology-decision-2026-04-15.md',
    );

    expect(existsSync(decisionDocPath)).toBe(true);

    const content = readFileSync(decisionDocPath, 'utf8');
    expect(content).toContain('AG Grid Community 兼容优先');
    expect(content).toContain('Enterprise 只作为可选增强');
    expect(content).toContain('OrderWorkbenchMainGridEntry');
  });

  it('routes the three order workbench pages through the shared main-grid entry', () => {
    const reviewPage = readRepoFile('apps/erp-web/src/pages/orders/OrderReviewPage.tsx');
    const exceptionsPage = readRepoFile('apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx');
    const ordersPage = readRepoFile('apps/erp-web/src/pages/orders/OrderQueryPage.tsx');

    expect(reviewPage).toContain("import { OrderWorkbenchMainGridEntry } from './shared/OrderWorkbenchMainGridEntry';");
    expect(reviewPage).toContain('<OrderWorkbenchMainGridEntry');
    expect(reviewPage).not.toContain("import { OrderReviewMainGrid } from './review-native/OrderReviewMainGrid';");

    expect(exceptionsPage).toContain("import { OrderWorkbenchMainGridEntry } from './shared/OrderWorkbenchMainGridEntry';");
    expect(exceptionsPage).toContain('<OrderWorkbenchMainGridEntry');
    expect(exceptionsPage).not.toContain(
      "import { OrderExceptionsMainGrid } from './exceptions-native/OrderExceptionsMainGrid';",
    );

    expect(ordersPage).toContain("import { OrderWorkbenchMainGridEntry } from './shared/OrderWorkbenchMainGridEntry';");
    expect(ordersPage).toContain('<OrderWorkbenchMainGridEntry');
    expect(ordersPage).not.toContain("import { ErpDataGridNext, type ErpDataGridNextColumn } from '@/components/ErpDataGridNext';");
  });

  it('keeps review footer scrollbar visible and exports the native grid prop types for shared entry reuse', () => {
    const reviewPage = readRepoFile('apps/erp-web/src/pages/orders/OrderReviewPage.tsx');
    const reviewGrid = readRepoFile('apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx');
    const exceptionsGrid = readRepoFile(
      'apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx',
    );

    expect(reviewPage).toContain('showFooterScrollbarRow={true}');
    expect(reviewGrid).toContain('export type OrderReviewMainGridProps = {');
    expect(exceptionsGrid).toContain('export type OrderExceptionsMainGridProps = {');
  });

  it('persists focus and selection context in the review and recheck saved views', () => {
    const reviewPage = readRepoFile('apps/erp-web/src/pages/orders/OrderReviewPage.tsx');
    const exceptionsPage = readRepoFile('apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx');

    expect(reviewPage).toContain('selectedRowKeys?: string[];');
    expect(reviewPage).toContain('focusedOrderKey?: string | null;');
    expect(reviewPage).toContain('selectionAnchorKey?: string | null;');
    expect(reviewPage).toContain('activeOrderKey?: string | null;');
    expect(reviewPage).toContain('selectedRowKeys: selectedRowKeys.map((item) => String(item))');
    expect(reviewPage).toContain('focusedOrderKey,');
    expect(reviewPage).toContain('selectionAnchorKey,');
    expect(reviewPage).toContain('activeOrderKey: resolveOrderRowKey(activeOrder)');

    expect(exceptionsPage).toContain('selectedRowKeys?: string[];');
    expect(exceptionsPage).toContain('focusedOrderKey?: string | null;');
    expect(exceptionsPage).toContain('activeOrderId?: string | null;');
    expect(exceptionsPage).toContain('selectedRowKeys: selectedRowKeys.map((item) => String(item))');
    expect(exceptionsPage).toContain('focusedOrderKey,');
    expect(exceptionsPage).toContain('activeOrderId,');
  });

  it('expands the recheck main grid with richer operational ERP columns', () => {
    const exceptionsGrid = readRepoFile(
      'apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx',
    );

    expect(exceptionsGrid).toContain("colId: 'fulfillment_status'");
    expect(exceptionsGrid).toContain("colId: 'reserve_status'");
    expect(exceptionsGrid).toContain("colId: 'latest_action'");
    expect(exceptionsGrid).toContain("colId: 'sla_deadline_at'");
  });
});
