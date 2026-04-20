import type { Column, ColumnState, GridApi } from 'ag-grid-community';

const workbenchStructuralColumnIds = new Set(['__erpSelection', '__erpRowIndex']);

export type WorkbenchColumnSortDirection = 'none' | 'asc' | 'desc';

export type WorkbenchColumnSettingsItem = {
  colId: string;
  label: string;
  hidden: boolean;
  sortDirection: WorkbenchColumnSortDirection;
  sortPriority: number | null;
};

type WorkbenchColumnSettingsGridApi<TRow> = Pick<
  GridApi<TRow>,
  'getAllGridColumns' | 'getColumnState' | 'getDisplayNameForColumn'
>;

const normalizeColumnLabel = (column: Column, api: Pick<GridApi<unknown>, 'getDisplayNameForColumn'> | null) => {
  const displayName = api?.getDisplayNameForColumn?.(column, 'columnToolPanel' as never);
  if (typeof displayName === 'string' && displayName.trim()) {
    return displayName.trim();
  }

  const headerName = String(column.getColDef().headerName ?? '').trim();
  if (headerName) {
    return headerName;
  }

  return String(column.getColId?.() ?? '').trim();
};

const normalizeSortPriority = (value: number | null | undefined) => {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return Number.MAX_SAFE_INTEGER;
  }
  return Math.max(1, Math.trunc(value));
};

export const buildWorkbenchColumnSettingsItems = <TRow,>(
  api: WorkbenchColumnSettingsGridApi<TRow>,
): WorkbenchColumnSettingsItem[] => {
  const gridColumns = api.getAllGridColumns() ?? [];
  const stateMap = new Map(api.getColumnState().map((item) => [String(item.colId ?? '').trim(), item]));

  return gridColumns
    .filter((column) => {
      const colId = String(column.getColId?.() ?? '').trim();
      return Boolean(colId) && !workbenchStructuralColumnIds.has(colId);
    })
    .map((column) => {
      const colId = String(column.getColId?.() ?? '').trim();
      const columnState = stateMap.get(colId);
      const sortDirection =
        columnState?.sort === 'asc' || columnState?.sort === 'desc' ? columnState.sort : ('none' as const);

      return {
        colId,
        label: normalizeColumnLabel(column, api as Pick<GridApi<unknown>, 'getDisplayNameForColumn'>),
        hidden: columnState?.hide === true || column.isVisible?.() === false,
        sortDirection,
        sortPriority:
          sortDirection === 'none'
            ? null
            : typeof columnState?.sortIndex === 'number' && Number.isFinite(columnState.sortIndex)
              ? columnState.sortIndex + 1
              : 1,
      };
    });
};

export const buildWorkbenchColumnSettingsApplyState = (
  items: WorkbenchColumnSettingsItem[],
): ColumnState[] => {
  const sortedItems = items
    .filter((item) => item.sortDirection === 'asc' || item.sortDirection === 'desc')
    .map((item, index) => ({
      ...item,
      originalIndex: index,
      normalizedPriority: normalizeSortPriority(item.sortPriority),
    }))
    .sort((left, right) => {
      if (left.normalizedPriority !== right.normalizedPriority) {
        return left.normalizedPriority - right.normalizedPriority;
      }
      return left.originalIndex - right.originalIndex;
    });

  const sortIndexMap = new Map(sortedItems.map((item, index) => [item.colId, index]));

  return items.map((item) => {
    const nextState: ColumnState = {
      colId: item.colId,
      hide: item.hidden,
    };

    if (item.sortDirection === 'asc' || item.sortDirection === 'desc') {
      nextState.sort = item.sortDirection;
      nextState.sortIndex = sortIndexMap.get(item.colId) ?? 0;
    }

    return nextState;
  });
};
