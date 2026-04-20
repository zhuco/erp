import { useCallback, useMemo, type Dispatch, type ReactNode, type SetStateAction } from 'react';

import type { OrderExceptionSummary } from '@/api/orders';
import type { UiModule08OrderReviewLeftStatsItem } from '@/components/ui-modules';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';
import { useLeftFilterModule, type LeftFilterPanelProps } from '@/pages/orders/shared/useLeftFilterModule';

import { OrderExceptionsLeftPanel } from './OrderExceptionsLeftPanel';
import {
  buildOrderExceptionsCheckedKeys,
  filterOrderExceptionsRows,
  hasOrderExceptionsLeftManualFilter,
  resolveExceptionSlaBucket,
  reduceOrderExceptionsFiltersByNode,
  type OrderExceptionsLeftFilters,
} from './orderExceptionsLeftInteractions';

type UseOrderExceptionsLeftModuleOptions = {
  rows: OrderExceptionSummary[];
  currentUserId: string;
  isFetching: boolean;
  overdueThresholdMinutes: number;
  resolveExceptionLabel: (status?: string | null) => string;
  isOverdue: (item: OrderExceptionSummary) => boolean;
  filters?: OrderExceptionsLeftFilters;
  onFiltersChange?: Dispatch<SetStateAction<OrderExceptionsLeftFilters>>;
  onBeforeLeafFilterApply?: () => void;
  screenContent: ReactNode;
  hasScreenFilterActive: boolean;
};

type UseOrderExceptionsLeftModuleResult = {
  filteredRows: OrderExceptionSummary[];
  hasManualLeftFilterActive: boolean;
  leftPanel: ReactNode;
};

