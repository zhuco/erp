import './orderReviewPage.css';

import { Button, message } from 'antd';
import type { MenuProps } from 'antd';
import type { GridApi } from 'ag-grid-community';
import type { Key } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import { listCouriers } from '@/api/couriers';
import { useOrdersList } from '@/api/hooks';
import {
  addOrderTags,
  getOrderDetail,
  getOrderReviewStats,
  updateOrder,
  type OrderListQuery,
  type OrderSummary,
} from '@/api/orders';
import { listOrderTagDefinitions } from '@/api/orderTagDefinitions';
import { listShops } from '@/api/shops';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import {
  WorkbenchMainTableTemplate,
  WorkbenchToolbarTemplate,
  useWorkbenchTablePagination,
} from '@/components/workbench/templates';
import {
  UiModule03ContextMenu,
  createUiModule08StrategyScreenFilterState,
  type UiModule08StrategyScreenFilterChangeHandler,
  type UiModule08StrategyScreenFilterState,
} from '@/components/ui-modules';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';
import { useLeftFilterModule, type LeftFilterPanelProps } from '@/pages/orders/shared/useLeftFilterModule';
import { fetchAllOrdersByScan } from '@/pages/orders/shared/fetchAllOrdersByScan';
import { useTenantId } from '@/features/tenant';

import { OrderReviewBottomPanel } from './review-native/OrderReviewBottomPanel';
import { OrderReviewDetailPanel } from './review-native/OrderReviewDetailPanel';
import {
  defaultPageSize,
  moneyValue,
  normalizeMulti,
  pageSizeOptions,
  platformLabel,
  platformLabelToCodeMap,
  resolveOrderActionId,
  resolveOrderRowKey,
} from './review-native/helpers';
import {
  exportOrderReviewGridAsExcel,
  printOrderReviewGrid,
} from './review-native/orderReviewGridActions';
import {
  buildOrderReviewLeftCheckedKeys,
  hasOrderReviewLeftManualFilter,
  reduceOrderReviewFiltersByNode,
} from './review-native/orderReviewLeftInteractions';
import { hasMainGridColumnFilters } from './review-native/OrderReviewGridFilters';
import { OrderReviewLeftPanel } from './review-native/OrderReviewLeftPanel';
import { OrderReviewMainGrid } from './review-native/OrderReviewMainGrid';
import type { ColumnFilters, OrderReviewLeftStatsItem } from './review-native/types';

type ContextMenuState<T> = {
  open: boolean;
  position: { x: number; y: number } | null;
  row: T | null;
};

type DetailOrderPayload = {
  id?: string;
  platform_order_id?: string;
  platform?: string;
  shop_id?: string;
  receiver_name?: string;
  receiver_phone?: string;
  total_amount_cent?: number | null;
  logistics_company?: string;
  ship_warehouse_id?: string;
  items?: Record<string, unknown>[];
};

type GoodsGridRow = {
  __localKey: string;
  id?: string;
  platform_sku_id?: string | null;
  outer_sku_id?: string | null;
  title?: string | null;
  quantity?: number | null;
  price_cent?: number | null;
};

const toGoodsRows = (rows: Record<string, unknown>[]) =>
  rows.map((row, index) => {
    const id = row.id == null ? undefined : String(row.id);
    const fallbackId = `${String(row.platform_sku_id ?? row.outer_sku_id ?? 'goods')}-${index}`;
    const localKey = id ?? fallbackId;
    return {
      __localKey: localKey,
      id,
      platform_sku_id: row.platform_sku_id == null ? null : String(row.platform_sku_id),
      outer_sku_id: row.outer_sku_id == null ? null : String(row.outer_sku_id),
      title: row.title == null ? null : String(row.title),
      quantity: row.quantity == null ? 1 : Number(row.quantity),
      price_cent: row.price_cent == null ? 0 : Number(row.price_cent),
    } as GoodsGridRow;
  });

const DAY_MS = 24 * 60 * 60 * 1000;

const readStringField = (record: OrderSummary, keys: string[]) => {
  const rawRecord = record as Record<string, unknown>;
  for (const key of keys) {
    const raw = rawRecord[key];
    if (raw == null) continue;
    const value = String(raw).trim();
    if (value) {
      return value;
    }
  }
  return '';
};

const readStringListField = (record: OrderSummary, keys: string[]) => {
  const rawRecord = record as Record<string, unknown>;
  for (const key of keys) {
    const raw = rawRecord[key];
    if (Array.isArray(raw)) {
      return raw
        .map((item) => String(item ?? '').trim())
        .filter(Boolean);
    }
    if (typeof raw === 'string') {
      return raw
        .split(',')
        .map((item) => item.trim())
        .filter(Boolean);
    }
  }
  return [];
};

const readBooleanField = (record: OrderSummary, keys: string[]) => {
  const rawRecord = record as Record<string, unknown>;
  for (const key of keys) {
    const raw = rawRecord[key];
    if (typeof raw === 'boolean') {
      return raw;
    }
  }
  return false;
};

const hasAnyField = (record: OrderSummary, keys: string[]) => {
  const rawRecord = record as Record<string, unknown>;
  return keys.some((key) => Object.prototype.hasOwnProperty.call(rawRecord, key));
};

const toTimestamp = (value: unknown) => {
  const timestamp = Date.parse(String(value ?? ''));
  if (Number.isFinite(timestamp)) {
    return timestamp;
  }
  return null;
};

const getOrderExceptionFlags = (record: OrderSummary) =>
  (record.exception_flags ?? []).map((value) => String(value).trim().toUpperCase()).filter(Boolean);

const hasExceptionKeyword = (flags: string[], keywords: string[]) =>
  flags.some((flag) => keywords.some((keyword) => flag.includes(keyword)));

const resolveWarehouseId = (record: OrderSummary) =>
  readStringField(record, ['ship_warehouse_id', 'target_warehouse_id', 'warehouse_id']);

const resolveLogisticsCompany = (record: OrderSummary) =>
  readStringField(record, ['logistics_company', 'carrier_name', 'courier_name']);

const hasRemark = (record: OrderSummary) =>
  Boolean(
    readStringField(record, [
      'seller_remark',
      'buyer_message',
      'extra_remark',
      'remark',
      'freeze_reason',
      'cancel_reason',
      'exception_reason',
    ]),
  );

const hasTag = (record: OrderSummary) => {
  const tags = readStringListField(record, ['tags', 'tag_list', 'marks']);
  if (tags.length > 0) {
    return true;
  }
  return getOrderExceptionFlags(record).length > 0;
};

const matchesTimeRange = (
  record: OrderSummary,
  range: 'today' | 'yesterday' | 'within3d' | 'within7d' | 'within30d' | 'gt30d',
  now: Date,
) => {
  const createdAtMs = toTimestamp(record.created_at);
  if (createdAtMs == null) {
    return false;
  }
  const nowMs = now.getTime();
  const todayStart = new Date(now);
  todayStart.setHours(0, 0, 0, 0);
  const todayStartMs = todayStart.getTime();
  const yesterdayStartMs = todayStartMs - DAY_MS;
  if (range === 'today') {
    return createdAtMs >= todayStartMs && createdAtMs <= nowMs;
  }
  if (range === 'yesterday') {
    return createdAtMs >= yesterdayStartMs && createdAtMs < todayStartMs;
  }
  if (range === 'within3d') {
    return createdAtMs >= nowMs - 3 * DAY_MS;
  }
  if (range === 'within7d') {
    return createdAtMs >= nowMs - 7 * DAY_MS;
  }
  if (range === 'within30d') {
    return createdAtMs >= nowMs - 30 * DAY_MS;
  }
  return createdAtMs < nowMs - 30 * DAY_MS;
};

const isFrozenOrder = (record: OrderSummary) => {
  const flags = getOrderExceptionFlags(record);
  return (
    hasExceptionKeyword(flags, ['ORDER_FROZEN', 'FROZEN', 'FREEZE']) ||
    Boolean(readStringField(record, ['freeze_reason']))
  );
};

