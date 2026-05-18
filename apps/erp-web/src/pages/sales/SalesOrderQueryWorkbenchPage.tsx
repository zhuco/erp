import '../orders/orderQueryPage.css';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Select, Space, Typography, message } from 'antd';
import type { MenuProps, SelectProps } from 'antd';
import dayjs from 'dayjs';
import type { Dispatch, Key, SetStateAction } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { listCouriers } from '@/api/couriers';
import { useOrderDetail, useOrdersList } from '@/api/hooks';
import {
  createOrderTagDefinition,
  deleteOrderTagDefinition,
  listOrderTagDefinitions,
} from '@/api/orderTagDefinitions';
import { addOrderTags, updateOrder, type OrderListQuery, type OrderSummary } from '@/api/orders';
import { listShipments } from '@/api/shipments';
import { listWarehouses } from '@/api/warehouses';
import {
  WorkbenchPageTemplate,
  WorkbenchToolbar,
  useWorkbenchInteractionController,
  useWorkbenchSavedViewState,
  useWorkbenchSelectionController,
  useWorkbenchTablePagination,
  useWorkbenchViewStateController,
} from '@/components/page-templates';
import { UiModule03ContextMenu } from '@/components/ui-modules';
import { platformOptions, type PlatformCode } from '@/features/platform';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';
import { OrderQueryLeftPanel } from '@/pages/orders/components/OrderQueryLeftPanel';
import {
  buildOrderQueryLeftCheckedKeys,
  hasOrderQueryLeftManualFilter,
  reduceOrderQueryLeftFiltersByNode,
  type OrderQueryLeftFilters,
} from '@/pages/orders/components/orderQueryLeftInteractions';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';
import { OrderReviewBottomPanel } from '@/pages/orders/review-native/OrderReviewBottomPanel';
import { OrderReviewDetailPanel } from '@/pages/orders/review-native/OrderReviewDetailPanel';
import { defaultPageSize, pageSizeOptions } from '@/pages/orders/review-native/helpers';
import type { ColumnFilters as OrderReviewColumnFilters } from '@/pages/orders/review-native/types';
import { buildCourierOptionLabel, resolveCourierOptionValue } from '@/pages/orders/shared/courierDisplay';
import { OrderReviewQueryMainGrid } from '@/pages/orders/shared/OrderReviewGridAdapters';
import {
  resolveOrderWorkbenchDetailItems,
  resolveOrderWorkbenchFeeRows,
  resolveOrderWorkbenchPlatformOrderRows,
  resolveOrderWorkbenchShipmentRows,
} from '@/pages/orders/shared/orderWorkbenchDetailData';
import { toOrderWorkbenchGoodsRows, type OrderWorkbenchGoodsRow } from '@/pages/orders/shared/orderWorkbenchGoodsRows';
import { useLeftFilterModule, type LeftFilterPanelProps } from '@/pages/orders/shared/useLeftFilterModule';

type TimeRangeValue = 'last_7_days' | 'last_30_days' | 'last_90_days';
type DetailSelectOption = NonNullable<SelectProps['options']>;

type SavedViewState = {
  searchInput?: string;
  searchKeyword?: string;
  timeRange?: TimeRangeValue;
  platform?: PlatformCode | 'all';
  leftFilters?: OrderQueryLeftFilters;
  quickFilterExpanded?: boolean;
  columnFilters?: OrderReviewColumnFilters;
};

const timeRangeOptions: Array<{ label: string; value: TimeRangeValue; days: number }> = [
  { label: '近 7 天', value: 'last_7_days', days: 7 },
  { label: '近 30 天', value: 'last_30_days', days: 30 },
  { label: '近 90 天', value: 'last_90_days', days: 90 },
];

const resolveOrderRowKey = (record: OrderSummary | null | undefined) =>
  String(record?.id ?? record?.platform_order_id ?? '').trim() || null;


