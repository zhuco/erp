import type { GridApi } from 'ag-grid-community';
import { useEffect, useMemo, useState } from 'react';

import { resolveAgGridHeaderSelectionState } from './agGridSelectionHelpers';

type AgGridSelectionHeaderCheckboxProps<TRow> = {
  api: GridApi<TRow>;
  onToggleAllSelection: () => void;
  inputClassName?: string;
  labelClassName?: string;
  testId?: string;
  title?: string;
  ariaLabel?: string;
};

const selectionStateEvents = [
  'selectionChanged',
  'modelUpdated',
  'filterChanged',
  'rowDataUpdated',
] as const;

const buildSelectionState = <TRow,>(api: GridApi<TRow>) =>
  resolveAgGridHeaderSelectionState({
    forEachNodeAfterFilter: (callback) => {
      api.forEachNodeAfterFilter(callback);
    },
  });

export const AgGridSelectionHeaderCheckbox = <TRow,>({
  api,
  onToggleAllSelection,
  inputClassName = 'order-review-main-selection-input',
  labelClassName = 'order-review-main-selection-header',
  testId = 'erp-selection-header-checkbox',
  title = '全选/取消全选',
  ariaLabel = '全选/取消全选',
}: AgGridSelectionHeaderCheckboxProps<TRow>) => {
  const [selectionState, setSelectionState] = useState(() => buildSelectionState(api));

  useEffect(() => {
    const refreshSelectionState = () => {
      setSelectionState(buildSelectionState(api));
    };

    refreshSelectionState();
    selectionStateEvents.forEach((eventName) => {
      api.addEventListener(eventName, refreshSelectionState);
    });

    return () => {
      selectionStateEvents.forEach((eventName) => {
        api.removeEventListener(eventName, refreshSelectionState);
      });
    };
  }, [api]);

  const checked = useMemo(() => selectionState.checked, [selectionState.checked]);
  const indeterminate = useMemo(() => selectionState.indeterminate, [selectionState.indeterminate]);

  return (
    <label className={labelClassName} title={title} onClick={(event) => event.stopPropagation()}>
      <input
        type="checkbox"
        className={inputClassName}
        data-testid={testId}
        checked={checked}
        ref={(node) => {
          if (node) {
            node.indeterminate = indeterminate;
          }
        }}
        onChange={(event) => {
          event.stopPropagation();
          queueMicrotask(() => {
            onToggleAllSelection();
          });
        }}
        onClick={(event) => event.stopPropagation()}
        aria-label={ariaLabel}
      />
    </label>
  );
};