const isNoLogisticsOrder = (record: OrderSummary) => {
  if (!hasAnyField(record, ['logistics_company', 'carrier_name', 'courier_name', 'tracking_no'])) {
    return false;
  }
  const logisticsCompany = resolveLogisticsCompany(record);
  const trackingNo = readStringField(record, ['tracking_no']);
  return !logisticsCompany && !trackingNo;
};

const isStopShippingOrder = (record: OrderSummary) => {
  const flags = getOrderExceptionFlags(record);
  return (
    readBooleanField(record, ['shipping_failed']) ||
    hasExceptionKeyword(flags, ['SHIPPING_FAILED', 'STOP_SHIPPING', 'STOPPED'])
  );
};

const isInventoryShortageOrder = (record: OrderSummary) => {
  const flags = getOrderExceptionFlags(record);
  return (
    readBooleanField(record, ['inventory_shortage']) ||
    hasExceptionKeyword(flags, ['INVENTORY_SHORTAGE', 'SHORTAGE', 'OUT_OF_STOCK'])
  );
};

const isInvalidGoodsOrder = (record: OrderSummary) => {
  const flags = getOrderExceptionFlags(record);
  return (
    readBooleanField(record, ['mapping_missing']) ||
    hasExceptionKeyword(flags, ['MAPPING_MISSING', 'INVALID_GOODS', 'INVALID_ITEM', 'MISSING_SKU'])
  );
};

const isRefundReturnOrder = (record: OrderSummary) => {
  const flags = getOrderExceptionFlags(record);
  return hasExceptionKeyword(flags, ['REFUND', 'RETURN', 'AFTERSALE', 'AFTER_SALE']);
};

const isAbnormalOrder = (record: OrderSummary) => {
  if (isFrozenOrder(record)) return true;
  if (isNoLogisticsOrder(record)) return true;
  if (isStopShippingOrder(record)) return true;
  if (isInventoryShortageOrder(record)) return true;
  if (isInvalidGoodsOrder(record)) return true;
  if (isRefundReturnOrder(record)) return true;
  if (getOrderExceptionFlags(record).length > 0) return true;
  return String(record.review_status ?? '').trim().toUpperCase() === 'REJECTED';
};

const isNormalOrder = (record: OrderSummary) => !isAbnormalOrder(record);

const matchesExceptionType = (
  record: OrderSummary,
  exceptionType: 'frozen' | 'noLogistics' | 'stopShipping' | 'inventoryShortage' | 'invalidGoods' | 'refundReturn',
) => {
  if (exceptionType === 'frozen') return isFrozenOrder(record);
  if (exceptionType === 'noLogistics') return isNoLogisticsOrder(record);
  if (exceptionType === 'stopShipping') return isStopShippingOrder(record);
  if (exceptionType === 'inventoryShortage') return isInventoryShortageOrder(record);
  if (exceptionType === 'invalidGoods') return isInvalidGoodsOrder(record);
  return isRefundReturnOrder(record);
};

const orderTimeOptions: Array<{
  key: 'today' | 'yesterday' | 'within3d' | 'within7d' | 'within30d' | 'gt30d';
  label: string;
}> = [
  { key: 'today', label: '今天' },
  { key: 'yesterday', label: '昨天' },
  { key: 'within3d', label: '3天内' },
  { key: 'within7d', label: '7天内' },
  { key: 'within30d', label: '30天内' },
  { key: 'gt30d', label: '30天以上' },
];

const exceptionTypeOptions: Array<{
  key: 'frozen' | 'noLogistics' | 'stopShipping' | 'inventoryShortage' | 'invalidGoods' | 'refundReturn';
  label: string;
}> = [
  { key: 'frozen', label: '冻结' },
  { key: 'noLogistics', label: '无物流' },
  { key: 'stopShipping', label: '停发' },
  { key: 'inventoryShortage', label: '库存不足' },
  { key: 'invalidGoods', label: '含无效货品' },
  { key: 'refundReturn', label: '有退款退货' },
];