const buildLeftNodes = (rows: OrderSummary[]): OrderReviewAgLeftNode[] => [
  {
    key: 'query-scope-group',
    label: '订单范围',
    nodeType: 'group',
    count: rows.length,
    children: [{ key: 'all', label: '全部订单', nodeType: 'exception', count: rows.length }],
  },
  {
    key: 'query-channel-group',
    label: '销售渠道',
    nodeType: 'group',
    count: rows.length,
    children: platformOptions.map((option) => ({
      key: `channel-${option.value}`,
      label: option.label,
      nodeType: 'platform' as const,
      count: rows.filter((row) => String(row.platform ?? '').trim().toLowerCase() === option.value).length,
    })),
  },
];

export const SalesOrderQueryWorkbenchPage = () => {
  const tenantId = useTenantId();
  const userId = useSessionStore((state) => state.userId);
  const queryClient = useQueryClient();
  const tableRef = useRef<HTMLDivElement | null>(null);

  const savedViewKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    return `sales.order-query.workbench.saved-view.v1.${tenantId}.${String(userId ?? '').trim() || 'anonymous'}`;
  }, [tenantId, userId]);

  const paginationStorageKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    return `sales.order-query.pagination.v1.${tenantId}.${String(userId ?? '').trim() || 'anonymous'}`;
  }, [tenantId, userId]);

  const workbenchStateKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    return `sales.order-query.workbench-state.v1.${tenantId}.${String(userId ?? '').trim() || 'anonymous'}`;
  }, [tenantId, userId]);

  const { pagination, resetPagination, handlePaginationChange } = useWorkbenchTablePagination({
    defaultPageSize,
    pageSizeOptions,
    storageKey: paginationStorageKey,
  });

  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [timeRange, setTimeRange] = useState<TimeRangeValue>('last_30_days');
  const [platform, setPlatform] = useState<PlatformCode | 'all'>('all');
  const [leftFilters, setLeftFilters] = useState<OrderQueryLeftFilters>({ selectedFilterId: 'all' });
  const [quickFilterExpanded, setQuickFilterExpanded] = useState(true);
  const [columnFilters, setColumnFilters] = useState<OrderReviewColumnFilters>({});
  const [detailWarehouseId, setDetailWarehouseId] = useState<string | undefined>(undefined);
  const [detailLogisticsCompany, setDetailLogisticsCompany] = useState<string | undefined>(undefined);
  const [detailUpdatingField, setDetailUpdatingField] = useState<'warehouse' | 'logistics' | null>(null);
  const [orderTagsApplying, setOrderTagsApplying] = useState(false);

  const range = useMemo(() => {
    const option = timeRangeOptions.find((item) => item.value === timeRange);
    if (!option) {
      return undefined;
    }
    return [dayjs().subtract(option.days, 'day').toISOString(), dayjs().toISOString()] as const;
  }, [timeRange]);

  const query = useMemo<OrderListQuery>(
    () => ({
      keyword: searchKeyword || undefined,
      platform: platform === 'all' ? undefined : platform,
      created_from: range?.[0],
      created_to: range?.[1],
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? defaultPageSize),
      limit: pagination.pageSize ?? defaultPageSize,
    }),
    [pagination.current, pagination.pageSize, platform, range, searchKeyword],
  );

  const ordersQuery = useOrdersList({ query, enabled: Boolean(tenantId) });
  const rows = ordersQuery.data?.items ?? [];
  const total = ordersQuery.data?.total ?? 0;

  const {
    activeRow,
    activeRowKey,
    focusedRowKey,
    selectedRowKeys,
    selectionState,
    setActiveRowKey,
    setFocusedRowKey,
    setSelectedRowKeys,
    setSelectionAnchorKey,
  } = useWorkbenchSelectionController<OrderSummary>({
    rows,
    loading: ordersQuery.isFetching,
    resolveRowKey: resolveOrderRowKey,
  });

  const { hydrateViewState, serializeViewState } = useWorkbenchViewStateController({
    extraState: {
      searchInput,
      searchKeyword,
      timeRange,
      platform,
      leftFilters,
      quickFilterExpanded,
      columnFilters,
    },
    extraStateKeys: [
      'searchInput',
      'searchKeyword',
      'timeRange',
      'platform',
      'leftFilters',
      'quickFilterExpanded',
      'columnFilters',
    ],
    persistSelectionState: false,
    selectionState,
    onHydrateExtraState: (state) => {
      if (typeof state.searchInput === 'string') {
        setSearchInput(state.searchInput);
      }
      if (typeof state.searchKeyword === 'string') {
        setSearchKeyword(state.searchKeyword);
      }
      if (
        state.timeRange === 'last_7_days' ||
        state.timeRange === 'last_30_days' ||
        state.timeRange === 'last_90_days'
      ) {
        setTimeRange(state.timeRange);
      }
      if (
        state.platform === 'all' ||
        platformOptions.some((item) => item.value === state.platform)
      ) {
        setPlatform((state.platform ?? 'all') as PlatformCode | 'all');
      }
      if (state.leftFilters && typeof state.leftFilters === 'object') {
        setLeftFilters(state.leftFilters);
      }
      if (typeof state.quickFilterExpanded === 'boolean') {
        setQuickFilterExpanded(state.quickFilterExpanded);
      }
      if (state.columnFilters && typeof state.columnFilters === 'object') {
        setColumnFilters(state.columnFilters as OrderReviewColumnFilters);
      }
    },
    onHydrateSelectionState: (state) => {
      if (Object.prototype.hasOwnProperty.call(state, 'selectedRowKeys')) {
        setSelectedRowKeys((state.selectedRowKeys ?? []) as string[]);
      }
      if (Object.prototype.hasOwnProperty.call(state, 'activeRowKey')) {
        setActiveRowKey((state.activeRowKey as string | null) ?? null);
      }
      if (Object.prototype.hasOwnProperty.call(state, 'focusedRowKey')) {
        setFocusedRowKey((state.focusedRowKey as string | null) ?? null);
      }
      if (Object.prototype.hasOwnProperty.call(state, 'selectionAnchorKey')) {
        setSelectionAnchorKey((state.selectionAnchorKey as string | null) ?? null);
      }
    },
  });

  useWorkbenchSavedViewState<SavedViewState>({
    storageKey: savedViewKey,
    hydrate: hydrateViewState,
    serialize: serializeViewState,
  });

  const detailQuery = useOrderDetail(activeRowKey ?? undefined, Boolean(activeRowKey));
  const detailData = detailQuery.data;
  const detailOrder = detailData?.order ?? null;
  const detailLoadedId = String(detailOrder?.id ?? activeRowKey ?? '').trim();

  const goodsRows = useMemo<OrderWorkbenchGoodsRow[]>(
    () => toOrderWorkbenchGoodsRows(resolveOrderWorkbenchDetailItems(detailData as Record<string, unknown> | null | undefined)),
    [detailData],
  );

  const logRows = useMemo(
    () =>
      Array.isArray((detailData as Record<string, unknown> | undefined)?.audit_logs)
        ? ((detailData as Record<string, unknown>).audit_logs as Record<string, unknown>[]).map((item, index) => ({
            id: String(item.audit_id ?? item.id ?? index),
            created_at: String(item.created_at ?? ''),
            action: String(item.action ?? '-'),
            operator: String(item.operator_name ?? item.operator_id ?? '系统'),
            detail: String(item.detail ?? item.message ?? '-'),
          }))
        : [],
    [detailData],
  );

  const orderTagDefinitionsQuery = useQuery({
    queryKey: ['order-tag-definitions', tenantId, 'sales-order-query'],
    queryFn: () =>
      listOrderTagDefinitions(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200,
      }),
    enabled: Boolean(tenantId),
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'sales-order-query'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const courierQuery = useQuery({
    queryKey: ['couriers', tenantId, 'sales-order-query'],
    queryFn: () => listCouriers(tenantId, { limit: 200, status: 'ACTIVE' }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const orderTagDefinitions = orderTagDefinitionsQuery.data?.items ?? [];

  const orderTagOptions = useMemo<DetailSelectOption>(
    () =>
      orderTagDefinitions.map((tag) => ({
        value: tag.name,
        label: tag.name,
      })),
    [orderTagDefinitions],
  );

  const shipmentsQuery = useQuery({
    queryKey: ['shipments', tenantId, 'sales-order-query', detailOrder?.platform_order_id ?? ''],
    queryFn: () =>
      listShipments(tenantId, {
        platform_order_id: String(detailOrder?.platform_order_id ?? '').trim(),
        limit: 50,
      }),
    enabled: Boolean(tenantId && String(detailOrder?.platform_order_id ?? '').trim()),
    staleTime: 60_000,
  });

  useEffect(() => {
    const nextWarehouseId = String(detailOrder?.ship_warehouse_id ?? '').trim();
    const nextLogisticsCompany = String(detailOrder?.logistics_company ?? '').trim();
    setDetailWarehouseId(nextWarehouseId || undefined);
    setDetailLogisticsCompany(nextLogisticsCompany || undefined);
  }, [detailOrder?.id, detailOrder?.logistics_company, detailOrder?.ship_warehouse_id]);

  const mergedWarehouseOptions = useMemo<DetailSelectOption>(() => {
    const options = new Map<string, { label: string; value: string }>();
    (warehouseQuery.data?.items ?? []).forEach((warehouse) => {
      const value = String(warehouse.warehouse_id ?? '').trim();
      if (!value) {
        return;
      }
      options.set(value, {
        value,
        label: String(warehouse.name ?? value),
      });
    });
    if (detailWarehouseId && !options.has(detailWarehouseId)) {
      options.set(detailWarehouseId, {
        value: detailWarehouseId,
        label: detailWarehouseId,
      });
    }
    return Array.from(options.values());
  }, [detailWarehouseId, warehouseQuery.data?.items]);

  const mergedCourierOptions = useMemo<DetailSelectOption>(() => {
    const options = new Map<string, { label: string; value: string }>();
    (courierQuery.data?.items ?? []).forEach((courier) => {
      const name = String(courier.name ?? '').trim();
      if (!name) {
        return;
      }
      const code = String(courier.code ?? '').trim();
      const value = resolveCourierOptionValue(name, code);
      if (!value) {
        return;
      }
      options.set(value, {
        value,
        label: buildCourierOptionLabel(name, code),
      });
    });
    if (detailLogisticsCompany && !options.has(detailLogisticsCompany)) {
      options.set(detailLogisticsCompany, {
        value: detailLogisticsCompany,
        label: detailLogisticsCompany,
      });
    }
    return Array.from(options.values());
  }, [courierQuery.data?.items, detailLogisticsCompany]);

  const currentOrderTags = useMemo(
    () =>
      (detailData?.tags ?? [])
        .map((tag) => String(tag.tag ?? '').trim())
        .filter(Boolean),
    [detailData?.tags],
  );

  const platformOrderRows = useMemo(
    () => resolveOrderWorkbenchPlatformOrderRows(detailData as Record<string, unknown> | null | undefined),
    [detailData],
  );

  const shipmentRows = useMemo(
    () =>
      resolveOrderWorkbenchShipmentRows(
        detailData as Record<string, unknown> | null | undefined,
        shipmentsQuery.data?.items ?? [],
      ),
    [detailData, shipmentsQuery.data?.items],
  );

  const feeRows = useMemo(
    () => resolveOrderWorkbenchFeeRows(detailData as Record<string, unknown> | null | undefined),
    [detailData],
  );

  const handleSearch = useCallback(() => {
    setSearchKeyword(searchInput.trim());
    resetPagination();
  }, [resetPagination, searchInput]);

  const handleExport = useCallback(() => {
    message.info('订单查询工作台的导出动作已预留。');
  }, []);

  const handlePrint = useCallback(() => {
    message.info('订单查询工作台的打印动作已预留。');
  }, []);

  const handleHelp = useCallback(() => {
    message.info('这里将展示订单查询工作台的统一帮助说明。');
  }, []);

  const handleMainSelectionChange = useCallback(
    (keys: Key[]) => {
      const normalized = Array.from(
        new Set(
          keys
            .map((key) => String(key ?? '').trim())
            .filter(Boolean),
        ),
      );
      setSelectedRowKeys(normalized);
      setSelectionAnchorKey((current) => {
        if (!normalized.length) {
          return null;
        }
        if (current && normalized.includes(current)) {
          return current;
        }
        return normalized[0] ?? null;
      });
    },
    [setSelectedRowKeys, setSelectionAnchorKey],
  );

  const handleDetailWarehouseChange = useCallback(
    async (nextValue: string) => {
      const nextWarehouseId = String(nextValue ?? '').trim() || undefined;
      if (nextWarehouseId === detailWarehouseId) {
        return;
      }

      const orderId = String(detailOrder?.id ?? activeRow?.id ?? '').trim();
      if (!tenantId || !orderId) {
        setDetailWarehouseId(nextWarehouseId);
        return;
      }

      const previousWarehouseId = detailWarehouseId;
      setDetailWarehouseId(nextWarehouseId);
      setDetailUpdatingField('warehouse');
      try {
        await updateOrder(tenantId, orderId, { ship_warehouse_id: nextWarehouseId });
        message.success('发货仓库已更新');
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['orders', tenantId] }),
          queryClient.invalidateQueries({ queryKey: ['order-detail', tenantId, orderId] }),
          detailQuery.refetch(),
        ]);
      } catch (error) {
        setDetailWarehouseId(previousWarehouseId);
        message.error(error instanceof Error ? error.message : '更新发货仓库失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [activeRow?.id, detailOrder?.id, detailQuery, detailWarehouseId, queryClient, tenantId],
  );

  const handleDetailLogisticsChange = useCallback(
    async (nextValue: string) => {
      const nextLogisticsCompany = String(nextValue ?? '').trim() || undefined;
      if (nextLogisticsCompany === detailLogisticsCompany) {
        return;
      }

      const orderId = String(detailOrder?.id ?? activeRow?.id ?? '').trim();
      if (!tenantId || !orderId) {
        setDetailLogisticsCompany(nextLogisticsCompany);
        return;
      }

      const previousLogisticsCompany = detailLogisticsCompany;
      setDetailLogisticsCompany(nextLogisticsCompany);
      setDetailUpdatingField('logistics');
      try {
        await updateOrder(tenantId, orderId, { logistics_company: nextLogisticsCompany });
        message.success('物流公司已更新');
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['orders', tenantId] }),
          queryClient.invalidateQueries({ queryKey: ['order-detail', tenantId, orderId] }),
          detailQuery.refetch(),
        ]);
      } catch (error) {
        setDetailLogisticsCompany(previousLogisticsCompany);
        message.error(error instanceof Error ? error.message : '更新物流公司失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [activeRow?.id, detailLogisticsCompany, detailOrder?.id, detailQuery, queryClient, tenantId],
  );

  const handleApplyOrderTags = useCallback(
    async (tags: string[]) => {
      const normalizedTags = Array.from(new Set(tags.map((tag) => tag.trim()).filter(Boolean)));

      const orderId = String(detailOrder?.id ?? activeRow?.id ?? '').trim();
      if (!tenantId || !orderId) {
        message.warning('请先选择订单');
        return false;
      }

      setOrderTagsApplying(true);
      try {
        await addOrderTags(tenantId, orderId, normalizedTags);
        message.success(normalizedTags.length > 0 ? '订单记号已应用' : '订单记号已清空');
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['orders', tenantId] }),
          queryClient.invalidateQueries({ queryKey: ['order-detail', tenantId, orderId] }),
          detailQuery.refetch(),
        ]);
        return true;
      } catch (error) {
        message.error(error instanceof Error ? error.message : '应用订单记号失败');
        return false;
      } finally {
        setOrderTagsApplying(false);
      }
    },
    [activeRow?.id, detailOrder?.id, detailQuery, queryClient, tenantId],
  );

  const handleRefreshOrderTagDefinitions = useCallback(async () => {
    if (!tenantId) {
      return;
    }
    await queryClient.invalidateQueries({ queryKey: ['order-tag-definitions', tenantId] });
    await orderTagDefinitionsQuery.refetch();
  }, [orderTagDefinitionsQuery, queryClient, tenantId]);

  const handleCreateOrderTagDefinition = useCallback(
    async (name: string) => {
      const normalized = String(name ?? '').trim();
      if (!tenantId || !normalized) {
        message.warning('请输入记号名称');
        return false;
      }
      try {
        await createOrderTagDefinition(tenantId, { name: normalized });
        message.success('订单记号已创建');
        await handleRefreshOrderTagDefinitions();
        return true;
      } catch (error) {
        message.error(error instanceof Error ? error.message : '创建订单记号失败');
        return false;
      }
    },
    [handleRefreshOrderTagDefinitions, tenantId],
  );

  const handleDeleteOrderTagDefinition = useCallback(
    async (tagId: string) => {
      const normalized = String(tagId ?? '').trim();
      if (!tenantId || !normalized) {
        return false;
      }
      try {
        await deleteOrderTagDefinition(tenantId, normalized);
        message.success('订单记号已删除');
        await handleRefreshOrderTagDefinitions();
        return true;
      } catch (error) {
        message.error(error instanceof Error ? error.message : '删除订单记号失败');
        return false;
      }
    },
    [handleRefreshOrderTagDefinitions, tenantId],
  );

  const {
    closeContextMenu,
    contextMenuState,
    openContextMenu,
  } = useWorkbenchInteractionController<OrderSummary>();

  const handleMainContextMenu = useCallback(
    (record: OrderSummary, event: MouseEvent) => {
      const rowKey = resolveOrderRowKey(record);
      setActiveRowKey(rowKey);
      setFocusedRowKey(rowKey);
      if (rowKey && !selectedRowKeys.some((item) => String(item) === rowKey)) {
        setSelectedRowKeys([rowKey]);
        setSelectionAnchorKey(rowKey);
      }
      openContextMenu(record, event);
    },
    [openContextMenu, selectedRowKeys, setActiveRowKey, setFocusedRowKey, setSelectedRowKeys, setSelectionAnchorKey],
  );

  const contextMenuConfig = useMemo<MenuProps>(
    () => ({
      items: [
        { key: 'copy-order-id', label: '复制订单号' },
        { key: 'clear-selection', label: '清空选择' },
      ],
      onClick: ({ key }) => {
        if (key === 'copy-order-id' && contextMenuState.row) {
          const orderId = resolveOrderRowKey(contextMenuState.row);
          if (orderId && navigator?.clipboard) {
            void navigator.clipboard.writeText(orderId);
          }
          message.success(orderId ? `已复制：${orderId}` : '当前订单没有可复制的编号');
        }
        if (key === 'clear-selection') {
          setSelectedRowKeys([]);
          setSelectionAnchorKey(null);
        }
        closeContextMenu();
      },
    }),
    [closeContextMenu, contextMenuState.row, setSelectedRowKeys, setSelectionAnchorKey],
  );

  const renderLeftPanel = useCallback(
    ({ leftTab, onLeftTabChange, leftRowsForRender, onLeftRowClick, hasManualFilterActive }: LeftFilterPanelProps) => (
      <OrderQueryLeftPanel
        leftTab={leftTab}
        onLeftTabChange={onLeftTabChange}
        leftRowsForRender={leftRowsForRender}
        onLeftRowClick={onLeftRowClick}
        leftStatsItems={[{ key: 'total', label: '全部订单', value: rows.length, color: '#2d4f6c' }]}
        leftStatsPieBackground="conic-gradient(#2d4f6c 0deg 360deg)"
        globalOrderTotal={rows.length}
        isFetching={ordersQuery.isFetching}
        statsScopeText={hasManualFilterActive ? '口径：当前查询结果的左侧筛选' : '口径：当前查询结果'}
      />
    ),
    [ordersQuery.isFetching, rows.length],
  );

  const handleLeftFiltersChange = useCallback<Dispatch<SetStateAction<OrderQueryLeftFilters>>>(
    (action) => {
      setLeftFilters((previous) => (typeof action === 'function' ? action(previous) : action));
      resetPagination();
    },
    [resetPagination],
  );

  const { leftPanel } = useLeftFilterModule<OrderSummary, OrderQueryLeftFilters>({
    rows,
    filters: leftFilters,
    onFiltersChange: handleLeftFiltersChange,
    initialExpandedKeys: ['query-scope-group', 'query-channel-group'],
    emptyRowLabel: '没有可用筛选项',
    hasManualFilter: hasOrderQueryLeftManualFilter,
    filterRows: (sourceRows) => sourceRows,
    buildCheckedKeys: buildOrderQueryLeftCheckedKeys,
    buildLeftNodes,
    reduceFiltersByNode: reduceOrderQueryLeftFiltersByNode,
    renderPanel: renderLeftPanel,
  });

  const rightPanel = useMemo(
    () => (
      <OrderReviewDetailPanel
        detailOrder={detailOrder}
        error={detailQuery.error}
        detailLoadedId={detailLoadedId}
        warehouseOptions={mergedWarehouseOptions}
        courierOptions={mergedCourierOptions}
        detailWarehouseId={detailWarehouseId}
        detailLogisticsCompany={detailLogisticsCompany}
        warehouseUpdating={detailUpdatingField === 'warehouse'}
        logisticsUpdating={detailUpdatingField === 'logistics'}
        onWarehouseChange={handleDetailWarehouseChange}
        onLogisticsChange={handleDetailLogisticsChange}
        orderTagDefinitions={orderTagDefinitions}
        orderTagOptions={orderTagOptions}
        currentOrderTags={currentOrderTags}
        orderTagsApplying={orderTagsApplying}
        onApplyOrderTags={handleApplyOrderTags}
        onCreateOrderTagDefinition={handleCreateOrderTagDefinition}
        onDeleteOrderTagDefinition={handleDeleteOrderTagDefinition}
        onRefreshOrderTagDefinitions={handleRefreshOrderTagDefinitions}
      />
    ),
    [
      currentOrderTags,
      detailLoadedId,
      detailLogisticsCompany,
      detailOrder,
      detailQuery.error,
      detailUpdatingField,
      detailWarehouseId,
      handleApplyOrderTags,
      handleCreateOrderTagDefinition,
      handleDeleteOrderTagDefinition,
      handleDetailLogisticsChange,
      handleDetailWarehouseChange,
      handleRefreshOrderTagDefinitions,
      mergedCourierOptions,
      mergedWarehouseOptions,
      orderTagDefinitions,
      orderTagOptions,
      orderTagsApplying,
    ],
  );

  const rightPanelSyncKey = useMemo(
    () =>
      JSON.stringify({
        detailLoadedId,
        detailOrderId: detailOrder?.id ?? '',
        detailWarehouseId: detailWarehouseId ?? '',
        detailLogisticsCompany: detailLogisticsCompany ?? '',
        orderTagsApplying,
        currentOrderTags,
        orderTagDefinitions: orderTagDefinitions.map((item) => `${item.tag_id}:${item.name}`),
        orderTagOptions: orderTagOptions.map((option) => String(option.value ?? '')),
        warehouseOptions: mergedWarehouseOptions.map((option) => String(option.value ?? '')),
        courierOptions: mergedCourierOptions.map((option) => String(option.value ?? '')),
        detailError:
          detailQuery.error instanceof Error
            ? detailQuery.error.message
            : detailQuery.error
              ? 'error'
              : '',
      }),
    [
      currentOrderTags,
      detailLoadedId,
      detailLogisticsCompany,
      detailOrder?.id,
      detailQuery.error,
      detailWarehouseId,
      mergedCourierOptions,
      mergedWarehouseOptions,
      orderTagDefinitions,
      orderTagOptions,
      orderTagsApplying,
    ],
  );

  const bottomPanel = useMemo(
    () => (
      <OrderReviewBottomPanel
        activeOrderId={detailLoadedId || 'none'}
        goodsRows={goodsRows}
        logRows={logRows}
        platformOrderRows={platformOrderRows}
        shipmentRows={shipmentRows}
        feeRows={feeRows}
        goodsEditable={false}
      />
    ),
    [detailLoadedId, feeRows, goodsRows, logRows, platformOrderRows, shipmentRows],
  );

  const bottomPanelSyncKey = useMemo(
    () =>
      JSON.stringify({
        detailLoadedId,
        goodsRows: goodsRows.map((row) => ({
          key: row.__localKey,
          id: row.id ?? '',
          title: row.title ?? '',
          quantity: row.quantity ?? null,
          priceCent: row.price_cent ?? null,
        })),
        logRows: logRows.map((row, index) => ({
          key: row.id ?? index,
          createdAt: String(row.created_at ?? ''),
          action: String(row.action ?? ''),
          operator: String(row.operator ?? ''),
          detail: String(row.detail ?? ''),
        })),
        platformOrderRows,
        shipmentRows,
        feeRows,
      }),
    [detailLoadedId, feeRows, goodsRows, logRows, platformOrderRows, shipmentRows],
  );

  const toolbar = (
    <WorkbenchToolbar
      className="order-query-toolbar"
      leftContent={
        <Space size={8} wrap>
          <Typography.Text strong>订单查询</Typography.Text>
          <Select
            size="small"
            value={timeRange}
            style={{ width: 120 }}
            options={timeRangeOptions.map((item) => ({ label: item.label, value: item.value }))}
            onChange={(value) => {
              setTimeRange(value as TimeRangeValue);
              resetPagination();
            }}
          />
          <Select
            size="small"
            value={platform}
            style={{ width: 140 }}
            options={[{ label: '全部平台', value: 'all' }, ...platformOptions]}
            onChange={(value) => {
              setPlatform(value as PlatformCode | 'all');
              resetPagination();
            }}
          />
          <Select
            size="small"
            value={quickFilterExpanded ? 'expanded' : 'collapsed'}
            style={{ width: 136 }}
            options={[
              { label: '展开列筛选', value: 'expanded' },
              { label: '收起列筛选', value: 'collapsed' },
            ]}
            onChange={(value) => setQuickFilterExpanded(value === 'expanded')}
          />
        </Space>
      }
      searchValue={searchInput}
      onSearchChange={setSearchInput}
      onSearch={handleSearch}
      searchPlaceholder="搜索系统单号 / 平台单号 / 收件人"
      searchInputClassName="order-query-search"
      searchInputTestId="sales-order-query-search-input"
      searchButtonTestId="sales-order-query-search-trigger"
      searchButtonAriaLabel="查询订单"
      onExport={handleExport}
      onPrint={handlePrint}
      onHelp={handleHelp}
    />
  );

  return (
    <WorkbenchPageTemplate
      pageTestId="sales-order-query-workbench-page"
      className="order-query-page"
      leftPanel={leftPanel}
      toolbar={toolbar}
      tableContainerRef={tableRef}
      tableContent={
        <OrderReviewQueryMainGrid
          rows={rows}
          columnFilters={columnFilters}
          quickFilterExpanded={quickFilterExpanded}
          selectedRowKeys={selectedRowKeys}
          focusedOrderKey={focusedRowKey}
          onColumnFiltersChange={setColumnFilters}
          onSelectionChange={handleMainSelectionChange}
          onRowActive={(record) => {
            const rowKey = resolveOrderRowKey(record);
            setActiveRowKey(rowKey);
            setFocusedRowKey(rowKey);
          }}
          onRowDoubleClick={(record) => {
            const rowKey = resolveOrderRowKey(record);
            setActiveRowKey(rowKey);
            setFocusedRowKey(rowKey);
          }}
          onFocus={(record) => {
            setFocusedRowKey(resolveOrderRowKey(record));
          }}
          onContextMenu={handleMainContextMenu}
          loading={ordersQuery.isFetching}
          error={ordersQuery.error}
          onRetry={() => {
            void ordersQuery.refetch();
          }}
        />
      }
      footerSummary={
        <>
          <span>{`总数 ${total} 条，本页 ${rows.length} 条，已选 ${selectedRowKeys.length} 条`}</span>
          <span>{ordersQuery.isFetching ? '数据刷新中...' : `当前焦点：${focusedRowKey ?? '-'}`}</span>
        </>
      }
      footerSyncKey={`sales-order-query-${total}-${pagination.current ?? 1}-${pagination.pageSize ?? defaultPageSize}`}
      total={total}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
      stateCacheKey={workbenchStateKey}
      showFooterScrollbarRow={true}
      rightPanel={rightPanel}
      rightPanelSyncKey={rightPanelSyncKey}
      bottomPanel={bottomPanel}
      bottomPanelSyncKey={bottomPanelSyncKey}
      overlays={
        <UiModule03ContextMenu
          open={contextMenuState.open}
          position={contextMenuState.position}
          menu={contextMenuConfig}
          onClose={closeContextMenu}
        />
      }
    />
  );
};
