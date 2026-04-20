import { useCallback } from 'react';

import type { WorkbenchSelectionState } from './contracts';

type WorkbenchViewStatePayload<TExtraState extends object, TKey extends string = string> = Partial<TExtraState> &
  Partial<WorkbenchSelectionState<TKey>>;

type UseWorkbenchViewStateControllerOptions<TExtraState extends object, TKey extends string = string> = {
  extraState: TExtraState;
  extraStateKeys: readonly (keyof TExtraState)[];
  selectionState: Partial<WorkbenchSelectionState<TKey>>;
  onHydrateExtraState: (state: Partial<TExtraState>) => void;
  onHydrateSelectionState: (state: Partial<WorkbenchSelectionState<TKey>>) => void;
  persistSelectionState?: boolean;
};

export const serializeWorkbenchViewState = <TExtraState extends object, TKey extends string = string>({
  extraState,
  extraStateKeys,
  selectionState,
  persistSelectionState = true,
}: Pick<
  UseWorkbenchViewStateControllerOptions<TExtraState, TKey>,
  'extraState' | 'extraStateKeys' | 'selectionState' | 'persistSelectionState'
>): WorkbenchViewStatePayload<TExtraState, TKey> => {
  const payload = {} as WorkbenchViewStatePayload<TExtraState, TKey>;

  extraStateKeys.forEach((key) => {
    payload[key] = extraState[key];
  });

  if (!persistSelectionState) {
    return payload;
  }

  payload.selectedRowKeys = Array.from(
    new Set(
      (selectionState.selectedRowKeys ?? [])
        .map((key) => String(key ?? '').trim() as TKey)
        .filter(Boolean),
    ),
  );
  payload.focusedRowKey = selectionState.focusedRowKey ?? null;
  payload.activeRowKey = selectionState.activeRowKey ?? null;
  payload.selectionAnchorKey = selectionState.selectionAnchorKey ?? null;

  return payload;
};

export const splitWorkbenchViewState = <TExtraState extends object, TKey extends string = string>(
  state: Partial<WorkbenchViewStatePayload<TExtraState, TKey>>,
  extraStateKeys: readonly (keyof TExtraState)[],
  persistSelectionState = true,
) => {
  const extraState = {} as Partial<TExtraState>;
  extraStateKeys.forEach((key) => {
    if (Object.prototype.hasOwnProperty.call(state, key)) {
      extraState[key] = state[key];
    }
  });

  const selectionState: Partial<WorkbenchSelectionState<TKey>> = {};
  if (!persistSelectionState) {
    return {
      extraState,
      selectionState,
    };
  }
  if (Object.prototype.hasOwnProperty.call(state, 'selectedRowKeys')) {
    selectionState.selectedRowKeys = Array.isArray(state.selectedRowKeys)
      ? Array.from(
          new Set(
            state.selectedRowKeys
              .map((key) => String(key ?? '').trim() as TKey)
              .filter(Boolean),
          ),
        )
      : [];
  }
  if (Object.prototype.hasOwnProperty.call(state, 'focusedRowKey')) {
    selectionState.focusedRowKey = state.focusedRowKey ? String(state.focusedRowKey).trim() as TKey : null;
  }
  if (Object.prototype.hasOwnProperty.call(state, 'activeRowKey')) {
    selectionState.activeRowKey = state.activeRowKey ? String(state.activeRowKey).trim() as TKey : null;
  }
  if (Object.prototype.hasOwnProperty.call(state, 'selectionAnchorKey')) {
    selectionState.selectionAnchorKey = state.selectionAnchorKey
      ? (String(state.selectionAnchorKey).trim() as TKey)
      : null;
  }

  return {
    extraState,
    selectionState,
  };
};

export const useWorkbenchViewStateController = <TExtraState extends object, TKey extends string = string>({
  extraState,
  extraStateKeys,
  selectionState,
  onHydrateExtraState,
  onHydrateSelectionState,
  persistSelectionState = true,
}: UseWorkbenchViewStateControllerOptions<TExtraState, TKey>) => {
  const hydrateViewState = useCallback(
    (state: Partial<WorkbenchViewStatePayload<TExtraState, TKey>>) => {
      const nextState = splitWorkbenchViewState(state, extraStateKeys, persistSelectionState);
      onHydrateExtraState(nextState.extraState);
      onHydrateSelectionState(
        persistSelectionState
          ? nextState.selectionState
          : {
              selectedRowKeys: [],
              focusedRowKey: null,
              activeRowKey: null,
              selectionAnchorKey: null,
            },
      );
    },
    [extraStateKeys, onHydrateExtraState, onHydrateSelectionState, persistSelectionState],
  );

  const serializeViewState = useCallback(
    () =>
      serializeWorkbenchViewState({
        extraState,
        extraStateKeys,
        selectionState,
        persistSelectionState,
      }),
    [extraState, extraStateKeys, persistSelectionState, selectionState],
  );

  return {
    hydrateViewState,
    serializeViewState,
  };
};
