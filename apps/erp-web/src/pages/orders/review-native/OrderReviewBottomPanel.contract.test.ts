import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('OrderReviewBottomPanel', () => {
  it('extends the unified bottom panel to industrial multi-tab detail sections', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewBottomPanel.tsx'), 'utf8');
    const css = readFileSync(resolve(__dirname, 'OrderReviewBottomPanel.css'), 'utf8');

    expect(source).toContain("import './OrderReviewBottomPanel.css';");
    expect(source).toContain('goodsEditable = true');
    expect(source).toContain("const [activeTab, setActiveTab] = useState('goods')");
    expect(source).toContain("key: 'platformOrders'");
    expect(source).toContain("key: 'shipments'");
    expect(source).toContain("key: 'fees'");
    expect(source).toContain('平台订单');
    expect(source).toContain('发货单');
    expect(source).toContain('订单费用');
    expect(source).toContain("syncKey={`${activeOrderId || 'none'}:${activeTab}`}");
    expect(source).toContain('formatDateTimeSeconds');
    expect(source).not.toContain("setActiveTab('goods');");
    expect(source).not.toContain('order-review-goods-selected-count');
    expect(source).toContain('rowHeight={ORDER_WORKBENCH_GRID_ROW_HEIGHT}');
    expect(source).toContain('headerHeight={ORDER_WORKBENCH_GRID_HEADER_HEIGHT}');
    expect(source).toContain('showSorterTooltip={false}');
    expect(source).toContain('bordered');
    expect(source).toContain('sortable: true');
    expect(source).toContain('className="order-workbench-bottom-panel order-review-bottom-panel"');
    expect(css).toContain('.order-review-bottom-panel .ant-tabs-nav');
    expect(css).toContain('.order-review-bottom-ag-shell');
    expect(css).toContain('.order-review-goods-tab');
    expect(css).toContain('.order-review-bottom-panel .ant-table-wrapper');
    expect(css).toContain('margin: 0 0 2px;');
    expect(css).toContain('padding: 0 2px;');
    expect(css).toContain('padding: 0 2px 2px;');
    expect(css).toContain('padding: 4px 8px;');
    expect(css).toContain('font-size: var(--erp-font-size-body);');
    expect(css).toContain('justify-content: flex-start;');
    expect(css).toContain('scrollbar-width: none;');
    expect(css).toContain('border: 1px solid #d7e0ea;');
    expect(css).toContain('.order-review-bottom-ag-shell .ag-body-horizontal-scroll');
    expect(css).toContain('.order-review-bottom-panel .ant-tabs-tabpane {');
    expect(css).toContain('overflow: hidden;');
    expect(css).toContain('.order-review-bottom-panel .ant-table-thead > tr > th:not(:last-child)');
    expect(css).toContain('.order-review-bottom-panel .ant-table-tbody > tr > td:not(:last-child)');
    expect(css).toContain('.order-review-bottom-ag-shell .ag-header-cell,');
    expect(css).toContain('.order-review-bottom-ag-shell .ag-row .ag-cell:last-child');
  });
});
