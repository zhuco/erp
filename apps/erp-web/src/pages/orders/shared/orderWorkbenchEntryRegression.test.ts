import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

const repoRoot = path.resolve(__dirname, '../../../../../..');

const readRepoFile = (relativePath: string) =>
  readFileSync(path.resolve(repoRoot, relativePath), 'utf8');

describe('order workbench shared entry regression', () => {
  it('在合并后的前端规范中记录当前表格技术决策', () => {
    const decisionDocPath = path.resolve(
      repoRoot,
      'docs/FRONTEND_GUIDE.md',
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
    const sharedEntry = readRepoFile('apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx');
    const salesQueryPage = readRepoFile(
      'apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx',
    );

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

    expect(sharedEntry).toContain("if (props.variant === 'recheck')");
    expect(sharedEntry).toContain('return <OrderExceptionsMainGrid');
    expect(sharedEntry).not.toContain('return <OrderReviewRecheckMainGrid');
    expect(sharedEntry).toContain('return <ErpDataGridNext');

    expect(salesQueryPage).toContain('OrderReviewQueryMainGrid');
  });

  it('keeps review and exceptions footer scrollbars visible and exports the native grid prop types for shared entry reuse', () => {
    const reviewPage = readRepoFile('apps/erp-web/src/pages/orders/OrderReviewPage.tsx');
    const exceptionsPage = readRepoFile('apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx');
    const reviewGrid = readRepoFile('apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx');
    const exceptionsGrid = readRepoFile(
      'apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx',
    );

    expect(reviewPage).toContain('showFooterScrollbarRow={true}');
    expect(exceptionsPage).toContain('showFooterScrollbarRow={true}');
    expect(exceptionsPage).toContain('order-review-table-external-scrollbar');
    expect(reviewGrid).toContain('export type OrderReviewMainGridProps = {');
    expect(exceptionsGrid).toContain('export type OrderExceptionsMainGridProps = {');
  });

  it('persists focus and selection context in the review and recheck saved views', () => {
    const reviewPage = readRepoFile('apps/erp-web/src/pages/orders/OrderReviewPage.tsx');
    const exceptionsPage = readRepoFile('apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx');
    const channelsPage = readRepoFile('apps/erp-web/src/pages/channels/ChannelsPage.tsx');
    const channelDirectoryPage = readRepoFile(
      'apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx',
    );

    expect(reviewPage).toContain('selectedRowKeys?: string[];');
    expect(reviewPage).toContain('focusedOrderKey?: string | null;');
    expect(reviewPage).toContain('selectionAnchorKey?: string | null;');
    expect(reviewPage).toContain('activeOrderKey?: string | null;');
    expect(reviewPage).toContain('selectedRowKeys: selectedRowKeys.map((item) => String(item))');
    expect(reviewPage).toContain('focusedOrderKey,');
    expect(reviewPage).toContain('selectionAnchorKey,');
    expect(reviewPage).toContain('activeOrderKey: resolveOrderRowKey(activeOrder)');
    expect(reviewPage).toContain('storageKey: reviewPaginationStorageKey');
    expect(reviewPage).toContain('stateCacheKey={reviewWorkbenchStateCacheKey}');
    expect(reviewPage).toContain('if (!hasFullScopeColumnFilterActive) {');

    expect(exceptionsPage).toContain('selectedRowKeys?: string[];');
    expect(exceptionsPage).toContain('focusedRowKey?: string | null;');
    expect(exceptionsPage).toContain('activeOrderId?: string | null;');
    expect(exceptionsPage).toContain('selectionState,');
    expect(exceptionsPage).toContain('focusedRowKey: focusedOrderKey,');
    expect(exceptionsPage).toContain('activeOrderId,');
    expect(exceptionsPage).toContain('storageKey: exceptionsPaginationStorageKey');
    expect(exceptionsPage).toContain('stateCacheKey={exceptionsWorkbenchStateCacheKey}');
    expect(exceptionsPage).toContain('const rightPanelSyncKey = useMemo(');
    expect(exceptionsPage).toContain('const bottomPanelSyncKey = useMemo(');
    expect(exceptionsPage).toContain('rightPanelSyncKey={rightPanelSyncKey}');
    expect(exceptionsPage).toContain('bottomPanelSyncKey={bottomPanelSyncKey}');

    expect(channelsPage).toContain(
      "import { ChannelDirectoryPage } from './ChannelDirectoryPage';",
    );
    expect(channelsPage).toContain('return <ChannelDirectoryPage />;');

    expect(channelDirectoryPage).toContain('type ChannelDirectorySavedViewState = {');
    expect(channelDirectoryPage).toContain('selectedRowKeys');
    expect(channelDirectoryPage).toContain('persistSelectionState: false');
    expect(channelDirectoryPage).toContain('storageKey: paginationStorageKey');
    expect(channelDirectoryPage).toContain('stateCacheKey={stateCacheKey}');
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

  it('hides the shared workbench selection diagnostic pill by default', () => {
    const ordersCss = readRepoFile('apps/erp-web/src/pages/orders/orderReviewPage.css');

    expect(ordersCss).toMatch(/\.order-review-selection-diagnostic\s*\{[^}]*display:\s*none;/s);
  });
});
