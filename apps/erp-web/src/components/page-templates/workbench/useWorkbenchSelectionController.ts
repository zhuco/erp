import { useEffect, useMemo, useState, type Dispatch, type SetStateAction } from 'react';

import type { WorkbenchSelectionState } from './contracts';

export const normalizeWorkbenchSelectionKey = (value: unknown) => String(value ?? '').trim();

export const filterWorkbenchSelectionKeys = <TRow, TKey extends string = string>(
  rows: TRow[],
  keys: readonly unknown[],
  resolveRowKey: (row: TRow | null | undefined) => TKey | null,
): TKey[] => {
  const existingKeys = new Set(
    rows
      .map((row) => resolveRowKey(row))
      .filter((key): key is TKey => Boolean(key)),
  );

  return Array.from(
    new Set(
      keys
        .map((key) => normalizeWorkbenchSelectionKey(key) as TKey)
        .filter((key) => Boolean(key) && existingKeys.has(key)),
    ),
  );
};

export const resolveNextWorkbenchTrackedKey = <TRow, TKey extends string = string>(
  rows: TRow[],
  currentKey: unknown,
  resolveRowKey: (row: TRow | null | undefined) => TKey | null,
): TKey | null => {
  const normalizedCurrentKey = normalizeWorkbenchSelectionKey(currentKey) as TKey;
  if (
    normalizedCurrentKey &&
    rows.some((row) => normalizeWorkbenchSelectionKey(resolveRowKey(row)) === normalizedCurrentKey)
  ) {
    return normalizedCurrentKey;
  }

  return resolveRowKey(rows[0]) ?? null;
};

type ResolveWorkbenchSelectionStateOptions<TRow, TKey extends string = string> = {
  rows: TRow[];
  selectionRows?: TRow[];
  loading?: boolean;
  state: WorkbenchSelectionState<TKey>;
  resolveRowKey: (row: TRow | null | undefined) => TKey | null;
  autoActivateFirstRow?: boolean;
};

export const resolveWorkbenchSelectionState = <TRow, TKey extends string = string>({
  rows,
  selectionRows = rows,
  loading = false,
  state,
  resolveRowKey,
  autoActivateFirstRow = true,
}: ResolveWorkbenchSelectionStateOptions<TRow, TKey>): WorkbenchSelectionState<TKey> => {
  if (rows.length === 0) {
    if (loading) {
      return state;
    }

    return {
      selectedRowKeys: [],
      focusedRowKey: null,
      activeRowKey: null,
      selectionAnchorKey: null,
    };
  }

  const selectedRowKeys = filterWorkbenchSelectionKeys(selectionRows, state.selectedRowKeys, resolveRowKey);
  const activeRowKey = autoActivateFirstRow
    ? resolveNextWorkbenchTrackedKey(rows, state.activeRowKey, resolveRowKey)
    : normalizeWorkbenchSelectionKey(state.activeRowKey)
      ? (normalizeWorkbenchSelectionKey(state.activeRowKey) as TKey)
      : null;

  const visibleRowKeySet = new Set(
    rows
      .map((row) => resolveRowKey(row))
      .filter((key): key is TKey => Boolean(key)),
  );
  const fallbackRowKey = activeRowKey ?? resolveRowKey(rows[0]) ?? null;
  const focusedCandidate = normalizeWorkbenchSelectionKey(state.focusedRowKey) as TKey;
  const focusedRowKey =
    focusedCandidate && visibleRowKeySet.has(focusedCandidate) ? focusedCandidate : fallbackRowKey;
  const selectionAnchorCandidate = normalizeWorkbenchSelectionKey(state.selectionAnchorKey) as TKey;
  const selectionAnchorKey =
    selectionAnchorCandidate && selectedRowKeys.includes(selectionAnchorCandidate)
      ? selectionAnchorCandidate
      : selectedRowKeys[0] ?? null;

  return {
    selectedRowKeys,
    focusedRowKey,
    activeRowKey,
    selectionAnchorKey,
  };
};