export const useOrderExceptionsLeftModule = ({
  rows,
  currentUserId,
  isFetching,
  overdueThresholdMinutes,
  resolveExceptionLabel,
  isOverdue,
  filters,
  onFiltersChange,
  onBeforeLeafFilterApply,
  screenContent,
  hasScreenFilterActive,
}: UseOrderExceptionsLeftModuleOptions): UseOrderExceptionsLeftModuleResult => {
  const nowTs = Date.now();

  const buildLeftNodes = useCallback(
    (sourceRows: OrderExceptionSummary[]): OrderReviewAgLeftNode[] => {
      const statusEntries = Array.from(
        sourceRows.reduce((map, row) => {
          const key = String(row.exception_status ?? '').trim() || 'UNKNOWN';
          map.set(key, (map.get(key) ?? 0) + 1);
          return map;
        }, new Map<string, number>()),
      ).sort((a, b) => b[1] - a[1]);

      const assigneeEntries = Array.from(
        sourceRows.reduce((map, row) => {
          const key = String(row.assignee_name ?? '').trim() || '__unassigned__';
          map.set(key, (map.get(key) ?? 0) + 1);
          return map;
        }, new Map<string, number>()),
      ).sort((a, b) => b[1] - a[1]);

      const warehouseEntries = Array.from(
        sourceRows.reduce((map, row) => {
          const key = String(row.target_warehouse_id ?? '').trim() || '__none__';
          map.set(key, (map.get(key) ?? 0) + 1);
          return map;
        }, new Map<string, number>()),
      ).sort((a, b) => b[1] - a[1]);

      const overdueCount = sourceRows.filter((row) => {
        const bucket = resolveExceptionSlaBucket(row, nowTs, overdueThresholdMinutes);
        return bucket === 'overdue' || (bucket === 'normal' && isOverdue(row));
      }).length;
      const dueSoonCount = sourceRows.filter((row) => {
        return resolveExceptionSlaBucket(row, nowTs, overdueThresholdMinutes) === 'due_soon';
      }).length;
      const mineCount = sourceRows.filter((row) => String(row.assignee_id ?? '').trim() === currentUserId).length;
      const normalCount = Math.max(sourceRows.length - overdueCount - dueSoonCount, 0);

      return [
        {
          key: 'exception-status-group',
          label: '异常状态',
          nodeType: 'group',
          count: sourceRows.length,
          children: [
            { key: 'status:*', label: '全部状态', nodeType: 'status', count: sourceRows.length },
            ...statusEntries.map(([status, count]) => ({
              key: `status:${status}`,
              label: resolveExceptionLabel(status),
              nodeType: 'status' as const,
              count,
            })),
          ],
        },
        {
          key: 'exception-assignee-group',
          label: '责任人',
          nodeType: 'group',
          count: sourceRows.length,
          children: [
            { key: 'assignee:*', label: '全部责任人', nodeType: 'assignee', count: sourceRows.length },
            ...assigneeEntries.map(([assignee, count]) => ({
              key: `assignee:${assignee}`,
              label: assignee === '__unassigned__' ? '未分配' : assignee,
              nodeType: 'assignee' as const,
              count,
            })),
          ],
        },
        {
          key: 'exception-sla-group',
          label: 'SLA',
          nodeType: 'group',
          count: sourceRows.length,
          children: [
            { key: 'sla:*', label: '全部 SLA', nodeType: 'sla', count: sourceRows.length },
            { key: 'sla:overdue', label: '已超时', nodeType: 'sla', count: overdueCount },
            { key: 'sla:due_soon', label: '即将超时', nodeType: 'sla', count: dueSoonCount },
            { key: 'sla:normal', label: '未超时', nodeType: 'sla', count: normalCount },
            { key: 'sla:mine', label: '我的负责', nodeType: 'sla', count: mineCount },
          ],
        },
        {
          key: 'exception-warehouse-group',
          label: '发货仓',
          nodeType: 'group',
          count: sourceRows.length,
          children: [
            { key: 'warehouse:*', label: '全部仓库', nodeType: 'warehouse', count: sourceRows.length },
            ...warehouseEntries.map(([warehouseId, count]) => ({
              key: `warehouse:${warehouseId}`,
              label: warehouseId === '__none__' ? '未指定仓' : warehouseId,
              nodeType: 'warehouse' as const,
              count,
            })),
          ],
        },
      ];
    },
    [currentUserId, isOverdue, nowTs, overdueThresholdMinutes, resolveExceptionLabel],
  );

  const leftStatsItems = useMemo<UiModule08OrderReviewLeftStatsItem[]>(() => {
    const palette = ['#f97316', '#0ea5e9', '#6366f1', '#22c55e', '#64748b'];
    const statusEntries = Array.from(
      rows.reduce((map, row) => {
        const key = String(row.exception_status ?? '').trim() || 'UNKNOWN';
        map.set(key, (map.get(key) ?? 0) + 1);
        return map;
      }, new Map<string, number>()),
    )
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5);

    if (statusEntries.length === 0) {
      return [{ key: 'all', label: '复核订单', value: rows.length, color: palette[0] ?? '#f97316' }];
    }

    return statusEntries.map(([key, value], index) => ({
      key,
      label: resolveExceptionLabel(key),
      value,
      color: palette[index % palette.length] ?? palette[0] ?? '#f97316',
    }));
  }, [resolveExceptionLabel, rows]);

  const leftStatsPieBackground = useMemo(() => {
    if (leftStatsItems.length === 0) {
      return 'conic-gradient(#dce4ee 0deg 360deg)';
    }

    let angle = 0;
    const total = leftStatsItems.reduce((sum, item) => sum + item.value, 0) || 1;
    const stops = leftStatsItems.map((item) => {
      const start = angle;
      const next = angle + (item.value / total) * 360;
      angle = next;
      return `${item.color} ${start.toFixed(2)}deg ${next.toFixed(2)}deg`;
    });

    if (angle < 360) {
      stops.push(`#dce4ee ${angle.toFixed(2)}deg 360deg`);
    }
    return `conic-gradient(${stops.join(', ')})`;
  }, [leftStatsItems]);

  const renderExceptionsLeftPanel = useCallback(
    ({
      leftTab,
      onLeftTabChange,
      leftRowsForRender,
      onLeftRowClick,
      hasManualFilterActive,
    }: LeftFilterPanelProps) => (
      <OrderExceptionsLeftPanel
        leftTab={leftTab}
        onLeftTabChange={onLeftTabChange}
        leftRowsForRender={leftRowsForRender}
        onLeftRowClick={onLeftRowClick}
        leftStatsItems={leftStatsItems}
        leftStatsPieBackground={leftStatsPieBackground}
        globalOrderTotal={rows.length}
        statsScopeText={
          isFetching
            ? '统计更新中...'
            : hasManualFilterActive && hasScreenFilterActive
              ? '口径：筛选条件 + 左侧过滤后'
              : hasManualFilterActive
                ? '口径：左侧过滤后'
                : hasScreenFilterActive
              ? '口径：筛选条件后'
              : '口径：待复核订单'
        }
        screenContent={screenContent}
      />
    ),
    [hasScreenFilterActive, isFetching, leftStatsItems, leftStatsPieBackground, rows.length, screenContent],
  );

  const { filteredRows, hasManualFilterActive, leftPanel } = useLeftFilterModule<OrderExceptionSummary, OrderExceptionsLeftFilters>({
    rows,
    defaultFilters: {},
    filters,
    onFiltersChange,
    initialExpandedKeys: ['exception-status-group'],
    emptyRowLabel: '无匹配项',
    hasManualFilter: hasOrderExceptionsLeftManualFilter,
    filterRows: (sourceRows, nextFilters) =>
      filterOrderExceptionsRows(sourceRows, nextFilters, {
        currentUserId,
        overdueThresholdMinutes,
      }),
    buildCheckedKeys: buildOrderExceptionsCheckedKeys,
    buildLeftNodes,
    reduceFiltersByNode: reduceOrderExceptionsFiltersByNode,
    renderPanel: renderExceptionsLeftPanel,
    onBeforeLeafFilterApply,
  });

  return {
    filteredRows,
    hasManualLeftFilterActive: hasManualFilterActive,
    leftPanel,
  };
};
