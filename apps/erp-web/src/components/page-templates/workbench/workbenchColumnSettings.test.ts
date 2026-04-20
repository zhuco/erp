import type { Column, ColumnState, GridApi } from 'ag-grid-community';
import { describe, expect, it } from 'vitest';

import {
  buildWorkbenchColumnSettingsApplyState,
  buildWorkbenchColumnSettingsItems,
  type WorkbenchColumnSettingsItem,
} from './workbenchColumnSettings';

type MockRow = {
  id: string;
};

const createMockColumn = (colId: string, headerName: string) =>
  ({
    getColId: () => colId,
    getColDef: () => ({
      colId,
      headerName,
    }),
    isVisible: () => true,
  }) as unknown as Column;

describe('workbenchColumnSettings', () => {
  it('builds business-column settings items and excludes structural columns', () => {
    const columns = [
      createMockColumn('__erpSelection', ''),
      createMockColumn('__erpRowIndex', '序号'),
      createMockColumn('id', '订单号'),
      createMockColumn('receiver_name', '收件人'),
      createMockColumn('goods_qty', '商品数'),
    ];

    const state: ColumnState[] = [
      { colId: '__erpSelection', hide: false },
      { colId: '__erpRowIndex', hide: false },
      { colId: 'id', hide: false, sort: 'asc', sortIndex: 1 },
      { colId: 'receiver_name', hide: true },
      { colId: 'goods_qty', hide: false, sort: 'desc', sortIndex: 0 },
    ];

    const api = {
      getAllGridColumns: () => columns,
      getColumnState: () => state,
      getDisplayNameForColumn: (column: Column) => column.getColDef().headerName ?? column.getColId(),
    } as unknown as Pick<GridApi<MockRow>, 'getAllGridColumns' | 'getColumnState' | 'getDisplayNameForColumn'>;

    expect(buildWorkbenchColumnSettingsItems(api)).toEqual([
      {
        colId: 'id',
        hidden: false,
        label: '订单号',
        sortDirection: 'asc',
        sortPriority: 2,
      },
      {
        colId: 'receiver_name',
        hidden: true,
        label: '收件人',
        sortDirection: 'none',
        sortPriority: null,
      },
      {
        colId: 'goods_qty',
        hidden: false,
        label: '商品数',
        sortDirection: 'desc',
        sortPriority: 1,
      },
    ]);
  });

  it('builds normalized column state for display and multi-column sorting', () => {
    const items: WorkbenchColumnSettingsItem[] = [
      {
        colId: 'id',
        hidden: false,
        label: '订单号',
        sortDirection: 'asc',
        sortPriority: 3,
      },
      {
        colId: 'receiver_name',
        hidden: true,
        label: '收件人',
        sortDirection: 'none',
        sortPriority: null,
      },
      {
        colId: 'goods_qty',
        hidden: false,
        label: '商品数',
        sortDirection: 'desc',
        sortPriority: 1,
      },
      {
        colId: 'shop_id',
        hidden: false,
        label: '店铺',
        sortDirection: 'asc',
        sortPriority: 1,
      },
    ];

    expect(buildWorkbenchColumnSettingsApplyState(items)).toEqual([
      { colId: 'id', hide: false, sort: 'asc', sortIndex: 2 },
      { colId: 'receiver_name', hide: true },
      { colId: 'goods_qty', hide: false, sort: 'desc', sortIndex: 0 },
      { colId: 'shop_id', hide: false, sort: 'asc', sortIndex: 1 },
    ]);
  });
});
