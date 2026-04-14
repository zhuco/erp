import type { ColumnState } from 'ag-grid-community';
import { describe, expect, it } from 'vitest';

import {
  buildAgGridApplyColumnState,
  ERP_AG_GRID_LOCALE_TEXT_ZH_CN,
  ERP_AG_GRID_MAIN_MENU_ITEMS,
  buildAgGridColumnStateStorageKey,
  normalizePersistedColumnState,
  persistAgGridColumnState,
  readPersistedAgGridColumnState,
  resolveAgGridMainMenuItems,
} from './agGridColumnState';

const createStorage = () => {
  const map = new Map<string, string>();
  return {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => {
      map.set(key, value);
    },
    removeItem: (key: string) => {
      map.delete(key);
    },
  };
};

describe('agGridColumnState', () => {
  it('returns no native column menu for structural columns', () => {
    expect(resolveAgGridMainMenuItems('__erpSelection')).toEqual([]);
    expect(resolveAgGridMainMenuItems('__erpRowIndex')).toEqual([]);
    expect(resolveAgGridMainMenuItems('platform_order_id')).toEqual(ERP_AG_GRID_MAIN_MENU_ITEMS);
  });

  it('builds tenant and user scoped storage keys', () => {
    expect(
      buildAgGridColumnStateStorageKey({
        columnSettingsKey: 'orders.main',
        tenantId: 'tenant-1',
        userId: 'user-1',
      }),
    ).toBe('erp.aggrid.columns.tenant-1.user-1.orders.main');
  });

  it('normalizes persisted column state and filters structural columns', () => {
    const input: ColumnState[] = [
      { colId: '__erpSelection', width: 40, pinned: 'left' },
      { colId: 'order_no', width: 180, hide: false, sort: 'asc', sortIndex: 0 },
      { colId: 'shop_id', width: 0, hide: true, pinned: 'right' },
      { colId: 'invalid_sort', sort: 'sideways' as never },
      { colId: '', width: 120 },
    ];

    expect(normalizePersistedColumnState(input)).toEqual([
      {
        colId: 'order_no',
        width: 180,
        hide: false,
        sort: 'asc',
        sortIndex: 0,
      },
      {
        colId: 'shop_id',
        hide: true,
        pinned: 'right',
      },
      {
        colId: 'invalid_sort',
      },
    ]);
  });

  it('persists and restores normalized column state', () => {
    const storage = createStorage();
    const storageKey = 'erp.aggrid.columns.tenant-1.user-1.orders.main';

    persistAgGridColumnState(storage, storageKey, [
      { colId: '__erpRowIndex', width: 56, pinned: 'left' },
      { colId: 'created_at', width: 160, sort: 'desc', sortIndex: 0 },
      { colId: 'receiver_name', hide: true, pinned: 'left' },
    ]);

    expect(readPersistedAgGridColumnState(storage, storageKey)).toEqual([
      { colId: 'created_at', width: 160, sort: 'desc', sortIndex: 0 },
      { colId: 'receiver_name', hide: true, pinned: 'left' },
    ]);
  });

  it('rebuilds apply state with structural columns kept at the front without forcing left pinning', () => {
    const applyState = buildAgGridApplyColumnState([
      { colId: 'created_at', width: 160, sort: 'desc', sortIndex: 0 },
      { colId: '__erpSelection', width: 36, pinned: 'right' },
      { colId: 'receiver_name', hide: true, pinned: 'right' },
    ]);

    expect(applyState).toEqual([
      { colId: '__erpSelection', hide: false },
      { colId: '__erpRowIndex', hide: false },
      { colId: 'created_at', width: 160, sort: 'desc', sortIndex: 0 },
      { colId: 'receiver_name', hide: true, pinned: 'right' },
    ]);
  });

  it('provides Chinese locale text for native column actions', () => {
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.sortAscending).toBe('升序');
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.sortDescending).toBe('降序');
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.sortUnSort).toBe('取消排序');
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.pinLeft).toBe('固定到左侧');
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.pinRight).toBe('固定到右侧');
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.noPin).toBe('取消固定');
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.chooseColumns).toBe('选择列');
    expect(ERP_AG_GRID_LOCALE_TEXT_ZH_CN.resetColumns).toBe('重置列');
  });
});
