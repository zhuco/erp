import { describe, expect, it, vi } from 'vitest';

import {
  exportOrderReviewGridAsExcel,
  printOrderReviewGrid,
} from './orderReviewGridActions';

type MockColumn = {
  getColId: () => string;
};

type MockGridApi = {
  exportDataAsExcel: ReturnType<typeof vi.fn>;
  getAllDisplayedColumns: () => MockColumn[];
  getDataAsCsv: ReturnType<typeof vi.fn>;
};

const createGridApi = (): MockGridApi => ({
  exportDataAsExcel: vi.fn(),
  getAllDisplayedColumns: () => [
    { getColId: () => '__erpSelection' },
    { getColId: () => '__erpRowIndex' },
    { getColId: () => 'id' },
    { getColId: () => 'platform_order_id' },
  ],
  getDataAsCsv: vi.fn(() => '"订单号","平台单号"\n"SO-001","P-001"\n"SO-002","含,逗号"'),
});

describe('orderReviewGridActions', () => {
  it('exports the current grid as excel without internal utility columns', () => {
    const api = createGridApi();

    exportOrderReviewGridAsExcel(api as never, '订单审核');

    expect(api.exportDataAsExcel).toHaveBeenCalledTimes(1);
    expect(api.exportDataAsExcel).toHaveBeenCalledWith(
      expect.objectContaining({
        sheetName: '订单审核',
        columnKeys: ['id', 'platform_order_id'],
      }),
    );
  });

  it('prints the current filtered grid data via a popup window', () => {
    const api = createGridApi();
    const print = vi.fn();
    const focus = vi.fn();
    const close = vi.fn();
    const write = vi.fn();
    const docClose = vi.fn();
    const popup = {
      document: {
        write,
        close: docClose,
      },
      focus,
      print,
      close,
      onafterprint: null as null | (() => void),
    };
    const openWindow = vi.fn(() => popup);

    printOrderReviewGrid(api as never, '订单审核', openWindow);

    expect(api.getDataAsCsv).toHaveBeenCalledWith(
      expect.objectContaining({
        columnKeys: ['id', 'platform_order_id'],
      }),
    );
    expect(openWindow).toHaveBeenCalledTimes(1);
    expect(write).toHaveBeenCalledTimes(1);
    const html = write.mock.calls[0]?.[0] as string;
    expect(html).toContain('订单审核');
    expect(html).toContain('<th>订单号</th>');
    expect(html).toContain('<td>含,逗号</td>');
    expect(docClose).toHaveBeenCalledTimes(1);
    expect(focus).toHaveBeenCalledTimes(1);
    expect(print).toHaveBeenCalledTimes(1);
    expect(typeof popup.onafterprint).toBe('function');
    popup.onafterprint?.();
    expect(close).toHaveBeenCalledTimes(1);
  });
});
