
import { Alert, Button, Descriptions, Empty, Form, Input, InputNumber, Modal, Select, Space, Spin, Tabs, Typography, message } from 'antd';
import type { MenuProps, TablePaginationConfig } from 'antd';
import {
  AppstoreOutlined,
  CheckSquareFilled,
  FilterOutlined,
  FolderOpenOutlined,
  FolderOutlined,
  MinusSquareOutlined,
  PieChartOutlined,
  PlusSquareOutlined,
  SearchOutlined,
  ShopOutlined,
} from '@ant-design/icons';
import type { Key, ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type {
  CellClickedEvent,
  CellContextMenuEvent,
  ColDef,
  GetRowIdParams,
  GridApi,
  GridReadyEvent,
  RowClickedEvent,
  SelectionChangedEvent,
  ValueFormatterParams,
} from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';

import { listCouriers } from '@/api/couriers';
import { useOrdersList } from '@/api/hooks';
import {
  forceReviewOrder,
  getOrderDetail,
  getOrderReviewStats,
  mergeOrders,
  reviewOrder,
  splitOrder,
  updateOrder,
  type OrderListQuery,
  type OrderSummary,
} from '@/api/orders';
import { listShops } from '@/api/shops';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { ErpDataGrid, type ErpDataGridColumn } from '@/components/ErpDataGrid';
import {
  UiModule03ContextMenu,
  UiModule03TableFooter,
  UiModule04BottomTable,
  UiModule05DetailPanel,
  UiModule08StrategyScreenFilterForm,
  createUiModule08StrategyScreenFilterState,
  type UiModule08StrategyScreenFilterState,
} from '@/components/ui-modules';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

import {
  flattenAgLeftTree,
  toggleCheckedValue,
  type OrderReviewAgLeftNode,
  type OrderReviewAgLeftRow,
} from './orderReviewAgLeftModel';
import { buildCourierOptionLabel, resolveCourierOptionValue } from './shared/courierDisplay';

import './orderReviewPage.css';

type OrderReviewPageMode = 'review' | 'split-merge';

type OrderReviewPageProps = {
  mode?: OrderReviewPageMode;
  pageTitle?: string;
};

type ColumnFilters = {
  platform?: string[];
  shopId?: string[];
};

type ContextMenuState<T> = {
  open: boolean;
  position: { x: number; y: number } | null;
  row: T | null;
};

type OrderReviewLeftStatsItem = {
  key: string;
  label: string;
  value: number;
  ratio: number;
  color: string;
};

type OrderReviewAgLeftRenderRow = OrderReviewAgLeftRow & {
  checked: boolean;
};

type GoodsGridRow = Record<string, unknown> & {
  __localKey: string;
  id?: string;
  platform_sku_id?: string | null;
  outer_sku_id?: string | null;
  title?: string | null;
  quantity?: number | null;
  price_cent?: number | null;
};

type GoodsEditorValues = {
  platform_sku_id?: string;
  outer_sku_id?: string;
  title: string;
  quantity: number;
  price_cent?: number;
};

const defaultPageSize = 100;
const pageSizeOptions = ['50', '100', '200', '500'];

const normalizeMulti = (value?: string | string[]) => {
  if (!value) return [];
  if (Array.isArray(value)) return Array.from(new Set(value.map((item) => item.trim()).filter(Boolean)));
  const normalized = value.trim();
  return normalized ? [normalized] : [];
};

const money = (value?: number | null) =>
  value == null
    ? '-'
    : (value / 100).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

const moneyValue = (value?: number | null) =>
  (value ?? 0).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

const platformLabel = (value: string) => {
  const option = platformOptions.find((item) => String(item.value) === value);
  if (!option) return value;
  return typeof option.label === 'string' ? option.label : value;
};

const platformLabelToCodeMap: Record<string, string> = {
  淘宝: 'taobao',
  京东: 'jd',
  拼多多: 'pdd',
  抖音: 'douyin',
};

const toGoodsRows = (rows: Record<string, unknown>[]) =>
  rows.map((row, index) => {
    const id = row.id == null ? undefined : String(row.id);
    const fallbackId = `${String(row.platform_sku_id ?? row.outer_sku_id ?? 'goods')}-${index}`;
    const localKey = id ?? fallbackId;
    return {
      ...(row as GoodsGridRow),
      __localKey: localKey,
      id,
      platform_sku_id: row.platform_sku_id == null ? null : String(row.platform_sku_id),
      outer_sku_id: row.outer_sku_id == null ? null : String(row.outer_sku_id),
      title: row.title == null ? null : String(row.title),
      quantity: row.quantity == null ? 1 : Number(row.quantity),
      price_cent: row.price_cent == null ? 0 : Number(row.price_cent),
    } as GoodsGridRow;
  });

const invalidGoodsKeywords = ['MAPPING_MISSING', 'INVALID_GOODS', 'INVALID_ITEM', 'MISSING_SKU'];

const getOrderExceptionFlags = (record: OrderSummary) =>
  (record.exception_flags ?? []).map((value) => String(value).trim().toUpperCase()).filter(Boolean);

const readMappingMissing = (record: OrderSummary) => {
  const raw = (record as Record<string, unknown>).mapping_missing;
  if (typeof raw === 'boolean') return raw;
  if (typeof raw === 'number') return raw === 1;
  if (typeof raw === 'string') {
    const normalized = raw.trim().toLowerCase();
    return normalized === 'true' || normalized === '1';
  }
  return false;
};

const isInvalidGoodsOrder = (record: OrderSummary) => {
  if (readMappingMissing(record)) {
    return true;
  }
  const flags = getOrderExceptionFlags(record);
  return flags.some((flag) => invalidGoodsKeywords.some((keyword) => flag.includes(keyword)));
};

export const OrderReviewPage = ({ mode = 'review' }: OrderReviewPageProps = {}) => {
  void mode;
  const tenantId = useTenantId();
  const canWriteOrders = useCan('orders.write');
  const {
    setLeftPanel,
    setLeftPanelVisible,
    setLeftPanelWidth,
    setRightPanel,
    setRightPanelVisible,
    setBottomPanel,
    setBottomPanelVisible,
  } = useWorkbenchPanels();
  const queryClient = useQueryClient();

  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: defaultPageSize,
    pageSizeOptions,
  });
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [columnFilters, setColumnFilters] = useState<ColumnFilters>({});
  const [quickFilterExpanded, setQuickFilterExpanded] = useState(true);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [focusedOrderKey, setFocusedOrderKey] = useState<string | null>(null);
  const [selectionAnchorKey, setSelectionAnchorKey] = useState<string | null>(null);
  const mainGridApiRef = useRef<GridApi<OrderSummary> | null>(null);
  const syncingMainSelectionRef = useRef(false);
  const [activeOrder, setActiveOrder] = useState<OrderSummary | null>(null);

  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [leftExpanded, setLeftExpanded] = useState<Set<string>>(
    () => new Set(['platform-group', 'shop-group']),
  );
  const [leftScreenFiltersDraft, setLeftScreenFiltersDraft] = useState<UiModule08StrategyScreenFilterState>(
    () => createUiModule08StrategyScreenFilterState(),
  );
  const [leftScreenFiltersApplied, setLeftScreenFiltersApplied] = useState<UiModule08StrategyScreenFilterState>(
    () => createUiModule08StrategyScreenFilterState(),
  );

  const [detailWarehouseId, setDetailWarehouseId] = useState<string | undefined>(undefined);
  const [detailLogistics, setDetailLogistics] = useState<string | undefined>(undefined);
  const [detailUpdatingField, setDetailUpdatingField] = useState<'warehouse' | 'logistics' | null>(null);

  const [splitModalOpen, setSplitModalOpen] = useState(false);
  const [splitCount, setSplitCount] = useState(2);
  const fetchStartedAtRef = useRef<number | null>(null);
  const [lastFetchDurationMs, setLastFetchDurationMs] = useState<number | null>(null);
  const [loadCycle, setLoadCycle] = useState(0);
  const [actionLoading, setActionLoading] = useState({
    review: false,
    force: false,
    merge: false,
    split: false,
  });

  const [mainContextMenu, setMainContextMenu] = useState<ContextMenuState<OrderSummary>>({
    open: false,
    position: null,
    row: null,
  });

  const [goodsRowsState, setGoodsRowsState] = useState<GoodsGridRow[]>([]);
  const [goodsSelectedRowKeys, setGoodsSelectedRowKeys] = useState<Key[]>([]);
  const [goodsSaving, setGoodsSaving] = useState(false);
  const [goodsEditorOpen, setGoodsEditorOpen] = useState(false);
  const [goodsEditorMode, setGoodsEditorMode] = useState<'create' | 'replace'>('create');
  const [goodsEditorTargetKey, setGoodsEditorTargetKey] = useState<Key | null>(null);
  const [goodsContextMenu, setGoodsContextMenu] = useState<ContextMenuState<GoodsGridRow>>({
    open: false,
    position: null,
    row: null,
  });
  const [goodsForm] = Form.useForm<GoodsEditorValues>();

  const { data: shopData } = useQuery({
    queryKey: ['shops', tenantId, 'orders-review'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const shopOptions = useMemo(
    () => shopData?.items?.map((shop) => ({ label: shop.shop_name ?? shop.shop_id, value: shop.shop_id })) ?? [],
    [shopData?.items],
  );

  const { data: warehouseData } = useQuery({
    queryKey: ['warehouses', tenantId, 'orders-review'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const warehouseOptions = useMemo(
    () => warehouseData?.items?.map((w) => ({ label: w.name, value: w.warehouse_id })) ?? [],
    [warehouseData?.items],
  );

  const { data: courierData } = useQuery({
    queryKey: ['couriers', tenantId, 'orders-review'],
    queryFn: () => listCouriers(tenantId, { limit: 200, status: 'ACTIVE' }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const courierOptions = useMemo(
    () =>
      (courierData?.items ?? [])
        .map((courier) => ({
          label: buildCourierOptionLabel(String(courier.name ?? ''), String(courier.code ?? '')),
          value: resolveCourierOptionValue(String(courier.name ?? ''), String(courier.code ?? '')),
        }))
        .filter((option) => String(option.value ?? '').trim().length > 0),
    [courierData?.items],
  );

  const appliedWarehouseId = useMemo(() => {
    const warehouseKeyword = leftScreenFiltersApplied.warehouse.trim();
    if (!warehouseKeyword) {
      return undefined;
    }
    const matched = warehouseOptions.find((option) => {
      const label = String(option.label ?? '').trim();
      const value = String(option.value ?? '').trim();
      return label === warehouseKeyword || value === warehouseKeyword;
    });
    return matched ? String(matched.value) : undefined;
  }, [leftScreenFiltersApplied.warehouse, warehouseOptions]);

  const baseQuery = useMemo<OrderListQuery>(() => {
    const rawSize = Number(pagination.pageSize ?? defaultPageSize);
    const pageSize = Number.isFinite(rawSize) && rawSize > 0 ? rawSize : defaultPageSize;
    const current = Number(pagination.current ?? 1);
    return {
      keyword: searchKeyword.trim() || undefined,
      created_from: leftScreenFiltersApplied.createdFrom.trim() || undefined,
      created_to: leftScreenFiltersApplied.createdTo.trim() || undefined,
      ship_warehouse_id: appliedWarehouseId,
      item_keyword: leftScreenFiltersApplied.includeSkuKeyword.trim() || undefined,
      order_source: leftScreenFiltersApplied.orderType.trim() || undefined,
      receiver_address:
        leftScreenFiltersApplied.regionScope === 'all'
          ? leftScreenFiltersApplied.regionKeyword.trim() || undefined
          : undefined,
      offset: (current - 1) * pageSize,
      limit: pageSize,
    };
  }, [
    appliedWarehouseId,
    leftScreenFiltersApplied.createdFrom,
    leftScreenFiltersApplied.createdTo,
    leftScreenFiltersApplied.includeSkuKeyword,
    leftScreenFiltersApplied.orderType,
    leftScreenFiltersApplied.regionKeyword,
    leftScreenFiltersApplied.regionScope,
    pagination.current,
    pagination.pageSize,
    searchKeyword,
  ]);

  const statsScopeQuery = useMemo<OrderListQuery>(
    () => ({
      keyword: searchKeyword.trim() || undefined,
      created_from: leftScreenFiltersApplied.createdFrom.trim() || undefined,
      created_to: leftScreenFiltersApplied.createdTo.trim() || undefined,
      ship_warehouse_id: appliedWarehouseId,
      item_keyword: leftScreenFiltersApplied.includeSkuKeyword.trim() || undefined,
      order_source: leftScreenFiltersApplied.orderType.trim() || undefined,
      receiver_address:
        leftScreenFiltersApplied.regionScope === 'all'
          ? leftScreenFiltersApplied.regionKeyword.trim() || undefined
          : undefined,
    }),
    [
      appliedWarehouseId,
      leftScreenFiltersApplied.createdFrom,
      leftScreenFiltersApplied.createdTo,
      leftScreenFiltersApplied.includeSkuKeyword,
      leftScreenFiltersApplied.orderType,
      leftScreenFiltersApplied.regionKeyword,
      leftScreenFiltersApplied.regionScope,
      searchKeyword,
    ],
  );

  const reviewQuery = useMemo<OrderListQuery>(
    () => ({
      ...baseQuery,
      review_status: 'PENDING',
    }),
    [baseQuery],
  );

  const {
    data: reviewData,
    isFetching: isReviewFetching,
    error: reviewError,
    refetch: refetchReview,
  } = useOrdersList({ query: reviewQuery, enabled: Boolean(tenantId) });

  const noManualFilter =
    normalizeMulti(columnFilters.platform).length === 0 &&
    normalizeMulti(columnFilters.shopId).length === 0 &&
    !searchKeyword.trim() &&
    !leftScreenFiltersApplied.createdFrom.trim() &&
    !leftScreenFiltersApplied.createdTo.trim() &&
    !leftScreenFiltersApplied.includeSkuKeyword.trim() &&
    !leftScreenFiltersApplied.orderType.trim() &&
    !leftScreenFiltersApplied.regionKeyword.trim() &&
    !leftScreenFiltersApplied.warehouse.trim();
  const shouldFallbackToAll =
    noManualFilter &&
    !isReviewFetching &&
    !reviewError &&
    (reviewData?.total ?? 0) === 0;

  const {
    data: allData,
    isFetching: isAllFetching,
    error: allError,
    refetch: refetchAll,
  } = useOrdersList({ query: baseQuery, enabled: Boolean(tenantId) && shouldFallbackToAll });

  const data = shouldFallbackToAll ? allData : reviewData;
  const isFetching = isReviewFetching || (shouldFallbackToAll && isAllFetching);
  const error = reviewError ?? (shouldFallbackToAll ? allError : null);
  const refetch = useCallback(async () => {
    const first = await refetchReview();
    if (shouldFallbackToAll) {
      await refetchAll();
    }
    return first;
  }, [refetchAll, refetchReview, shouldFallbackToAll]);

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

  const { data: globalStatsData, isFetching: isGlobalStatsFetching } = useQuery({
    queryKey: ['orders-review', tenantId, 'left-global-stats', statsQuery],
    queryFn: ({ signal }) => getOrderReviewStats(tenantId, statsQuery, { signal }),
    enabled: Boolean(tenantId),
    staleTime: 30_000,
    retry: false,
    placeholderData: (previousData) => previousData,
  });

  useEffect(() => {
    const now = () => (typeof performance !== 'undefined' ? performance.now() : Date.now());
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
  const [rowsSnapshotWithoutManualFilter, setRowsSnapshotWithoutManualFilter] = useState<OrderSummary[]>([]);

  useEffect(() => {
    if (normalizeMulti(columnFilters.platform).length > 0 || normalizeMulti(columnFilters.shopId).length > 0) {
      return;
    }
    if (isFetching || error) {
      return;
    }
    if (serverRows.length > 0) {
      setRowsSnapshotWithoutManualFilter(serverRows);
    }
  }, [columnFilters.platform, columnFilters.shopId, error, isFetching, serverRows]);

  const rows = useMemo(() => {
    const selectedPlatformSet = new Set(normalizeMulti(columnFilters.platform));
    const selectedShopSet = new Set(normalizeMulti(columnFilters.shopId));

    const source = serverRows.length > 0 ? serverRows : rowsSnapshotWithoutManualFilter;

    if (selectedPlatformSet.size === 0 && selectedShopSet.size === 0) {
      if (source.length > 0) {
        return source;
      }
      return serverRows;
    }

    return source.filter((record) => {
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
      return true;
    });
  }, [columnFilters.platform, columnFilters.shopId, rowsSnapshotWithoutManualFilter, serverRows]);

  const total = useMemo(() => {
    const hasManualLeftFilter =
      normalizeMulti(columnFilters.platform).length > 0 || normalizeMulti(columnFilters.shopId).length > 0;
    if (hasManualLeftFilter) {
      return rows.length;
    }
    return data?.total ?? rows.length;
  }, [columnFilters.platform, columnFilters.shopId, data?.total, rows.length]);

  const resolveOrderRowKey = useCallback((record: OrderSummary) => {
    const id = String(record.id ?? '').trim();
    if (id) {
      return id;
    }
    const platformOrderId = String(record.platform_order_id ?? '').trim();
    if (platformOrderId) {
      return platformOrderId;
    }
    const fallback = [
      String(record.shop_id ?? '').trim(),
      String(record.receiver_phone ?? '').trim(),
      String(record.created_at ?? '').trim(),
    ]
      .filter(Boolean)
      .join('|');
    return fallback ? `fallback:${fallback}` : '';
  }, []);

  const resolveOrderActionId = useCallback((record: OrderSummary) => {
    const id = String(record.id ?? '').trim();
    if (id) {
      return id;
    }
    const platformOrderId = String(record.platform_order_id ?? '').trim();
    if (platformOrderId) {
      return platformOrderId;
    }
    return '';
  }, []);

  const resolveOrderDetailId = useCallback((record: OrderSummary) => String(record.id ?? '').trim(), []);

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
  }, [activeOrder, focusedOrderKey, resolveOrderRowKey, rows]);

  const activeDetailOrderId = useMemo(
    () => (activeOrder ? resolveOrderDetailId(activeOrder) : ''),
    [activeOrder, resolveOrderDetailId],
  );

  useEffect(() => {
    if (!tenantId) {
      return;
    }
    void queryClient.cancelQueries({
      predicate: (query) => {
        if (!Array.isArray(query.queryKey)) {
          return false;
        }
        return (
          query.queryKey[0] === 'orders' &&
          query.queryKey[1] === 'detail' &&
          query.queryKey[2] === tenantId &&
          query.queryKey[3] !== activeDetailOrderId
        );
      },
    });
  }, [activeDetailOrderId, queryClient, tenantId]);

  const prefetchOrderDetail = useCallback(
    (orderId: string) => {
      if (!tenantId || !orderId) {
        return;
      }
      void queryClient.prefetchQuery({
        queryKey: ['orders', 'detail', tenantId, orderId],
        queryFn: ({ signal }) => getOrderDetail(tenantId, orderId, { signal }),
        staleTime: 45_000,
        gcTime: 300_000,
      });
    },
    [queryClient, tenantId],
  );

  useEffect(() => {
    if (!tenantId || !rows.length) {
      return;
    }
    const anchorRowKey = focusedOrderKey ?? (activeOrder ? resolveOrderRowKey(activeOrder) : null);
    const anchorIndex =
      anchorRowKey == null ? 0 : rows.findIndex((row) => resolveOrderRowKey(row) === anchorRowKey);
    const center = anchorIndex >= 0 ? anchorIndex : 0;
    const candidates = [center, center + 1, center + 2, center - 1];
    candidates.forEach((index) => {
      if (index < 0 || index >= rows.length) {
        return;
      }
      const row = rows[index];
      if (!row) {
        return;
      }
      const detailOrderId = resolveOrderDetailId(row);
      if (!detailOrderId || detailOrderId === activeDetailOrderId) {
        return;
      }
      prefetchOrderDetail(detailOrderId);
    });
  }, [
    activeDetailOrderId,
    activeOrder,
    focusedOrderKey,
    prefetchOrderDetail,
    resolveOrderDetailId,
    resolveOrderRowKey,
    rows,
    tenantId,
  ]);

  const detailQuery = useQuery({
    queryKey: ['orders', 'detail', tenantId, activeDetailOrderId],
    queryFn: ({ signal }) => getOrderDetail(tenantId, activeDetailOrderId, { signal }),
    enabled: Boolean(tenantId && activeDetailOrderId),
    staleTime: 45_000,
    gcTime: 300_000,
    retry: 1,
    refetchOnWindowFocus: false,
  });

  const detailOrder = detailQuery.data?.order;
  const logRows = (detailQuery.data?.audit_logs ?? []) as Record<string, unknown>[];

  useEffect(() => {
    setDetailWarehouseId(detailOrder?.ship_warehouse_id ?? undefined);
    setDetailLogistics(detailOrder?.logistics_company ?? undefined);
  }, [detailOrder?.id, detailOrder?.ship_warehouse_id, detailOrder?.logistics_company]);

  useEffect(() => {
    setGoodsRowsState(toGoodsRows((detailQuery.data?.items ?? []) as Record<string, unknown>[]));
    setGoodsSelectedRowKeys([]);
  }, [detailQuery.data?.items, detailOrder?.id]);

  const selectedPlatform = normalizeMulti(columnFilters.platform);
  const selectedShop = normalizeMulti(columnFilters.shopId);

  const checkedKeys = useMemo(() => {
    const keys: string[] = [];
    if (!selectedPlatform.length) {
      keys.push('platform:*');
    } else {
      keys.push(...selectedPlatform.map((value) => `platform:${value}`));
    }

    if (!selectedShop.length) {
      keys.push('shop:*');
    } else {
      keys.push(...selectedShop.map((value) => `shop:${value}`));
    }

    return keys;
  }, [selectedPlatform, selectedShop]);

  const platformCountMap = useMemo(() => {
    const map = new Map<string, number>();
    const platformCounts = globalStatsData?.platform_counts ?? {};
    Object.entries(platformCounts).forEach(([platform, count]) => {
      if (!platform || count <= 0) return;
      map.set(platform, count);
    });
    return map;
  }, [globalStatsData?.platform_counts]);

  const shopCountMap = useMemo(() => {
    const map = new Map<string, number>();
    const shopCounts = globalStatsData?.shop_counts ?? {};
    Object.entries(shopCounts).forEach(([shopId, count]) => {
      if (!shopId || count <= 0) return;
      map.set(shopId, count);
    });
    return map;
  }, [globalStatsData?.shop_counts]);

  const globalOrderTotal = useMemo(
    () => Number(globalStatsData?.total ?? data?.total ?? rows.length),
    [data?.total, globalStatsData?.total, rows.length],
  );

  const leftNodes = useMemo<OrderReviewAgLeftNode[]>(() => {
    const platformOptionValues = platformOptions
      .map((option) => (option.value == null ? '' : String(option.value)))
      .filter(Boolean);
    const platformValues = Array.from(new Set([...platformOptionValues, ...platformCountMap.keys()]));

    const shopOptionLabelMap = new Map<string, string>();
    shopOptions.forEach((option) => {
      const value = String(option.value ?? '').trim();
      if (!value) return;
      shopOptionLabelMap.set(value, typeof option.label === 'string' ? option.label : value);
    });
    const shopValues = Array.from(new Set([...shopOptionLabelMap.keys(), ...shopCountMap.keys()]));

    const platformChildren: OrderReviewAgLeftNode[] = [
      {
        key: 'platform:*',
        label: '全部平台',
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

    if (shopChildren.length === 1) {
      shopChildren.push({
        key: 'shop-placeholder',
        label: '暂无店铺数据',
        count: 0,
        nodeType: 'group',
      });
    }

    return [
      {
        key: 'platform-group',
        label: `平台过滤（命中 ${rows.length} / 总 ${globalOrderTotal}）`,
        nodeType: 'group',
        children: platformChildren,
      },
      { key: 'shop-group', label: '店铺过滤', nodeType: 'group', children: shopChildren },
    ];
  }, [globalOrderTotal, platformCountMap, rows.length, shopCountMap, shopOptions]);

  const leftRows = useMemo(() => flattenAgLeftTree(leftNodes, leftExpanded), [leftExpanded, leftNodes]);

  const visibleLeftRows = useMemo(() => {
    const keyword = leftKeyword.trim().toLowerCase();
    if (!keyword) return leftRows;
    return leftRows.filter((row) => row.nodeType === 'group' || row.label.toLowerCase().includes(keyword));
  }, [leftKeyword, leftRows]);

  const leftRowsForRenderBase = useMemo<OrderReviewAgLeftRow[]>(
    () =>
      visibleLeftRows.length
        ? visibleLeftRows
        : [
            {
              key: '__left-empty__',
              label: '无匹配项',
              count: 0,
              nodeType: 'group',
              level: 0,
              hasChildren: false,
              expanded: false,
            },
          ],
    [visibleLeftRows],
  );

  const checkedKeySet = useMemo(() => new Set(checkedKeys), [checkedKeys]);

  const leftRowsForRender = useMemo<OrderReviewAgLeftRenderRow[]>(
    () => leftRowsForRenderBase.map((row) => ({ ...row, checked: checkedKeySet.has(row.key) })),
    [checkedKeySet, leftRowsForRenderBase],
  );

  const leftColDefs = useMemo<ColDef<OrderReviewAgLeftRenderRow>[]>(
    () => [
      {
        headerName: '过滤',
        field: 'label',
        flex: 1,
        sortable: false,
        filter: false,
        cellRenderer: (params: ValueFormatterParams<OrderReviewAgLeftRenderRow>) => {
          const row = params.data;
          if (!row) return null;

          const checked = row.checked;
          const isGroup = row.nodeType === 'group';
          let nodeIcon: ReactNode = <AppstoreOutlined />;
          if (isGroup) {
            nodeIcon = row.expanded ? <FolderOpenOutlined /> : <FolderOutlined />;
          } else if (row.nodeType === 'shop') {
            nodeIcon = <ShopOutlined />;
          }

          return (
            <div className="order-review-left-ag-row" style={{ paddingLeft: `${row.level * 14}px` }}>
              <span className="order-review-left-ag-toggle">
                {row.hasChildren ? row.expanded ? <MinusSquareOutlined /> : <PlusSquareOutlined /> : null}
              </span>
              <span className={`order-review-left-ag-node-icon${checked ? ' is-checked' : ''}`}>{nodeIcon}</span>
              <span className="order-review-left-ag-label">{row.label}</span>
              {checked && !isGroup ? (
                <span className="order-review-left-ag-check-mark">
                  <CheckSquareFilled />
                </span>
              ) : null}
              {typeof row.count === 'number' ? (
                <span className="order-review-left-ag-count" data-testid="order-review-left-node-count">
                  {row.count}
                </span>
              ) : null}
            </div>
          );
        },
      },
    ],
    [],
  );

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

  const handleLeftClick = useCallback((event: CellClickedEvent<OrderReviewAgLeftRenderRow>) => {
    const row = event.data;
    if (!row) return;
    if (row.key === '__left-empty__') return;

    if (row.nodeType === 'group') {
      if (!row.hasChildren) return;
      setLeftExpanded((prev) => {
        const next = new Set(prev);
        if (next.has(row.key)) next.delete(row.key);
        else next.add(row.key);
        return next;
      });
      return;
    }

    if (row.key === 'platform:*') {
      setColumnFilters((prev) => ({ ...prev, platform: undefined }));
      return;
    }
    if (row.key === 'shop:*') {
      setColumnFilters((prev) => ({ ...prev, shopId: undefined }));
      return;
    }
    if (row.key === 'shop-placeholder') {
      return;
    }

    if (row.nodeType === 'platform') {
      const value = row.key.slice('platform:'.length);
      setColumnFilters((prev) => ({
        ...prev,
        platform: toggleCheckedValue(normalizeMulti(prev.platform), value),
      }));
      return;
    }

    if (row.nodeType === 'shop') {
      const value = row.key.slice('shop:'.length);
      setColumnFilters((prev) => ({
        ...prev,
        shopId: toggleCheckedValue(normalizeMulti(prev.shopId), value),
      }));
    }
  }, []);

  const updateLeftScreenFilter = useCallback(
    <K extends keyof UiModule08StrategyScreenFilterState>(
      key: K,
      value: UiModule08StrategyScreenFilterState[K],
    ) => {
      setLeftScreenFiltersDraft((prev) => ({ ...prev, [key]: value }));
    },
    [],
  );

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
    setPagination((prev) => ({ ...prev, current: 1 }));

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
    setPagination((prev) => ({ ...prev, current: 1 }));
    setColumnFilters((prev) => ({ ...prev, platform: undefined }));
  }, []);

  useEffect(() => {
    if (!selectionAnchorKey) return;
    const exists = rows.some((record) => resolveOrderRowKey(record) === selectionAnchorKey);
    if (!exists) {
      setSelectionAnchorKey(null);
    }
  }, [resolveOrderRowKey, rows, selectionAnchorKey]);

  useEffect(() => {
    const api = mainGridApiRef.current;
    if (!api) {
      return;
    }
    const selectedSet = new Set(selectedRowKeys.map((item) => String(item)));
    syncingMainSelectionRef.current = true;
    api.forEachNode((node) => {
      if (!node.data) return;
      const key = resolveOrderRowKey(node.data);
      const shouldSelect = Boolean(key) && selectedSet.has(String(key));
      if (node.isSelected() !== shouldSelect) {
        node.setSelected(shouldSelect);
      }
    });
    syncingMainSelectionRef.current = false;
  }, [resolveOrderRowKey, rows, selectedRowKeys]);

  const handleMainRowClick = useCallback(
    (event: RowClickedEvent<OrderSummary>) => {
      const record = event.data;
      if (!record) return;
      setActiveOrder(record);

      const clickedKey = resolveOrderRowKey(record);
      if (!clickedKey) return;

      const target = event.event?.target as HTMLElement | null;
      if (
        target?.closest('.ag-checkbox-input') ||
        target?.closest('.ag-selection-checkbox') ||
        target?.closest('.ag-cell[col-id="__erpSelection"]')
      ) {
        return;
      }

      setFocusedOrderKey(clickedKey);
    },
    [resolveOrderRowKey],
  );

  const handleMainSelectionChanged = useCallback(
    (event: SelectionChangedEvent<OrderSummary>) => {
      if (syncingMainSelectionRef.current) {
        return;
      }
      const nextKeys = event.api
        .getSelectedNodes()
        .map((node) => (node.data ? resolveOrderRowKey(node.data) : ''))
        .filter(Boolean);
      setSelectedRowKeys(nextKeys);
      setSelectionAnchorKey((prev) => {
        if (!nextKeys.length) return null;
        if (prev && nextKeys.some((key) => String(key) === String(prev))) {
          return prev;
        }
        return String(nextKeys[0]);
      });
    },
    [resolveOrderRowKey],
  );

  const handleMainCellClicked = useCallback((event: CellClickedEvent<OrderSummary>) => {
    if (!event.data) return;
    if (event.column?.getColId() === '__erpSelection') {
      setActiveOrder(event.data);
    }
  }, []);

  const handleMainCellContextMenu = useCallback(
    (event: CellContextMenuEvent<OrderSummary>) => {
      event.event?.preventDefault?.();
      const record = event.data;
      const nativeEvent = event.event as MouseEvent | undefined;
      if (!record || !nativeEvent) {
        return;
      }
      const key = resolveOrderRowKey(record) || null;
      setActiveOrder(record);
      setFocusedOrderKey(key);
      if (key && !selectedRowKeys.some((item) => String(item) === String(key))) {
        setSelectedRowKeys([key]);
        setSelectionAnchorKey(key);
      }
      setMainContextMenu({
        open: true,
        position: { x: nativeEvent.clientX, y: nativeEvent.clientY },
        row: record,
      });
    },
    [resolveOrderRowKey, selectedRowKeys],
  );

  useEffect(() => {
    if (!mainGridApiRef.current) {
      return;
    }
    mainGridApiRef.current.redrawRows();
  }, [focusedOrderKey, selectedRowKeys.length]);

  const getMainGridRowId = useCallback(
    (params: GetRowIdParams<OrderSummary>) => resolveOrderRowKey(params.data) || `row-${params.rowIndex ?? 0}`,
    [resolveOrderRowKey],
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
  }, [resolveOrderActionId, resolveOrderRowKey, rows, selectedRowKeys]);

  const focusedOrderId = useMemo(() => {
    if (!focusedOrderKey) {
      return '';
    }
    const row = rows.find((record) => resolveOrderRowKey(record) === focusedOrderKey);
    if (!row) {
      return '';
    }
    return resolveOrderActionId(row);
  }, [focusedOrderKey, resolveOrderActionId, resolveOrderRowKey, rows]);

  const actionOrderIds = useMemo(() => {
    if (selectedOrderIds.length) {
      return selectedOrderIds;
    }
    if (focusedOrderId) {
      return [focusedOrderId];
    }
    return [];
  }, [focusedOrderId, selectedOrderIds]);

  const actionHasInvalidGoods = useMemo(() => {
    if (!actionOrderIds.length) {
      return false;
    }
    const actionOrderSet = new Set(actionOrderIds);
    return rows.some((record) => {
      const actionOrderId = resolveOrderActionId(record);
      if (!actionOrderId || !actionOrderSet.has(actionOrderId)) {
        return false;
      }
      return isInvalidGoodsOrder(record);
    });
  }, [actionOrderIds, resolveOrderActionId, rows]);

  const checkActionOrdersHasEmptyLogistics = useCallback(async (): Promise<boolean> => {
    if (!tenantId || !actionOrderIds.length) {
      return false;
    }

    const detailList = await Promise.all(
      actionOrderIds.map((orderId) =>
        queryClient.fetchQuery({
          queryKey: ['orders', 'detail', tenantId, orderId],
          queryFn: ({ signal }) => getOrderDetail(tenantId, orderId, { signal }),
          staleTime: 45_000,
          gcTime: 300_000,
        }),
      ),
    );

    return detailList.some((detail) => !String(detail.order?.logistics_company ?? '').trim());
  }, [actionOrderIds, queryClient, tenantId]);

  const selectedAmountCent = useMemo(() => {
    if (!selectedOrderIds.length) return 0;
    const selected = new Set(selectedOrderIds);
    return rows.reduce((totalValue, row) => {
      const rowId = String(row.id ?? row.platform_order_id ?? '');
      if (!selected.has(rowId)) return totalValue;
      return totalValue + Number(row.total_amount_cent ?? 0);
    }, 0);
  }, [rows, selectedOrderIds]);

  const pageAmountCent = useMemo(
    () => rows.reduce((totalValue, row) => totalValue + Number(row.total_amount_cent ?? 0), 0),
    [rows],
  );

  const persistGoodsRows = useCallback(
    async (nextRows: GoodsGridRow[], successText: string) => {
      if (!tenantId || !activeOrder?.id) {
        setGoodsRowsState(nextRows);
        return;
      }
      if (!canWriteOrders) {
        message.warning('暂无订单修改权限');
        return;
      }

      const payloadItems = nextRows.map((row, index) => ({
        line_id: row.id ?? `${activeOrder.id}-LINE-${index + 1}`,
        sku_id: row.platform_sku_id ?? undefined,
        title: row.title ?? '',
        qty: Math.max(1, Number(row.quantity ?? 1)),
        price_cent: Math.max(0, Number(row.price_cent ?? 0)),
      }));

      setGoodsSaving(true);
      try {
        await updateOrder(tenantId, String(activeOrder.id), {
          items: payloadItems,
        });
        setGoodsRowsState(nextRows);
        setGoodsSelectedRowKeys([]);
        message.success(successText);
        await Promise.all([detailQuery.refetch(), refetch()]);
      } catch (error) {
        message.error(error instanceof Error ? error.message : '货品操作失败');
      } finally {
        setGoodsSaving(false);
      }
    },
    [activeOrder?.id, canWriteOrders, detailQuery, refetch, tenantId],
  );

  const openGoodsCreate = useCallback(() => {
    setGoodsEditorMode('create');
    setGoodsEditorTargetKey(null);
    goodsForm.setFieldsValue({
      platform_sku_id: undefined,
      outer_sku_id: undefined,
      title: '',
      quantity: 1,
      price_cent: 0,
    });
    setGoodsEditorOpen(true);
  }, [goodsForm]);

  const openGoodsReplace = useCallback(
    (targetKey?: Key | null) => {
      const key = targetKey ?? goodsSelectedRowKeys[0] ?? null;
      if (!key) {
        message.warning('请先选择要替换的货品行');
        return;
      }
      const row = goodsRowsState.find((item) => item.__localKey === String(key));
      if (!row) {
        message.warning('未找到要替换的货品行');
        return;
      }

      setGoodsEditorMode('replace');
      setGoodsEditorTargetKey(key);
      goodsForm.setFieldsValue({
        platform_sku_id: row.platform_sku_id ?? undefined,
        outer_sku_id: row.outer_sku_id ?? undefined,
        title: row.title ?? '',
        quantity: Number(row.quantity ?? 1),
        price_cent: Number(row.price_cent ?? 0),
      });
      setGoodsEditorOpen(true);
    },
    [goodsForm, goodsRowsState, goodsSelectedRowKeys],
  );

  const removeSelectedGoods = useCallback(async () => {
    if (!goodsSelectedRowKeys.length) {
      message.warning('请先勾选要删除的货品行');
      return;
    }
    const deleteSet = new Set(goodsSelectedRowKeys.map((item) => String(item)));
    const nextRows = goodsRowsState.filter((item) => !deleteSet.has(item.__localKey));
    await persistGoodsRows(nextRows, '货品删除成功');
  }, [goodsRowsState, goodsSelectedRowKeys, persistGoodsRows]);

  const submitGoodsEditor = useCallback(async () => {
    try {
      const values = await goodsForm.validateFields();
      const title = values.title?.trim() ?? '';
      const quantity = Math.max(1, Number(values.quantity ?? 1));
      const priceCent = Math.max(0, Number(values.price_cent ?? 0));

      if (goodsEditorMode === 'create') {
        const id = `${String(activeOrder?.id ?? 'ORDER')}-NEW-${Date.now()}`;
        const row: GoodsGridRow = {
          __localKey: id,
          id,
          platform_sku_id: values.platform_sku_id?.trim() || null,
          outer_sku_id: values.outer_sku_id?.trim() || null,
          title,
          quantity,
          price_cent: priceCent,
        };
        await persistGoodsRows([row, ...goodsRowsState], '货品新建成功');
      } else {
        const targetKey = String(goodsEditorTargetKey ?? goodsSelectedRowKeys[0] ?? '');
        if (!targetKey) {
          message.warning('未找到替换目标行');
          return;
        }
        const nextRows = goodsRowsState.map((item) => {
          if (item.__localKey !== targetKey) {
            return item;
          }
          return {
            ...item,
            platform_sku_id: values.platform_sku_id?.trim() || null,
            outer_sku_id: values.outer_sku_id?.trim() || null,
            title,
            quantity,
            price_cent: priceCent,
          } as GoodsGridRow;
        });
        await persistGoodsRows(nextRows, '货品替换成功');
      }

      setGoodsEditorOpen(false);
    } catch {
      return;
    }
  }, [
    activeOrder?.id,
    goodsEditorMode,
    goodsEditorTargetKey,
    goodsForm,
    goodsRowsState,
    goodsSelectedRowKeys,
    persistGoodsRows,
  ]);

  const performReviewOrders = useCallback(async () => {
    if (!actionOrderIds.length) {
      message.warning('请先选中或勾选要审核的订单');
      return;
    }
    if (!tenantId) {
      return;
    }
    if (!canWriteOrders) {
      message.warning('暂无订单修改权限');
      return;
    }
    if (actionHasInvalidGoods) {
      message.warning('含无效货品的订单不允许审核或强制审核');
      return;
    }
    let hasEmptyLogistics = false;
    try {
      hasEmptyLogistics = await checkActionOrdersHasEmptyLogistics();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '校验物流公司失败');
      return;
    }
    if (hasEmptyLogistics) {
      message.warning('物流公司为空的订单不允许审核或强制审核');
      return;
    }

    setActionLoading((prev) => ({ ...prev, review: true }));
    try {
      await Promise.all(
        actionOrderIds.map((orderId) =>
          reviewOrder(tenantId, orderId, {
            action: 'PASS',
            reason: '订单审核页面审核通过',
          }),
        ),
      );
      message.success(`审核完成，共 ${actionOrderIds.length} 单`);
      await Promise.all([refetch(), detailQuery.refetch()]);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '审核失败');
    } finally {
      setActionLoading((prev) => ({ ...prev, review: false }));
    }
  }, [actionHasInvalidGoods, actionOrderIds, canWriteOrders, checkActionOrdersHasEmptyLogistics, detailQuery, refetch, tenantId]);

  const performForceReview = useCallback(async () => {
    if (!actionOrderIds.length) {
      message.warning('请先选中或勾选要审核的订单');
      return;
    }
    if (!tenantId) {
      return;
    }
    if (!canWriteOrders) {
      message.warning('暂无订单修改权限');
      return;
    }
    if (actionHasInvalidGoods) {
      message.warning('含无效货品的订单不允许审核或强制审核');
      return;
    }
    let hasEmptyLogistics = false;
    try {
      hasEmptyLogistics = await checkActionOrdersHasEmptyLogistics();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '校验物流公司失败');
      return;
    }
    if (hasEmptyLogistics) {
      message.warning('物流公司为空的订单不允许审核或强制审核');
      return;
    }

    setActionLoading((prev) => ({ ...prev, force: true }));
    try {
      await Promise.all(
        actionOrderIds.map((orderId) =>
          forceReviewOrder(tenantId, orderId, {
            password: 'force-review',
            reason: '订单审核页面强制审核',
          }),
        ),
      );
      message.success(`审核完成，共 ${actionOrderIds.length} 单`);
      await Promise.all([refetch(), detailQuery.refetch()]);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '审核失败');
    } finally {
      setActionLoading((prev) => ({ ...prev, force: false }));
    }
  }, [actionHasInvalidGoods, actionOrderIds, canWriteOrders, checkActionOrdersHasEmptyLogistics, detailQuery, refetch, tenantId]);

  const performMergeOrders = useCallback(async () => {
    if (actionOrderIds.length < 2) {
      message.warning('合单至少需要选择 2 单');
      return;
    }
    if (!tenantId) {
      return;
    }
    if (!canWriteOrders) {
      message.warning('暂无订单修改权限');
      return;
    }

    setActionLoading((prev) => ({ ...prev, merge: true }));
    try {
      await mergeOrders(tenantId, {
        order_ids: actionOrderIds,
      });
      message.success(`合单完成，共处理 ${actionOrderIds.length} 单`);
      await Promise.all([refetch(), detailQuery.refetch()]);
      setSelectedRowKeys([]);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '合单失败');
    } finally {
      setActionLoading((prev) => ({ ...prev, merge: false }));
    }
  }, [actionOrderIds, canWriteOrders, detailQuery, refetch, tenantId]);

  const openSplitModal = useCallback(() => {
    if (!actionOrderIds.length) {
      message.warning('请先选择要拆分的订单');
      return;
    }
    setSplitCount(2);
    setSplitModalOpen(true);
  }, [actionOrderIds.length]);

  const performSplitOrder = useCallback(async () => {
    if (!actionOrderIds.length || !tenantId) {
      setSplitModalOpen(false);
      return;
    }
    if (!canWriteOrders) {
      message.warning('暂无订单修改权限');
      return;
    }

    const sourceOrderId = actionOrderIds[0];
    setActionLoading((prev) => ({ ...prev, split: true }));
    try {
      await splitOrder(tenantId, sourceOrderId, {
        split_count: Math.max(2, Math.floor(splitCount)),
      });
      message.success(`拆单完成：${sourceOrderId}`);
      setSplitModalOpen(false);
      await Promise.all([refetch(), detailQuery.refetch()]);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '拆单失败');
    } finally {
      setActionLoading((prev) => ({ ...prev, split: false }));
    }
  }, [actionOrderIds, canWriteOrders, detailQuery, refetch, splitCount, tenantId]);

  const handleDetailWarehouseChange = useCallback(
    async (nextValue: string | undefined) => {
      const nextWarehouseId = String(nextValue ?? '').trim() || undefined;
      if (nextWarehouseId === detailWarehouseId) {
        return;
      }
      if (!detailOrder?.id || !tenantId) {
        setDetailWarehouseId(nextWarehouseId);
        return;
      }
      if (!canWriteOrders) {
        message.warning('暂无订单修改权限');
        return;
      }

      const previousWarehouseId = detailWarehouseId;
      setDetailWarehouseId(nextWarehouseId);
      setDetailUpdatingField('warehouse');
      try {
        await updateOrder(tenantId, detailOrder.id, { ship_warehouse_id: nextWarehouseId });
        message.success('发货仓库已更新');
        await Promise.all([refetch(), detailQuery.refetch()]);
      } catch (error) {
        setDetailWarehouseId(previousWarehouseId);
        message.error(error instanceof Error ? error.message : '更新发货仓库失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [canWriteOrders, detailOrder?.id, detailQuery, detailWarehouseId, refetch, tenantId],
  );

  const handleDetailLogisticsChange = useCallback(
    async (nextValue: string | undefined) => {
      const nextLogistics = String(nextValue ?? '').trim() || undefined;
      if (nextLogistics === detailLogistics) {
        return;
      }
      if (!detailOrder?.id || !tenantId) {
        setDetailLogistics(nextLogistics);
        return;
      }
      if (!canWriteOrders) {
        message.warning('暂无订单修改权限');
        return;
      }

      const previousLogistics = detailLogistics;
      setDetailLogistics(nextLogistics);
      setDetailUpdatingField('logistics');
      try {
        await updateOrder(tenantId, detailOrder.id, { logistics_company: nextLogistics });
        message.success('物流公司已更新');
        await Promise.all([refetch(), detailQuery.refetch()]);
      } catch (error) {
        setDetailLogistics(previousLogistics);
        message.error(error instanceof Error ? error.message : '更新物流公司失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [canWriteOrders, detailLogistics, detailOrder?.id, detailQuery, refetch, tenantId],
  );

  const selectedOrderKeySet = useMemo(() => new Set(selectedRowKeys.map((item) => String(item))), [selectedRowKeys]);
  const selectedInMainGridCount = useMemo(
    () =>
      rows.reduce((count, record) => {
        const rowKey = resolveOrderRowKey(record);
        if (!rowKey) return count;
        return selectedOrderKeySet.has(String(rowKey)) ? count + 1 : count;
      }, 0),
    [resolveOrderRowKey, rows, selectedOrderKeySet],
  );
  const mainHeaderChecked = rows.length > 0 && selectedInMainGridCount === rows.length;
  const mainHeaderIndeterminate = selectedInMainGridCount > 0 && selectedInMainGridCount < rows.length;

  const mainGridColDefs = useMemo<ColDef<OrderSummary>[]>(
    () => [
      {
        colId: '__erpSelection',
        headerName: '',
        width: 48,
        minWidth: 48,
        maxWidth: 52,
        pinned: 'left',
        sortable: false,
        filter: false,
        resizable: false,
        lockPinned: true,
        suppressMovable: true,
        checkboxSelection: true,
        headerComponent: () => (
          <label className="order-review-main-selection-header" title="全选 / 取消全选">
            <input
              type="checkbox"
              className="order-review-main-selection-input"
              data-testid="erp-selection-header-checkbox"
              checked={mainHeaderChecked}
              ref={(node) => {
                if (node) {
                  node.indeterminate = mainHeaderIndeterminate;
                }
              }}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                const api = mainGridApiRef.current;
                if (!api) {
                  return;
                }
                if (mainHeaderChecked) {
                  api.deselectAll();
                  return;
                }
                api.forEachNodeAfterFilter((node) => {
                  node.setSelected(true);
                });
              }}
              readOnly
              aria-label="全选 / 取消全选"
            />
          </label>
        ),
      },
      {
        colId: '__erpRowIndex',
        headerName: '序',
        width: 40,
        minWidth: 40,
        maxWidth: 50,
        pinned: 'left',
        sortable: false,
        filter: false,
        resizable: false,
        lockPinned: true,
        suppressMovable: true,
        valueGetter: (params) => Number(params.node?.rowIndex ?? 0) + 1,
      },
      { colId: 'id', headerName: '订单号', field: 'id', minWidth: 160, sortable: false },
      { colId: 'platform_order_id', headerName: '平台单号', field: 'platform_order_id', minWidth: 160, sortable: false },
      {
        colId: 'platform',
        headerName: '平台',
        field: 'platform',
        width: 110,
        sortable: false,
        valueFormatter: (params) => platformLabel(String(params.value ?? '-')),
      },
      { colId: 'shop_id', headerName: '店铺', field: 'shop_id', minWidth: 120, sortable: false },
      { colId: 'receiver_name', headerName: '收件人', field: 'receiver_name', width: 120, sortable: false },
      { colId: 'receiver_phone', headerName: '手机号', field: 'receiver_phone', minWidth: 140, sortable: false },
      { colId: 'logistics_company', headerName: '物流公司', field: 'logistics_company', minWidth: 130, sortable: false },
      {
        colId: 'total_amount_cent',
        headerName: '应收合计',
        field: 'total_amount_cent',
        width: 130,
        sortable: false,
        cellStyle: { textAlign: 'right' },
        valueFormatter: (params) => money((params.value as number | null | undefined) ?? null),
      },
    ],
    [mainHeaderChecked, mainHeaderIndeterminate],
  );

  const mainGridDefaultColDef = useMemo<ColDef<OrderSummary>>(
    () => ({
      filter: true,
      floatingFilter: quickFilterExpanded,
      resizable: true,
      minWidth: 100,
      editable: false,
      sortable: false,
    }),
    [quickFilterExpanded],
  );

  const goodsColumns = useMemo<ErpDataGridColumn<GoodsGridRow>[]>(
    () => [
      { title: '货品行ID', field: 'id', width: 170, headerSort: false },
      { title: '平台SKU', field: 'platform_sku_id', minWidth: 140, headerSort: false },
      { title: '商家SKU', field: 'outer_sku_id', minWidth: 140, headerSort: false },
      { title: '标题', field: 'title', minWidth: 220, headerSort: false },
      {
        title: '数量',
        field: 'quantity',
        width: 90,
        hozAlign: 'right',
        headerSort: false,
      },
      {
        title: '单价',
        field: 'price_cent',
        width: 110,
        hozAlign: 'right',
        headerSort: false,
        renderCell: (value) => money((value as number | null | undefined) ?? null),
      },
    ],
    [],
  );

  const logColumns = useMemo<ErpDataGridColumn<Record<string, unknown>>[]>(
    () => [
      { title: '时间', field: 'created_at', minWidth: 180, headerSort: false },
      { title: '操作', field: 'action', width: 120, headerSort: false },
      { title: '操作者', field: 'operator', width: 120, headerSort: false },
      { title: '详情', field: 'detail', minWidth: 220, headerSort: false },
    ],
    [],
  );

  const leftPanel = useMemo(
    () => (
      <div className="order-review-left-ag-panel" data-testid="order-review-left-ag-panel">
        <Tabs
          size="small"
          activeKey={leftTab}
          onChange={(key) => setLeftTab(key as 'filter' | 'screen')}
          items={[
            {
              key: 'filter',
              label: '过滤',
              children: (
                <div className="order-review-left-ag-filter-tab">
                  <div className="order-review-left-ag-stats-header">
                    <Typography.Text strong>左侧过滤（AG Grid）</Typography.Text>
                    <Button
                      size="small"
                      type="link"
                      disabled={!selectedPlatform.length && !selectedShop.length}
                      onClick={() => setColumnFilters({})}
                    >
                      清空
                    </Button>
                  </div>
                  <Input.Search
                    allowClear
                    size="small"
                    placeholder="搜索平台/店铺"
                    value={leftKeyword}
                    onChange={(event) => setLeftKeyword(event.target.value)}
                  />
                  <div className="ag-theme-quartz order-review-left-ag-grid-shell order-review-left-ag-filter-grid-area">
                    <AgGridReact<OrderReviewAgLeftRenderRow>
                      theme="legacy"
                      rowData={leftRowsForRender}
                      columnDefs={leftColDefs}
                      getRowId={(params) => params.data.key}
                      rowHeight={30}
                      headerHeight={32}
                      sideBar={false}
                      cellSelection={false}
                      enableCellTextSelection={false}
                      onCellClicked={handleLeftClick}
                      getRowClass={(params) => (params.data?.checked ? 'order-review-left-ag-row-checked' : undefined)}
                    />
                  </div>
                  <div className="order-review-left-ag-stats" data-testid="order-review-left-stats">
                    <div className="order-review-left-ag-stats-header">
                      <Typography.Text strong>过滤统计</Typography.Text>
                      <Typography.Text type="secondary" data-testid="order-review-left-stats-total">
                        总订单 {globalOrderTotal}
                      </Typography.Text>
                    </div>
                    <Typography.Text type="secondary">
                      {isGlobalStatsFetching ? '统计更新中…' : shouldFallbackToAll ? '口径：全部订单' : '口径：待复核订单'}
                    </Typography.Text>
                    <div className="order-review-left-ag-stats-body">
                      <div className="order-review-left-ag-stats-pie-wrap">
                        <div
                          className="order-review-left-ag-stats-pie"
                          style={{ backgroundImage: leftStatsPieBackground }}
                          data-testid="order-review-left-stats-pie"
                        >
                          <span className="order-review-left-ag-stats-pie-center">
                            <PieChartOutlined />
                          </span>
                        </div>
                      </div>
                      <div className="order-review-left-ag-stats-legend">
                        {leftStatsItems.map((item) => (
                          <div className="order-review-left-ag-stats-legend-row" key={item.key}>
                            <span
                              className="order-review-left-ag-stats-dot"
                              style={{ backgroundColor: item.color }}
                            />
                            <span className="order-review-left-ag-stats-label">{item.label}</span>
                            <span className="order-review-left-ag-stats-value">{item.value}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              ),
            },
            {
              key: 'screen',
              label: '筛选',
              children: (
                <div className="order-review-left-ag-screen-tab">
                  <div className="order-review-left-ag-stats-header">
                    <Typography.Text strong>筛选条件</Typography.Text>
                  </div>
                  <UiModule08StrategyScreenFilterForm
                    value={leftScreenFiltersDraft}
                    onFieldChange={updateLeftScreenFilter}
                    onApply={applyLeftScreenFilters}
                    onReset={resetLeftScreenFilters}
                  />
                </div>
              ),
            },
          ]}
        />
      </div>
    ),
    [
      applyLeftScreenFilters,
      handleLeftClick,
      leftColDefs,
      leftKeyword,
      leftStatsItems,
      leftStatsPieBackground,
      leftScreenFiltersDraft,
      leftTab,
      resetLeftScreenFilters,
      globalOrderTotal,
      isGlobalStatsFetching,
      rows.length,
      selectedPlatform.length,
      selectedShop.length,
      shouldFallbackToAll,
      leftRowsForRender,
      updateLeftScreenFilter,
    ],
  );

  const rightPanel = useMemo(
    () => (
      <UiModule05DetailPanel
        panels={[
          {
            key: 'quick',
            label: '快捷修改',
            children: (
              <Space direction="vertical" size={8} style={{ width: '100%' }}>
                <Select
                  size="small"
                  allowClear
                  placeholder="发货仓库"
                  options={warehouseOptions}
                  value={detailWarehouseId}
                  loading={detailUpdatingField === 'warehouse'}
                  disabled={detailUpdatingField === 'warehouse' || detailUpdatingField === 'logistics'}
                  onChange={(value) => {
                    void handleDetailWarehouseChange(value ?? undefined);
                  }}
                />
                <Select
                  size="small"
                  allowClear
                  showSearch
                  optionFilterProp="label"
                  placeholder="快递公司"
                  options={courierOptions}
                  value={detailLogistics}
                  loading={detailUpdatingField === 'logistics'}
                  disabled={detailUpdatingField === 'warehouse' || detailUpdatingField === 'logistics'}
                  onChange={(value) => {
                    void handleDetailLogisticsChange(value ?? undefined);
                  }}
                />
              </Space>
            ),
          },
          {
            key: 'detail',
            label: '详情',
            children: (
              <Descriptions size="small" column={1}>
                <Descriptions.Item label="订单号">
                  <span data-testid="order-review-detail-order-id">{detailOrder?.id ?? '-'}</span>
                </Descriptions.Item>
                <Descriptions.Item label="平台单号">{detailOrder?.platform_order_id ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="收件人">{detailOrder?.receiver_name ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="手机">{detailOrder?.receiver_phone ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="应收合计">{money(detailOrder?.total_amount_cent ?? null)}</Descriptions.Item>
              </Descriptions>
            ),
          },
        ]}
      />
    ),
    [
      courierOptions,
      detailUpdatingField,
      detailLogistics,
      detailOrder,
      detailWarehouseId,
      handleDetailLogisticsChange,
      handleDetailWarehouseChange,
      warehouseOptions,
    ],
  );

  const bottomPanel = useMemo(
    () => (
      <div className="order-workbench-bottom-panel">
        <UiModule04BottomTable syncKey={String(activeOrder?.id ?? 'none')}>
          <Tabs
            size="small"
            items={[
              {
                key: 'goods',
                label: '货品子表（AG）',
                children: (
                  <div className="order-review-goods-tab">
                    <div className="order-review-goods-toolbar">
                      <Space size={8}>
                        <Button size="small" onClick={openGoodsCreate} disabled={!activeOrder}>
                          新建
                        </Button>
                        <Button
                          size="small"
                          onClick={() => openGoodsReplace()}
                          disabled={!activeOrder || goodsSelectedRowKeys.length === 0}
                        >
                          替换
                        </Button>
                        <Button
                          size="small"
                          danger
                          onClick={() => void removeSelectedGoods()}
                          disabled={!activeOrder || goodsSelectedRowKeys.length === 0}
                          loading={goodsSaving}
                        >
                          删除
                        </Button>
                      </Space>
                      <Typography.Text type="secondary">
                        已选 {goodsSelectedRowKeys.length} / {goodsRowsState.length}
                      </Typography.Text>
                    </div>
                    <ErpDataGrid<GoodsGridRow>
                      data={goodsRowsState}
                      columns={goodsColumns}
                      rowKey={(record) => record.__localKey}
                      showSelectionColumn
                      selectedRowKeys={goodsSelectedRowKeys}
                      onSelectionChange={(keys) => setGoodsSelectedRowKeys(keys.map((key) => String(key)))}
                      onRowContextMenu={(record, event) => {
                        const key = record.__localKey;
                        if (!goodsSelectedRowKeys.some((item) => String(item) === String(key))) {
                          setGoodsSelectedRowKeys([key]);
                        }
                        setGoodsContextMenu({
                          open: true,
                          position: { x: event.clientX, y: event.clientY },
                          row: record,
                        });
                      }}
                      sidePanelConfig={{
                        enabled: true,
                        defaultOpen: false,
                        tabs: { columns: true, filters: true },
                        suppressRowGroups: true,
                        suppressValues: true,
                        suppressPivots: true,
                        suppressPivotMode: true,
                      }}
                      rowHeight={28}
                      headerHeight={32}
                      suppressContextMenu
                      enableCellTextSelection={false}
                      emptyText="暂无货品"
                      dataTestId="order-review-goods-grid"
                    />
                  </div>
                ),
              },
              {
                key: 'logs',
                label: '日志子表（AG）',
                children: (
                  <ErpDataGrid<Record<string, unknown>>
                    data={logRows}
                    columns={logColumns}
                    rowKey={(record) =>
                      String(record.audit_id ?? record.id ?? `${record.created_at ?? ''}-${record.action ?? ''}`)
                    }
                    sidePanelConfig={{
                      enabled: true,
                      defaultOpen: false,
                      tabs: { columns: true, filters: true },
                      suppressRowGroups: true,
                      suppressValues: true,
                      suppressPivots: true,
                      suppressPivotMode: true,
                    }}
                    rowHeight={28}
                    headerHeight={32}
                    suppressContextMenu
                    enableCellTextSelection={false}
                    emptyText="暂无日志"
                  />
                ),
              },
            ]}
          />
        </UiModule04BottomTable>
      </div>
    ),
    [
      activeOrder,
      goodsColumns,
      goodsRowsState,
      goodsSaving,
      goodsSelectedRowKeys,
      logColumns,
      logRows,
      openGoodsCreate,
      openGoodsReplace,
      removeSelectedGoods,
    ],
  );

  useEffect(() => {
    setLeftPanelVisible(true);
    setLeftPanelWidth(200);
    return () => {
      setLeftPanel(undefined);
      setLeftPanelVisible(false);
    };
  }, [setLeftPanel, setLeftPanelVisible, setLeftPanelWidth]);

  useEffect(() => {
    setLeftPanel(leftPanel);
  }, [leftPanel, setLeftPanel]);

  useEffect(() => {
    const disableBrowserContextMenu = (event: MouseEvent) => {
      event.preventDefault();
    };
    window.addEventListener('contextmenu', disableBrowserContextMenu, true);
    return () => {
      window.removeEventListener('contextmenu', disableBrowserContextMenu, true);
    };
  }, []);

  useEffect(() => {
    setRightPanelVisible(true);
    return () => {
      setRightPanel(undefined);
      setRightPanelVisible(false);
    };
  }, [setRightPanel, setRightPanelVisible]);

  useEffect(() => {
    setRightPanel(rightPanel);
  }, [rightPanel, setRightPanel]);

  useEffect(() => {
    setBottomPanelVisible(true);
    return () => {
      setBottomPanel(undefined);
      setBottomPanelVisible(false);
    };
  }, [setBottomPanel, setBottomPanelVisible]);

  useEffect(() => {
    setBottomPanel(bottomPanel);
  }, [bottomPanel, setBottomPanel]);

  const mainContextMenuConfig = useMemo<MenuProps>(
    () => ({
      items: [
        {
          key: 'force-review',
          label: '强审通过',
          disabled: actionOrderIds.length === 0 || actionLoading.force || actionHasInvalidGoods,
        },
        {
          key: 'split',
          label: '拆单',
          disabled: actionOrderIds.length === 0 || actionLoading.split,
        },
        {
          key: 'merge',
          label: '合单',
          disabled: actionOrderIds.length < 2 || actionLoading.merge,
        },
        { type: 'divider' },
        {
          key: 'clear-selection',
          label: '清空勾选',
          disabled: selectedRowKeys.length === 0,
        },
      ],
      onClick: ({ key }) => {
        setMainContextMenu({ open: false, position: null, row: null });
        if (key === 'force-review') {
          void performForceReview();
          return;
        }
        if (key === 'split') {
          openSplitModal();
          return;
        }
        if (key === 'merge') {
          void performMergeOrders();
          return;
        }
        if (key === 'clear-selection') {
          setSelectedRowKeys([]);
        }
      },
    }),
    [
      actionLoading.force,
      actionLoading.merge,
      actionLoading.split,
      actionHasInvalidGoods,
      actionOrderIds.length,
      openSplitModal,
      performForceReview,
      performMergeOrders,
      selectedRowKeys.length,
    ],
  );

  const goodsContextMenuConfig = useMemo<MenuProps>(
    () => ({
      items: [
        { key: 'create', label: '新建货品' },
        {
          key: 'replace',
          label: '替换货品',
          disabled: goodsSelectedRowKeys.length === 0,
        },
        {
          key: 'delete',
          label: '删除货品',
          disabled: goodsSelectedRowKeys.length === 0,
        },
      ],
      onClick: ({ key }) => {
        setGoodsContextMenu({ open: false, position: null, row: null });
        if (key === 'create') {
          openGoodsCreate();
          return;
        }
        if (key === 'replace') {
          openGoodsReplace(goodsContextMenu.row?.__localKey ?? null);
          return;
        }
        if (key === 'delete') {
          void removeSelectedGoods();
        }
      },
    }),
    [
      goodsContextMenu.row?.__localKey,
      goodsSelectedRowKeys.length,
      openGoodsCreate,
      openGoodsReplace,
      removeSelectedGoods,
    ],
  );

  const tableRef = useRef<HTMLDivElement | null>(null);

  const loadTimingText = isFetching ? '加载中' : `加载耗时 ${lastFetchDurationMs ?? 0}ms`;
  const summary = (
    <>
      总数 {rows.length} 条，已选 {selectedRowKeys.length} 条，已选金额 ¥{moneyValue(selectedAmountCent / 100)}
      ，当前页金额 ¥{moneyValue(pageAmountCent / 100)}，
      <span data-testid="order-review-load-timing">{loadTimingText}</span>
      <span data-testid="order-review-load-cycle" style={{ display: 'none' }}>
        {loadCycle}
      </span>
      <span data-testid="order-review-detail-loaded-id" style={{ display: 'none' }}>
        {detailOrder?.id ?? '-'}
      </span>
    </>
  );

  return (
    <div className="order-workbench-page order-review-page" data-testid="order-review-page">
      <div className="order-workbench-toolbar">
        <div className="order-workbench-toolbar-left">
          <Button
            data-testid="order-review-approve"
            size="small"
            type="primary"
            onClick={() => void performReviewOrders()}
            loading={actionLoading.review}
            disabled={actionOrderIds.length > 0 && actionHasInvalidGoods}
          >
            审核
          </Button>
          <Button
            data-testid="order-review-split"
            size="small"
            onClick={openSplitModal}
            loading={actionLoading.split}
          >
            拆单
          </Button>
          <Button
            data-testid="order-review-merge"
            size="small"
            onClick={() => void performMergeOrders()}
            loading={actionLoading.merge}
          >
            合单
          </Button>
          <Button
            size="small"
            icon={<FilterOutlined />}
            onClick={() => setQuickFilterExpanded((prev) => !prev)}
          >
            {quickFilterExpanded ? '收起列筛选' : '展开列筛选'}
          </Button>
        </div>
        <div className="order-workbench-toolbar-center">
          <Space size={8}>
            <Input
              size="small"
              className="order-review-toolbar-search"
              data-testid="order-review-toolbar-search-input"
              placeholder="搜索订单"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              onPressEnter={() => setSearchKeyword(searchInput.trim())}
            />
            <Button
              size="small"
              type="primary"
              icon={<SearchOutlined />}
              data-testid="order-review-toolbar-search-trigger"
              onClick={() => setSearchKeyword(searchInput.trim())}
            >
              查询
            </Button>
          </Space>
        </div>
      </div>

      <div
        className="order-workbench-table order-review-table order-workbench-table-module-03 order-workbench-table-with-footer"
        ref={tableRef}
      >
        <div className="order-review-grid-main">
          <div className="ag-theme-quartz order-review-main-grid-shell" data-testid="order-review-data-grid">
            {error ? (
              <div className="order-review-main-grid-state">
                <Alert
                  type="error"
                  showIcon
                  message="订单加载失败"
                  description={error instanceof Error ? error.message : '请稍后重试'}
                  action={
                    <Button size="small" onClick={() => refetch()}>
                      重试
                    </Button>
                  }
                />
              </div>
            ) : (
              <AgGridReact<OrderSummary>
                theme="legacy"
                rowData={rows}
                columnDefs={mainGridColDefs}
                defaultColDef={mainGridDefaultColDef}
                getRowId={getMainGridRowId}
                rowSelection={{
                  mode: 'multiRow',
                  checkboxes: true,
                  headerCheckbox: false,
                  enableClickSelection: false,
                }}
                suppressRowClickSelection
                rowHeight={28}
                headerHeight={32}
                animateRows={false}
                cellSelection
                suppressContextMenu
                enableCellTextSelection={false}
                rowBuffer={10}
                suppressRowVirtualisation={false}
                suppressColumnVirtualisation={false}
                suppressScrollOnNewData
                debounceVerticalScrollbar
                onGridReady={(event: GridReadyEvent<OrderSummary>) => {
                  mainGridApiRef.current = event.api;
                }}
                onSelectionChanged={handleMainSelectionChanged}
                onRowClicked={handleMainRowClick}
                onCellClicked={handleMainCellClicked}
                onCellContextMenu={handleMainCellContextMenu}
                getRowClass={(params) => {
                  const row = params.data;
                  if (!row) return undefined;
                  const rowKey = resolveOrderRowKey(row);
                  if (!rowKey) return undefined;
                  if (selectedRowKeys.length > 0) return undefined;
                  return rowKey === focusedOrderKey ? 'order-review-main-row-active' : undefined;
                }}
              />
            )}

            {!error && !isFetching && rows.length === 0 ? (
              <div className="order-review-main-grid-empty">
                <Empty description="暂无订单" />
              </div>
            ) : null}

            {isFetching ? (
              <div className="order-review-main-grid-loading-mask">
                <Spin size="large" tip="加载中..." />
              </div>
            ) : null}
          </div>
        </div>

        <UiModule03TableFooter
          tableContainerRef={tableRef}
          syncKey={`order-review-${total}`}
          summary={summary}
          total={total}
          pagination={pagination}
          showScrollbarRow={false}
          onPaginationChange={(next) =>
            setPagination((prev) => ({
              ...prev,
              current: next.current ?? 1,
              pageSize: next.pageSize ?? prev.pageSize,
            }))
          }
        />
      </div>

      <UiModule03ContextMenu
        open={mainContextMenu.open}
        position={mainContextMenu.position}
        menu={mainContextMenuConfig}
        onClose={() => setMainContextMenu({ open: false, position: null, row: null })}
      />

      <UiModule03ContextMenu
        open={goodsContextMenu.open}
        position={goodsContextMenu.position}
        menu={goodsContextMenuConfig}
        onClose={() => setGoodsContextMenu({ open: false, position: null, row: null })}
      />

      <Modal
        title="拆单设置"
        open={splitModalOpen}
        onCancel={() => setSplitModalOpen(false)}
        onOk={() => void performSplitOrder()}
        confirmLoading={actionLoading.split}
        okText="确认拆单"
        cancelText="取消"
      >
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            将使用当前勾选的第一单进行拆分，拆分份数支持 2-10。
          </Typography.Text>
          <InputNumber
            min={2}
            max={10}
            precision={0}
            value={splitCount}
            onChange={(value) => setSplitCount(Number(value ?? 2))}
            style={{ width: '100%' }}
          />
        </Space>
      </Modal>

      <Modal
        title={goodsEditorMode === 'create' ? '新建货品' : '替换货品'}
        open={goodsEditorOpen}
        onCancel={() => setGoodsEditorOpen(false)}
        onOk={() => void submitGoodsEditor()}
        confirmLoading={goodsSaving}
        okText="保存"
        cancelText="取消"
      >
        <Form form={goodsForm} layout="vertical" initialValues={{ quantity: 1, price_cent: 0 }}>
          <Form.Item label="平台 SKU" name="platform_sku_id">
            <Input size="small" placeholder="平台SKU" />
          </Form.Item>
          <Form.Item label="商家 SKU" name="outer_sku_id">
            <Input size="small" placeholder="商家SKU" />
          </Form.Item>
          <Form.Item
            label="货品标题"
            name="title"
            rules={[{ required: true, message: '请输入货品标题' }]}
          >
            <Input size="small" placeholder="货品标题" />
          </Form.Item>
          <Form.Item
            label="数量"
            name="quantity"
            rules={[{ required: true, message: '请输入数量' }]}
          >
            <InputNumber size="small" min={1} precision={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label="单价（分）" name="price_cent">
            <InputNumber size="small" min={0} precision={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

