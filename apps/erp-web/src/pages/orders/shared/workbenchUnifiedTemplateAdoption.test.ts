import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('workbench unified template adoption', () => {
  it('moves left-panel composition into WorkbenchPageTemplate usage for order review', () => {
    const source = readFileSync(resolve(__dirname, '../OrderReviewPage.tsx'), 'utf8');

    expect(source).toContain('leftPanel={leftPanel}');
  });

  it('moves left-panel composition into WorkbenchPageTemplate usage for order recheck', () => {
    const source = readFileSync(resolve(__dirname, '../OrderExceptionsPage.tsx'), 'utf8');

    expect(source).toContain('leftPanel={leftPanel}');
  });

  it('moves left-panel composition into WorkbenchPageTemplate usage for sales order query', () => {
    const source = readFileSync(resolve(__dirname, '../../sales/SalesOrderQueryWorkbenchPage.tsx'), 'utf8');

    expect(source).toContain('WorkbenchPageTemplate');
    expect(source).toContain('leftPanel={leftPanel}');
  });

  it('routes order recheck state and context-menu wiring through shared workbench controllers', () => {
    const source = readFileSync(resolve(__dirname, '../OrderExceptionsPage.tsx'), 'utf8');

    expect(source).toContain('useWorkbenchInteractionController');
    expect(source).toContain('useWorkbenchSelectionController');
    expect(source).toContain('useWorkbenchViewStateController');
    expect(source).toContain('handleMainSelectionChange');
  });

  it('keeps order recheck aligned with the unified industrial workbench contract', () => {
    const source = readFileSync(resolve(__dirname, '../OrderExceptionsPage.tsx'), 'utf8');
    const css = readFileSync(resolve(__dirname, '../orderExceptionsPage.css'), 'utf8');

    expect(source).toContain('screenFiltersDraft');
    expect(source).toContain('screenFiltersApplied');
    expect(source).toContain('OrderExceptionsScreenFilterForm');
    expect(source).toContain('useOrderExceptionsLeftModule({');
    expect(source).toContain('screenContent,');
    expect(source).toContain('className="order-review-page order-exceptions-page"');
    expect(source).toContain(
      'tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"',
    );
    expect(source).toContain('onRowActive={handleMainRowActive}');
    expect(source).toContain('onRowDoubleClick={handleMainRowDoubleClick}');
    expect(source).toContain('onFocus={handleMainFocus}');
    expect(source).toContain('rows.map((row) => resolveOrderExceptionSlaStatus(row))');
    expect(css).not.toContain('.order-exceptions-grid-panel');
    expect(css).not.toContain('.order-exceptions-main-stack');
    expect(css).not.toContain('.order-exceptions-detail-panel');
    expect(css).not.toContain('.order-exceptions-toolbar-left');
    expect(css).not.toContain('.order-exceptions-left-screen-tip');
  });

  it('keeps the recheck right panel limited to the shared order detail panel without extra duplicate summary sections', () => {
    const source = readFileSync(resolve(__dirname, '../OrderExceptionsPage.tsx'), 'utf8');

    expect(source).not.toContain('renderDetailContent(activeRow, detailQuery.data, detailQuery.isLoading)');
  });

  it('keeps the shared left-filter module free of direct panel-store mounting', () => {
    const source = readFileSync(resolve(__dirname, 'useLeftFilterModule.tsx'), 'utf8');

    expect(source).not.toContain('useWorkbenchPanels');
    expect(source).toContain('leftPanel');
    expect(source).toContain('return {');
  });
});
