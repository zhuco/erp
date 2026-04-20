import { useCallback, useState } from 'react';
import type { GridApi } from 'ag-grid-community';

import { buildAgGridApplyColumnState, persistAgGridColumnState } from '@/components/agGridColumnState';

import { sanitizeWorkbenchColumnState } from './workbenchGridColumnPolicy';
import {
  buildWorkbenchColumnSettingsApplyState,
  buildWorkbenchColumnSettingsItems,
  type WorkbenchColumnSettingsItem,
} from './workbenchColumnSettings';

type UseWorkbenchColumnSettingsOptions<TRow> = {
  columnStateStorageKey?: string | null;
  gridApi: GridApi<TRow> | null;
  onColumnStateApplied?: (api: GridApi<TRow>) => void;
};

const updateWorkbenchColumnSettingsItems = (
  items: WorkbenchColumnSettingsItem[],
  colId: string,
  updater: (current: WorkbenchColumnSettingsItem) => WorkbenchColumnSettingsItem,
) =>
  items.map((item) => {
    if (item.colId !== colId) {
      return item;
    }
    return updater(item);
  });

export const useWorkbenchColumnSettings = <TRow,>({
  columnStateStorageKey,
  gridApi,
  onColumnStateApplied,
}: UseWorkbenchColumnSettingsOptions<TRow>) => {
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<WorkbenchColumnSettingsItem[]>([]);

  const reloadItems = useCallback(
    (api?: GridApi<TRow> | null) => {
      const targetApi = api ?? gridApi;
      if (!targetApi) {
        setItems([]);
        return;
      }
      setItems(buildWorkbenchColumnSettingsItems(targetApi));
    },
    [gridApi],
  );

  const openDialog = useCallback(() => {
    if (!gridApi) {
      return;
    }
    reloadItems(gridApi);
    setOpen(true);
  }, [gridApi, reloadItems]);

  const closeDialog = useCallback(() => {
    setOpen(false);
  }, []);

  const updateItem = useCallback(
    (colId: string, updater: (current: WorkbenchColumnSettingsItem) => WorkbenchColumnSettingsItem) => {
      setItems((current) => updateWorkbenchColumnSettingsItems(current, colId, updater));
    },
    [],
  );

  const persistCurrentColumnState = useCallback(
    (api: GridApi<TRow>) => {
      if (!columnStateStorageKey || typeof window === 'undefined') {
        return;
      }
      persistAgGridColumnState(window.localStorage, columnStateStorageKey, api.getColumnState());
    },
    [columnStateStorageKey],
  );

  const applyDraft = useCallback(() => {
    if (!gridApi) {
      return;
    }

    const draftState = buildWorkbenchColumnSettingsApplyState(items);
    gridApi.applyColumnState({
      state: sanitizeWorkbenchColumnState(buildAgGridApplyColumnState(draftState)),
      applyOrder: false,
    });
    gridApi.refreshHeader();
    persistCurrentColumnState(gridApi);
    onColumnStateApplied?.(gridApi);
    setOpen(false);
  }, [gridApi, items, onColumnStateApplied, persistCurrentColumnState]);

  const resetDraft = useCallback(() => {
    reloadItems();
  }, [reloadItems]);

  const resetToDefault = useCallback(() => {
    if (!gridApi) {
      return;
    }

    gridApi.resetColumnState();
    gridApi.refreshHeader();
    if (columnStateStorageKey && typeof window !== 'undefined') {
      persistAgGridColumnState(window.localStorage, columnStateStorageKey, []);
    }
    onColumnStateApplied?.(gridApi);
    reloadItems(gridApi);
  }, [columnStateStorageKey, gridApi, onColumnStateApplied, reloadItems]);

  return {
    items,
    open,
    openDialog,
    closeDialog,
    updateItem,
    applyDraft,
    resetDraft,
    resetToDefault,
  };
};
