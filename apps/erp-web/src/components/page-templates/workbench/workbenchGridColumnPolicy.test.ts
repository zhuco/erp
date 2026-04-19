import type { ColDef, ColumnState } from 'ag-grid-community';
import { describe, expect, it } from 'vitest';

import {
  applyWorkbenchGridColumnPolicy,
  sanitizeWorkbenchColumnState,
} from './workbenchGridColumnPolicy';

type MockRow = {
  id: string;
  name: string;
  qty: number;
};

describe('workbenchGridColumnPolicy', () => {
  it('locks the sequence column sorting and preserves business-column sorting', () => {
    const colDefs: ColDef<MockRow>[] = [
      { colId: '__erpRowIndex', headerName: '序号', sortable: true, filter: false },
      { colId: 'id', field: 'id', headerName: '订单号', sortable: true, filter: 'agTextColumnFilter' },
      { colId: 'qty', field: 'qty', headerName: '商品数', sortable: true, filter: false },
    ];

    const nextColDefs = applyWorkbenchGridColumnPolicy(colDefs);
    const colDefMap = new Map(nextColDefs.map((colDef) => [String(colDef.colId ?? ''), colDef]));

    expect(colDefMap.get('__erpRowIndex')?.sortable).toBe(false);
    expect(colDefMap.get('__erpRowIndex')?.filter).toBe('agNumberColumnFilter');
    expect(colDefMap.get('id')?.sortable).toBe(true);
    expect(colDefMap.get('qty')?.filter).toBe('agNumberColumnFilter');
  });

  it('adds a default text filter for computed business columns', () => {
    const colDefs: ColDef<MockRow>[] = [
      {
        colId: 'computed_status',
        headerName: '状态',
        valueGetter: () => '待审核',
        filter: false,
      },
    ];

    const [computedColumn] = applyWorkbenchGridColumnPolicy(colDefs);

    expect(computedColumn?.filter).toBe('agTextColumnFilter');
    expect(typeof computedColumn?.filterValueGetter).toBe('function');
    expect(
      computedColumn?.filterValueGetter?.({
        data: { id: 'HC202604190001', name: '订单', qty: 3 },
      } as never),
    ).toBe('待审核');
  });

  it('sanitizes persisted sequence-column sorting state', () => {
    const columnState: ColumnState[] = [
      { colId: '__erpRowIndex', hide: false, sort: 'desc', sortIndex: 0 },
      { colId: 'id', sort: 'asc', sortIndex: 1 },
    ];

    const sanitizedState = sanitizeWorkbenchColumnState(columnState);

    expect(sanitizedState).toEqual([
      { colId: '__erpRowIndex', hide: false },
      { colId: 'id', sort: 'asc', sortIndex: 1 },
    ]);
  });
});
