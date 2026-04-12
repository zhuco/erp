import type { CellClickedEvent } from 'ag-grid-community';
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';

import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import type { UiModule08OrderReviewLeftRenderRow } from '@/components/ui-modules';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';
import { flattenAgLeftTree } from '@/pages/orders/orderReviewAgLeftModel';

type LeftTab = 'filter' | 'screen';
type LeftNodeType = UiModule08OrderReviewLeftRenderRow['nodeType'];
type LeafNodeType = Exclude<LeftNodeType, 'group'>;

type LeftFilterNode = {
  key: string;
  nodeType: LeafNodeType;
};

type LeftFilterSetState<TFilters> = Dispatch<SetStateAction<TFilters>>;

export type LeftFilterPanelProps = {
  leftTab: LeftTab;
  onLeftTabChange: (tab: LeftTab) => void;
  leftRowsForRender: UiModule08OrderReviewLeftRenderRow[];
  onLeftRowClick: (event: CellClickedEvent<UiModule08OrderReviewLeftRenderRow>) => void;
  hasManualFilterActive: boolean;
};

type UseLeftFilterModuleOptions<TRow, TFilters> = {
  rows: TRow[];
  defaultFilters?: TFilters;
  filters?: TFilters;
  onFiltersChange?: LeftFilterSetState<TFilters>;
  initialExpandedKeys: string[];
  leftPanelWidth?: number;
  emptyRowLabel?: string;
  hasManualFilter: (filters: TFilters) => boolean;
  filterRows: (rows: TRow[], filters: TFilters) => TRow[];
  buildCheckedKeys: (filters: TFilters) => Set<string>;
  buildLeftNodes: (rows: TRow[]) => OrderReviewAgLeftNode[];
  reduceFiltersByNode: (previous: TFilters, node: LeftFilterNode) => TFilters;
  renderPanel: (props: LeftFilterPanelProps) => ReactNode;
};

type UseLeftFilterModuleResult<TRow, TFilters> = {
  filteredRows: TRow[];
  hasManualFilterActive: boolean;
  filters: TFilters;
  setFilters: LeftFilterSetState<TFilters>;
};

const resolveNextState = <TState,>(prev: TState, action: SetStateAction<TState>) =>
  typeof action === 'function' ? (action as (value: TState) => TState)(prev) : action;

export const toggleExpandedGroupKeys = (current: Set<string>, key: string) => {
  const next = new Set(current);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  return next;
};

export const buildLeftRowsForRender = (
  leftNodes: OrderReviewAgLeftNode[],
  leftExpanded: Set<string>,
  checkedKeySet: Set<string>,
  emptyRowLabel = '无匹配项',
): UiModule08OrderReviewLeftRenderRow[] => {
  const leftRows = flattenAgLeftTree(leftNodes, leftExpanded);
  if (!leftRows.length) {
    return [
      {
        key: '__left-empty__',
        label: emptyRowLabel,
        count: 0,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: false,
      },
    ];
  }

  return leftRows.map((row) => ({ ...row, checked: checkedKeySet.has(row.key) }));
};

export const useLeftFilterModule = <TRow, TFilters>({
  rows,
  defaultFilters,
  filters: controlledFilters,
  onFiltersChange,
  initialExpandedKeys,
  leftPanelWidth,
  emptyRowLabel = '无匹配项',
  hasManualFilter,
  filterRows,
  buildCheckedKeys,
  buildLeftNodes,
  reduceFiltersByNode,
  renderPanel,
}: UseLeftFilterModuleOptions<TRow, TFilters>): UseLeftFilterModuleResult<TRow, TFilters> => {
  const { setLeftPanel, setLeftPanelVisible, setLeftPanelWidth, setLeftPanelCollapsed } =
    useWorkbenchPanels();
  const [leftTab, setLeftTab] = useState<LeftTab>('filter');
  const [leftExpanded, setLeftExpanded] = useState<Set<string>>(() => new Set(initialExpandedKeys));

  const [innerFilters, setInnerFilters] = useState<TFilters | undefined>(defaultFilters);
  const hasControlledFilters = controlledFilters !== undefined && onFiltersChange !== undefined;
  const filters = (hasControlledFilters ? controlledFilters : innerFilters) as TFilters;

  const setFilters = useCallback<LeftFilterSetState<TFilters>>(
    (action) => {
      if (hasControlledFilters) {
        onFiltersChange((prev) => resolveNextState(prev, action));
        return;
      }
      setInnerFilters((prev) => {
        const safePrev = prev as TFilters;
        return resolveNextState(safePrev, action);
      });
    },
    [hasControlledFilters, onFiltersChange],
  );

  const hasManualFilterActive = hasManualFilter(filters);
  const filteredRows = useMemo(() => filterRows(rows, filters), [filterRows, filters, rows]);
  const checkedKeySet = useMemo(() => buildCheckedKeys(filters), [buildCheckedKeys, filters]);
  const leftNodes = useMemo(() => buildLeftNodes(rows), [buildLeftNodes, rows]);
  const leftRowsForRender = useMemo(
    () => buildLeftRowsForRender(leftNodes, leftExpanded, checkedKeySet, emptyRowLabel),
    [checkedKeySet, emptyRowLabel, leftExpanded, leftNodes],
  );

  const onLeftRowClick = useCallback(
    (event: CellClickedEvent<UiModule08OrderReviewLeftRenderRow>) => {
      const row = event.data;
      if (!row || row.key === '__left-empty__') {
        return;
      }

      if (row.nodeType === 'group') {
        if (!row.hasChildren) {
          return;
        }
        setLeftExpanded((prev) => toggleExpandedGroupKeys(prev, row.key));
        return;
      }

      setFilters((prev) =>
        reduceFiltersByNode(prev, { key: row.key, nodeType: row.nodeType as LeafNodeType }),
      );
    },
    [reduceFiltersByNode, setFilters],
  );

  const leftPanel = useMemo(
    () =>
      renderPanel({
        leftTab,
        onLeftTabChange: setLeftTab,
        leftRowsForRender,
        onLeftRowClick,
        hasManualFilterActive,
      }),
    [hasManualFilterActive, leftRowsForRender, leftTab, onLeftRowClick, renderPanel],
  );

  useEffect(() => {
    if (leftPanelWidth != null) {
      setLeftPanelWidth(leftPanelWidth);
    }
    return () => {
      setLeftPanel(undefined);
      setLeftPanelVisible(false);
    };
  }, [leftPanelWidth, setLeftPanel, setLeftPanelVisible, setLeftPanelWidth]);

  useEffect(() => {
    if (typeof setLeftPanelCollapsed === 'function') {
      setLeftPanelCollapsed(false);
    }
    setLeftPanelVisible(true);
    setLeftPanel(leftPanel);
  }, [leftPanel, setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible]);

  return {
    filteredRows,
    hasManualFilterActive,
    filters,
    setFilters,
  };
};