export const OrderReviewPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const {
    setRightPanel,
    setRightPanelVisible,
    setBottomPanel,
    setBottomPanelVisible,
  } = useWorkbenchPanels();

  const { pagination, resetPagination, handlePaginationChange } = useWorkbenchTablePagination({
    defaultPageSize,
    pageSizeOptions,
  });
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [quickFilterExpanded, setQuickFilterExpanded] = useState(true);
  const [columnFilters, setColumnFilters] = useState<ColumnFilters>({});
  const hasManualLeftFilterActive = hasOrderReviewLeftManualFilter(columnFilters);
  const hasMainGridColumnFilterActive = hasMainGridColumnFilters(columnFilters);
  const hasFullScopeColumnFilterActive = hasManualLeftFilterActive || hasMainGridColumnFilterActive;

  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [focusedOrderKey, setFocusedOrderKey] = useState<string | null>(null);
  const [selectionAnchorKey, setSelectionAnchorKey] = useState<string | null>(null);
  const [activeOrder, setActiveOrder] = useState<OrderSummary | null>(null);

  const [leftScreenFiltersDraft, setLeftScreenFiltersDraft] = useState<UiModule08StrategyScreenFilterState>(() =>
    createUiModule08StrategyScreenFilterState(),
  );
  const [leftScreenFiltersApplied, setLeftScreenFiltersApplied] = useState<UiModule08StrategyScreenFilterState>(() =>
    createUiModule08StrategyScreenFilterState(),
  );

  const [mainContextMenu, setMainContextMenu] = useState<ContextMenuState<OrderSummary>>({
    open: false,
    position: null,
    row: null,
  });

  const tableRef = useRef<HTMLDivElement | null>(null);
  const mainGridApiRef = useRef<GridApi<OrderSummary> | null>(null);
  const fetchStartedAtRef = useRef<number | null>(null);
  const [lastFetchDurationMs, setLastFetchDurationMs] = useState<number | null>(null);
  const [loadCycle, setLoadCycle] = useState(0);

  const { data: shopData } = useQuery({
    queryKey: ['shops', tenantId, 'orders-review-native'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const shopOptions = useMemo(
    () => shopData?.items?.map((shop) => ({ label: shop.shop_name ?? shop.shop_id, value: shop.shop_id })) ?? [],
    [shopData?.items],
  );

  const { data: warehouseData } = useQuery({
    queryKey: ['warehouses', tenantId, 'orders-review-native'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const warehouseOptions = useMemo(
    () => warehouseData?.items?.map((warehouse) => ({ label: warehouse.name, value: warehouse.warehouse_id })) ?? [],
    [warehouseData?.items],
  );

  const { data: courierData } = useQuery({
    queryKey: ['couriers', tenantId, 'orders-review-native'],
    queryFn: () => listCouriers(tenantId, { limit: 200, status: 'ACTIVE' }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const courierOptions = useMemo(
    () =>
      courierData?.items?.map((courier) => ({
        label: `${courier.name} (${courier.code})`,
        value: courier.name,
      })) ?? [],
    [courierData?.items],
  );

  const orderTagDefinitionsQuery = useQuery({
    queryKey: ['order-tag-definitions', tenantId, 'orders-review-native'],
    queryFn: () =>
      listOrderTagDefinitions(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200,
      }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const orderTagOptions = useMemo(
    () =>
      (orderTagDefinitionsQuery.data?.items ?? []).map((tag) => ({
        label: tag.name,
        value: tag.name,
      })),
    [orderTagDefinitionsQuery.data?.items],
  );

  const baseQuery = useMemo<OrderListQuery>(() => {
    const rawSize = Number(pagination.pageSize ?? defaultPageSize);
    const pageSize = Number.isFinite(rawSize) && rawSize > 0 ? rawSize : defaultPageSize;
    const current = Number(pagination.current ?? 1);
    return {
      keyword: searchKeyword.trim() || undefined,
      created_from: leftScreenFiltersApplied.createdFrom.trim() || undefined,
      created_to: leftScreenFiltersApplied.createdTo.trim() || undefined,
      item_keyword: leftScreenFiltersApplied.includeSkuKeyword.trim() || undefined,
      order_source: leftScreenFiltersApplied.orderType.trim() || undefined,
      ship_warehouse_id: leftScreenFiltersApplied.warehouse.trim() || undefined,
      receiver_address:
        leftScreenFiltersApplied.regionScope === 'all'
          ? leftScreenFiltersApplied.regionKeyword.trim() || undefined
          : undefined,
      offset: (current - 1) * pageSize,
      limit: pageSize,
    };
  }, [
    leftScreenFiltersApplied.createdFrom,
    leftScreenFiltersApplied.createdTo,
    leftScreenFiltersApplied.includeSkuKeyword,
    leftScreenFiltersApplied.orderType,
    leftScreenFiltersApplied.regionKeyword,
    leftScreenFiltersApplied.regionScope,
    leftScreenFiltersApplied.warehouse,
    pagination.current,
    pagination.pageSize,
    searchKeyword,
  ]);

  const statsScopeQuery = useMemo<OrderListQuery>(
    () => ({
      keyword: searchKeyword.trim() || undefined,
      created_from: leftScreenFiltersApplied.createdFrom.trim() || undefined,
      created_to: leftScreenFiltersApplied.createdTo.trim() || undefined,
      item_keyword: leftScreenFiltersApplied.includeSkuKeyword.trim() || undefined,
      order_source: leftScreenFiltersApplied.orderType.trim() || undefined,
      ship_warehouse_id: leftScreenFiltersApplied.warehouse.trim() || undefined,
      receiver_address:
        leftScreenFiltersApplied.regionScope === 'all'
          ? leftScreenFiltersApplied.regionKeyword.trim() || undefined
          : undefined,
    }),
    [
      leftScreenFiltersApplied.createdFrom,
      leftScreenFiltersApplied.createdTo,
      leftScreenFiltersApplied.includeSkuKeyword,
      leftScreenFiltersApplied.orderType,
      leftScreenFiltersApplied.regionKeyword,
      leftScreenFiltersApplied.regionScope,
      leftScreenFiltersApplied.warehouse,
      searchKeyword,
    ],
  );

  const reviewQuery = useMemo<OrderListQuery>(() => ({ ...baseQuery, review_status: 'PENDING' }), [baseQuery]);

  const {
    data: reviewData,
    isFetching: isReviewFetching,
    error: reviewError,
    refetch: refetchReview,
  } = useOrdersList({ query: reviewQuery, enabled: Boolean(tenantId) });

  const noManualFilter =
    !hasManualLeftFilterActive &&
    !searchKeyword.trim() &&
    !leftScreenFiltersApplied.createdFrom.trim() &&
    !leftScreenFiltersApplied.createdTo.trim() &&
    !leftScreenFiltersApplied.includeSkuKeyword.trim() &&
    !leftScreenFiltersApplied.orderType.trim() &&
    !leftScreenFiltersApplied.regionKeyword.trim() &&
    !leftScreenFiltersApplied.warehouse.trim();

  const shouldFallbackToAll =
    noManualFilter && !isReviewFetching && !reviewError && (reviewData?.total ?? 0) === 0;

  const {
    data: allData,
    isFetching: isAllFetching,
    error: allError,
    refetch: refetchAll,
  } = useOrdersList({ query: baseQuery, enabled: Boolean(tenantId) && shouldFallbackToAll });

  const data = shouldFallbackToAll ? allData : reviewData;

  const statsQuery = useMemo<OrderListQuery>(
    () =>
      shouldFallbackToAll
        ? { ...statsScopeQuery }
        : {
            ...statsScopeQuery,
            review_status: 'PENDING',
          },
    [shouldFallbackToAll, statsScopeQuery],
  );

  const fullScopeScanQuery = useMemo<OrderListQuery>(() => ({ ...statsScopeQuery }), [statsScopeQuery]);
  const {
    data: manualFilterScopeRows,
    isFetching: isManualFilterScopeFetching,
    error: manualFilterScopeError,
    refetch: refetchManualFilterScope,
  } = useQuery({
    queryKey: ['orders-review-native', tenantId, 'left-filter-scan-all', fullScopeScanQuery],
    queryFn: ({ signal }) => fetchAllOrdersByScan(tenantId, fullScopeScanQuery, { signal }),
    enabled: Boolean(tenantId),
    staleTime: 20_000,
  });

  const isFetching =
    isReviewFetching ||
    (shouldFallbackToAll && isAllFetching) ||
    (hasFullScopeColumnFilterActive && isManualFilterScopeFetching);
  const baseError = reviewError ?? (shouldFallbackToAll ? allError : null);
  const error = hasFullScopeColumnFilterActive ? manualFilterScopeError ?? baseError : baseError;
  const refetch = useCallback(async () => {
    const first = await refetchReview();
    if (shouldFallbackToAll) {
      await refetchAll();
    }
    if (hasFullScopeColumnFilterActive) {
      await refetchManualFilterScope();
    }
    return first;
  }, [hasFullScopeColumnFilterActive, refetchAll, refetchManualFilterScope, refetchReview, shouldFallbackToAll]);

  const { data: globalStatsData, isFetching: isGlobalStatsFetching } = useQuery({
    queryKey: ['orders-review-native', tenantId, 'global-stats', statsQuery],
    queryFn: ({ signal }) => getOrderReviewStats(tenantId, statsQuery, { signal }),
    enabled: Boolean(tenantId),
    staleTime: 20_000,
  });

  useEffect(() => {
    const now = () =>
      typeof performance !== 'undefined' && typeof performance.now === 'function' ? performance.now() : Date.now();
    if (isFetching) {
      if (fetchStartedAtRef.current == null) {
        fetchStartedAtRef.current = now();
      }
      return;
    }
    if (fetchStartedAtRef.current == null) {
      return;
    }
    const elapsed = Math.max(0, Math.round(now() - fetchStartedAtRef.current));
    fetchStartedAtRef.current = null;
    setLastFetchDurationMs(elapsed);
    setLoadCycle((previous) => previous + 1);
  }, [isFetching]);

  const serverRows = data?.items ?? [];
  const sourceRowsForFilter = hasFullScopeColumnFilterActive ? (manualFilterScopeRows ?? []) : serverRows;

  const filteredRows = useMemo(() => {
    const selectedPlatformSet = new Set(normalizeMulti(columnFilters.platform));
    const selectedShopSet = new Set(normalizeMulti(columnFilters.shopId));
    const orderIdKeyword = String(columnFilters.orderIdKeyword ?? '').trim().toLowerCase();
    const platformOrderIdKeyword = String(columnFilters.platformOrderIdKeyword ?? '').trim().toLowerCase();
    const receiverNameKeyword = String(columnFilters.receiverNameKeyword ?? '').trim().toLowerCase();
    const receiverPhoneKeyword = String(columnFilters.receiverPhoneKeyword ?? '').trim().toLowerCase();
    const createdAtFrom = String(columnFilters.createdAtFrom ?? '').trim();
    const createdAtTo = String(columnFilters.createdAtTo ?? '').trim();
    const createdAtFromMs = createdAtFrom ? toTimestamp(`${createdAtFrom}T00:00:00`) : null;
    const createdAtToMs = createdAtTo ? toTimestamp(`${createdAtTo}T23:59:59.999`) : null;
    const selectedStatusSet = new Set(normalizeMulti(columnFilters.status));
    const selectedTimeSet = new Set(normalizeMulti(columnFilters.timeRange));
    const selectedExceptionSet = new Set(normalizeMulti(columnFilters.exceptionType));
    const selectedWarehouseSet = new Set(normalizeMulti(columnFilters.warehouseId));
    const selectedLogisticsSet = new Set(normalizeMulti(columnFilters.logisticsCompany));
    const selectedRemarkSet = new Set(normalizeMulti(columnFilters.remarkState));
    const selectedTagSet = new Set(normalizeMulti(columnFilters.tagState));
    const source = sourceRowsForFilter;
    if (
      selectedPlatformSet.size === 0 &&
      selectedShopSet.size === 0 &&
      !orderIdKeyword &&
      !platformOrderIdKeyword &&
      !receiverNameKeyword &&
      !receiverPhoneKeyword &&
      createdAtFromMs == null &&
      createdAtToMs == null &&
      selectedStatusSet.size === 0 &&
      selectedTimeSet.size === 0 &&
      selectedExceptionSet.size === 0 &&
      selectedWarehouseSet.size === 0 &&
      selectedLogisticsSet.size === 0 &&
      selectedRemarkSet.size === 0 &&
      selectedTagSet.size === 0
    ) {
      return source;
    }
    const now = new Date();
    return source.filter((record) => {
      if (orderIdKeyword) {
        const currentOrderId = String(record.id ?? '').trim().toLowerCase();
        if (!currentOrderId.includes(orderIdKeyword)) {
          return false;
        }
      }
      if (platformOrderIdKeyword) {
        const currentPlatformOrderId = String(record.platform_order_id ?? '').trim().toLowerCase();
        if (!currentPlatformOrderId.includes(platformOrderIdKeyword)) {
          return false;
        }
      }
      if (receiverNameKeyword) {
        const currentReceiverName = String(record.receiver_name ?? '').trim().toLowerCase();
        if (!currentReceiverName.includes(receiverNameKeyword)) {
          return false;
        }
      }
      if (receiverPhoneKeyword) {
        const currentReceiverPhone = String(record.receiver_phone ?? '').trim().toLowerCase();
        if (!currentReceiverPhone.includes(receiverPhoneKeyword)) {
          return false;
        }
      }
      if (createdAtFromMs != null || createdAtToMs != null) {
        const currentCreatedAtMs = toTimestamp(record.created_at);
        if (currentCreatedAtMs == null) {
          return false;
        }
        if (createdAtFromMs != null && currentCreatedAtMs < createdAtFromMs) {
          return false;
        }
        if (createdAtToMs != null && currentCreatedAtMs > createdAtToMs) {
          return false;
        }
      }
      if (selectedPlatformSet.size > 0) {
        const platform = String(record.platform ?? '').trim();
        if (!selectedPlatformSet.has(platform)) {
          return false;
        }
      }
      if (selectedShopSet.size > 0) {
        const shopId = String(record.shop_id ?? '').trim();
        if (!selectedShopSet.has(shopId)) {
          return false;
        }
      }
      if (selectedStatusSet.size > 0) {
        const matchesStatus =
          (selectedStatusSet.has('normal') && isNormalOrder(record)) ||
          (selectedStatusSet.has('exception') && isAbnormalOrder(record));
        if (!matchesStatus) {
          return false;
        }
      }
      if (selectedTimeSet.size > 0) {
        const matchesAnyTime = Array.from(selectedTimeSet).some((timeRange) =>
          matchesTimeRange(
            record,
            timeRange as 'today' | 'yesterday' | 'within3d' | 'within7d' | 'within30d' | 'gt30d',
            now,
          ),
        );
        if (!matchesAnyTime) {
          return false;
        }
      }
      if (selectedExceptionSet.size > 0) {
        const matchesAnyException = Array.from(selectedExceptionSet).some((exceptionType) =>
          matchesExceptionType(
            record,
            exceptionType as 'frozen' | 'noLogistics' | 'stopShipping' | 'inventoryShortage' | 'invalidGoods' | 'refundReturn',
          ),
        );
        if (!matchesAnyException) {
          return false;
        }
      }
      if (selectedWarehouseSet.size > 0) {
        const warehouseId = resolveWarehouseId(record);
        if (!selectedWarehouseSet.has(warehouseId)) {
          return false;
        }
      }
      if (selectedLogisticsSet.size > 0) {
        const logisticsCompany = resolveLogisticsCompany(record);
        if (!selectedLogisticsSet.has(logisticsCompany)) {
          return false;
        }
      }
      if (selectedRemarkSet.size > 0) {
        const currentHasRemark = hasRemark(record);
        const matchesRemark =
          (currentHasRemark && selectedRemarkSet.has('has')) || (!currentHasRemark && selectedRemarkSet.has('none'));
        if (!matchesRemark) {
          return false;
        }
      }
      if (selectedTagSet.size > 0) {
        const currentHasTag = hasTag(record);
        const matchesTag = (currentHasTag && selectedTagSet.has('has')) || (!currentHasTag && selectedTagSet.has('none'));
        if (!matchesTag) {
          return false;
        }
      }
      return true;
    });
  }, [
    columnFilters.createdAtFrom,
    columnFilters.createdAtTo,
    columnFilters.exceptionType,
    columnFilters.logisticsCompany,
    columnFilters.orderIdKeyword,
    columnFilters.platform,
    columnFilters.platformOrderIdKeyword,
    columnFilters.remarkState,
    columnFilters.receiverNameKeyword,
    columnFilters.receiverPhoneKeyword,
    columnFilters.shopId,
    columnFilters.status,
    columnFilters.tagState,
    columnFilters.timeRange,
    columnFilters.warehouseId,
    sourceRowsForFilter,
  ]);

  const rows = useMemo(() => {
    if (!hasFullScopeColumnFilterActive) {
      return filteredRows;
    }
    const rawSize = Number(pagination.pageSize ?? defaultPageSize);
    const pageSize = Number.isFinite(rawSize) && rawSize > 0 ? rawSize : defaultPageSize;
    const current = Number(pagination.current ?? 1);
    const start = Math.max(0, (current - 1) * pageSize);
    const end = start + pageSize;
    return filteredRows.slice(start, end);
  }, [filteredRows, hasFullScopeColumnFilterActive, pagination.current, pagination.pageSize]);

  const total = useMemo(() => {
    if (hasFullScopeColumnFilterActive) {
      return filteredRows.length;
    }
    return data?.total ?? rows.length;
  }, [data?.total, filteredRows.length, hasFullScopeColumnFilterActive, rows.length]);

  useEffect(() => {
    if (!rows.length) {
      setActiveOrder(null);
      setFocusedOrderKey(null);
      return;
    }
    const firstKey = resolveOrderRowKey(rows[0]) || null;
    if (!activeOrder) {
      setActiveOrder(rows[0]);
      if (!focusedOrderKey) {
        setFocusedOrderKey(firstKey);
      }
    } else {
      const activeKey = resolveOrderRowKey(activeOrder);
      const exists = Boolean(activeKey) && rows.some((row) => resolveOrderRowKey(row) === activeKey);
      if (!exists) {
        setActiveOrder(rows[0]);
      }
    }
    if (focusedOrderKey) {
      const focusExists = rows.some((row) => resolveOrderRowKey(row) === focusedOrderKey);
      if (!focusExists) {
        setFocusedOrderKey(firstKey);
      }
      return;
    }
    if (firstKey) {
      setFocusedOrderKey(firstKey);
    }
  }, [activeOrder, focusedOrderKey, rows]);

  useEffect(() => {
    if (!selectionAnchorKey) return;
    const exists = rows.some((record) => resolveOrderRowKey(record) === selectionAnchorKey);
    if (!exists) {
      setSelectionAnchorKey(null);
    }
  }, [rows, selectionAnchorKey]);

  const activeDetailOrderId = useMemo(() => (activeOrder ? resolveOrderActionId(activeOrder) : ''), [activeOrder]);

  const detailQuery = useQuery({
    queryKey: ['orders-review-native', tenantId, 'detail', activeDetailOrderId],
    queryFn: ({ signal }) => getOrderDetail(tenantId, activeDetailOrderId, { signal }),
    enabled: Boolean(tenantId && activeDetailOrderId),
    staleTime: 20_000,
    gcTime: 5 * 60_000,
  });

  useEffect(() => {
    if (!tenantId || !activeDetailOrderId || !rows.length) {
      return;
    }
    const currentIndex = rows.findIndex((record) => resolveOrderActionId(record) === activeDetailOrderId);
    if (currentIndex < 0) {
      return;
    }
    const neighborIndices = [currentIndex - 1, currentIndex + 1].filter(
      (index) => index >= 0 && index < rows.length,
    );
    neighborIndices.forEach((index) => {
      const record = rows[index];
      if (!record) return;
      const orderId = resolveOrderActionId(record);
      if (!orderId) return;
      void queryClient.prefetchQuery({
        queryKey: ['orders-review-native', tenantId, 'detail', orderId],
        queryFn: ({ signal }) => getOrderDetail(tenantId, orderId, { signal }),
        staleTime: 20_000,
        gcTime: 5 * 60_000,
      });
    });
  }, [activeDetailOrderId, queryClient, rows, tenantId]);

  const detailOrder = (detailQuery.data?.order ?? null) as DetailOrderPayload | null;
  const goodsRows = useMemo(() => toGoodsRows((detailOrder?.items ?? []) as Record<string, unknown>[]), [detailOrder]);
  const logRows = useMemo(() => (detailQuery.data?.audit_logs ?? []) as Record<string, unknown>[], [detailQuery.data]);
  const currentOrderTags = useMemo(
    () =>
      ((detailQuery.data as { tags?: Array<{ tag?: string | null }> } | undefined)?.tags ?? [])
        .map((item) => String(item.tag ?? '').trim())
        .filter(Boolean),
    [detailQuery.data],
  );
  const [orderTagsApplying, setOrderTagsApplying] = useState(false);
  const [goodsRowsState, setGoodsRowsState] = useState<GoodsGridRow[]>([]);
  const [goodsSaving, setGoodsSaving] = useState(false);

  useEffect(() => {
    setGoodsRowsState(goodsRows);
  }, [activeDetailOrderId, goodsRows]);

  const [detailLoadedId, setDetailLoadedId] = useState('');
  useEffect(() => {
    const loadedId = String(detailOrder?.id ?? '');
    if (loadedId) {
      setDetailLoadedId(loadedId);
      return;
    }
    if (activeDetailOrderId) {
      setDetailLoadedId(activeDetailOrderId);
    }
  }, [activeDetailOrderId, detailOrder?.id]);

  const [detailWarehouseId, setDetailWarehouseId] = useState<string | undefined>(undefined);
  const [detailLogisticsCompany, setDetailLogisticsCompany] = useState<string | undefined>(undefined);
  const [detailUpdatingField, setDetailUpdatingField] = useState<'warehouse' | 'logistics' | null>(null);

  useEffect(() => {
    const nextWarehouseId = String(detailOrder?.ship_warehouse_id ?? '').trim();
    const nextLogisticsCompany = String(detailOrder?.logistics_company ?? '').trim();
    setDetailWarehouseId(nextWarehouseId || undefined);
    setDetailLogisticsCompany(nextLogisticsCompany || undefined);
  }, [detailOrder?.id, detailOrder?.logistics_company, detailOrder?.ship_warehouse_id]);

  const mergedWarehouseOptions = useMemo(() => {
    const map = new Map<string, { label: string; value: string }>();
    warehouseOptions.forEach((option) => {
      const value = String(option.value ?? '').trim();
      if (!value) return;
      map.set(value, {
        label: typeof option.label === 'string' ? option.label : value,
        value,
      });
    });
    if (detailWarehouseId && !map.has(detailWarehouseId)) {
      map.set(detailWarehouseId, { label: detailWarehouseId, value: detailWarehouseId });
    }
    return Array.from(map.values());
  }, [detailWarehouseId, warehouseOptions]);

  const mergedCourierOptions = useMemo(() => {
    const map = new Map<string, { label: string; value: string }>();
    courierOptions.forEach((option) => {
      const value = String(option.value ?? '').trim();
      if (!value) return;
      map.set(value, {
        label: typeof option.label === 'string' ? option.label : value,
        value,
      });
    });
    if (detailLogisticsCompany && !map.has(detailLogisticsCompany)) {
      map.set(detailLogisticsCompany, { label: detailLogisticsCompany, value: detailLogisticsCompany });
    }
    return Array.from(map.values());
  }, [courierOptions, detailLogisticsCompany]);

  const handleDetailWarehouseChange = useCallback(
    async (nextValue: string) => {
      const nextWarehouseId = String(nextValue ?? '').trim() || undefined;
      if (nextWarehouseId === detailWarehouseId) {
        return;
      }

      const orderId = String(detailOrder?.id ?? '').trim();
      if (!tenantId || !orderId) {
        setDetailWarehouseId(nextWarehouseId);
        return;
      }

      const previousWarehouseId = detailWarehouseId;
      setDetailWarehouseId(nextWarehouseId);
      setDetailUpdatingField('warehouse');
      try {
        await updateOrder(tenantId, orderId, {
          ship_warehouse_id: nextWarehouseId,
        });
        message.success('发货仓库已更新');
        await Promise.all([refetch(), detailQuery.refetch()]);
      } catch (error) {
        setDetailWarehouseId(previousWarehouseId);
        message.error(error instanceof Error ? error.message : '更新发货仓库失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [detailOrder?.id, detailQuery, detailWarehouseId, refetch, tenantId],
  );

  const handleDetailLogisticsChange = useCallback(
    async (nextValue: string) => {
      const normalized = String(nextValue ?? '').trim();
      const nextLogistics = normalized || undefined;
      if (nextLogistics === detailLogisticsCompany) {
        return;
      }

      const orderId = String(detailOrder?.id ?? '').trim();
      if (!tenantId || !orderId) {
        setDetailLogisticsCompany(nextLogistics);
        return;
      }

      const previousLogistics = detailLogisticsCompany;
      setDetailLogisticsCompany(nextLogistics);
      setDetailUpdatingField('logistics');
      try {
        await updateOrder(tenantId, orderId, {
          logistics_company: nextLogistics,
        });
        message.success('物流公司已更新');
        await Promise.all([refetch(), detailQuery.refetch()]);
      } catch (error) {
        setDetailLogisticsCompany(previousLogistics);
        message.error(error instanceof Error ? error.message : '更新物流公司失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [detailLogisticsCompany, detailOrder?.id, detailQuery, refetch, tenantId],
  );

  const handleApplyOrderTags = useCallback(
    async (tags: string[]) => {
      const normalizedTags = Array.from(new Set(tags.map((item) => item.trim()).filter(Boolean)));
      if (!normalizedTags.length) {
        message.warning('请至少选择或输入一个订单记号');
        return false;
      }

      const orderId = String(detailOrder?.id ?? '').trim();
      if (!tenantId || !orderId) {
        message.warning('请先选中订单');
        return false;
      }

      setOrderTagsApplying(true);
      try {
        await addOrderTags(tenantId, orderId, normalizedTags);
        message.success('订单记号已应用');
        await Promise.all([refetch(), detailQuery.refetch()]);
        return true;
      } catch (error) {
        message.error(error instanceof Error ? error.message : '应用订单记号失败');
        return false;
      } finally {
        setOrderTagsApplying(false);
      }
    },
    [detailOrder?.id, detailQuery, refetch, tenantId],
  );

  const persistGoodsRows = useCallback(
    async (nextRows: GoodsGridRow[], successText: string) => {
      const previousRows = goodsRowsState;
      setGoodsRowsState(nextRows);

      const orderId = String(detailOrder?.id ?? activeDetailOrderId ?? '').trim();
      if (!tenantId || !orderId) {
        message.success(successText);
        return;
      }

      const payloadItems = nextRows.map((row, index) => ({
        line_id: row.id ?? `${orderId}-LINE-${index + 1}`,
        sku_id: row.platform_sku_id ?? undefined,
        title: row.title ?? '',
        qty: Math.max(1, Number(row.quantity ?? 1)),
        price_cent: Math.max(0, Number(row.price_cent ?? 0)),
      }));

      setGoodsSaving(true);
      try {
        await updateOrder(tenantId, orderId, { items: payloadItems });
        message.success(successText);
        await Promise.all([refetch(), detailQuery.refetch()]);
      } catch (error) {
        setGoodsRowsState(previousRows);
        message.error(error instanceof Error ? error.message : '货品操作失败');
      } finally {
        setGoodsSaving(false);
      }
    },
    [activeDetailOrderId, detailOrder?.id, detailQuery, goodsRowsState, refetch, tenantId],
  );

  const platformCountMap = useMemo(() => {
    const map = new Map<string, number>();
    const counts = globalStatsData?.platform_counts ?? {};
    Object.entries(counts).forEach(([platform, count]) => {
      if (!platform || count <= 0) return;
      map.set(platform, count);
    });
    return map;
  }, [globalStatsData?.platform_counts]);

  const shopCountMap = useMemo(() => {
    const map = new Map<string, number>();
    const counts = globalStatsData?.shop_counts ?? {};
    Object.entries(counts).forEach(([shopId, count]) => {
      if (!shopId || count <= 0) return;
      map.set(shopId, count);
    });
    return map;
  }, [globalStatsData?.shop_counts]);

  const globalOrderTotal = useMemo(
    () => Number(globalStatsData?.total ?? data?.total ?? rows.length),
    [data?.total, globalStatsData?.total, rows.length],
  );

  const leftStatsSourceRows = useMemo(() => manualFilterScopeRows ?? serverRows, [manualFilterScopeRows, serverRows]);

  const leftNodes = useMemo<OrderReviewAgLeftNode[]>(() => {
    const now = new Date();
    const sourceTotal = leftStatsSourceRows.length;
    const abnormalCount = leftStatsSourceRows.filter((record) => isAbnormalOrder(record)).length;
    const remarkCount = leftStatsSourceRows.filter((record) => hasRemark(record)).length;
    const tagCount = leftStatsSourceRows.filter((record) => hasTag(record)).length;

    const platformValues = Array.from(new Set([...platformCountMap.keys()]));
    const shopOptionLabelMap = new Map<string, string>();
    shopOptions.forEach((option) => {
      const value = String(option.value ?? '').trim();
      if (!value) return;
      shopOptionLabelMap.set(value, typeof option.label === 'string' ? option.label : value);
    });
    const shopValues = Array.from(new Set([...shopOptionLabelMap.keys(), ...shopCountMap.keys()]));
    const warehouseValues = Array.from(
      new Set(leftStatsSourceRows.map((record) => resolveWarehouseId(record)).filter(Boolean)),
    );
    const logisticsValues = Array.from(
      new Set(leftStatsSourceRows.map((record) => resolveLogisticsCompany(record)).filter(Boolean)),
    );

    const timeChildren: OrderReviewAgLeftNode[] = [
      { key: 'time:*', label: '全部时间', count: sourceTotal, nodeType: 'time' },
      ...orderTimeOptions.map((item) => ({
        key: `time:${item.key}`,
        label: item.label,
        count: leftStatsSourceRows.filter((record) => matchesTimeRange(record, item.key, now)).length,
        nodeType: 'time' as const,
      })),
    ];

    const statusChildren: OrderReviewAgLeftNode[] = [
      { key: 'status:*', label: '全部状态', count: sourceTotal, nodeType: 'status' },
      {
        key: 'status:normal',
        label: '正常订单',
        count: leftStatsSourceRows.filter((record) => isNormalOrder(record)).length,
        nodeType: 'status',
      },
      {
        key: 'status:exception',
        label: '异常订单',
        count: abnormalCount,
        nodeType: 'status',
      },
    ];

    const exceptionChildren: OrderReviewAgLeftNode[] = [
      { key: 'exception:*', label: '全部异常', count: abnormalCount, nodeType: 'exception' },
      ...exceptionTypeOptions.map((item) => ({
        key: `exception:${item.key}`,
        label: item.label,
        count: leftStatsSourceRows.filter((record) => matchesExceptionType(record, item.key)).length,
        nodeType: 'exception' as const,
      })),
    ];

    const platformChildren: OrderReviewAgLeftNode[] = [
      {
        key: 'platform:*',
        label: '全部销售渠道',
        count: globalOrderTotal,
        nodeType: 'platform',
      },
      ...platformValues
        .map((value) => {
          if (!value) return null;
          return {
            key: `platform:${value}`,
            label: platformLabel(value),
            count: platformCountMap.get(value) ?? 0,
            nodeType: 'platform' as const,
          };
        })
        .filter((node): node is OrderReviewAgLeftNode => node != null),
    ];

    const warehouseChildren: OrderReviewAgLeftNode[] = [
      { key: 'warehouse:*', label: '全部仓库', count: sourceTotal, nodeType: 'warehouse' },
      ...warehouseValues.map((value) => ({
        key: `warehouse:${value}`,
        label: value,
        count: leftStatsSourceRows.filter((record) => resolveWarehouseId(record) === value).length,
        nodeType: 'warehouse' as const,
      })),
    ];

    const logisticsChildren: OrderReviewAgLeftNode[] = [
      { key: 'logistics:*', label: '全部物流公司', count: sourceTotal, nodeType: 'logistics' },
      ...logisticsValues.map((value) => ({
        key: `logistics:${value}`,
        label: value,
        count: leftStatsSourceRows.filter((record) => resolveLogisticsCompany(record) === value).length,
        nodeType: 'logistics' as const,
      })),
    ];

    const remarkChildren: OrderReviewAgLeftNode[] = [
      { key: 'remark:*', label: '全部备注状态', count: sourceTotal, nodeType: 'remark' },
      { key: 'remark:has', label: '有备注', count: remarkCount, nodeType: 'remark' },
      { key: 'remark:none', label: '无备注', count: Math.max(0, sourceTotal - remarkCount), nodeType: 'remark' },
    ];

    const tagChildren: OrderReviewAgLeftNode[] = [
      { key: 'tag:*', label: '全部标记状态', count: sourceTotal, nodeType: 'tag' },
      { key: 'tag:has', label: '有标记', count: tagCount, nodeType: 'tag' },
      { key: 'tag:none', label: '无标记', count: Math.max(0, sourceTotal - tagCount), nodeType: 'tag' },
    ];

    const shopChildren: OrderReviewAgLeftNode[] = [
      {
        key: 'shop:*',
        label: '全部店铺',
        count: globalOrderTotal,
        nodeType: 'shop',
      },
      ...shopValues
        .map((value) => {
          if (!value) return null;
          return {
            key: `shop:${value}`,
            label: shopOptionLabelMap.get(value) ?? value,
            count: shopCountMap.get(value) ?? 0,
            nodeType: 'shop' as const,
          };
        })
        .filter((node): node is OrderReviewAgLeftNode => node != null),
    ];

    return [
      { key: 'order-time-group', label: '全部订单', nodeType: 'group', children: timeChildren },
      { key: 'order-status-group', label: '订单状态', nodeType: 'group', children: statusChildren },
      { key: 'exception-group', label: '异常订单', nodeType: 'group', children: exceptionChildren },
      { key: 'platform-group', label: '销售渠道', nodeType: 'group', children: platformChildren },
      { key: 'warehouse-group', label: '发货仓库', nodeType: 'group', children: warehouseChildren },
      { key: 'logistics-group', label: '物流公司', nodeType: 'group', children: logisticsChildren },
      { key: 'remark-group', label: '备注', nodeType: 'group', children: remarkChildren },
      { key: 'tag-group', label: '标记', nodeType: 'group', children: tagChildren },
      { key: 'shop-group', label: '店铺', nodeType: 'group', children: shopChildren },
    ];
  }, [globalOrderTotal, leftStatsSourceRows, platformCountMap, shopCountMap, shopOptions]);

  const leftStatsItems = useMemo<OrderReviewLeftStatsItem[]>(() => {
    const palette = ['#334e68', '#7da888', '#8bcdd3', '#d48935', '#b76e79'];
    const platformEntries = Array.from(platformCountMap.entries()).filter(([, value]) => value > 0);
    const sorted = platformEntries.sort((a, b) => b[1] - a[1]);
    const head = sorted.slice(0, 4);
    const restTotal = sorted.slice(4).reduce((sum, [, value]) => sum + value, 0);
    const merged = restTotal > 0 ? [...head, ['others', restTotal] as const] : head;
    const normalized = merged.length > 0 ? merged : [['all', globalOrderTotal] as const];
    const totalValue = normalized.reduce((sum, [, value]) => sum + value, 0) || 1;
    return normalized.map(([key, value], index) => ({
      key,
      value,
      ratio: value / totalValue,
      color: palette[index % palette.length] ?? palette[0],
      label: key === 'all' ? '全部订单' : key === 'others' ? '其他平台' : platformLabel(key),
    }));
  }, [globalOrderTotal, platformCountMap]);

  const leftStatsPieBackground = useMemo(() => {
    if (leftStatsItems.length === 0) {
      return 'conic-gradient(#dce4ee 0deg 360deg)';
    }
    let angle = 0;
    const stops = leftStatsItems.map((item) => {
      const start = angle;
      const next = angle + item.ratio * 360;
      angle = next;
      return `${item.color} ${start.toFixed(2)}deg ${next.toFixed(2)}deg`;
    });
    if (angle < 360) {
      stops.push(`#dce4ee ${angle.toFixed(2)}deg 360deg`);
    }
    return `conic-gradient(${stops.join(', ')})`;
  }, [leftStatsItems]);

  const updateLeftScreenFilter = useCallback<UiModule08StrategyScreenFilterChangeHandler>((key, value) => {
    setLeftScreenFiltersDraft((prev) => ({ ...prev, [key]: value }));
  }, []);

  const applyLeftScreenFilters = useCallback(() => {
    const next = {
      ...leftScreenFiltersDraft,
      orderQuery: leftScreenFiltersDraft.orderQuery.trim(),
      createdFrom: leftScreenFiltersDraft.createdFrom.trim(),
      createdTo: leftScreenFiltersDraft.createdTo.trim(),
      includeSkuKeyword: leftScreenFiltersDraft.includeSkuKeyword.trim(),
      regionKeyword: leftScreenFiltersDraft.regionKeyword.trim(),
      orderType: leftScreenFiltersDraft.orderType.trim(),
      warehouse: leftScreenFiltersDraft.warehouse.trim(),
    };
    setLeftScreenFiltersApplied(next);
    setSearchInput(next.orderQuery);
    setSearchKeyword(next.orderQuery);
    resetPagination();
    const mappedPlatform = platformLabelToCodeMap[next.salesChannel];
    if (mappedPlatform) {
      setColumnFilters((prev) => ({ ...prev, platform: [mappedPlatform] }));
    } else if (!next.salesChannel) {
      setColumnFilters((prev) => ({ ...prev, platform: undefined }));
    }
  }, [leftScreenFiltersDraft]);

  const resetLeftScreenFilters = useCallback(() => {
    const next = createUiModule08StrategyScreenFilterState();
    setLeftScreenFiltersDraft(next);
    setLeftScreenFiltersApplied(next);
    setSearchInput('');
    setSearchKeyword('');
    resetPagination();
    setColumnFilters((prev) => ({ ...prev, platform: undefined }));
  }, []);

  const renderOrderReviewLeftPanel = useCallback(
    ({ leftTab, onLeftTabChange, leftRowsForRender, onLeftRowClick }: LeftFilterPanelProps) => (
      <OrderReviewLeftPanel
        leftTab={leftTab}
        onLeftTabChange={onLeftTabChange}
        leftRowsForRender={leftRowsForRender}
        onLeftRowClick={onLeftRowClick}
        leftScreenFiltersDraft={leftScreenFiltersDraft}
        onLeftScreenFieldChange={updateLeftScreenFilter}
        onApplyLeftScreenFilters={applyLeftScreenFilters}
        onResetLeftScreenFilters={resetLeftScreenFilters}
        leftStatsItems={leftStatsItems}
        leftStatsPieBackground={leftStatsPieBackground}
        globalOrderTotal={globalOrderTotal}
        isGlobalStatsFetching={isGlobalStatsFetching}
        shouldFallbackToAll={shouldFallbackToAll}
      />
    ),
    [
      applyLeftScreenFilters,
      globalOrderTotal,
      isGlobalStatsFetching,
      leftScreenFiltersDraft,
      leftStatsItems,
      leftStatsPieBackground,
      resetLeftScreenFilters,
      shouldFallbackToAll,
      updateLeftScreenFilter,
    ],
  );

  useLeftFilterModule<OrderSummary, ColumnFilters>({
    rows: leftStatsSourceRows,
    filters: columnFilters,
    onFiltersChange: setColumnFilters,
    initialExpandedKeys: ['order-time-group'],
    emptyRowLabel: '无匹配项',
    hasManualFilter: hasOrderReviewLeftManualFilter,
    filterRows: (sourceRows) => sourceRows,
    buildCheckedKeys: buildOrderReviewLeftCheckedKeys,
    buildLeftNodes: () => leftNodes,
    reduceFiltersByNode: reduceOrderReviewFiltersByNode,
    renderPanel: renderOrderReviewLeftPanel,
  });

  const handleMainGridColumnFiltersChange = useCallback((nextFilters: ColumnFilters) => {
    setColumnFilters(nextFilters);
    resetPagination();
  }, []);

  const handleExportGrid = useCallback(() => {
    const api = mainGridApiRef.current;
    if (!api) {
      message.warning('表格尚未准备完成');
      return;
    }

    try {
      exportOrderReviewGridAsExcel(api, '订单审核');
    } catch (error) {
      if (error instanceof Error && error.message === 'NO_EXPORTABLE_COLUMNS') {
        message.warning('当前没有可导出的列');
        return;
      }
      message.error(error instanceof Error ? error.message : '导出失败');
    }
  }, []);

  const handlePrintGrid = useCallback(() => {
    const api = mainGridApiRef.current;
    if (!api) {
      message.warning('表格尚未准备完成');
      return;
    }

    try {
      printOrderReviewGrid(api, '订单审核');
    } catch (error) {
      if (error instanceof Error) {
        if (error.message === 'NO_PRINTABLE_COLUMNS') {
          message.warning('当前没有可打印的列');
          return;
        }
        if (error.message === 'NO_PRINTABLE_ROWS') {
          message.warning('当前没有可打印的数据');
          return;
        }
        if (error.message === 'PRINT_WINDOW_BLOCKED') {
          message.error('打印窗口被浏览器拦截，请允许弹窗后重试');
          return;
        }
      }
      message.error(error instanceof Error ? error.message : '打印失败');
    }
  }, []);

  const handleMainGridApiChange = useCallback((api: GridApi<OrderSummary> | null) => {
    mainGridApiRef.current = api;
  }, []);

  const handleMainSelectionChange = useCallback((keys: Key[]) => {
    setSelectedRowKeys(keys);
    setSelectionAnchorKey((prev) => {
      if (!keys.length) return null;
      if (prev && keys.some((item) => String(item) === String(prev))) {
        return prev;
      }
      return String(keys[0]);
    });
  }, []);

  const handleMainRowActive = useCallback((record: OrderSummary) => {
    setActiveOrder(record);
  }, []);

  const handleMainFocus = useCallback((record: OrderSummary) => {
    setFocusedOrderKey(resolveOrderRowKey(record) || null);
  }, []);

  const handleMainRowDoubleClick = useCallback(
    (record: OrderSummary) => {
      const orderId = resolveOrderActionId(record);
      if (!orderId) {
        message.warning('订单ID为空，无法打开编辑页');
        return;
      }
      navigate(`/orders/${encodeURIComponent(orderId)}`);
    },
    [navigate],
  );

  const handleMainContextMenu = useCallback(
    (record: OrderSummary, event: MouseEvent) => {
      const key = resolveOrderRowKey(record) || null;
      setActiveOrder(record);
      setFocusedOrderKey(key);
      if (key && !selectedRowKeys.some((item) => String(item) === String(key))) {
        setSelectedRowKeys([key]);
        setSelectionAnchorKey(key);
      }
      setMainContextMenu({
        open: true,
        position: { x: event.clientX, y: event.clientY },
        row: record,
      });
    },
    [selectedRowKeys],
  );

  const selectedOrderIds = useMemo(() => {
    if (!selectedRowKeys.length || !rows.length) {
      return [];
    }
    const selectedSet = new Set(selectedRowKeys.map((item) => String(item)));
    const ids = rows
      .filter((record) => selectedSet.has(resolveOrderRowKey(record)))
      .map((record) => resolveOrderActionId(record))
      .filter(Boolean);
    return Array.from(new Set(ids));
  }, [rows, selectedRowKeys]);

  const selectedAmountCent = useMemo(() => {
    if (!selectedOrderIds.length) return 0;
    const selected = new Set(selectedOrderIds);
    return rows.reduce((totalValue, row) => {
      const rowId = String(row.id ?? row.platform_order_id ?? '');
      if (!selected.has(rowId)) return totalValue;
      return totalValue + Number(row.total_amount_cent ?? 0);
    }, 0);
  }, [rows, selectedOrderIds]);

  const summary = useMemo(
    () => (
      <>
        <span>
          总数 {total} 条，本页 {rows.length} 条，已选 {selectedRowKeys.length} 条，已选金额 ¥{moneyValue(selectedAmountCent / 100)}
        </span>
        <span data-testid="order-review-load-timing" style={{ marginInlineStart: 8 }}>
          {isFetching ? '加载中...' : lastFetchDurationMs == null ? '加载耗时 --' : `加载耗时 ${lastFetchDurationMs}ms`}
        </span>
        <span data-testid="order-review-load-cycle" style={{ display: 'none' }}>
          {loadCycle}
        </span>
        <span data-testid="order-review-detail-loaded-id" style={{ display: 'none' }}>
          {detailLoadedId}
        </span>
      </>
    ),
    [detailLoadedId, isFetching, lastFetchDurationMs, loadCycle, rows.length, selectedAmountCent, selectedRowKeys.length, total],
  );

  const mainContextMenuConfig = useMemo<MenuProps>(
    () => ({
      items: [
        { key: 'copy-id', label: '复制订单号' },
        { key: 'focus', label: '定位详情' },
        { key: 'clear-selection', label: '清空勾选' },
      ],
      onClick: ({ key }) => {
        const current = mainContextMenu.row;
        if (key === 'copy-id' && current) {
          const id = resolveOrderActionId(current);
          if (id && navigator?.clipboard) {
            void navigator.clipboard.writeText(id);
          }
          message.success(id ? `已复制：${id}` : '无可复制订单号');
        }
        if (key === 'focus' && current) {
          const rowKey = resolveOrderRowKey(current) || null;
          setFocusedOrderKey(rowKey);
          setActiveOrder(current);
        }
        if (key === 'clear-selection') {
          setSelectedRowKeys([]);
          setSelectionAnchorKey(null);
        }
        setMainContextMenu({ open: false, position: null, row: null });
      },
    }),
    [mainContextMenu.row],
  );

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
        orderTagOptions={orderTagOptions}
        currentOrderTags={currentOrderTags}
        orderTagsApplying={orderTagsApplying}
        onApplyOrderTags={handleApplyOrderTags}
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
      handleDetailLogisticsChange,
      handleDetailWarehouseChange,
      mergedCourierOptions,
      mergedWarehouseOptions,
      orderTagOptions,
      orderTagsApplying,
    ],
  );

  const bottomPanel = useMemo(
    () => (
      <OrderReviewBottomPanel
        activeOrderId={activeDetailOrderId || 'none'}
        goodsRows={goodsRowsState}
        logRows={logRows}
        goodsSaving={goodsSaving}
        onPersistGoodsRows={persistGoodsRows}
      />
    ),
    [activeDetailOrderId, goodsRowsState, goodsSaving, logRows, persistGoodsRows],
  );

  useEffect(() => {
    setRightPanelVisible(true);
    return () => {
      setRightPanel(undefined);
      setRightPanelVisible(false);
    };
  }, [setRightPanel, setRightPanelVisible]);

  useEffect(() => {
    setRightPanel(rightPanel);
    setRightPanelVisible(true);
  }, [rightPanel, setRightPanel, setRightPanelVisible]);

  useEffect(() => {
    setBottomPanelVisible(true);
    return () => {
      setBottomPanel(undefined);
      setBottomPanelVisible(false);
    };
  }, [setBottomPanel, setBottomPanelVisible]);

  useEffect(() => {
    setBottomPanel(bottomPanel);
    setBottomPanelVisible(true);
  }, [bottomPanel, setBottomPanel, setBottomPanelVisible]);

  const toolbar = (
    <WorkbenchToolbarTemplate
      className="order-review-toolbar"
      leftContent={
        <>
          <Button data-testid="order-review-approve" size="small" type="primary">
            审核
          </Button>
          <Button data-testid="order-review-split" size="small">
            拆单
          </Button>
          <Button data-testid="order-review-merge" size="small">
            合单
          </Button>
          <Button size="small" onClick={() => setQuickFilterExpanded((prev) => !prev)}>
            {quickFilterExpanded ? '收起列筛选' : '展开列筛选'}
          </Button>
        </>
      }
      searchValue={searchInput}
      onSearchChange={setSearchInput}
      onSearch={() => {
        setSearchKeyword(searchInput.trim());
        resetPagination();
      }}
      searchPlaceholder="网店订单号/客户帐号/手机"
      searchInputClassName="order-review-toolbar-search"
      searchButtonClassName="order-review-toolbar-search-trigger"
      searchInputTestId="order-review-toolbar-search-input"
      searchButtonTestId="order-review-toolbar-search-trigger"
      searchButtonAriaLabel="查询"
      onExport={handleExportGrid}
      onPrint={handlePrintGrid}
      onHelp={() => message.info('帮助功能待接入')}
    />
  );

  const mainContent = (
    <WorkbenchMainTableTemplate
      className="order-review-page"
      toolbar={toolbar}
      tableContainerRef={tableRef}
      tableClassName="order-review-table order-workbench-table-module-03"
      tableContent={
        <div className="order-review-grid-main">
          <OrderReviewMainGrid
            rows={rows}
            quickFilterExpanded={quickFilterExpanded}
            columnFilters={columnFilters}
            shopOptions={shopOptions}
            logisticsOptions={courierOptions}
            selectedRowKeys={selectedRowKeys}
            focusedOrderKey={focusedOrderKey}
            onGridApiChange={handleMainGridApiChange}
            onColumnFiltersChange={handleMainGridColumnFiltersChange}
            onSelectionChange={handleMainSelectionChange}
            onRowActive={handleMainRowActive}
            onRowDoubleClick={handleMainRowDoubleClick}
            onFocus={handleMainFocus}
            onContextMenu={handleMainContextMenu}
            loading={isFetching}
            error={error}
            onRetry={() => {
              void refetch();
            }}
          />
        </div>
      }
      footerSummary={summary}
      footerSyncKey={`order-review-native-${total}`}
      total={total}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
      showFooterScrollbarRow={false}
    />
  );

  return (
    <div
      data-testid="order-review-page"
      style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0, height: '100%' }}
    >
      {mainContent}
      <UiModule03ContextMenu
        open={mainContextMenu.open}
        position={mainContextMenu.position}
        menu={mainContextMenuConfig}
        onClose={() => setMainContextMenu({ open: false, position: null, row: null })}
      />
    </div>
  );
};

