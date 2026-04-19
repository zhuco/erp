import {
  Alert,
  Button,
  Card,
  Checkbox,
  Descriptions,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import type { MenuProps, SelectProps } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useRef, useState, type Dispatch, type ReactNode, type SetStateAction } from 'react';
import { useQueries, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import { useOrdersList, useSkusList } from '@/api/hooks';
import { createExportJob } from '@/api/exports';
import { listOmsOrderShipments, type OmsShipmentSummary } from '@/api/omsShipments';
import { listCouriers } from '@/api/couriers';
import {
  addOrderNote,
  addOrderTags,
  getOrderDetail,
  handleOrderException,
  rematchOrderItems,
  updateOrder,
  updateOrderReceiver,
} from '@/api/orders';
import type { OrderDetail, OrderListQuery, OrderSummary } from '@/api/orders';
import { listWarehouses } from '@/api/warehouses';
import {
  listShipmentPackages,
  upsertShipmentPackages,
  type ShipmentPackage,
} from '@/api/shipments';
import type { MappingPayload } from '@/api/mappings';
import { createMapping } from '@/api/mappings';
import type { Sku } from '@/api/skus';
import { listOrderTagDefinitions } from '@/api/orderTagDefinitions';
import {
  UiModule04BottomTable,
  UiModule05DetailPanel,
  UiModule05OrderReviewRightPanelTemplate,
  type UiModule08OrderReviewLeftStatsItem,
} from '@/components/ui-modules';
import { OrderTagBadges } from '@/components/OrderTagBadges';
import { PlatformTag } from '@/components/PlatformTag';
import { buildOrderQuickFilterCells } from '@/components/ErpDataGrid';
import type { ErpDataGridNextColumn } from '@/components/ErpDataGridNext';
import { resolveOrderDisplayTags } from '@/features/orderTags';
import {
  OrderReviewWorkbenchTemplate,
  useWorkbenchSavedViewState,
  OrderReviewWorkbenchToolbarTemplate,
  useWorkbenchTablePagination,
} from '@/components/workbench/templates';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';
import { fetchAllOrdersByScan } from '@/pages/orders/shared/fetchAllOrdersByScan';
import {
  toOrderWorkbenchGoodsRows,
  type OrderWorkbenchGoodsRow,
} from '@/pages/orders/shared/orderWorkbenchGoodsRows';
import { useLeftFilterModule, type LeftFilterPanelProps } from '@/pages/orders/shared/useLeftFilterModule';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { buildCourierOptionLabel, resolveCourierOptionValue } from './shared/courierDisplay';
import { OrderWorkbenchMainGridEntry } from './shared/OrderWorkbenchMainGridEntry';
import { OrderQueryLeftPanel } from './components/OrderQueryLeftPanel';
import {
  buildOrderQueryLeftCheckedKeys,
  hasOrderQueryLeftManualFilter,
  reduceOrderQueryLeftFiltersByNode,
  type OrderQueryLeftFilters,
} from './components/orderQueryLeftInteractions';
import {
  OrderQueryToolbarLeft,
} from './components/OrderQueryToolbar';
import { buildOrderManagementColumns } from './components/orderManagementColumns';

import './orderQueryPage.css';

type OrdersPageDefaults = Partial<OrderListQuery> & {
  exception_flags?: string[] | string;
};

type OrdersPageProps = {
  title?: string;
  defaultFilters?: OrdersPageDefaults;
  savedViewKey?: string;
  enableExceptionActions?: boolean;
  headerExtra?: ReactNode;
};

type OrderSummaryExtras = {
  erp_order_no?: string;
  shop_name?: string;
  warehouse_name?: string;
  receiver_address?: string;
  customer_note?: string | null;
  remark?: string | null;
  order_source?: string | null;
  items?: Array<{
    title?: string | null;
    quantity?: number | null;
    platform_sku_id?: string | null;
    outer_sku_id?: string | null;
    id?: string | null;
  }>;
  item_count?: number | null;
  goods_count?: number | null;
  goods_total_qty?: number | null;
  platform_fee_cent?: number | null;
  shop_fee_cent?: number | null;
  shipping_fee_cent?: number | null;
  goods_cost_cent?: number | null;
  estimated_postage_cent?: number | null;
  logistics_company?: string | null;
};

type FilterValues = {
  time_range: string;
  archive_status?: string;
  platform?: PlatformCode;
};

type ColumnFilters = {
  trackingNo?: string;
  platformOrderId?: string;
  receiverAddress?: string;
  itemKeyword?: string;
  processedRange?: [dayjs.Dayjs, dayjs.Dayjs];
};

type PersistedColumnFilters = {
  trackingNo?: string;
  platformOrderId?: string;
  receiverAddress?: string;
  itemKeyword?: string;
  processedRange?: [string, string];
};

type ColumnFilterFieldKey = keyof ColumnFilters;

const columnFilterFieldKeys: ColumnFilterFieldKey[] = [
  'trackingNo',
  'platformOrderId',
  'receiverAddress',
  'itemKeyword',
  'processedRange',
];

const isColumnFilterValueActive = (
  key: ColumnFilterFieldKey,
  value: ColumnFilters[ColumnFilterFieldKey] | undefined,
) => {
  if (key === 'processedRange') {
    return Array.isArray(value) && value.length === 2 && value[0] != null && value[1] != null;
  }
  return typeof value === 'string' && value.trim().length > 0;
};

const hasActiveColumnFilters = (filters: ColumnFilters) =>
  columnFilterFieldKeys.some((key) =>
    isColumnFilterValueActive(key, filters[key] as ColumnFilters[ColumnFilterFieldKey]),
  );

const buildSingleColumnFilters = (
  key: ColumnFilterFieldKey,
  value: ColumnFilters[ColumnFilterFieldKey] | undefined,
): ColumnFilters => {
  if (!isColumnFilterValueActive(key, value)) {
    return {};
  }
  return { [key]: value } as ColumnFilters;
};

const columnTextFilterKeys = ['trackingNo', 'platformOrderId', 'receiverAddress', 'itemKeyword'] as const;
type ColumnTextFilterKey = (typeof columnTextFilterKeys)[number];

const serializeColumnFilters = (filters: ColumnFilters): PersistedColumnFilters | undefined => {
  const state: PersistedColumnFilters = {};

  columnTextFilterKeys.forEach((key) => {
    const value = filters[key];
    if (typeof value === 'string' && value.trim()) {
      state[key] = value;
    }
  });

  if (
    Array.isArray(filters.processedRange) &&
    filters.processedRange.length === 2 &&
    filters.processedRange[0] &&
    filters.processedRange[1]
  ) {
    state.processedRange = [
      filters.processedRange[0].toISOString(),
      filters.processedRange[1].toISOString(),
    ];
  }

  return Object.keys(state).length > 0 ? state : undefined;
};

const deserializeColumnFilters = (
  state: PersistedColumnFilters | undefined,
): ColumnFilters | undefined => {
  if (!state) {
    return undefined;
  }

  const filters: ColumnFilters = {};
  columnTextFilterKeys.forEach((key) => {
    const value = state[key];
    if (typeof value === 'string' && value.trim()) {
      filters[key] = value;
    }
  });

  const range = state.processedRange;
  if (Array.isArray(range) && range.length === 2) {
    const start = dayjs(range[0]);
    const end = dayjs(range[1]);
    if (start.isValid() && end.isValid()) {
      filters.processedRange = [start, end];
    }
  }

  return Object.keys(filters).length > 0 ? filters : undefined;
};

type ExtendedOrderListQuery = OrderListQuery & {
  archive_status?: string;
  tracking_no?: string;
  platform_order_id?: string;
  receiver_address?: string;
  item_keyword?: string;
};

type TreeFilter = {
  platform?: PlatformCode;
  oms_status?: OrderSummary['oms_status'];
  exception_flags?: string[];
  tagged?: boolean;
};

type ReceiverFormValues = {
  receiver_name: string;
  receiver_phone: string;
  receiver_address: string;
};

type PackageFormValues = {
  packages: Array<{
    package_no: string;
    tracking_no?: string;
    carrier_code?: string;
    weight_gram?: number;
  }>;
};

type DetailSelectOption = NonNullable<SelectProps['options']>;

type SavedViewState = {
  filters?: FilterValues;
  treeSelection?: string[];
  searchValue?: string;
  searchKeyword?: string;
  columnFilters?: PersistedColumnFilters;
  columnFilterDrafts?: PersistedColumnFilters;
};

const timeRangeOptions = [
  { label: '近一个月', value: 'last_30_days', days: 30 },
  { label: '近七天', value: 'last_7_days', days: 7 },
  { label: '近三个月', value: 'last_90_days', days: 90 },
];

const orderQueryPageSizeOptions = ['20', '50', '100', '200'];

const archiveStatusOptions = [
  { label: '未归档数据', value: 'ACTIVE' },
  { label: '已归档数据', value: 'ARCHIVED' },
];

const defaultFilterValues: FilterValues = {
  time_range: 'last_30_days',
  archive_status: 'ACTIVE',
  platform: undefined,
};

const exceptionFlagDefaults = [
  'ADDRESS_INVALID',
  'INVENTORY_SHORTAGE',
  'SHIPPING_FAILED',
  'MAPPING_MISSING',
  'REVIEW_FLAGGED',
  'ORDER_FROZEN',
  'ORDER_MERGED',
];

const orderStatusMeta: Record<OrderSummary['oms_status'], { label: string; color: string }> = {
  NEW: { label: '新建', color: 'blue' },
  REVIEW_PENDING: { label: '待复核', color: 'orange' },
  PENDING_SHIPMENT: { label: '待发货', color: 'cyan' },
  ALLOCATED: { label: '已分配', color: 'geekblue' },
  PICKING: { label: '拣货中', color: 'purple' },
  PACKED: { label: '已打包', color: 'gold' },
  SHIPPED: { label: '已发货', color: 'green' },
  DONE: { label: '已完成', color: 'default' },
  CANCELLED: { label: '已取消', color: 'default' },
};

const orderQueryStatusKeys: Array<OrderSummary['oms_status']> = [
  'NEW',
  'REVIEW_PENDING',
  'PENDING_SHIPMENT',
  'CANCELLED',
];
const orderQueryDefaultOmsStatus = orderQueryStatusKeys.join(',');

const orderStatusOptions = orderQueryStatusKeys.map((value) => ({
  value,
  label: orderStatusMeta[value]?.label ?? value,
}));

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;
const formatDateTime = (value?: string | null) => (value ? new Date(value).toLocaleString() : '-');
const now = () =>
  typeof performance !== 'undefined' && typeof performance.now === 'function'
    ? performance.now()
    : Date.now();

const formatDuration = (from?: string | null) => {
  if (!from) {
    return '-';
  }
  const start = dayjs(from);
  if (!start.isValid()) {
    return '-';
  }
  const diffMs = Math.max(dayjs().diff(start), 0);
  const totalMinutes = Math.floor(diffMs / 60000);
  const days = Math.floor(totalMinutes / (60 * 24));
  const hours = Math.floor((totalMinutes - days * 24 * 60) / 60);
  const minutes = totalMinutes % 60;
  if (days > 0) {
    return `${days}天${hours}小时`;
  }
  if (hours > 0) {
    return `${hours}小时${minutes}分钟`;
  }
  return `${minutes}分钟`;
};

const truncateText = (value: string, maxLength = 12) => {
  const chars = Array.from(value);
  if (chars.length <= maxLength) {
    return value;
  }
  return `${chars.slice(0, maxLength).join('')}...`;
};

const resolveInvalidReason = (item: OrderDetail['items'][number]) => {
  if (!item.outer_sku_id) {
    return '平台未提供商家编码';
  }
  return 'ERP货品档案缺少关键编码或条码';
};

const resolveTimeRange = (value: string) => {
  const target = timeRangeOptions.find((option) => option.value === value);
  if (!target) {
    return undefined;
  }
  const end = dayjs();
  const start = end.subtract(target.days, 'day');
  return [start, end] as const;
};

const resolveTreeFilter = (key: string): TreeFilter => {
  if (key.startsWith('status-')) {
    return { oms_status: key.replace('status-', '') as OrderSummary['oms_status'] };
  }
  if (key.startsWith('channel-')) {
    return { platform: key.replace('channel-', '') as PlatformCode };
  }
  if (key === 'exception') {
    return { exception_flags: exceptionFlagDefaults };
  }
  if (key === 'invalid-goods') {
    return { exception_flags: ['MAPPING_MISSING'] };
  }
  if (key === 'tagged') {
    return { tagged: true };
  }
  if (key === 'untagged') {
    return { tagged: false };
  }
  return {};
};

const resolveOrderFlagSet = (record: OrderSummary) =>
  new Set((record.exception_flags ?? []).map((flag) => String(flag).trim().toUpperCase()).filter(Boolean));

const resolveHasOrderTag = (record: OrderSummary) => {
  const extra = record as OrderSummary & OrderSummaryExtras & { tags?: string[] };
  const tags = extra.tags ?? [];
  return tags.some((item) => String(item ?? '').trim().length > 0);
};

const matchesOrderQueryLeftFilter = (filterId: string, record: OrderSummary) => {
  if (filterId === 'all') {
    return true;
  }
  const flags = resolveOrderFlagSet(record);
  if (filterId === 'exception') {
    return flags.size > 0;
  }
  if (filterId === 'invalid-goods') {
    return flags.has('MAPPING_MISSING');
  }
  if (filterId.startsWith('status-')) {
    return String(record.oms_status ?? '').trim() === filterId.slice('status-'.length);
  }
  if (filterId.startsWith('channel-')) {
    return String(record.platform ?? '').trim() === filterId.slice('channel-'.length);
  }
  if (filterId === 'tagged') {
    return resolveHasOrderTag(record);
  }
  if (filterId === 'untagged') {
    return !resolveHasOrderTag(record);
  }
  return false;
};

const normalizeExceptionFlags = (value?: string | string[]) => {
  if (!value) {
    return undefined;
  }
  if (Array.isArray(value)) {
    return value.map((flag) => flag.trim()).filter(Boolean);
  }
  return value
    .split(',')
    .map((flag) => flag.trim())
    .filter(Boolean);
};

const resolveInitialTreeSelection = (defaults?: OrdersPageDefaults) => {
  if (!defaults) {
    return ['all'];
  }
  if (defaults.exception_flags && normalizeExceptionFlags(defaults.exception_flags)?.length) {
    return ['exception'];
  }
  if (typeof defaults.tagged === 'boolean') {
    return [defaults.tagged ? 'tagged' : 'untagged'];
  }
  if (defaults.platform) {
    return [`channel-${defaults.platform}`];
  }
  if (defaults.oms_status) {
    return [`status-${defaults.oms_status}`];
  }
  return ['all'];
};

export const OrderQueryPage = ({
  defaultFilters,
  savedViewKey = 'orders.list.savedView',
  enableExceptionActions = false,
  headerExtra,
}: OrdersPageProps) => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const canWriteOrders = useCan('orders.write');
  const canManageMappings = useCan('channel_sku_mappings.write');
  const canExport = useCan('exports.write');
  const canWriteShipping = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [receiverForm] = Form.useForm<ReceiverFormValues>();
  const [packageForm] = Form.useForm<PackageFormValues>();

  const [filters, setFilters] = useState<FilterValues>(defaultFilterValues);
  const [columnFilters, setColumnFilters] = useState<ColumnFilters>({});
  const [columnFilterDrafts, setColumnFilterDrafts] = useState<ColumnFilters>({});
  const { pagination, setPagination, resetPagination, handlePaginationChange: applyPaginationChange } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions: orderQueryPageSizeOptions,
  });
  const [treeSelection, setTreeSelection] = useState<string[]>(
    resolveInitialTreeSelection(defaultFilters),
  );
  const leftFilters = useMemo<OrderQueryLeftFilters>(
    () => ({ selectedFilterId: treeSelection[0] ?? 'all' }),
    [treeSelection],
  );
  const clearColumnFiltersForLeftSelection = useCallback(() => {
    setColumnFilters((prev) => (hasActiveColumnFilters(prev) ? {} : prev));
    setColumnFilterDrafts((prev) => (hasActiveColumnFilters(prev) ? {} : prev));
  }, []);
  const setLeftFilters = useCallback<Dispatch<SetStateAction<OrderQueryLeftFilters>>>(
    (action) => {
      setTreeSelection((prev) => {
        const previous = { selectedFilterId: prev[0] ?? 'all' };
        const next = typeof action === 'function' ? action(previous) : action;
        const nextId = String(next.selectedFilterId ?? '').trim() || 'all';
        const prevId = previous.selectedFilterId ?? 'all';
        if (nextId === prevId) {
          return prev;
        }
        clearColumnFiltersForLeftSelection();
        resetPagination();
        return [nextId];
      });
    },
    [clearColumnFiltersForLeftSelection, resetPagination],
  );
  const [searchValue, setSearchValue] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [selectedOrders, setSelectedOrders] = useState<OrderSummary[]>([]);
  const [activeOrder, setActiveOrder] = useState<OrderSummary | null>(null);
  const [tagModalOpen, setTagModalOpen] = useState(false);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [orderTagsApplying, setOrderTagsApplying] = useState(false);
  const [noteModalOpen, setNoteModalOpen] = useState(false);
  const [noteInput, setNoteInput] = useState('');
  const [receiverModalOpen, setReceiverModalOpen] = useState(false);
  const [receiverSaving, setReceiverSaving] = useState(false);
  const [detailWarehouseId, setDetailWarehouseId] = useState<string | undefined>(undefined);
  const [detailLogisticsCompany, setDetailLogisticsCompany] = useState<string | undefined>(undefined);
  const [detailUpdatingField, setDetailUpdatingField] = useState<'warehouse' | 'logistics' | null>(null);
  const [exporting, setExporting] = useState(false);
  const [exceptionProcessing, setExceptionProcessing] = useState(false);
  const [activeBottomTab, setActiveBottomTab] = useState('goods');
  const [packageEditOpen, setPackageEditOpen] = useState(false);
  const [packageEditingShipment, setPackageEditingShipment] = useState<OmsShipmentSummary | null>(
    null,
  );
  const [packageSaving, setPackageSaving] = useState(false);
  const [matchModalOpen, setMatchModalOpen] = useState(false);
  const [matchSaving, setMatchSaving] = useState(false);
  const [matchItemId, setMatchItemId] = useState<string | null>(null);
  const [matchSkuId, setMatchSkuId] = useState<string | null>(null);
  const [matchApplyAll, setMatchApplyAll] = useState(true);
  const [matchTargetOrderIds, setMatchTargetOrderIds] = useState<string[]>([]);
  const [skuKeyword, setSkuKeyword] = useState('');
  const tableContainerRef = useRef<HTMLDivElement | null>(null);
  const fetchStartedAtRef = useRef<number | null>(null);
  const [lastFetchDurationMs, setLastFetchDurationMs] = useState<number | null>(null);

  const treeFilter = useMemo(() => resolveTreeFilter(treeSelection[0] ?? 'all'), [treeSelection]);

  const baseExceptionFlags = useMemo(
    () => normalizeExceptionFlags(defaultFilters?.exception_flags),
    [defaultFilters?.exception_flags],
  );

  useEffect(() => {
    setPagination((prev) => {
      if ((prev.current ?? 1) === 1) {
        return prev;
      }
      return { ...prev, current: 1 };
    });
  }, [columnFilters]);

  const baseScopeQuery = useMemo<ExtendedOrderListQuery>(() => {
    const range = columnFilters.processedRange ?? resolveTimeRange(filters.time_range);
    const exceptionFlags = baseExceptionFlags;
    const resolvedPlatform = filters.platform ?? defaultFilters?.platform;
    const resolvedOmsStatus = defaultFilters?.oms_status ?? orderQueryDefaultOmsStatus;
    return {
      platform: resolvedPlatform,
      oms_status: resolvedOmsStatus,
      exception_flags: exceptionFlags?.length ? exceptionFlags.join(',') : undefined,
      tagged: defaultFilters?.tagged,
      review_status: defaultFilters?.review_status,
      shop_id: defaultFilters?.shop_id,
      created_from: range?.[0]?.toISOString(),
      created_to: range?.[1]?.toISOString(),
      keyword: searchKeyword.trim() || undefined,
      tracking_no: columnFilters.trackingNo?.trim() || undefined,
      platform_order_id: columnFilters.platformOrderId?.trim() || undefined,
      receiver_address: columnFilters.receiverAddress?.trim() || undefined,
      item_keyword: columnFilters.itemKeyword?.trim() || undefined,
      archive_status: filters.archive_status,
    };
  }, [
    baseExceptionFlags,
    columnFilters.itemKeyword,
    columnFilters.platformOrderId,
    columnFilters.processedRange,
    columnFilters.receiverAddress,
    columnFilters.trackingNo,
    defaultFilters?.oms_status,
    defaultFilters?.platform,
    defaultFilters?.review_status,
    defaultFilters?.shop_id,
    defaultFilters?.tagged,
    filters.archive_status,
    filters.platform,
    filters.time_range,
    searchKeyword,
  ]);

  const query = useMemo<ExtendedOrderListQuery>(() => {
    const exceptionFlags = baseExceptionFlags ?? treeFilter.exception_flags;
    const resolvedPlatform = filters.platform ?? treeFilter.platform ?? defaultFilters?.platform;
    const resolvedOmsStatus =
      defaultFilters?.oms_status ?? treeFilter.oms_status ?? orderQueryDefaultOmsStatus;

    return {
      ...baseScopeQuery,
      platform: resolvedPlatform,
      oms_status: resolvedOmsStatus,
      exception_flags: exceptionFlags?.length ? exceptionFlags.join(',') : undefined,
      tagged: defaultFilters?.tagged ?? treeFilter.tagged,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20,
    };
  }, [
    baseExceptionFlags,
    baseScopeQuery,
    defaultFilters?.oms_status,
    defaultFilters?.platform,
    defaultFilters?.tagged,
    pagination.current,
    pagination.pageSize,
    treeFilter.oms_status,
    treeFilter.platform,
    treeFilter.tagged,
  ]);

  const { data, isFetching, error, refetch } = useOrdersList({
    query: query as OrderListQuery,
  });
  const {
    data: leftScopeRows = [],
    isFetching: isLeftScopeFetching,
  } = useQuery({
    queryKey: ['orders-query', tenantId, 'left-scope-scan', baseScopeQuery],
    queryFn: ({ signal }) => fetchAllOrdersByScan(tenantId, baseScopeQuery as OrderListQuery, { signal }),
    enabled: Boolean(tenantId),
    staleTime: 20_000,
  });
  const total = data?.total ?? 0;
  const listItems = data?.items ?? [];
  useEffect(() => {
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
  }, [isFetching]);
  const rowOffset = useMemo(
    () => ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
    [pagination.current, pagination.pageSize],
  );
  const tableLoading = isFetching && listItems.length === 0;
  const orderIds = useMemo(() => listItems.map((item) => item.id), [listItems]);
  const currentPageKeys = useMemo(() => listItems.map((item) => item.id), [listItems]);
  const currentPageKeySet = useMemo(() => new Set(currentPageKeys), [currentPageKeys]);
  const allCurrentSelected = useMemo(
    () =>
      currentPageKeys.length > 0 && currentPageKeys.every((key) => selectedRowKeys.includes(key)),
    [currentPageKeys, selectedRowKeys],
  );
  useEffect(() => {
    if (!listItems.length) {
      if (selectedRowKeys.length) {
        setSelectedRowKeys([]);
        setSelectedOrders([]);
      }
      if (activeOrder) {
        setActiveOrder(null);
      }
      return;
    }
    const nextKeys = selectedRowKeys.filter((key) => currentPageKeySet.has(key));
    if (nextKeys.length !== selectedRowKeys.length) {
      setSelectedRowKeys(nextKeys);
    }
    const nextOrders = listItems.filter((item) => nextKeys.includes(item.id));
    const sameOrders =
      nextOrders.length === selectedOrders.length &&
      nextOrders.every((item) => selectedOrders.some((order) => order.id === item.id));
    if (!sameOrders) {
      setSelectedOrders(nextOrders);
    }
    if (!activeOrder || !currentPageKeySet.has(activeOrder.id)) {
      setActiveOrder(listItems[0]);
    }
  }, [activeOrder, currentPageKeySet, listItems, selectedOrders, selectedRowKeys]);

  const detailQueries = useQueries({
    queries: orderIds.map((orderId) => ({
      queryKey: ['order-detail', tenantId, orderId],
      queryFn: () => getOrderDetail(tenantId, orderId),
      enabled: Boolean(tenantId && orderId),
      staleTime: 60_000,
    })),
  });
  const detailRevision = detailQueries
    .map((queryResult) => queryResult.dataUpdatedAt ?? 0)
    .join('|');
  const { data: skuData, isLoading: skuLoading } = useSkusList({
    pageSize: 200,
    keyword: skuKeyword.trim() || undefined,
    is_active: true,
  });
  const skuOptions = useMemo(() => skuData?.data ?? [], [skuData?.data]);
  const orderTagDefinitionsQuery = useQuery({
    queryKey: ['order-tag-definitions', tenantId, 'options'],
    queryFn: () =>
      listOrderTagDefinitions(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200,
      }),
    enabled: Boolean(tenantId),
  });
  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'orders-page'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });
  const courierQuery = useQuery({
    queryKey: ['couriers', tenantId, 'orders-page'],
    queryFn: () => listCouriers(tenantId, { limit: 200, status: 'ACTIVE' }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });
  const orderTagOptions = useMemo(
    () =>
      (orderTagDefinitionsQuery.data?.items ?? []).map((tag) => ({
        value: tag.name,
        label: tag.name,
      })),
    [orderTagDefinitionsQuery.data?.items],
  );

  const detailMap = useMemo(() => {
    const map = new Map<string, OrderDetail>();
    detailQueries.forEach((queryResult, index) => {
      const orderId = orderIds[index];
      if (orderId && queryResult.data) {
        map.set(orderId, queryResult.data);
      }
    });
    return map;
  }, [detailRevision, orderIds]);

  const detailQuery = useQuery({
    queryKey: ['order-detail', tenantId, activeOrder?.id],
    queryFn: () => getOrderDetail(tenantId, activeOrder?.id ?? ''),
    enabled: Boolean(tenantId && activeOrder?.id),
  });

  const orderShipmentsQuery = useQuery({
    queryKey: ['oms-order-shipments', tenantId, activeOrder?.id],
    queryFn: () => listOmsOrderShipments(tenantId, activeOrder?.id ?? ''),
    enabled: Boolean(tenantId && activeOrder?.id && activeBottomTab === 'package'),
    retry: false,
  });
  const orderShipments = orderShipmentsQuery.data?.items ?? [];

  const packageQueries = useQueries({
    queries: orderShipments.map((shipment) => ({
      queryKey: ['shipment-packages', tenantId, shipment.shipment_id],
      queryFn: () => listShipmentPackages(tenantId, shipment.shipment_id),
      enabled: Boolean(tenantId && shipment.shipment_id && activeBottomTab === 'package'),
      retry: false,
    })),
  });

  const packagesMap = useMemo(() => {
    const map = new Map<string, ShipmentPackage[]>();
    packageQueries.forEach((queryResult, index) => {
      const shipmentId = orderShipments[index]?.shipment_id;
      if (shipmentId && queryResult.data) {
        map.set(shipmentId, queryResult.data.items);
      }
    });
    return map;
  }, [packageQueries, orderShipments]);

  const detailData = detailQuery.data;
  const detailOrder = detailData?.order;
  const detailGoodsRows = useMemo(
    () => toOrderWorkbenchGoodsRows((detailData?.items ?? []) as Record<string, unknown>[]),
    [detailData?.items],
  );
  const receiverEditDisabled = !canWriteOrders || detailOrder?.review_status === 'PASSED';
  const receiverEditDisabledReason = !canWriteOrders
    ? '暂无权限'
    : detailOrder?.review_status === 'PASSED'
      ? '已审核订单仅可修改物流备注'
      : undefined;
  const invalidItems = useMemo(
    () => (detailData?.items ?? []).filter((item) => item.mapping_missing),
    [detailData?.items],
  );
  const invalidGoodsRows = useMemo(
    () => detailGoodsRows.filter((item) => item.mappingMissing),
    [detailGoodsRows],
  );
  const detailLoadedId = String(detailOrder?.id ?? activeOrder?.id ?? '');

  useEffect(() => {
    const nextWarehouseId = String(detailOrder?.ship_warehouse_id ?? '').trim();
    const nextLogisticsCompany = String(detailOrder?.logistics_company ?? '').trim();
    setDetailWarehouseId(nextWarehouseId || undefined);
    setDetailLogisticsCompany(nextLogisticsCompany || undefined);
  }, [detailOrder?.id, detailOrder?.logistics_company, detailOrder?.ship_warehouse_id]);

  const mergedWarehouseOptions = useMemo<DetailSelectOption>(() => {
    const map = new Map<string, { label: string; value: string }>();
    (warehouseQuery.data?.items ?? []).forEach((warehouse) => {
      const value = String(warehouse.warehouse_id ?? '').trim();
      if (!value) return;
      map.set(value, {
        label: String(warehouse.name ?? value),
        value,
      });
    });
    if (detailWarehouseId && !map.has(detailWarehouseId)) {
      map.set(detailWarehouseId, { label: detailWarehouseId, value: detailWarehouseId });
    }
    return Array.from(map.values());
  }, [detailWarehouseId, warehouseQuery.data?.items]);

  const mergedCourierOptions = useMemo<DetailSelectOption>(() => {
    const map = new Map<string, { label: string; value: string }>();
    (courierQuery.data?.items ?? []).forEach((courier) => {
      const value = String(courier.name ?? '').trim();
      if (!value) return;
      const code = String(courier.code ?? '').trim();
      const optionValue = resolveCourierOptionValue(value, code);
      if (!optionValue) return;
      map.set(optionValue, {
        label: buildCourierOptionLabel(value, code),
        value: optionValue,
      });
    });
    if (detailLogisticsCompany && !map.has(detailLogisticsCompany)) {
      map.set(detailLogisticsCompany, { label: detailLogisticsCompany, value: detailLogisticsCompany });
    }
    return Array.from(map.values());
  }, [courierQuery.data?.items, detailLogisticsCompany]);

  const explicitOrderTags = useMemo(
    () => (detailData?.tags ?? []).map((tag) => String(tag.tag ?? '').trim()).filter(Boolean),
    [detailData?.tags],
  );

  const normalizedOrderTags = useMemo(
    () => Array.from(new Set(explicitOrderTags.map((item) => item.trim()).filter(Boolean))),
    [explicitOrderTags],
  );

  const displayOrderTags = useMemo(
    () =>
      resolveOrderDisplayTags({
        ...(detailOrder ?? {}),
        tags: normalizedOrderTags,
      }),
    [detailOrder, normalizedOrderTags],
  );

  const tagMenuItems = useMemo<NonNullable<MenuProps['items']>>(
    () => {
      const items = orderTagOptions
        .map((option) => {
          const key = String(option?.value ?? '').trim();
          if (!key) return null;
          return {
            key,
            label: String(option?.label ?? key).trim() || key,
          };
        })
        .filter((item): item is { key: string; label: string } => item != null);
      if (items.length > 0) {
        return items;
      }
      return [{ key: '__empty__', label: '暂无可选标记', disabled: true }];
    },
    [orderTagOptions],
  );

  const detailSelectDisabled = !String(detailOrder?.id ?? activeOrder?.id ?? '').trim();

  const handleDetailWarehouseChange = useCallback(
    async (nextValue: string) => {
      const nextWarehouseId = String(nextValue ?? '').trim() || undefined;
      if (nextWarehouseId === detailWarehouseId) {
        return;
      }

      const orderId = String(detailOrder?.id ?? activeOrder?.id ?? '').trim();
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
    [activeOrder?.id, detailOrder?.id, detailQuery, detailWarehouseId, queryClient, tenantId],
  );

  const handleDetailLogisticsChange = useCallback(
    async (nextValue: string) => {
      const normalized = String(nextValue ?? '').trim();
      const nextLogistics = normalized || undefined;
      if (nextLogistics === detailLogisticsCompany) {
        return;
      }

      const orderId = String(detailOrder?.id ?? activeOrder?.id ?? '').trim();
      if (!tenantId || !orderId) {
        setDetailLogisticsCompany(nextLogistics);
        return;
      }

      const previousLogistics = detailLogisticsCompany;
      setDetailLogisticsCompany(nextLogistics);
      setDetailUpdatingField('logistics');
      try {
        await updateOrder(tenantId, orderId, { logistics_company: nextLogistics });
        message.success('物流公司已更新');
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['orders', tenantId] }),
          queryClient.invalidateQueries({ queryKey: ['order-detail', tenantId, orderId] }),
          detailQuery.refetch(),
        ]);
      } catch (error) {
        setDetailLogisticsCompany(previousLogistics);
        message.error(error instanceof Error ? error.message : '更新物流公司失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [activeOrder?.id, detailLogisticsCompany, detailOrder?.id, detailQuery, queryClient, tenantId],
  );

  const handleApplyOrderTags = useCallback(
    async (tags: string[]) => {
      const normalizedTags = Array.from(new Set(tags.map((item) => item.trim()).filter(Boolean)));
      if (!normalizedTags.length) {
        message.warning('请至少选择一个订单标记');
        return false;
      }

      const orderId = String(detailOrder?.id ?? activeOrder?.id ?? '').trim();
      if (!tenantId || !orderId) {
        message.warning('请先选中订单');
        return false;
      }

      setOrderTagsApplying(true);
      try {
        await addOrderTags(tenantId, orderId, normalizedTags);
        message.success('订单标记已应用');
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
    [activeOrder?.id, detailOrder?.id, detailQuery, queryClient, tenantId],
  );

  const handleMarkSelect = useCallback(
    (tagValue: string) => {
      const normalized = String(tagValue ?? '').trim();
      if (!normalized || detailSelectDisabled || orderTagsApplying) {
        return;
      }
      void handleApplyOrderTags([...normalizedOrderTags, normalized]);
    },
    [detailSelectDisabled, handleApplyOrderTags, normalizedOrderTags, orderTagsApplying],
  );

  const selectedMatchItem = useMemo(() => {
    if (!invalidItems.length) {
      return null;
    }
    const matched = invalidItems.find((item) => item.id === matchItemId);
    return matched ?? invalidItems[0];
  }, [invalidItems, matchItemId]);

  const selectedMatchGoodsRow = useMemo(() => {
    if (!invalidGoodsRows.length) {
      return null;
    }
    const matched = invalidGoodsRows.find((item) => item.id === matchItemId);
    return matched ?? invalidGoodsRows[0];
  }, [invalidGoodsRows, matchItemId]);

  const handleKeywordChange = useCallback((value: string) => {
    setSearchValue(value);
    if (!value.trim()) {
      setSearchKeyword('');
      resetPagination();
    }
  }, []);

  const handleKeywordSearch = useCallback((value: string) => {
    setSearchKeyword(value.trim());
    resetPagination();
  }, []);

  const handleSearch = (value: string) => {
    handleKeywordSearch(value);
  };

  const updateColumnFilterDraft = useCallback(
    (key: ColumnFilterFieldKey, value: ColumnFilters[ColumnFilterFieldKey] | undefined) => {
      setColumnFilterDrafts((prev) => {
        if (
          value == null &&
          !Object.prototype.hasOwnProperty.call(prev, key)
        ) {
          return prev;
        }
        return {
          ...prev,
          [key]: value,
        };
      });
    },
    [],
  );

  const applySingleColumnFilter = useCallback(
    (key: ColumnFilterFieldKey, value: ColumnFilters[ColumnFilterFieldKey] | undefined) => {
      const next = buildSingleColumnFilters(key, value);
      setColumnFilters(next);
      setColumnFilterDrafts(next);
      resetPagination();
    },
    [resetPagination],
  );

  const resolveColumnFilterInputValue = useCallback(
    (key: ColumnTextFilterKey): string => {
      const draftValue = columnFilterDrafts[key];
      if (typeof draftValue === 'string') {
        return draftValue;
      }
      if (Object.prototype.hasOwnProperty.call(columnFilterDrafts, key)) {
        return '';
      }
      const appliedValue = columnFilters[key];
      return typeof appliedValue === 'string' ? appliedValue : '';
    },
    [columnFilterDrafts, columnFilters],
  );

  const handlePaginationChange = (nextPagination: Parameters<typeof applyPaginationChange>[0]) => {
    applyPaginationChange(nextPagination);
  };

  const handleFilterChange = (_: unknown, values: FilterValues) => {
    setFilters(values);
    resetPagination();
  };

  const hydrateSavedView = useCallback(
    (parsed: SavedViewState) => {
      if (parsed.filters) {
        const nextFilters: FilterValues = {
          time_range: parsed.filters.time_range ?? defaultFilterValues.time_range,
          archive_status: parsed.filters.archive_status ?? defaultFilterValues.archive_status,
          platform: parsed.filters.platform ?? defaultFilterValues.platform,
        };
        setFilters(nextFilters);
        form.setFieldsValue(nextFilters);
      }
      if (parsed.treeSelection?.length) {
        setTreeSelection(parsed.treeSelection);
      }
      if (typeof parsed.searchValue === 'string') {
        setSearchValue(parsed.searchValue);
      }
      if (typeof parsed.searchKeyword === 'string') {
        setSearchKeyword(parsed.searchKeyword);
      }
      const restoredColumnFilters = deserializeColumnFilters(parsed.columnFilters);
      const restoredColumnFilterDrafts =
        deserializeColumnFilters(parsed.columnFilterDrafts) ?? restoredColumnFilters;
      if (restoredColumnFilters) {
        setColumnFilters(restoredColumnFilters);
      }
      if (restoredColumnFilterDrafts) {
        setColumnFilterDrafts(restoredColumnFilterDrafts);
      }
    },
    [form],
  );

  const serializeSavedView = useCallback(
    (): SavedViewState => ({
      filters,
      treeSelection,
      searchValue,
      searchKeyword,
      columnFilters: serializeColumnFilters(columnFilters),
      columnFilterDrafts: serializeColumnFilters(columnFilterDrafts),
    }),
    [columnFilterDrafts, columnFilters, filters, searchKeyword, searchValue, treeSelection],
  );

  useWorkbenchSavedViewState<SavedViewState>({
    storageKey: savedViewKey,
    hydrate: hydrateSavedView,
    serialize: serializeSavedView,
  });

  const handleRefresh = () => {
    refetch();
    if (detailQuery.isFetched) {
      detailQuery.refetch();
    }
  };

  const handleOpenPackageEditor = (shipment: OmsShipmentSummary) => {
    const packages = packagesMap.get(shipment.shipment_id) ?? [];
    const draftPackages =
      packages.length > 0
        ? packages.map((pkg) => ({
            package_no: pkg.package_no,
            tracking_no: pkg.tracking_no ?? undefined,
            carrier_code: pkg.carrier_code ?? undefined,
            weight_gram: pkg.weight_gram ?? undefined,
          }))
        : [
            {
              package_no: '',
              tracking_no: shipment.tracking_no ?? undefined,
              carrier_code: shipment.carrier_code ?? undefined,
              weight_gram: shipment.weight_gram ?? undefined,
            },
          ];
    packageForm.setFieldsValue({ packages: draftPackages });
    setPackageEditingShipment(shipment);
    setPackageEditOpen(true);
  };

  const handleSubmitPackages = async () => {
    if (!tenantId || !packageEditingShipment) {
      return;
    }
    try {
      const values = await packageForm.validateFields();
      const packages = (values.packages ?? [])
        .map((pkg) => ({
          package_no: pkg.package_no?.trim(),
          tracking_no: pkg.tracking_no?.trim() || undefined,
          carrier_code: pkg.carrier_code?.trim() || undefined,
          weight_gram: pkg.weight_gram ?? undefined,
        }))
        .filter((pkg) => pkg.package_no);
      setPackageSaving(true);
      await upsertShipmentPackages(tenantId, packageEditingShipment.shipment_id, {
        packages: packages as Array<{
          package_no: string;
          tracking_no?: string;
          carrier_code?: string;
          weight_gram?: number;
        }>,
      });
      message.success('包裹信息已更新');
      setPackageEditOpen(false);
      packageForm.resetFields();
      await queryClient.invalidateQueries({
        queryKey: ['shipment-packages', tenantId, packageEditingShipment.shipment_id],
      });
      await queryClient.invalidateQueries({ queryKey: ['shipments', tenantId] });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    } finally {
      setPackageSaving(false);
    }
  };

  const handleExport = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!canExport) {
      message.warning('暂无导出权限');
      return;
    }
    const range = resolveTimeRange(filters.time_range);
    const params: Record<string, unknown> = {};
    if (range?.[0] && range?.[1]) {
      params.start_at = range[0].toISOString();
      params.end_at = range[1].toISOString();
    }
    if (query.platform) {
      params.platform = query.platform;
    }
    if (query.shop_id) {
      params.shop_id = query.shop_id;
    }
    setExporting(true);
    try {
      const job = await createExportJob({
        type: 'ORDER_EXPORT',
        params,
      });
      message.success(`导出任务已创建：${job.job_id}，请到导出中心下载`);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '导出失败');
    } finally {
      setExporting(false);
    }
  };
  const handleToggleSelectAll = () => {
    if (!currentPageKeys.length) {
      return;
    }
    if (allCurrentSelected) {
      setSelectedRowKeys([]);
      setSelectedOrders([]);
      return;
    }
    setSelectedRowKeys(currentPageKeys);
    setSelectedOrders(listItems);
  };

  const resolveTargetOrders = () => {
    if (selectedOrders.length > 0) {
      return selectedOrders;
    }
    return activeOrder ? [activeOrder] : [];
  };

  const submitExceptionAction = async (action: 'RESOLVE' | 'REJECT' | 'REOPEN', label: string) => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!canWriteOrders) {
      message.warning('暂无订单操作权限');
      return;
    }
    const targets = resolveTargetOrders();
    if (!targets.length) {
      message.warning('请选择需要处理的订单');
      return;
    }
    setExceptionProcessing(true);
    try {
      await Promise.all(
        targets.map((order) =>
          handleOrderException(tenantId, order.id, {
            action,
            reason: `${label} by exception pool`,
          }),
        ),
      );
      message.success(`已${label} ${targets.length} 单`);
      setSelectedRowKeys([]);
      setSelectedOrders([]);
      await refetch();
      if (detailQuery.isFetched) {
        detailQuery.refetch();
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : `${label}失败`);
    } finally {
      setExceptionProcessing(false);
    }
  };

  const confirmExceptionAction = (action: 'RESOLVE' | 'REJECT' | 'REOPEN', label: string) => {
    const targets = resolveTargetOrders();
    if (!targets.length) {
      message.warning('请选择需要处理的订单');
      return;
    }
    Modal.confirm({
      title: `${label}异常订单`,
      content: `确认对 ${targets.length} 单执行${label}操作？`,
      okText: '确认',
      cancelText: '取消',
      onOk: () => submitExceptionAction(action, label),
    });
  };

  const resolveSkuInfo = (skuId?: string | null): Sku | undefined =>
    skuId ? skuOptions.find((sku) => sku.sku_id === skuId) : undefined;

  const openMatchModal = (itemId?: string | null) => {
    if (!detailOrder) {
      message.warning('请先选择订单');
      return;
    }
    if (!invalidItems.length) {
      message.info('当前订单没有无效货品');
      return;
    }
    const targets = resolveTargetOrders().filter(
      (order) => order.shop_id === detailOrder.shop_id && order.platform === detailOrder.platform,
    );
    setMatchTargetOrderIds(
      (targets.length ? targets : resolveTargetOrders()).map((order) => order.id),
    );
    setMatchItemId(itemId ?? invalidItems[0].id);
    setMatchSkuId(null);
    setMatchApplyAll(true);
    setSkuKeyword('');
    setMatchModalOpen(true);
  };

  const handleSubmitMatch = async () => {
    if (!tenantId || !detailOrder) {
      message.error('请先选择订单');
      return;
    }
    if (!canWriteOrders || !canManageMappings) {
      message.warning('暂无匹配权限');
      return;
    }
    if (!selectedMatchItem) {
      message.warning('请选择待匹配货品');
      return;
    }
    if (!matchSkuId) {
      message.warning('请选择ERP货品');
      return;
    }
    const platformSkuId = selectedMatchItem.platform_sku_id ?? selectedMatchItem.outer_sku_id;
    if (!platformSkuId) {
      message.error('平台未提供可用编码，无法匹配');
      return;
    }
    const selectedSku = resolveSkuInfo(matchSkuId);
    const payload: MappingPayload = {
      platform: detailOrder.platform as MappingPayload['platform'],
      shop_id: detailOrder.shop_id,
      platform_sku_id: platformSkuId,
      outer_sku_id: selectedMatchItem.outer_sku_id ?? null,
      sku_id: matchSkuId,
      sku_code: selectedSku?.sku_code ?? null,
      barcode: selectedSku?.barcode ?? null,
    };
    setMatchSaving(true);
    try {
      await createMapping(tenantId, payload);
      const targets = matchApplyAll ? matchTargetOrderIds : [detailOrder.id].filter(Boolean);
      await Promise.all(
        (targets.length ? targets : [detailOrder.id]).map((orderId) =>
          rematchOrderItems(tenantId, orderId),
        ),
      );
      message.success('货品匹配完成');
      setMatchModalOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['orders', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['order-detail', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '货品匹配失败');
    } finally {
      setMatchSaving(false);
    }
  };

  const openTagModal = () => {
    if (!activeOrder) {
      message.warning('请先选择订单');
      return;
    }
    setSelectedTags([]);
    setTagModalOpen(true);
  };

  const submitTags = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!activeOrder) {
      message.warning('请先选择订单');
      return;
    }
    const tags = selectedTags.map((tag) => tag.trim()).filter(Boolean);
    if (!tags.length) {
      message.warning('请输入标记内容');
      return;
    }
    try {
      await addOrderTags(tenantId, activeOrder.id, tags);
      message.success('标记已更新');
      setTagModalOpen(false);
      detailQuery.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '标记更新失败');
    }
  };

  const openNoteModal = () => {
    if (!activeOrder) {
      message.warning('请先选择订单');
      return;
    }
    setNoteInput('');
    setNoteModalOpen(true);
  };

  const submitNote = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!activeOrder) {
      message.warning('请先选择订单');
      return;
    }
    const note = noteInput.trim();
    if (!note) {
      message.warning('请输入备注内容');
      return;
    }
    try {
      await addOrderNote(tenantId, activeOrder.id, note);
      message.success('备注已更新');
      setNoteModalOpen(false);
      detailQuery.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '备注更新失败');
    }
  };

  const openReceiverModal = () => {
    if (!detailOrder) {
      message.warning('请先选择订单');
      return;
    }
    if (detailOrder.review_status === 'PASSED') {
      message.warning('已审核订单仅允许修改物流/备注');
      return;
    }
    receiverForm.setFieldsValue({
      receiver_name: detailOrder.receiver_name ?? '',
      receiver_phone: detailOrder.receiver_phone ?? '',
      receiver_address: detailOrder.receiver_address ?? '',
    });
    setReceiverModalOpen(true);
  };

  const submitReceiverUpdate = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!activeOrder) {
      message.warning('请先选择订单');
      return;
    }
    try {
      const values = await receiverForm.validateFields();
      if (!canWriteOrders) {
        message.warning('暂无修改收货信息权限');
        return;
      }
      setReceiverSaving(true);
      await updateOrderReceiver(tenantId, activeOrder.id, {
        receiver_name: values.receiver_name,
        receiver_phone: values.receiver_phone,
        receiver_address: values.receiver_address,
      });
      message.success('收货信息已更新');
      setReceiverModalOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['orders', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['order-detail', tenantId, activeOrder.id] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setReceiverSaving(false);
    }
  };

  const columns = useMemo<ColumnsType<OrderSummary>>(() => {
    const resolveOrderNo = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      return extra.erp_order_no ?? record.id ?? '-';
    };
    const resolveShopName = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      return extra.shop_name ?? record.shop_id ?? '-';
    };
    const resolveWarehouseName = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      return extra.warehouse_name ?? '-';
    };
    const resolveReceiverAddress = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      return extra.receiver_address ?? detailMap.get(record.id)?.order.receiver_address ?? '-';
    };
    const resolveItemSummary = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      const items = extra.items ?? detailMap.get(record.id)?.items ?? [];
      if (!items.length) {
        return '-';
      }
      const labels = items
        .map((item) => item.title ?? item.outer_sku_id ?? item.platform_sku_id ?? item.id)
        .filter((label): label is string => Boolean(label));
      if (!labels.length) {
        return '-';
      }
      const joined = labels.join('、');
      const truncated = truncateText(joined, 12);
      if (joined === truncated) {
        return joined;
      }
      return <Tooltip title={joined}>{truncated}</Tooltip>;
    };
    const resolveItemQuantity = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      if (extra.goods_total_qty != null) {
        return extra.goods_total_qty;
      }
      if (extra.item_count != null) {
        return extra.item_count;
      }
      if (extra.goods_count != null) {
        return extra.goods_count;
      }
      const items = extra.items ?? detailMap.get(record.id)?.items ?? [];
      if (!items.length) {
        return null;
      }
      return items.reduce((sum, item) => sum + (item.quantity ?? 0), 0);
    };
    const resolveEstimatedPostage = (record: OrderSummary) => {
      const extra = record as OrderSummary &
        OrderSummaryExtras & { postage_amount_cent?: number | null };
      return (
        extra.estimated_postage_cent ?? extra.postage_amount_cent ?? extra.shipping_fee_cent ?? null
      );
    };
    const resolveEstimatedProfit = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      const total = record.total_amount_cent;
      const platformFee = extra.platform_fee_cent ?? extra.shop_fee_cent;
      const shippingFee = resolveEstimatedPostage(record);
      const goodsCost = extra.goods_cost_cent;
      if (total == null || platformFee == null || shippingFee == null || goodsCost == null) {
        return null;
      }
      return total - platformFee - shippingFee - goodsCost;
    };
    const resolveEstimatedProfitRate = (record: OrderSummary) => {
      const profit = resolveEstimatedProfit(record);
      if (profit == null || !record.total_amount_cent) {
        return null;
      }
      return profit / record.total_amount_cent;
    };
    const resolveRemark = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      return (
        extra.customer_note ??
        extra.remark ??
        detailMap.get(record.id)?.order.customer_note ??
        detailMap.get(record.id)?.order.remark ??
        '-'
      );
    };
    const resolveOrderSource = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      return extra.order_source ?? detailMap.get(record.id)?.order.order_source ?? '-';
    };
    const resolveLogisticsCompany = (record: OrderSummary) => {
      const extra = record as OrderSummary & OrderSummaryExtras;
      return extra.logistics_company ?? detailMap.get(record.id)?.order.logistics_company ?? '-';
    };
    const resolveOrderTags = (record: OrderSummary) => {
      const detail = detailMap.get(record.id);
      const extra = record as OrderSummary & OrderSummaryExtras & { tags?: string[] };
      const tags = extra.tags ?? detail?.tags?.map((tag) => tag.tag) ?? [];
      return <OrderTagBadges tags={resolveOrderDisplayTags({ ...(detail?.order ?? {}), ...record, tags })} />;
    };

    return buildOrderManagementColumns({
      rowOffset,
      orderStatusMeta,
      formatCurrency,
      formatDateTime,
      formatDuration,
      resolveOrderNo,
      resolveShopName,
      resolveWarehouseName,
      resolveItemSummary,
      resolveItemQuantity,
      resolveReceiverAddress,
      resolveRemark,
      resolveOrderSource,
      resolveLogisticsCompany,
      resolveTags: resolveOrderTags,
      resolveEstimatedPostage,
      resolveEstimatedProfit,
      resolveEstimatedProfitRate,
      includeExceptions: true,
      includeTrackingNo: true,
      includeActions: true,
      onViewDetail: (record) => navigate(`/orders/${record.id}`),
    });
  }, [detailMap, navigate, rowOffset]);

  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${listItems.length}`,
    [columns.length, listItems.length],
  );

  const dataGridColumns = useMemo<ErpDataGridNextColumn<OrderSummary>[]>(() => {
    const keyCount = new Map<string, number>();
    const resolveColumnKey = (column: ColumnsType<OrderSummary>[number], index: number) => {
      let baseKey = '';
      if (column.key != null) {
        baseKey = String(column.key);
      } else if (column.dataIndex != null) {
        baseKey = Array.isArray(column.dataIndex)
          ? column.dataIndex.map(String).join('.')
          : String(column.dataIndex);
      } else {
        baseKey = `column-${index}`;
      }
      const next = (keyCount.get(baseKey) ?? 0) + 1;
      keyCount.set(baseKey, next);
      return next === 1 ? baseKey : `${baseKey}__${next}`;
    };

    const resolveValue = (record: OrderSummary, dataIndex: unknown) => {
      if (dataIndex == null) {
        return undefined;
      }
      const path = Array.isArray(dataIndex) ? dataIndex : [dataIndex];
      let current: unknown = record as unknown;
      for (const segment of path) {
        if (current == null) {
          return undefined;
        }
        current = (current as Record<string, unknown>)[String(segment)];
      }
      return current;
    };

    const resolveSortValue = (key: string) => {
      if (key !== 'created_at') {
        return undefined;
      }
      return (record: OrderSummary) => {
        const value = record.created_at;
        if (!value) {
          return 0;
        }
        const parsed = Date.parse(value);
        return Number.isFinite(parsed) ? parsed : 0;
      };
    };

    const walk = (
      items: ColumnsType<OrderSummary>,
      parentLabel?: string,
    ): ErpDataGridNextColumn<OrderSummary>[] =>
      items.flatMap((column, index) => {
        if ('children' in column && Array.isArray(column.children) && column.children.length) {
          const label =
            typeof column.title === 'string' || typeof column.title === 'number'
              ? String(column.title)
              : parentLabel;
          return walk(column.children, label);
        }

        const columnKey = resolveColumnKey(column, index);
        const rawTitle = column.title;
        const title =
          typeof rawTitle === 'string' || typeof rawTitle === 'number'
            ? String(rawTitle)
            : (rawTitle ?? columnKey);
        const header =
          parentLabel && typeof title === 'string' ? `${parentLabel} / ${title}` : title;

        const width = typeof column.width === 'number' ? column.width : undefined;
        const sortValue = resolveSortValue(columnKey);

        return [
          {
            key: columnKey,
            header,
            width,
            minWidth: width,
            sortValue,
            render: (record, rowIndex) => {
              const value = resolveValue(record, column.dataIndex);
              if (column.render) {
                return column.render(value, record, rowIndex);
              }
              if (value == null || value === '') {
                return '-';
              }
              if (typeof value === 'string' || typeof value === 'number') {
                return value;
              }
              return String(value);
            },
          },
        ];
      });

    return walk(columns);
  }, [columns]);

  const columnQueryCells = useMemo(
    () =>
      buildOrderQuickFilterCells([
        {
          key: 'id',
          type: 'input',
          placeholder: '模糊查询',
          value: searchValue,
          onChange: (value) => handleKeywordChange(String(value ?? '')),
          onSearch: (value) => handleKeywordSearch(value),
        },
        {
          key: 'processing_time',
          type: 'date-time-range',
          rangePlaceholder: ['处理开始', '处理结束'],
          value: columnFilters.processedRange,
          onChange: (value) =>
            applySingleColumnFilter(
              'processedRange',
              value as [dayjs.Dayjs, dayjs.Dayjs] | undefined,
            ),
        },
        {
          key: 'tracking_no',
          type: 'input',
          placeholder: '物流单号',
          value: resolveColumnFilterInputValue('trackingNo'),
          onChange: (value) => updateColumnFilterDraft('trackingNo', String(value ?? '')),
          onSearch: (value) => applySingleColumnFilter('trackingNo', value),
          onPaste: (value) => applySingleColumnFilter('trackingNo', value),
        },
        {
          key: 'platform_order_id',
          type: 'input',
          placeholder: '网店订单号',
          value: resolveColumnFilterInputValue('platformOrderId'),
          onChange: (value) => updateColumnFilterDraft('platformOrderId', String(value ?? '')),
          onSearch: (value) => applySingleColumnFilter('platformOrderId', value),
          onPaste: (value) => applySingleColumnFilter('platformOrderId', value),
        },
        {
          key: 'receiver_address',
          type: 'input',
          placeholder: '收货地址',
          value: resolveColumnFilterInputValue('receiverAddress'),
          onChange: (value) => updateColumnFilterDraft('receiverAddress', String(value ?? '')),
          onSearch: (value) => applySingleColumnFilter('receiverAddress', value),
          onPaste: (value) => applySingleColumnFilter('receiverAddress', value),
        },
        {
          key: 'items',
          type: 'input',
          placeholder: '货品信息',
          value: resolveColumnFilterInputValue('itemKeyword'),
          onChange: (value) => updateColumnFilterDraft('itemKeyword', String(value ?? '')),
          onSearch: (value) => applySingleColumnFilter('itemKeyword', value),
          onPaste: (value) => applySingleColumnFilter('itemKeyword', value),
        },
      ]),
    [
      applySingleColumnFilter,
      columnFilters.processedRange,
      handleKeywordChange,
      handleKeywordSearch,
      resolveColumnFilterInputValue,
      searchValue,
      updateColumnFilterDraft,
    ],
  );

  const isInteractiveTarget = useCallback(
    (target: HTMLElement) =>
      Boolean(
        target.closest(
          'button, a, input, textarea, .ant-select, .ant-select-selector, .ant-input, .ant-input-affix-wrapper, .ant-input-number, .ant-picker, .ant-checkbox, .ant-checkbox-wrapper, .workbench-table-query',
        ),
      ),
    [],
  );

  const handleSelectionChange = useCallback(
    (keys: Array<string | number>, rows?: OrderSummary[]) => {
      const normalizedKeys = keys.map((key) => String(key));
      setSelectedRowKeys(normalizedKeys);
      if (rows) {
        setSelectedOrders(rows);
        return;
      }
      setSelectedOrders(listItems.filter((item) => normalizedKeys.includes(item.id)));
    },
    [listItems],
  );

  const handleTableRowClick = useCallback(
    (record: OrderSummary, event: React.MouseEvent) => {
      const target = event.target as HTMLElement;
      if (isInteractiveTarget(target)) {
        setActiveOrder(record);
        return;
      }
      setActiveOrder(record);
    },
    [isInteractiveTarget],
  );

  const handleTableRowDoubleClick = useCallback(
    (record: OrderSummary, event: React.MouseEvent) => {
      const target = event.target as HTMLElement;
      if (isInteractiveTarget(target)) {
        return;
      }
      if (record.id) {
        navigate(`/orders/${record.id}`);
      }
    },
    [isInteractiveTarget, navigate],
  );

  const handleTableRowContextMenu = useCallback(
    (_record: OrderSummary, _event: React.MouseEvent) => {
      _event.preventDefault();
      _event.stopPropagation();
      void _record;
      // 保留右键入口以抑制浏览器默认菜单，但不改变激活行和勾选集合。
    },
    [],
  );

  const goodsColumns = useMemo<ColumnsType<OrderWorkbenchGoodsRow>>(
    () => [
      {
        title: '货品编号',
        dataIndex: 'outer_sku_id',
        render: (_, record) => record.outer_sku_id ?? record.platform_sku_id ?? record.id,
      },
      {
        title: '货品名称',
        dataIndex: 'shopProductName',
        render: (value, record) => value ?? record.title ?? '-',
      },
      {
        title: '规格',
        dataIndex: 'shopSpec',
        render: (value) => value ?? '-',
      },
      {
        title: '数量',
        dataIndex: 'quantity',
      },
      {
        title: '单价',
        dataIndex: 'price_cent',
        render: (value) => formatCurrency(value),
      },
      {
        title: '优惠(折扣)',
        dataIndex: 'discount',
        render: () => '-',
      },
      {
        title: '金额',
        key: 'amount',
        render: (_, record) => formatCurrency(record.price_cent * record.quantity),
      },
      {
        title: '状态',
        dataIndex: 'mappingMissing',
        render: (_, record) => {
          if (!record.mappingMissing) {
            return '-';
          }
          return (
            <Space size={4} className="order-query-invalid-cell">
              <Tooltip title="网店商品未完成货品匹配">
                <Tag color="red">无效货品</Tag>
              </Tooltip>
              {canWriteOrders && canManageMappings ? (
                <Button type="link" size="small" onClick={() => openMatchModal(record.id)}>
                  去匹配
                </Button>
              ) : null}
            </Space>
          );
        },
      },
    ],
    [canManageMappings, canWriteOrders],
  );

  const auditColumns = useMemo<ColumnsType<OrderDetail['audit_logs'][number]>>(
    () => [
      { title: '动作', dataIndex: 'action', width: 160 },
      { title: '对象', dataIndex: 'entity_type', width: 140 },
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (value) => (value ? new Date(value).toLocaleString() : '-'),
      },
    ],
    [],
  );

  const buildOrderQueryLeftNodes = useCallback((sourceRows: OrderSummary[]): OrderReviewAgLeftNode[] => {
    const totalCount = sourceRows.length;
    const exceptionCount = sourceRows.filter((row) => matchesOrderQueryLeftFilter('exception', row)).length;
    const invalidGoodsCount = sourceRows.filter((row) => matchesOrderQueryLeftFilter('invalid-goods', row)).length;
    const taggedCount = sourceRows.filter((row) => matchesOrderQueryLeftFilter('tagged', row)).length;
    const untaggedCount = Math.max(0, totalCount - taggedCount);

    return [
      {
        key: 'query-scope-group',
        label: '订单范围',
        nodeType: 'group',
        count: totalCount,
        children: [
          { key: 'all', label: '全部订单', nodeType: 'exception', count: totalCount },
          { key: 'exception', label: '异常订单', nodeType: 'exception', count: exceptionCount },
          { key: 'invalid-goods', label: '无效货品', nodeType: 'exception', count: invalidGoodsCount },
        ],
      },
      {
        key: 'query-status-group',
        label: '订单状态',
        nodeType: 'group',
        count: totalCount,
        children: [
          ...orderStatusOptions.map((option) => ({
            key: `status-${option.value}`,
            label: option.label,
            nodeType: 'status' as const,
            count: sourceRows.filter((row) => matchesOrderQueryLeftFilter(`status-${option.value}`, row)).length,
          })),
        ],
      },
      {
        key: 'query-channel-group',
        label: '销售渠道',
        nodeType: 'group',
        count: totalCount,
        children: [
          ...platformOptions.map((option) => ({
            key: `channel-${option.value}`,
            label: option.label,
            nodeType: 'platform' as const,
            count: sourceRows.filter((row) => matchesOrderQueryLeftFilter(`channel-${option.value}`, row)).length,
          })),
        ],
      },
      {
        key: 'query-tag-group',
        label: '标记',
        nodeType: 'group',
        count: totalCount,
        children: [
          { key: 'tagged', label: '已标记', nodeType: 'tag', count: taggedCount },
          { key: 'untagged', label: '未标记', nodeType: 'tag', count: untaggedCount },
        ],
      },
    ];
  }, []);

  const leftScopeTotal = leftScopeRows.length;

  const leftStatsItems = useMemo<UiModule08OrderReviewLeftStatsItem[]>(() => {
    const palette = ['#334e68', '#7da888', '#8bcdd3', '#d48935', '#b76e79'];
    const platformCountMap = new Map<string, number>();
    leftScopeRows.forEach((row) => {
      const key = String(row.platform ?? '').trim() || 'UNKNOWN';
      platformCountMap.set(key, (platformCountMap.get(key) ?? 0) + 1);
    });
    const entries = Array.from(platformCountMap.entries()).sort((a, b) => b[1] - a[1]).slice(0, 5);
    if (entries.length === 0) {
      return [{ key: 'all', label: '全部订单', value: leftScopeTotal, color: palette[0] ?? '#334e68' }];
    }
    return entries.map(([key, value], index) => ({
      key,
      label: platformOptions.find((item) => item.value === key)?.label ?? key,
      value,
      color: palette[index % palette.length] ?? palette[0] ?? '#334e68',
    }));
  }, [leftScopeRows, leftScopeTotal]);

  const leftStatsPieBackground = useMemo(() => {
    if (leftStatsItems.length === 0) {
      return 'conic-gradient(#dce4ee 0deg 360deg)';
    }
    let angle = 0;
    const totalValue = leftStatsItems.reduce((sum, item) => sum + item.value, 0) || 1;
    const stops = leftStatsItems.map((item) => {
      const start = angle;
      const next = angle + (item.value / totalValue) * 360;
      angle = next;
      return `${item.color} ${start.toFixed(2)}deg ${next.toFixed(2)}deg`;
    });
    if (angle < 360) {
      stops.push(`#dce4ee ${angle.toFixed(2)}deg 360deg`);
    }
    return `conic-gradient(${stops.join(', ')})`;
  }, [leftStatsItems]);

  const renderOrderQueryLeftPanel = useCallback(
    ({ leftTab, onLeftTabChange, leftRowsForRender, onLeftRowClick, hasManualFilterActive }: LeftFilterPanelProps) => (
      <OrderQueryLeftPanel
        leftTab={leftTab}
        onLeftTabChange={onLeftTabChange}
        leftRowsForRender={leftRowsForRender}
        onLeftRowClick={onLeftRowClick}
        leftStatsItems={leftStatsItems}
        leftStatsPieBackground={leftStatsPieBackground}
        globalOrderTotal={leftScopeTotal}
        isFetching={isLeftScopeFetching}
        statsScopeText={hasManualFilterActive ? '口径：当前查询结果（未叠加左侧）' : '口径：当前查询结果'}
      />
    ),
    [isLeftScopeFetching, leftScopeTotal, leftStatsItems, leftStatsPieBackground],
  );

  useLeftFilterModule<OrderSummary, OrderQueryLeftFilters>({
    rows: leftScopeRows,
    filters: leftFilters,
    onFiltersChange: setLeftFilters,
    initialExpandedKeys: ['query-scope-group'],
    leftPanelWidth: 220,
    emptyRowLabel: '无匹配项',
    hasManualFilter: hasOrderQueryLeftManualFilter,
    filterRows: (sourceRows) => sourceRows,
    buildCheckedKeys: buildOrderQueryLeftCheckedKeys,
    buildLeftNodes: buildOrderQueryLeftNodes,
    reduceFiltersByNode: reduceOrderQueryLeftFiltersByNode,
    renderPanel: renderOrderQueryLeftPanel,
  });

  const renderDetailContent = (content: React.ReactNode) => {
    if (!activeOrder) {
      return <Empty description="请选择订单查看详情" />;
    }
    if (detailQuery.isFetching) {
      return (
        <div className="order-query-detail-loading">
          <span>加载中...</span>
        </div>
      );
    }
    if (detailQuery.error) {
      return (
        <div className="order-query-detail-error">
          <Space direction="vertical">
            <span>详情加载失败</span>
            <Button size="small" onClick={() => detailQuery.refetch()}>
              重试
            </Button>
          </Space>
        </div>
      );
    }
    return content;
  };

  const detailPanels = useMemo(
    () => [
      {
        key: 'tags',
        label: '标记',
        children: renderDetailContent(
          <Space direction="vertical" size={8}>
            <div className="order-query-detail-tags">
              <OrderTagBadges
                tags={displayOrderTags}
                emptyText={<Typography.Text type="secondary">暂无标记</Typography.Text>}
              />
              <Tooltip title={canWriteOrders ? '新增标记' : '暂无权限'}>
                <span>
                  <Button
                    size="small"
                    type="text"
                    icon={<PlusOutlined />}
                    aria-label="Add tag"
                    disabled={!canWriteOrders}
                    onClick={openTagModal}
                  />
                </span>
              </Tooltip>
            </div>
          </Space>,
        ),
      },
      {
        key: 'flag',
        label: '平台旗帜',
        children: renderDetailContent(
          <Space size={8}>
            <PlatformTag platform={detailOrder?.platform ?? ''} />
            <Tag>{detailOrder?.platform_status ?? '未同步'}</Tag>
          </Space>,
        ),
      },
      {
        key: 'common',
        label: '常用功能',
        children: (
          <Space wrap>
            <Button size="small" icon={<ReloadOutlined />} onClick={handleRefresh}>
              刷新 F5
            </Button>
            <Button size="small" onClick={handleToggleSelectAll}>
              {allCurrentSelected ? '取消全选' : '全选'}
            </Button>
            <Tooltip title={receiverEditDisabledReason}>
              <span>
                <Button size="small" disabled={receiverEditDisabled} onClick={openReceiverModal}>
                  修改收货信息
                </Button>
              </span>
            </Tooltip>
          </Space>
        ),
      },
      {
        key: 'detail',
        label: '详情',
        children: renderDetailContent(
          <Descriptions size="small" column={1}>
            <Descriptions.Item label="订单编号">{detailOrder?.id ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="收货人">
              {detailOrder?.receiver_name ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="手机">{detailOrder?.receiver_phone ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="收货地址">
              {detailOrder?.receiver_address ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="发货仓库">
              {(detailOrder as OrderDetail['order'] & { warehouse_name?: string })
                ?.warehouse_name ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="配送方式">
              {(detailOrder as OrderDetail['order'] & { delivery_method?: string })
                ?.delivery_method ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="物流公司">
              {detailOrder?.logistics_company ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="物流单号">
              {detailOrder?.tracking_no ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="订单类型">
              {(detailOrder as OrderDetail['order'] & { order_type?: string })?.order_type ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="订单状态">
              {orderStatusMeta[detailOrder?.oms_status ?? 'NEW']?.label ??
                detailOrder?.oms_status ??
                '-'}
            </Descriptions.Item>
          </Descriptions>,
        ),
      },
      {
        key: 'notes',
        label: '备注',
        children: renderDetailContent(
          <Space direction="vertical" size={8} className="order-query-detail-notes">
            <div>
              <Typography.Text type="secondary">客户备注</Typography.Text>
              <div>
                {(detailOrder as OrderDetail['order'] & { customer_note?: string })
                  ?.customer_note ?? '-'}
              </div>
            </div>
            <div>
              <Typography.Text type="secondary">客服备注</Typography.Text>
              <div>{detailData?.notes?.[0]?.note ?? '-'}</div>
            </div>
            <Button size="small" onClick={openNoteModal} disabled={!canWriteOrders}>
              编辑备注
            </Button>
          </Space>,
        ),
      },
    ],
    [
      activeOrder,
      allCurrentSelected,
      canWriteOrders,
      listItems,
      detailData,
      detailOrder,
      detailQuery.error,
      detailQuery.isFetching,
      receiverEditDisabled,
      receiverEditDisabledReason,
    ],
  );

  const supplementalDetailPanels = useMemo(
    () =>
      detailPanels.filter((panel) => {
        const key = String(panel?.key ?? '');
        return key === 'common' || key === 'flag' || key === 'notes';
      }),
    [detailPanels],
  );

  const rightPanel = useMemo(
    () => (
      <Space direction="vertical" size={8} style={{ width: '100%' }}>
        <UiModule05OrderReviewRightPanelTemplate
          detailError={detailQuery.error}
          detailLoadedId={detailLoadedId}
          orderId={detailOrder?.id ?? activeOrder?.id}
          platformOrderId={detailOrder?.platform_order_id}
          platformText={detailOrder?.platform}
          shopId={detailOrder?.shop_id}
          receiverName={detailOrder?.receiver_name}
          receiverPhone={detailOrder?.receiver_phone}
          logisticsCompany={detailLogisticsCompany}
          warehouseId={detailWarehouseId}
          totalAmountText={formatCurrency(detailOrder?.total_amount_cent)}
          courierOptions={mergedCourierOptions}
          warehouseOptions={mergedWarehouseOptions}
          logisticsUpdating={detailUpdatingField === 'logistics'}
          warehouseUpdating={detailUpdatingField === 'warehouse'}
          detailSelectDisabled={detailSelectDisabled}
          orderTagsApplying={orderTagsApplying}
          markTags={displayOrderTags}
          markMenuItems={tagMenuItems}
          onMarkSelect={handleMarkSelect}
          onLogisticsChange={handleDetailLogisticsChange}
          onWarehouseChange={handleDetailWarehouseChange}
        />
        <UiModule05DetailPanel
          panels={supplementalDetailPanels}
          defaultActiveKeys={['common', 'notes']}
        />
      </Space>
    ),
    [
      activeOrder?.id,
      detailLoadedId,
      detailLogisticsCompany,
      detailOrder?.id,
      detailOrder?.platform,
      detailOrder?.platform_order_id,
      detailOrder?.receiver_name,
      detailOrder?.receiver_phone,
      detailOrder?.shop_id,
      detailOrder?.total_amount_cent,
      detailQuery.error,
      detailSelectDisabled,
      detailUpdatingField,
      detailWarehouseId,
      handleDetailLogisticsChange,
      handleDetailWarehouseChange,
      handleMarkSelect,
      mergedCourierOptions,
      mergedWarehouseOptions,
      displayOrderTags,
      orderTagsApplying,
      supplementalDetailPanels,
      tagMenuItems,
    ],
  );

  const exceptionActionBar = useMemo(() => {
    if (!enableExceptionActions) {
      return null;
    }
    const disabled = !canWriteOrders || !resolveTargetOrders().length || exceptionProcessing;
    return (
      <Space size={8} wrap>
        <Button
          size="small"
          disabled={disabled}
          onClick={() => confirmExceptionAction('RESOLVE', '放行')}
        >
          放行
        </Button>
        <Button
          size="small"
          danger
          disabled={disabled}
          onClick={() => confirmExceptionAction('REJECT', '驳回')}
        >
          驳回
        </Button>
        <Button
          size="small"
          disabled={disabled}
          onClick={() => confirmExceptionAction('REOPEN', '转人工')}
        >
          转人工
        </Button>
      </Space>
    );
  }, [
    canWriteOrders,
    confirmExceptionAction,
    enableExceptionActions,
    exceptionProcessing,
    resolveTargetOrders,
  ]);

  const queryActionBar = useMemo(() => {
    if (!exceptionActionBar) {
      return null;
    }
    return <div className="order-query-bottom-actions">{exceptionActionBar}</div>;
  }, [exceptionActionBar]);

  const goodsTabContent = useMemo(
    () =>
      renderDetailContent(
        <UiModule04BottomTable syncKey={`${activeBottomTab}-${detailGoodsRows.length}`}>
          <Table
            size="small"
            rowKey={(record) => record.__localKey}
            columns={goodsColumns}
            dataSource={detailGoodsRows}
            pagination={false}
            scroll={{ x: 'max-content' }}
            locale={{ emptyText: <Empty description="暂无货品明细" /> }}
          />
        </UiModule04BottomTable>,
      ),
    [activeBottomTab, detailGoodsRows, goodsColumns, renderDetailContent],
  );

  const auditTabContent = useMemo(
    () =>
      renderDetailContent(
        <UiModule04BottomTable
          syncKey={`${activeBottomTab}-${detailData?.audit_logs?.length ?? 0}`}
        >
          <Table
            size="small"
            rowKey={(record) => record.audit_id}
            columns={auditColumns}
            dataSource={detailData?.audit_logs ?? []}
            pagination={false}
            scroll={{ x: 'max-content' }}
            locale={{ emptyText: <Empty description="暂无日志" /> }}
          />
        </UiModule04BottomTable>,
      ),
    [activeBottomTab, auditColumns, detailData?.audit_logs, renderDetailContent],
  );

  const packageTabContent = useMemo(
    () =>
      renderDetailContent(
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          {orderShipmentsQuery.isFetching ? <Typography.Text>包裹加载中...</Typography.Text> : null}
          {orderShipmentsQuery.error ? (
            <Alert
              type="warning"
              showIcon
              message="包裹加载失败"
              description={
                orderShipmentsQuery.error instanceof Error
                  ? orderShipmentsQuery.error.message
                  : '包裹加载失败'
              }
            />
          ) : null}
          {!orderShipmentsQuery.isFetching &&
          !orderShipmentsQuery.error &&
          orderShipments.length === 0 ? (
            <Empty description="暂无包裹" />
          ) : (
            orderShipments.map((shipment, index) => {
              const packageQuery = packageQueries[index];
              const packages = packagesMap.get(shipment.shipment_id) ?? [];
              return (
                <Card
                  key={shipment.shipment_id}
                  size="small"
                  title={`包裹 ${shipment.shipment_no ?? shipment.shipment_id}`}
                  extra={
                    <Button
                      size="small"
                      disabled={!canWriteShipping}
                      onClick={() => handleOpenPackageEditor(shipment)}
                    >
                      维护包裹
                    </Button>
                  }
                >
                  <Space direction="vertical" size={8} style={{ width: '100%' }}>
                    <Descriptions size="small" column={2}>
                      <Descriptions.Item label="发货单号">
                        {shipment.shipment_no ?? shipment.shipment_id}
                      </Descriptions.Item>
                      <Descriptions.Item label="状态">{shipment.status}</Descriptions.Item>
                      <Descriptions.Item label="物流公司">
                        {shipment.carrier_code ?? '-'}
                      </Descriptions.Item>
                      <Descriptions.Item label="运单号">
                        {shipment.tracking_no ?? '-'}
                      </Descriptions.Item>
                    </Descriptions>
                    <Typography.Text type="secondary">包裹明细</Typography.Text>
                    {packageQuery?.isFetching ? (
                      <Typography.Text>包裹明细加载中...</Typography.Text>
                    ) : packageQuery?.error ? (
                      <Alert
                        type="warning"
                        showIcon
                        message="包裹明细加载失败"
                        description={
                          packageQuery.error instanceof Error
                            ? packageQuery.error.message
                            : '包裹明细加载失败'
                        }
                      />
                    ) : packages.length > 0 ? (
                      <Descriptions size="small" column={1} bordered>
                        {packages.map((pkg, pkgIndex) => (
                          <Descriptions.Item key={pkg.package_id} label={`包裹 ${pkgIndex + 1}`}>
                            {`${pkg.package_no} / ${pkg.tracking_no ?? '-'} / ${
                              pkg.carrier_code ?? '-'
                            } / ${pkg.weight_gram ?? '-'}g`}
                          </Descriptions.Item>
                        ))}
                      </Descriptions>
                    ) : (
                      <Typography.Text type="secondary">暂无包裹明细</Typography.Text>
                    )}
                  </Space>
                </Card>
              );
            })
          )}
        </Space>,
      ),
    [
      canWriteShipping,
      orderShipments,
      orderShipmentsQuery.error,
      orderShipmentsQuery.isFetching,
      packageQueries,
      packagesMap,
      renderDetailContent,
    ],
  );

  const bottomTabs = useMemo(
    () => (
      <Tabs
        size="small"
        type="card"
        activeKey={activeBottomTab}
        onChange={setActiveBottomTab}
        items={[
          { key: 'goods', label: '货品', children: goodsTabContent },
          { key: 'logs', label: '日志', children: auditTabContent },
          { key: 'gift', label: '赠品策略', children: <Empty description="待接入" /> },
          { key: 'coupon', label: '优惠卡券', children: <Empty description="待接入" /> },
          { key: 'upstream', label: '上游单据', children: <Empty description="待接入" /> },
          { key: 'downstream', label: '下游单据', children: <Empty description="待接入" /> },
          { key: 'same', label: '同名订单', children: <Empty description="待接入" /> },
          { key: 'dispatch', label: '分拨发货申请单', children: <Empty description="待接入" /> },
          { key: 'shipment', label: '发货单', children: <Empty description="待接入" /> },
          { key: 'aftersale', label: '售后单', children: <Empty description="待接入" /> },
          { key: 'cost', label: '销售费用', children: <Empty description="待接入" /> },
          { key: 'settlement', label: '结算申请单', children: <Empty description="待接入" /> },
          { key: 'package', label: '包裹', children: packageTabContent },
          { key: 'packing', label: '包材', children: <Empty description="待接入" /> },
        ]}
      />
    ),
    [activeBottomTab, auditTabContent, goodsTabContent, packageTabContent],
  );

  const queryToolbar = (
    <OrderReviewWorkbenchToolbarTemplate
      leftContent={
        <OrderQueryToolbarLeft
          form={form}
          filters={filters}
          onValuesChange={handleFilterChange}
          timeRangeOptions={timeRangeOptions}
          archiveStatusOptions={archiveStatusOptions}
          platformOptions={platformOptions}
        />
      }
      searchValue={searchValue}
      onSearchChange={handleKeywordChange}
      onSearch={() => handleSearch(searchValue)}
      searchPlaceholder="网店订单号/客户帐号/手机"
      searchInputClassName="order-query-search"
      searchInputTestId="orders-keyword-input"
      searchButtonTestId="orders-search-submit"
      searchButtonAriaLabel="查询"
      canExport={canExport}
      exporting={exporting}
      onExport={handleExport}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('帮助功能待接入')}
    />
  );

  const queryHeader = headerExtra ? <div className="order-query-header">{headerExtra}</div> : null;
  const queryTableClassName = ['order-query-table', 'order-workbench-table-module-03'].filter(Boolean).join(' ');
  const footerSummary = (
    <>
      <span>{`共${total} 条，已勾选${selectedRowKeys.length} 条`}</span>
      <span className="order-workbench-load-timing" data-testid="order-query-load-timing">
        {isFetching ? '加载中...' : lastFetchDurationMs == null ? '加载耗时 --' : `加载耗时 ${lastFetchDurationMs}ms`}
      </span>
    </>
  );

  return (
    <>
      <OrderReviewWorkbenchTemplate
        pageTestId="order-query-page"
        className="order-query-page"
        tableClassName={queryTableClassName}
        tableContainerRef={tableContainerRef}
        toolbar={queryToolbar}
        header={queryHeader}
        actionBar={queryActionBar}
        tableContent={
          <OrderWorkbenchMainGridEntry
            variant="query"
            rows={listItems}
            columns={dataGridColumns}
            rowKey={(record) => record.id}
            columnSettingsKey="orders.query.main-grid"
            dataTestId="orders-grid"
            testIdPrefix="orders-grid"
            headerFilters={columnQueryCells}
            enableRowModifierSelection
            selectedRowKeys={selectedRowKeys}
            activeRowKey={activeOrder?.id ?? null}
            onSelectionChange={handleSelectionChange}
            onActiveRowChange={(record) => setActiveOrder(record)}
            onRowClick={handleTableRowClick}
            onRowDoubleClick={handleTableRowDoubleClick}
            onRowContextMenu={handleTableRowContextMenu}
            loading={tableLoading}
            error={error}
            errorTitle="订单列表加载失败"
            emptyText="暂无订单记录"
            onRetry={() => refetch()}
          />
        }
        footerSummary={footerSummary}
        footerSyncKey={tableFooterSyncKey}
        total={total}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={false}
        rightPanel={rightPanel}
        bottomPanel={bottomTabs}
      />
      <Modal
        title="匹配无效货品"
        open={matchModalOpen}
        onCancel={() => setMatchModalOpen(false)}
        onOk={handleSubmitMatch}
        okButtonProps={{
          disabled: !canWriteOrders || !canManageMappings || matchSaving,
          loading: matchSaving,
        }}
      >
        <Form layout="vertical">
          <Form.Item label="待匹配货品">
            <Select
              value={selectedMatchItem?.id ?? undefined}
              onChange={(value) => setMatchItemId(value)}
              options={invalidGoodsRows.map((item) => ({
                value: item.id,
                label: `${item.shopProductName ?? item.title ?? '未命名'} / ${item.shopSpec ?? '默认规格'}`,
              }))}
              placeholder="请选择无效货品"
            />
          </Form.Item>
          {selectedMatchItem ? (
            <div className="order-query-match-summary">
              <Typography.Text type="secondary">
                无效原因：{resolveInvalidReason(selectedMatchItem)}
              </Typography.Text>
              <div>网店商品：{selectedMatchGoodsRow?.shopProductName ?? selectedMatchItem.title ?? '-'}</div>
              <div>规格：{selectedMatchGoodsRow?.shopSpec ?? '-'}</div>
              <div>平台编码：{selectedMatchItem.platform_sku_id ?? '-'}</div>
              <div>商家编码：{selectedMatchItem.outer_sku_id ?? '-'}</div>
            </div>
          ) : null}
          <Form.Item label="ERP货品">
            <Select
              showSearch
              value={matchSkuId ?? undefined}
              onSearch={(value) => setSkuKeyword(value)}
              onChange={(value) => setMatchSkuId(value)}
              filterOption={false}
              loading={skuLoading}
              placeholder="搜索条码/编号/名称"
              options={skuOptions.map((sku) => ({
                value: sku.sku_id,
                label: `${sku.sku_code} ${sku.name ?? ''}`.trim(),
              }))}
            />
          </Form.Item>
          <Checkbox
            checked={matchApplyAll}
            onChange={(event) => setMatchApplyAll(event.target.checked)}
          >
            应用到所有相同平台编码/规格编码
          </Checkbox>
        </Form>
      </Modal>
      <Modal
        title="新增标记"
        open={tagModalOpen}
        onCancel={() => setTagModalOpen(false)}
        onOk={submitTags}
        okButtonProps={{ disabled: !canWriteOrders }}
      >
        <Form layout="vertical">
          <Form.Item label="标记内容">
            <Select
              mode="multiple"
              allowClear
              showSearch
              value={selectedTags}
              onChange={(value) => setSelectedTags(value)}
              options={orderTagOptions}
              optionFilterProp="label"
              placeholder="请选择标记"
              notFoundContent="暂无标记，请先在标记目录新建"
              loading={orderTagDefinitionsQuery.isLoading}
            />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="编辑备注"
        open={noteModalOpen}
        onCancel={() => setNoteModalOpen(false)}
        onOk={submitNote}
        okButtonProps={{ disabled: !canWriteOrders }}
      >
        <Form layout="vertical">
          <Form.Item label="备注内容">
            <Input.TextArea
              rows={3}
              value={noteInput}
              onChange={(event) => setNoteInput(event.target.value)}
              placeholder="请输入备注内容"
            />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title={`包裹维护${
          packageEditingShipment
            ? ` - ${packageEditingShipment.shipment_no ?? packageEditingShipment.shipment_id}`
            : ''
        }`}
        open={packageEditOpen}
        onCancel={() => {
          setPackageEditOpen(false);
          setPackageEditingShipment(null);
          packageForm.resetFields();
        }}
        onOk={handleSubmitPackages}
        confirmLoading={packageSaving}
        okButtonProps={{ disabled: !canWriteShipping }}
        width={760}
      >
        <Form form={packageForm} layout="vertical">
          <Form.List name="packages">
            {(fields, { add, remove }) => (
              <Space direction="vertical" size={12} style={{ width: '100%' }}>
                {fields.map((field) => (
                  <Space key={field.key} align="baseline" wrap>
                    <Form.Item
                      {...field}
                      label="包裹号"
                      name={[field.name, 'package_no']}
                      rules={[{ required: true, message: '请输入包裹号' }]}
                    >
                      <Input placeholder="包裹号" />
                    </Form.Item>
                    <Form.Item {...field} label="物流公司" name={[field.name, 'carrier_code']}>
                      <Input placeholder="物流公司" />
                    </Form.Item>
                    <Form.Item {...field} label="运单号" name={[field.name, 'tracking_no']}>
                      <Input placeholder="运单号" />
                    </Form.Item>
                    <Form.Item {...field} label="重量(g)" name={[field.name, 'weight_gram']}>
                      <InputNumber min={1} style={{ width: 120 }} />
                    </Form.Item>
                    <Button type="link" danger onClick={() => remove(field.name)}>
                      删除
                    </Button>
                  </Space>
                ))}
                <Button type="dashed" onClick={() => add()}>
                  添加包裹
                </Button>
              </Space>
            )}
          </Form.List>
        </Form>
      </Modal>
      <Modal
        title="修改收货信息"
        open={receiverModalOpen}
        onCancel={() => setReceiverModalOpen(false)}
        onOk={submitReceiverUpdate}
        confirmLoading={receiverSaving}
        okButtonProps={{ disabled: !canWriteOrders }}
      >
        <Form form={receiverForm} layout="vertical">
          <Form.Item
            name="receiver_name"
            label="收货人"
            rules={[{ required: true, message: '请输入收货人' }]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="receiver_phone"
            label="手机"
            rules={[
              { required: true, message: '请输入手机号' },
              { pattern: /^\d{7,}$/, message: '手机号格式不正确' },
            ]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="receiver_address"
            label="收货地址"
            rules={[{ required: true, message: '请输入收货地址' }]}
          >
            <Input.TextArea rows={3} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
