import type { ColDef, ColumnState, ValueGetterFunc } from 'ag-grid-community';

export const WORKBENCH_SELECTION_COLUMN_ID = '__erpSelection';
export const WORKBENCH_SEQUENCE_COLUMN_ID = '__erpRowIndex';

const workbenchStructuralColumnIds = new Set([WORKBENCH_SELECTION_COLUMN_ID, WORKBENCH_SEQUENCE_COLUMN_ID]);

const numberFieldPattern = /(qty|count|amount|price|weight|rate|minute|version|index|total|number)$/i;
const dateFieldPattern = /(date|time|_at|At)$/;

const normalizeColumnId = <TRow,>(colDef: ColDef<TRow>) => String(colDef.colId ?? colDef.field ?? '').trim();

const resolveDefaultFilterType = <TRow,>(colDef: ColDef<TRow>) => {
  const columnId = normalizeColumnId(colDef);
  if (columnId === WORKBENCH_SEQUENCE_COLUMN_ID) {
    return 'agNumberColumnFilter';
  }
  if (typeof colDef.valueGetter === 'function' && !colDef.field && !numberFieldPattern.test(columnId)) {
    return 'agTextColumnFilter';
  }
  if (dateFieldPattern.test(columnId)) {
    return 'agDateColumnFilter';
  }
  if (numberFieldPattern.test(columnId)) {
    return 'agNumberColumnFilter';
  }
  return 'agTextColumnFilter';
};

const normalizeFilterValue = (value: unknown): string | number | Date | null => {
  if (value == null) {
    return null;
  }
  if (value instanceof Date) {
    return value;
  }
  if (Array.isArray(value)) {
    return value
      .map((item) => String(item ?? '').trim())
      .filter(Boolean)
      .join(' / ');
  }
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  const normalizedText = String(value).trim();
  return normalizedText || null;
};

const buildFallbackFilterValueGetter = <TRow,>(colDef: ColDef<TRow>) => {
  if (typeof colDef.filterValueGetter === 'function') {
    return colDef.filterValueGetter;
  }

  const valueGetter = typeof colDef.valueGetter === 'function' ? (colDef.valueGetter as ValueGetterFunc<TRow>) : null;
  const fieldName = typeof colDef.field === 'string' ? colDef.field : null;
  if (!valueGetter && !fieldName) {
    return undefined;
  }

  return (params: Parameters<NonNullable<ColDef<TRow>['filterValueGetter']>>[0]) => {
    if (valueGetter) {
      return normalizeFilterValue(valueGetter(params as never));
    }
    if (fieldName && params.data) {
      return normalizeFilterValue((params.data as Record<string, unknown>)[fieldName]);
    }
    return null;
  };
};

export const applyWorkbenchGridColumnPolicy = <TRow,>(colDefs: ColDef<TRow>[]): ColDef<TRow>[] =>
  colDefs.map((colDef) => {
    const columnId = normalizeColumnId(colDef);
    const isStructuralColumn = workbenchStructuralColumnIds.has(columnId);
    const nextColDef: ColDef<TRow> = {
      ...colDef,
    };

    if (columnId === WORKBENCH_SEQUENCE_COLUMN_ID) {
      nextColDef.sortable = false;
    }

    if (columnId === WORKBENCH_SELECTION_COLUMN_ID) {
      nextColDef.sortable = false;
      nextColDef.filter = false;
      return nextColDef;
    }

    if (nextColDef.filter === false || nextColDef.filter == null) {
      nextColDef.filter = resolveDefaultFilterType(nextColDef);
    }

    if (nextColDef.filter !== false) {
      nextColDef.filterValueGetter = buildFallbackFilterValueGetter(nextColDef);
    }

    if (isStructuralColumn && columnId !== WORKBENCH_SEQUENCE_COLUMN_ID) {
      nextColDef.floatingFilter = false;
    }

    return nextColDef;
  });

export const sanitizeWorkbenchColumnState = (states: ColumnState[] | null | undefined): ColumnState[] => {
  if (!Array.isArray(states) || states.length === 0) {
    return [];
  }

  return states.map((state) => {
    const columnId = String(state.colId ?? '').trim();
    if (columnId !== WORKBENCH_SEQUENCE_COLUMN_ID) {
      return { ...state };
    }

    const nextState: ColumnState = {
      colId: columnId,
    };
    if (typeof state.hide === 'boolean') {
      nextState.hide = state.hide;
    }
    if (typeof state.width === 'number' && state.width > 0) {
      nextState.width = state.width;
    }
    if (state.pinned === 'left' || state.pinned === 'right') {
      nextState.pinned = state.pinned;
    }
    return nextState;
  });
};