type UseWorkbenchSelectionControllerOptions<TRow, TKey extends string = string> = {
  rows: TRow[];
  selectionRows?: TRow[];
  loading?: boolean;
  resolveRowKey: (row: TRow | null | undefined) => TKey | null;
  autoActivateFirstRow?: boolean;
  initialState?: Partial<WorkbenchSelectionState<TKey>>;
};

type UseWorkbenchSelectionControllerResult<TRow, TKey extends string = string> = {
  selectedRowKeys: TKey[];
  setSelectedRowKeys: Dispatch<SetStateAction<TKey[]>>;
  focusedRowKey: TKey | null;
  setFocusedRowKey: Dispatch<SetStateAction<TKey | null>>;
  activeRowKey: TKey | null;
  setActiveRowKey: Dispatch<SetStateAction<TKey | null>>;
  selectionAnchorKey: TKey | null;
  setSelectionAnchorKey: Dispatch<SetStateAction<TKey | null>>;
  activeRow: TRow | null;
  selectionState: WorkbenchSelectionState<TKey>;
};

export const useWorkbenchSelectionController = <TRow, TKey extends string = string>({
  rows,
  selectionRows,
  loading = false,
  resolveRowKey,
  autoActivateFirstRow = true,
  initialState,
}: UseWorkbenchSelectionControllerOptions<TRow, TKey>): UseWorkbenchSelectionControllerResult<TRow, TKey> => {
  const [selectedRowKeys, setSelectedRowKeys] = useState<TKey[]>(() => initialState?.selectedRowKeys ?? []);
  const [focusedRowKey, setFocusedRowKey] = useState<TKey | null>(() => initialState?.focusedRowKey ?? null);
  const [activeRowKey, setActiveRowKey] = useState<TKey | null>(() => initialState?.activeRowKey ?? null);
  const [selectionAnchorKey, setSelectionAnchorKey] = useState<TKey | null>(
    () => initialState?.selectionAnchorKey ?? null,
  );

  const selectionState = useMemo<WorkbenchSelectionState<TKey>>(
    () => ({
      selectedRowKeys,
      focusedRowKey,
      activeRowKey,
      selectionAnchorKey,
    }),
    [activeRowKey, focusedRowKey, selectedRowKeys, selectionAnchorKey],
  );

  useEffect(() => {
    const nextState = resolveWorkbenchSelectionState({
      rows,
      selectionRows,
      loading,
      state: selectionState,
      resolveRowKey,
      autoActivateFirstRow,
    });

    if (
      nextState.selectedRowKeys.length !== selectionState.selectedRowKeys.length ||
      nextState.selectedRowKeys.some((key, index) => key !== selectionState.selectedRowKeys[index])
    ) {
      setSelectedRowKeys(nextState.selectedRowKeys);
    }

    if (nextState.focusedRowKey !== selectionState.focusedRowKey) {
      setFocusedRowKey(nextState.focusedRowKey);
    }

    if (nextState.activeRowKey !== selectionState.activeRowKey) {
      setActiveRowKey(nextState.activeRowKey);
    }

    if (nextState.selectionAnchorKey !== selectionState.selectionAnchorKey) {
      setSelectionAnchorKey(nextState.selectionAnchorKey);
    }
  }, [autoActivateFirstRow, loading, resolveRowKey, rows, selectionRows, selectionState]);

  const activeRow = useMemo(
    () =>
      rows.find((row) => normalizeWorkbenchSelectionKey(resolveRowKey(row)) === activeRowKey) ?? null,
    [activeRowKey, resolveRowKey, rows],
  );

  return {
    selectedRowKeys,
    setSelectedRowKeys,
    focusedRowKey,
    setFocusedRowKey,
    activeRowKey,
    setActiveRowKey,
    selectionAnchorKey,
    setSelectionAnchorKey,
    activeRow,
    selectionState,
  };
};
