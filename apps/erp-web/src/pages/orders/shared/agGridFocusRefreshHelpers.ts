type FocusRefreshApi<TRowNode> = {
  getRowNode: (id: string) => TRowNode | null | undefined;
  refreshCells?: (params?: { rowNodes?: TRowNode[]; force?: boolean; suppressFlash?: boolean }) => void;
  redrawRows?: (params?: { rowNodes?: TRowNode[] }) => void;
};

type RedrawFocusedRowsOptions<TRowNode> = {
  api: FocusRefreshApi<TRowNode> | null | undefined;
  previousFocusedRowKey: string | null | undefined;
  nextFocusedRowKey: string | null | undefined;
};

const normalizeFocusedRowKeys = (previousFocusedRowKey: string | null | undefined, nextFocusedRowKey: string | null | undefined) =>
  Array.from(
    new Set(
      [previousFocusedRowKey, nextFocusedRowKey]
        .map((value) => String(value ?? '').trim())
        .filter(Boolean),
    ),
  );

export const redrawFocusedRows = <TRowNode>({
  api,
  previousFocusedRowKey,
  nextFocusedRowKey,
}: RedrawFocusedRowsOptions<TRowNode>) => {
  if (!api) {
    return;
  }

  const rowNodes = normalizeFocusedRowKeys(previousFocusedRowKey, nextFocusedRowKey)
    .map((rowKey) => api.getRowNode(rowKey))
    .filter((rowNode): rowNode is TRowNode => Boolean(rowNode));

  if (rowNodes.length === 0) {
    return;
  }

  const shouldUseCellRefresh = rowNodes.length <= 1;

  if (shouldUseCellRefresh && typeof api.refreshCells === 'function') {
    api.refreshCells({
      rowNodes,
      force: true,
      suppressFlash: true,
    });
    return;
  }

  if (typeof api.redrawRows === 'function') {
    api.redrawRows({ rowNodes });
    return;
  }

  api.refreshCells?.({
    rowNodes,
    force: true,
    suppressFlash: true,
  });
};
