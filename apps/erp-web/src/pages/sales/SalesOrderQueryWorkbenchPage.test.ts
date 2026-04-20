import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('SalesOrderQueryWorkbenchPage adoption', () => {
  it('uses the unified workbench template and shared controllers', () => {
    const source = readFileSync(resolve(__dirname, 'SalesOrderQueryWorkbenchPage.tsx'), 'utf8');

    expect(source).toContain('WorkbenchPageTemplate');
    expect(source).toContain('useWorkbenchInteractionController');
    expect(source).toContain('useWorkbenchSelectionController');
    expect(source).toContain('useWorkbenchViewStateController');
    expect(source).toContain('useLeftFilterModule');
    expect(source).toContain('OrderReviewQueryMainGrid');
    expect(source).toContain('OrderReviewBottomPanel');
  });

  it('keeps the page query-first and excludes review action wiring', () => {
    const source = readFileSync(resolve(__dirname, 'SalesOrderQueryWorkbenchPage.tsx'), 'utf8');

    expect(source).not.toContain("id: 'approve'");
    expect(source).not.toContain("id: 'merge'");
    expect(source).not.toContain("id: 'split'");
    expect(source).toContain('onExport={handleExport}');
    expect(source).toContain('onPrint={handlePrint}');
    expect(source).toContain('onHelp={handleHelp}');
  });

  it('aligns the page with the unified workbench state and right-panel marks contract', () => {
    const source = readFileSync(resolve(__dirname, 'SalesOrderQueryWorkbenchPage.tsx'), 'utf8');

    expect(source).toContain('quickFilterExpanded');
    expect(source).toContain('columnFilters');
    expect(source).toContain('listOrderTagDefinitions');
    expect(source).toContain('addOrderTags');
    expect(source).toContain('createOrderTagDefinition');
    expect(source).toContain('deleteOrderTagDefinition');
    expect(source).toContain('OrderReviewDetailPanel');
    expect(source).toContain('orderTagOptions={orderTagOptions}');
    expect(source).toContain('currentOrderTags={currentOrderTags}');
    expect(source).toContain('rightPanelSyncKey={rightPanelSyncKey}');
    expect(source).toContain('bottomPanelSyncKey={bottomPanelSyncKey}');
    expect(source).toContain('stateCacheKey={workbenchStateKey}');
    expect(source).toContain('storageKey: paginationStorageKey');
  });

  it('reuses the shared industrial bottom tabs for platform order, shipment and fee sections', () => {
    const source = readFileSync(resolve(__dirname, 'SalesOrderQueryWorkbenchPage.tsx'), 'utf8');

    expect(source).toContain('resolveOrderWorkbenchPlatformOrderRows');
    expect(source).toContain('resolveOrderWorkbenchShipmentRows');
    expect(source).toContain('resolveOrderWorkbenchFeeRows');
    expect(source).toContain('listShipments');
    expect(source).toContain('goodsEditable={false}');
  });

  it('reuses the shared pagination contract and keeps selection transient across reopen', () => {
    const source = readFileSync(resolve(__dirname, 'SalesOrderQueryWorkbenchPage.tsx'), 'utf8');

    expect(source).toContain('defaultPageSize');
    expect(source).toContain('pageSizeOptions');
    expect(source).toContain('persistSelectionState: false');
    expect(source).not.toContain('selectedRowKeys?: string[];');
    expect(source).not.toContain('defaultPageSize: 20');
    expect(source).not.toContain("pageSizeOptions: ['20', '50', '100', '200']");
  });

  it('aligns footer summary copy with the shared main-table row-count contract', () => {
    const source = readFileSync(resolve(__dirname, 'SalesOrderQueryWorkbenchPage.tsx'), 'utf8');

    expect(source).toContain('总数 ${total} 条，本页 ${rows.length} 条，已选 ${selectedRowKeys.length} 条');
  });

  it('registers the page in the router under the sales module', () => {
    const source = readFileSync(resolve(__dirname, '../../app/router.tsx'), 'utf8');

    expect(source).toContain('SalesOrderQueryWorkbenchPage');
    expect(source).toContain("path: 'sales/order-query'");
  });
});
