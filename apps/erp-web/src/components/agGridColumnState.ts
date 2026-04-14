import type { ColumnState } from 'ag-grid-community';

type StorageLike = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

type BuildAgGridColumnStateStorageKeyOptions = {
  columnSettingsKey?: string;
  tenantId?: string;
  userId?: string;
};

type PersistedAgGridColumnStatePayload = {
  version: 1;
  columns: ColumnState[];
};

const ERP_AG_GRID_STATE_STORAGE_PREFIX = 'erp.aggrid.columns';
const ERP_AG_GRID_STRUCTURAL_COLUMN_IDS = new Set(['__erpSelection', '__erpRowIndex']);
const ERP_AG_GRID_STRUCTURAL_COLUMN_ORDER = ['__erpSelection', '__erpRowIndex'] as const;

const normalizeScopeSegment = (value: string | undefined, fallback: string) => {
  const normalized = String(value ?? '').trim();
  if (!normalized) {
    return fallback;
  }
  return normalized.replace(/\s+/g, '-');
};

const normalizeColumnStateItem = (state: ColumnState): ColumnState | null => {
  const colId = String(state.colId ?? '').trim();
  if (!colId || ERP_AG_GRID_STRUCTURAL_COLUMN_IDS.has(colId)) {
    return null;
  }

  const nextState: ColumnState = {
    colId,
  };

  if (typeof state.width === 'number' && state.width > 0) {
    nextState.width = state.width;
  }
  if (typeof state.hide === 'boolean') {
    nextState.hide = state.hide;
  }
  if (state.pinned === 'left' || state.pinned === 'right') {
    nextState.pinned = state.pinned;
  }
  if (state.sort === 'asc' || state.sort === 'desc') {
    nextState.sort = state.sort;
  }
  if (typeof state.sortIndex === 'number' && Number.isFinite(state.sortIndex)) {
    nextState.sortIndex = state.sortIndex;
  }

  return nextState;
};

export const ERP_AG_GRID_MAIN_MENU_ITEMS = [
  'sortAscending',
  'sortDescending',
  'sortUnSort',
  'separator',
  'pinSubMenu',
  'separator',
  'columnChooser',
  'resetColumns',
] as const;

export const ERP_AG_GRID_LOCALE_TEXT_ZH_CN: Record<string, string> = {
  loadingOoo: '加载中...',
  noRowsToShow: '暂无数据',
  page: '页',
  more: '更多',
  to: '到',
  of: '共',
  next: '下一页',
  last: '末页',
  first: '首页',
  previous: '上一页',
  sortAscending: '升序',
  sortDescending: '降序',
  sortUnSort: '取消排序',
  clearSort: '取消排序',
  columnChooser: '选择列',
  chooseColumns: '选择列',
  resetColumns: '重置列',
  pinColumn: '固定列',
  pinLeft: '固定到左侧',
  pinRight: '固定到右侧',
  noPin: '取消固定',
  columns: '列',
  filters: '筛选',
};

export const buildAgGridColumnStateStorageKey = ({
  columnSettingsKey,
  tenantId,
  userId,
}: BuildAgGridColumnStateStorageKeyOptions): string | null => {
  const normalizedColumnSettingsKey = String(columnSettingsKey ?? '').trim();
  if (!normalizedColumnSettingsKey) {
    return null;
  }

  const normalizedTenantId = normalizeScopeSegment(tenantId, 'default');
  const normalizedUserId = normalizeScopeSegment(userId, 'anonymous');
  return [
    ERP_AG_GRID_STATE_STORAGE_PREFIX,
    normalizedTenantId,
    normalizedUserId,
    normalizedColumnSettingsKey,
  ].join('.');
};

export const normalizePersistedColumnState = (states: ColumnState[] | null | undefined): ColumnState[] => {
  if (!Array.isArray(states) || states.length === 0) {
    return [];
  }

  return states
    .map((state) => normalizeColumnStateItem(state))
    .filter((state): state is ColumnState => state != null);
};

export const buildAgGridApplyColumnState = (states: ColumnState[] | null | undefined): ColumnState[] => {
  const normalizedStates = normalizePersistedColumnState(states);

  return [
    ...ERP_AG_GRID_STRUCTURAL_COLUMN_ORDER.map<ColumnState>((colId) => ({
      colId,
      hide: false,
    })),
    ...normalizedStates,
  ];
};

export const resolveAgGridMainMenuItems = (colId: string | null | undefined) => {
  const normalizedColId = String(colId ?? '').trim();
  if (!normalizedColId || ERP_AG_GRID_STRUCTURAL_COLUMN_IDS.has(normalizedColId)) {
    return [];
  }
  return ERP_AG_GRID_MAIN_MENU_ITEMS;
};

export const persistAgGridColumnState = (
  storage: StorageLike,
  storageKey: string | null | undefined,
  states: ColumnState[] | null | undefined,
) => {
  const normalizedStorageKey = String(storageKey ?? '').trim();
  if (!normalizedStorageKey) {
    return;
  }

  const normalizedStates = normalizePersistedColumnState(states);
  if (normalizedStates.length === 0) {
    storage.removeItem(normalizedStorageKey);
    return;
  }

  const payload: PersistedAgGridColumnStatePayload = {
    version: 1,
    columns: normalizedStates,
  };

  storage.setItem(normalizedStorageKey, JSON.stringify(payload));
};

export const readPersistedAgGridColumnState = (
  storage: StorageLike,
  storageKey: string | null | undefined,
): ColumnState[] => {
  const normalizedStorageKey = String(storageKey ?? '').trim();
  if (!normalizedStorageKey) {
    return [];
  }

  const raw = storage.getItem(normalizedStorageKey);
  if (!raw) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw) as PersistedAgGridColumnStatePayload | ColumnState[];
    if (Array.isArray(parsed)) {
      return normalizePersistedColumnState(parsed);
    }
    return normalizePersistedColumnState(parsed.columns);
  } catch {
    storage.removeItem(normalizedStorageKey);
    return [];
  }
};
