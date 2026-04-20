import './channelsPage.css';

import { ReloadOutlined } from '@ant-design/icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Card,
  Descriptions,
  Drawer,
  Form,
  Input,
  Select,
  Space,
  Tag,
  Tabs,
  Typography,
  message,
} from 'antd';
import type { MenuProps } from 'antd';
import dayjs from 'dayjs';
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Key,
  type MouseEvent as ReactMouseEvent,
} from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

import type { Shop, ShopCreatePayload, ShopUpdatePayload } from '@/api/shops';
import {
  createShop,
  getShopAuthStatus,
  listShops,
  requestTokenRefresh,
  startShopAuthorize,
  submitShopAuthCallback,
  updateShop,
} from '@/api/shops';
import type { OrderSummary } from '@/api/orders';
import { PlatformTag } from '@/components/PlatformTag';
import {
  UiModule03ContextMenu,
  createUiModule08StrategyScreenFilterState,
  type UiModule08StrategyScreenFilterChangeHandler,
} from '@/components/ui-modules';
import {
  WorkbenchPageTemplate,
  WorkbenchToolbar,
  useWorkbenchSavedViewState,
  useWorkbenchTablePagination,
} from '@/components/page-templates';
import type { PlatformCode } from '@/features/platform';
import { platformOptions, platformValues } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';
import { OrderReviewLeftPanel } from '@/pages/orders/review-native/OrderReviewLeftPanel';
import type { ColumnFilters as OrderReviewMainColumnFilters } from '@/pages/orders/review-native/types';
import { OrderWorkbenchMainGridEntry } from '@/pages/orders/shared/OrderWorkbenchMainGridEntry';
import { useLeftFilterModule, type LeftFilterPanelProps } from '@/pages/orders/shared/useLeftFilterModule';
import { routerBasePath, routerMode } from '@/utils/env';

import { buildShopAuthCallbackUrl, parseShopAuthCallbackSearch } from './shopAccessModel';

type ShopStatusFilter = 'all' | 'AUTHORIZED' | 'EXPIRED' | 'DISABLED';
type TokenHealthFilter = 'all' | 'healthy' | 'expiring' | 'expired' | 'missing' | 'error';
type TokenHealth = Exclude<TokenHealthFilter, 'all'>;
type ShopSecondaryActionKey =
  | 'edit'
  | 'authorize'
  | 'refresh-token'
  | 'toggle-status'
  | 'goto-shop-access'
  | 'goto-sync-jobs';

type ShopWorkbenchFilters = {
  platform?: PlatformCode[];
  status?: Array<Exclude<ShopStatusFilter, 'all'>>;
  tokenHealth?: TokenHealth[];
};

type ShopColumnFilterState = {
  nameKeyword: string;
  platformShopIdKeyword: string;
  shopIdKeyword: string;
  status: ShopStatusFilter;
  tokenHealth: TokenHealthFilter;
};

type ShopFormValues = {
  platform: PlatformCode;
  platform_shop_id: string;
  name: string;
  status: string;
  auth_meta_json_input?: string;
};

type ShopContextMenuState = {
  open: boolean;
  position: { x: number; y: number } | null;
  shop: Shop | null;
};

type ChannelsWorkbenchSavedViewState = {
  searchInput?: string;
  searchKeyword?: string;
  selectedRowKeys?: string[];
  focusedOrderKey?: string | null;
};

const shopStatusOptions: Array<{ label: string; value: ShopStatusFilter }> = [
  { label: '全部状态', value: 'all' },
  { label: '已授权', value: 'AUTHORIZED' },
  { label: '授权过期', value: 'EXPIRED' },
  { label: '已停用', value: 'DISABLED' },
];

const tokenHealthOptions: Array<{ label: string; value: TokenHealthFilter }> = [
  { label: '全部 Token', value: 'all' },
  { label: '健康', value: 'healthy' },
  { label: '即将过期', value: 'expiring' },
  { label: '已过期', value: 'expired' },
  { label: '缺失', value: 'missing' },
  { label: '异常', value: 'error' },
];

const leftPanelStatusOptions = shopStatusOptions.filter(
  (option): option is { label: string; value: Exclude<ShopStatusFilter, 'all'> } =>
    option.value !== 'all',
);

const leftPanelTokenOptions = tokenHealthOptions.filter(
  (option): option is { label: string; value: TokenHealth } => option.value !== 'all',
);

const editableStatusOptions = shopStatusOptions
  .filter((option) => option.value !== 'all')
  .map((option) => ({ label: option.label, value: option.value }));

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveStatusTag = (status: string) => {
  switch (status) {
    case 'AUTHORIZED':
      return { color: 'success', label: '已授权' };
    case 'EXPIRED':
      return { color: 'warning', label: '授权过期' };
    case 'DISABLED':
      return { color: 'default', label: '已停用' };
    default:
      return { color: 'processing', label: status || '未知状态' };
  }
};

const resolveTokenHealth = (shop: Shop): TokenHealth => {
  const token = shop.token_status;
  if (!token || !token.has_access_token) {
    return 'missing';
  }
  if (token.last_error?.trim()) {
    return 'error';
  }
  if (token.is_expired) {
    return 'expired';
  }
  if (token.is_expiring_soon) {
    return 'expiring';
  }
  return 'healthy';
};

const resolveTokenTag = (health: TokenHealth) => {
  switch (health) {
    case 'healthy':
      return { color: 'success', label: '健康' };
    case 'expiring':
      return { color: 'warning', label: '即将过期' };
    case 'expired':
      return { color: 'error', label: '已过期' };
    case 'missing':
      return { color: 'default', label: '缺失' };
    case 'error':
      return { color: 'volcano', label: '异常' };
    default:
      return { color: 'default', label: '-' };
  }
};

const parseAuthMetaInput = (raw?: string): Record<string, unknown> | undefined => {
  if (!raw || !raw.trim()) {
    return undefined;
  }
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
    throw new Error('授权元信息必须是 JSON 对象');
  } catch (error) {
    throw new Error(error instanceof Error ? error.message : '授权元信息 JSON 格式错误');
  }
};

const EMPTY_SHOP_ITEMS: Shop[] = [];
const EMPTY_SHOP_WORKBENCH_FILTERS: ShopWorkbenchFilters = {};

const normalizeStringArray = (value?: string[]) =>
  value ? Array.from(new Set(value.map((item) => item.trim()).filter(Boolean))) : [];
const normalizeNullableString = (value: unknown) => {
  const nextValue = String(value ?? '').trim();
  return nextValue || null;
};

const resolveConicGradientBackground = (segments: Array<{ ratio: number; color: string }>) => {
  if (segments.length === 0) {
    return 'conic-gradient(#dce4ee 0deg 360deg)';
  }
  let angle = 0;
  const stops = segments.map((segment) => {
    const start = angle;
    const end = angle + segment.ratio * 360;
    angle = end;
    return `${segment.color} ${start.toFixed(2)}deg ${end.toFixed(2)}deg`;
  });
  if (angle < 360) {
    stops.push(`#dce4ee ${angle.toFixed(2)}deg 360deg`);
  }
  return `conic-gradient(${stops.join(', ')})`;
};

const mapShopToOrderSummary = (shop: Shop): OrderSummary => {
  const tokenHealth = resolveTokenHealth(shop);
  const tokenLabel = resolveTokenTag(tokenHealth).label;
  const statusForReview =
    shop.status === 'AUTHORIZED'
      ? 'REVIEW_PASSED'
      : shop.status === 'EXPIRED'
        ? 'REVIEW_REQUIRED'
        : 'REJECTED';

  const orderStatus = shop.status === 'DISABLED' ? 'CANCELED' : 'PENDING_SHIPMENT';
  const exceptionFlags =
    tokenHealth === 'healthy'
      ? []
      : tokenHealth === 'expiring'
        ? ['STOP_SHIPPING']
        : tokenHealth === 'expired'
          ? ['NO_LOGISTICS']
          : tokenHealth === 'missing'
            ? ['MISSING_SKU']
            : ['ORDER_FROZEN'];

  return {
    id: shop.shop_id,
    order_id: shop.shop_id,
    platform_order_id: shop.platform_shop_id,
    platform: shop.platform,
    shop_id: shop.shop_id,
    receiver_name: shop.name,
    receiver_phone: tokenLabel,
    review_status: statusForReview,
    order_status: orderStatus,
    tags: [tokenLabel],
    exception_flags: exceptionFlags,
    logistics_company: tokenLabel,
    created_at: shop.updated_at,
    total_amount_cent: 0,
  } as OrderSummary;
};

export const ChannelsPage = () => {
  const tenantId = useTenantId();
  const userId = useSessionStore((state) => state.userId);
  const canWrite = useCan('shops.write');
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();

  const [drawerForm] = Form.useForm<ShopFormValues>();

  const [workbenchFilters, setWorkbenchFilters] = useState<ShopWorkbenchFilters>(
    EMPTY_SHOP_WORKBENCH_FILTERS,
  );
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [shopColumnFilters] = useState<ShopColumnFilterState>({
    nameKeyword: '',
    platformShopIdKeyword: '',
    shopIdKeyword: '',
    status: 'all',
    tokenHealth: 'all',
  });
  const [reviewColumnFilters, setReviewColumnFilters] = useState<OrderReviewMainColumnFilters>({});
  const [focusedOrderKey, setFocusedOrderKey] = useState<string | null>(null);
  const channelsSavedViewKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    const normalizedUserId = String(userId ?? '').trim() || 'anonymous';
    return `channels.workbench.savedView.v1.${tenantId}.${normalizedUserId}`;
  }, [tenantId, userId]);
  const channelsPaginationStorageKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    const normalizedUserId = String(userId ?? '').trim() || 'anonymous';
    return `channels.workbench.pagination.v1.${tenantId}.${normalizedUserId}`;
  }, [tenantId, userId]);
  const channelsWorkbenchStateCacheKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    const normalizedUserId = String(userId ?? '').trim() || 'anonymous';
    return `channels.workbench.state.v1.${tenantId}.${normalizedUserId}`;
  }, [tenantId, userId]);
  const { pagination, setPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions: ['20', '50', '100', '200'],
    storageKey: channelsPaginationStorageKey,
  });
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [selectedShop, setSelectedShop] = useState<Shop | null>(null);
  const [mainContextMenu, setMainContextMenu] = useState<ShopContextMenuState>({
    open: false,
    position: null,
    shop: null,
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingShop, setEditingShop] = useState<Shop | null>(null);
  const [saving, setSaving] = useState(false);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [refreshingId, setRefreshingId] = useState<string | null>(null);
  const [authorizingId, setAuthorizingId] = useState<string | null>(null);
  const [leftScreenFiltersDraft, setLeftScreenFiltersDraft] = useState(
    createUiModule08StrategyScreenFilterState,
  );
  const [, setLeftScreenFiltersApplied] = useState(
    createUiModule08StrategyScreenFilterState,
  );

  const callbackHandledRef = useRef<string | null>(null);

  const clearMainSelection = useCallback(() => {
    setSelectedShop(null);
    setSelectedRowKeys([]);
  }, []);

  const listQuery = useMemo(
    () => ({
      offset: 0,
      limit: 1000,
    }),
    [],
  );

  const shopsQuery = useQuery({
    queryKey: ['shops', tenantId, listQuery],
    queryFn: () => listShops(tenantId, listQuery),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const callbackState = useMemo(
    () => parseShopAuthCallbackSearch(location.search),
    [location.search],
  );

  const callbackMutation = useMutation({
    mutationFn: ({
      shopId,
      payload,
    }: {
      shopId: string;
      payload: Parameters<typeof submitShopAuthCallback>[2];
    }) => submitShopAuthCallback(tenantId, shopId, payload),
    onSuccess: () => {
      message.success('店铺授权回调已处理');
      void queryClient.invalidateQueries({ queryKey: ['shops', tenantId] });
      void queryClient.invalidateQueries({ queryKey: ['shop-auth-status', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '处理授权回调失败');
    },
  });
  useEffect(() => {
    if (!callbackState?.shopId) {
      callbackHandledRef.current = null;
      return;
    }

    const key = [
      callbackState.shopId,
      callbackState.code ?? '',
      callbackState.error ?? '',
      callbackState.state ?? '',
    ].join('::');

    if (callbackHandledRef.current === key) {
      return;
    }
    callbackHandledRef.current = key;

    callbackMutation.mutate({
      shopId: callbackState.shopId,
      payload: {
        code: callbackState.code,
        state: callbackState.state,
        error: callbackState.error,
        error_description: callbackState.errorDescription,
      },
    });

    const params = new URLSearchParams(location.search);
    ['auth_callback', 'shop_id', 'code', 'state', 'error', 'error_description'].forEach((item) =>
      params.delete(item),
    );
    const nextSearch = params.toString();
    navigate(
      {
        pathname: location.pathname,
        search: nextSearch ? `?${nextSearch}` : '',
      },
      { replace: true },
    );
  }, [callbackMutation, callbackState, location.pathname, location.search, navigate]);

  const hydrateChannelsSavedView = useCallback((savedView: ChannelsWorkbenchSavedViewState) => {
    if (typeof savedView.searchInput === 'string') {
      setSearchInput(savedView.searchInput);
    }
    if (typeof savedView.searchKeyword === 'string') {
      setSearchKeyword(savedView.searchKeyword);
    }
    if (Object.prototype.hasOwnProperty.call(savedView, 'selectedRowKeys')) {
      setSelectedRowKeys(normalizeStringArray(savedView.selectedRowKeys));
    }
    if (Object.prototype.hasOwnProperty.call(savedView, 'focusedOrderKey')) {
      setFocusedOrderKey(normalizeNullableString(savedView.focusedOrderKey));
    }
  }, []);

  const serializeChannelsSavedView = useCallback(
    (): ChannelsWorkbenchSavedViewState => ({
      searchInput,
      searchKeyword,
      selectedRowKeys: selectedRowKeys.map((item) => String(item)),
      focusedOrderKey,
    }),
    [focusedOrderKey, searchInput, searchKeyword, selectedRowKeys],
  );

  useWorkbenchSavedViewState<ChannelsWorkbenchSavedViewState>({
    storageKey: channelsSavedViewKey,
    hydrate: hydrateChannelsSavedView,
    serialize: serializeChannelsSavedView,
  });

  const sourceItems = shopsQuery.data?.items ?? EMPTY_SHOP_ITEMS;

  const hasWorkbenchLeftFilter = useCallback((filters: ShopWorkbenchFilters) => {
    return (
      normalizeStringArray(filters.platform).length > 0 ||
      normalizeStringArray(filters.status).length > 0 ||
      normalizeStringArray(filters.tokenHealth).length > 0
    );
  }, []);

  const filterByWorkbenchFilters = useCallback((rows: Shop[], filters: ShopWorkbenchFilters) => {
    const platformFilters = normalizeStringArray(filters.platform);
    const statusFilters = normalizeStringArray(filters.status);
    const tokenHealthFilters = normalizeStringArray(filters.tokenHealth);

    return rows.filter((shop) => {
      if (platformFilters.length > 0 && !platformFilters.includes(shop.platform)) {
        return false;
      }
      if (statusFilters.length > 0 && !statusFilters.includes(shop.status)) {
        return false;
      }
      if (tokenHealthFilters.length > 0 && !tokenHealthFilters.includes(resolveTokenHealth(shop))) {
        return false;
      }
      return true;
    });
  }, []);

  const buildShopLeftCheckedKeys = useCallback((filters: ShopWorkbenchFilters) => {
    const platformKeys = normalizeStringArray(filters.platform).map((value) => `platform:${value}`);
    const statusKeys = normalizeStringArray(filters.status).map((value) => `status:${value}`);
    const tokenHealthKeys = normalizeStringArray(filters.tokenHealth).map((value) => `token:${value}`);
    return new Set([...platformKeys, ...statusKeys, ...tokenHealthKeys]);
  }, []);

  const buildShopLeftNodes = useCallback((rows: Shop[]): OrderReviewAgLeftNode[] => {
    const platformChildren: OrderReviewAgLeftNode[] = [
      {
        key: 'platform:*',
        label: '全部平台',
        count: rows.length,
        nodeType: 'platform',
      },
      ...platformOptions.map((option) => ({
        key: `platform:${option.value}`,
        label: option.label,
        count: rows.filter((shop) => shop.platform === option.value).length,
        nodeType: 'platform' as const,
      })),
    ];

    const statusChildren: OrderReviewAgLeftNode[] = [
      {
        key: 'status:*',
        label: '全部状态',
        count: rows.length,
        nodeType: 'status',
      },
      ...leftPanelStatusOptions.map((option) => ({
        key: `status:${option.value}`,
        label: option.label,
        count: rows.filter((shop) => shop.status === option.value).length,
        nodeType: 'status' as const,
      })),
    ];

    const tokenChildren: OrderReviewAgLeftNode[] = [
      {
        key: 'token:*',
        label: '全部 Token',
        count: rows.length,
        nodeType: 'exception',
      },
      ...leftPanelTokenOptions.map((option) => ({
        key: `token:${option.value}`,
        label: option.label,
        count: rows.filter((shop) => resolveTokenHealth(shop) === option.value).length,
        nodeType: 'exception' as const,
      })),
    ];

    return [
      {
        key: 'all:*',
        label: '全部店铺',
        count: rows.length,
        nodeType: 'group',
      },
      {
        key: 'group:platform',
        label: '平台',
        count: rows.length,
        nodeType: 'group',
        children: platformChildren,
      },
      {
        key: 'group:status',
        label: '接入状态',
        count: rows.length,
        nodeType: 'group',
        children: statusChildren,
      },
      {
        key: 'group:token',
        label: 'Token状态',
        count: rows.length,
        nodeType: 'group',
        children: tokenChildren,
      },
    ];
  }, []);

  const reduceShopFiltersByNode = useCallback(
    (previous: ShopWorkbenchFilters, node: { key: string; nodeType: string }): ShopWorkbenchFilters => {
      const cleared: ShopWorkbenchFilters = {
        ...previous,
        platform: undefined,
        status: undefined,
        tokenHealth: undefined,
      };

      if (node.key === 'all:*') {
        return cleared;
      }
      if (node.key === 'platform:*') {
        return { ...cleared, platform: undefined };
      }
      if (node.key === 'status:*') {
        return { ...cleared, status: undefined };
      }
      if (node.key === 'token:*') {
        return { ...cleared, tokenHealth: undefined };
      }
      if (node.key.startsWith('platform:')) {
        const value = node.key.slice('platform:'.length) as PlatformCode;
        return { ...cleared, platform: [value] };
      }
      if (node.key.startsWith('status:')) {
        const value = node.key.slice('status:'.length) as Exclude<ShopStatusFilter, 'all'>;
        return { ...cleared, status: [value] };
      }
      if (node.key.startsWith('token:')) {
        const value = node.key.slice('token:'.length) as TokenHealth;
        return { ...cleared, tokenHealth: [value] };
      }

      return previous;
    },
    [],
  );

  const leftStatsItems = useMemo(() => {
    const palette = ['#334e68', '#7da888', '#8bcdd3', '#d48935', '#b76e79'];
    const total = sourceItems.length || 1;
    const platformEntries = platformOptions
      .map((option) => ({
        key: option.value,
        label: option.label,
        value: sourceItems.filter((shop) => shop.platform === option.value).length,
      }))
      .filter((entry) => entry.value > 0);

    return platformEntries.map((entry, index) => ({
      key: entry.key,
      label: entry.label,
      value: entry.value,
      ratio: entry.value / total,
      color: palette[index % palette.length],
    }));
  }, [sourceItems]);

  const leftStatsPieBackground = useMemo(
    () =>
      resolveConicGradientBackground(
        leftStatsItems.map((item) => ({
          ratio: item.ratio,
          color: item.color,
        })),
      ),
    [leftStatsItems],
  );

  const updateLeftScreenFilter = useCallback<UiModule08StrategyScreenFilterChangeHandler>((key, value) => {
    setLeftScreenFiltersDraft((previous) => ({ ...previous, [key]: value }));
  }, []);

  const resolveSalesChannelToPlatform = useCallback((salesChannel: string): PlatformCode | undefined => {
    const normalized = salesChannel.trim().toLowerCase();
    if (!normalized) {
      return undefined;
    }
    const matched = platformOptions.find((option) => {
      const optionLabel = String(option.label).trim().toLowerCase();
      const optionValue = String(option.value).trim().toLowerCase();
      return normalized === optionLabel || normalized === optionValue;
    });
    return matched?.value as PlatformCode | undefined;
  }, []);

  const applyLeftScreenFilters = useCallback(() => {
    const next = {
      ...leftScreenFiltersDraft,
      orderQuery: leftScreenFiltersDraft.orderQuery.trim(),
      salesChannel: leftScreenFiltersDraft.salesChannel.trim(),
      warehouse: leftScreenFiltersDraft.warehouse.trim(),
      createdFrom: leftScreenFiltersDraft.createdFrom.trim(),
      createdTo: leftScreenFiltersDraft.createdTo.trim(),
      orderTag: leftScreenFiltersDraft.orderTag.trim(),
      excludedTag: leftScreenFiltersDraft.excludedTag.trim(),
      carrierLoss: leftScreenFiltersDraft.carrierLoss.trim(),
      regionKeyword: leftScreenFiltersDraft.regionKeyword.trim(),
      orderType: leftScreenFiltersDraft.orderType.trim(),
      includeSkuKeyword: leftScreenFiltersDraft.includeSkuKeyword.trim(),
    };
    setLeftScreenFiltersDraft(next);
    setLeftScreenFiltersApplied(next);
    setSearchInput(next.orderQuery);
    setSearchKeyword(next.orderQuery);
    setPagination((prev) => ({ ...prev, current: 1 }));
    clearMainSelection();

    const mappedPlatform = resolveSalesChannelToPlatform(next.salesChannel);
    setWorkbenchFilters((previous) => ({
      ...previous,
      platform: mappedPlatform ? [mappedPlatform] : undefined,
    }));
  }, [clearMainSelection, leftScreenFiltersDraft, resolveSalesChannelToPlatform]);

  const resetLeftScreenFilters = useCallback(() => {
    const next = createUiModule08StrategyScreenFilterState();
    setLeftScreenFiltersDraft(next);
    setLeftScreenFiltersApplied(next);
    setSearchInput('');
    setSearchKeyword('');
    setPagination((prev) => ({ ...prev, current: 1 }));
    clearMainSelection();
    setWorkbenchFilters((previous) => ({ ...previous, platform: undefined }));
  }, [clearMainSelection]);

  const renderChannelsLeftPanel = useCallback(
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
        globalOrderTotal={sourceItems.length}
        isGlobalStatsFetching={shopsQuery.isFetching}
        shouldFallbackToAll={true}
      />
    ),
    [
      applyLeftScreenFilters,
      leftScreenFiltersDraft,
      leftStatsItems,
      leftStatsPieBackground,
      resetLeftScreenFilters,
      shopsQuery.isFetching,
      sourceItems.length,
      updateLeftScreenFilter,
    ],
  );

  const { filteredRows: leftFilteredItems } = useLeftFilterModule<Shop, ShopWorkbenchFilters>({
    rows: sourceItems,
    filters: workbenchFilters,
    onFiltersChange: setWorkbenchFilters,
    initialExpandedKeys: ['group:platform', 'group:status', 'group:token'],
    hasManualFilter: hasWorkbenchLeftFilter,
    filterRows: filterByWorkbenchFilters,
    buildCheckedKeys: buildShopLeftCheckedKeys,
    buildLeftNodes: buildShopLeftNodes,
    reduceFiltersByNode: reduceShopFiltersByNode,
    renderPanel: renderChannelsLeftPanel,
    onBeforeLeafFilterApply: () => {
      setPagination((prev) => ({ ...prev, current: 1 }));
      clearMainSelection();
    },
  });

  const filteredItems = useMemo(() => {
    const keyword = searchKeyword.trim().toLowerCase();
    const nameKeyword = shopColumnFilters.nameKeyword.trim().toLowerCase();
    const platformShopIdKeyword = shopColumnFilters.platformShopIdKeyword.trim().toLowerCase();
    const shopIdKeyword = shopColumnFilters.shopIdKeyword.trim().toLowerCase();

    return leftFilteredItems.filter((shop) => {
      if (keyword) {
        const searchFields = [shop.name, shop.shop_id, shop.platform_shop_id, shop.platform, shop.status]
          .filter(Boolean)
          .map((value) => String(value).toLowerCase());
        if (!searchFields.some((value) => value.includes(keyword))) {
          return false;
        }
      }

      if (nameKeyword && !shop.name.toLowerCase().includes(nameKeyword)) {
        return false;
      }
      if (platformShopIdKeyword && !shop.platform_shop_id.toLowerCase().includes(platformShopIdKeyword)) {
        return false;
      }
      if (shopIdKeyword && !shop.shop_id.toLowerCase().includes(shopIdKeyword)) {
        return false;
      }
      if (shopColumnFilters.status !== 'all' && shop.status !== shopColumnFilters.status) {
        return false;
      }
      if (
        shopColumnFilters.tokenHealth !== 'all' &&
        resolveTokenHealth(shop) !== shopColumnFilters.tokenHealth
      ) {
        return false;
      }
      return true;
    });
  }, [
    shopColumnFilters.nameKeyword,
    shopColumnFilters.platformShopIdKeyword,
    shopColumnFilters.shopIdKeyword,
    shopColumnFilters.status,
    shopColumnFilters.tokenHealth,
    leftFilteredItems,
    searchKeyword,
  ]);

  const pagedItems = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    const start = (current - 1) * pageSize;
    return filteredItems.slice(start, start + pageSize);
  }, [filteredItems, pagination.current, pagination.pageSize]);

  useEffect(() => {
    if (!selectedShop) {
      return;
    }
    if (!filteredItems.some((item) => item.shop_id === selectedShop.shop_id)) {
      setSelectedShop(null);
    }
  }, [filteredItems, selectedShop]);

  useEffect(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    const maxPage = Math.max(1, Math.ceil(filteredItems.length / pageSize));
    if (current > maxPage) {
      setPagination((prev) => ({ ...prev, current: maxPage }));
    }
  }, [filteredItems.length, pagination.current, pagination.pageSize]);

  const rowShopsById = useMemo(() => {
    const map = new Map<string, Shop>();
    filteredItems.forEach((shop) => {
      map.set(shop.shop_id, shop);
    });
    return map;
  }, [filteredItems]);

  useEffect(() => {
    setSelectedRowKeys((previous) => {
      const next = previous.filter((key) => rowShopsById.has(String(key).trim()));
      return next.length === previous.length ? previous : next;
    });
  }, [rowShopsById]);

  useEffect(() => {
    const firstKey = String(selectedRowKeys[0] ?? '').trim();
    if (!firstKey) {
      if (selectedShop) {
        setSelectedShop(null);
      }
      return;
    }
    if (selectedShop && selectedRowKeys.some((key) => String(key).trim() === selectedShop.shop_id)) {
      return;
    }
    const nextShop = rowShopsById.get(firstKey) ?? null;
    if ((selectedShop?.shop_id ?? '') === (nextShop?.shop_id ?? '')) {
      return;
    }
    setSelectedShop(nextShop);
  }, [rowShopsById, selectedRowKeys, selectedShop]);

  const reviewRows = useMemo<OrderSummary[]>(
    () => pagedItems.map((shop) => mapShopToOrderSummary(shop)),
    [pagedItems],
  );

  const reviewFilterOptionRows = useMemo<OrderSummary[]>(
    () => filteredItems.map((shop) => mapShopToOrderSummary(shop)),
    [filteredItems],
  );

  const reviewShopOptions = useMemo(
    () =>
      sourceItems.map((shop) => ({
        label: shop.name,
        value: shop.shop_id,
      })),
    [sourceItems],
  );

  const reviewLogisticsOptions = useMemo(
    () =>
      leftPanelTokenOptions.map((option) => ({
        label: option.label,
        value: option.label,
      })),
    [],
  );

  const selectedAuthStatusQuery = useQuery({
    queryKey: ['shop-auth-status', tenantId, selectedShop?.shop_id],
    queryFn: () => getShopAuthStatus(tenantId, selectedShop!.shop_id),
    enabled: Boolean(tenantId && selectedShop?.shop_id),
    placeholderData: (previousData) => previousData,
  });

  const selectedShopView = selectedAuthStatusQuery.data ?? selectedShop;

  const selectedShopCallbackUrl = useMemo(() => {
    if (!selectedShopView || typeof window === 'undefined') {
      return null;
    }
    return buildShopAuthCallbackUrl({
      origin: window.location.origin,
      routerMode,
      routerBasePath,
      shopId: selectedShopView.shop_id,
    });
  }, [selectedShopView]);

  const toggleStatusMutation = useMutation({
    mutationFn: ({ shopId, status }: { shopId: string; status: string }) =>
      updateShop(tenantId, shopId, { status }),
    onMutate: ({ shopId }) => setTogglingId(shopId),
    onSuccess: (_payload, vars) => {
      message.success(vars.status === 'DISABLED' ? '店铺已停用' : '店铺已启用');
      void queryClient.invalidateQueries({ queryKey: ['shops', tenantId] });
      void queryClient.invalidateQueries({ queryKey: ['shop-auth-status', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '切换店铺状态失败');
    },
    onSettled: () => setTogglingId(null),
  });

  const refreshTokenMutation = useMutation({
    mutationFn: (shopId: string) => requestTokenRefresh(tenantId, shopId),
    onMutate: (shopId) => setRefreshingId(shopId),
    onSuccess: () => {
      message.success('已提交 Token 刷新请求');
      void queryClient.invalidateQueries({ queryKey: ['shops', tenantId] });
      void queryClient.invalidateQueries({ queryKey: ['shop-auth-status', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '提交 Token 刷新失败');
    },
    onSettled: () => setRefreshingId(null),
  });

  const authorizeMutation = useMutation({
    mutationFn: ({ shopId, redirectUri }: { shopId: string; redirectUri?: string }) =>
      startShopAuthorize(tenantId, shopId, { redirect_uri: redirectUri }),
    onMutate: ({ shopId }) => setAuthorizingId(shopId),
    onSuccess: async (payload) => {
      const popup = window.open(payload.authorize_url, '_blank', 'noopener,noreferrer');
      if (!popup) {
        if (navigator.clipboard?.writeText) {
          await navigator.clipboard.writeText(payload.authorize_url);
        }
        message.warning('浏览器阻止了新窗口，授权链接已复制到剪贴板');
      } else {
        message.success('授权窗口已打开');
      }
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '发起授权失败');
    },
    onSettled: () => setAuthorizingId(null),
  });

  const handleToolbarSearch = useCallback(() => {
    setSearchKeyword(searchInput.trim());
    setPagination((prev) => ({ ...prev, current: 1 }));
    clearMainSelection();
  }, [clearMainSelection, searchInput]);
  const openCreate = useCallback(() => {
    setEditingShop(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      platform: platformValues[0],
      status: 'AUTHORIZED',
      auth_meta_json_input: '',
    });
    setDrawerOpen(true);
  }, [drawerForm]);

  const openEdit = useCallback(
    (shop: Shop) => {
      setEditingShop(shop);
      drawerForm.setFieldsValue({
        platform: shop.platform,
        platform_shop_id: shop.platform_shop_id,
        name: shop.name,
        status: shop.status || 'AUTHORIZED',
        auth_meta_json_input: Object.keys(shop.auth_meta_json ?? {}).length
          ? JSON.stringify(shop.auth_meta_json, null, 2)
          : '',
      });
      setDrawerOpen(true);
    },
    [drawerForm],
  );

  const handleSave = useCallback(async () => {
    try {
      const values = await drawerForm.validateFields();
      const authMetaJson = parseAuthMetaInput(values.auth_meta_json_input);
      setSaving(true);

      if (editingShop) {
        const payload: ShopUpdatePayload = {
          name: values.name.trim(),
          status: values.status,
          auth_meta_json: authMetaJson,
        };
        await updateShop(tenantId, editingShop.shop_id, payload);
        message.success('店铺已更新');
      } else {
        const payload: ShopCreatePayload = {
          platform: values.platform,
          platform_shop_id: values.platform_shop_id.trim(),
          name: values.name.trim(),
          status: values.status,
          auth_meta_json: authMetaJson,
        };
        await createShop(tenantId, payload);
        message.success('店铺已创建');
      }

      setDrawerOpen(false);
      setEditingShop(null);
      await queryClient.invalidateQueries({ queryKey: ['shops', tenantId] });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  }, [drawerForm, editingShop, queryClient, tenantId]);

  const handleAuthorize = useCallback(
    (shop: Shop) => {
      const redirectUri =
        typeof window === 'undefined'
          ? undefined
          : buildShopAuthCallbackUrl({
              origin: window.location.origin,
              routerMode,
              routerBasePath,
              shopId: shop.shop_id,
            });
      authorizeMutation.mutate({ shopId: shop.shop_id, redirectUri });
    },
    [authorizeMutation],
  );

  const handleToggleStatus = useCallback(
    (shop: Shop) => {
      const nextStatus = shop.status === 'DISABLED' ? 'AUTHORIZED' : 'DISABLED';
      toggleStatusMutation.mutate({
        shopId: shop.shop_id,
        status: nextStatus,
      });
    },
    [toggleStatusMutation.mutate],
  );

  const runSecondaryAction = useCallback(
    (action: ShopSecondaryActionKey, shop: Shop) => {
      if (action === 'edit') {
        if (!canWrite) {
          return;
        }
        openEdit(shop);
        return;
      }
      if (action === 'authorize') {
        if (!canWrite) {
          return;
        }
        handleAuthorize(shop);
        return;
      }
      if (action === 'refresh-token') {
        if (!canWrite) {
          return;
        }
        refreshTokenMutation.mutate(shop.shop_id);
        return;
      }
      if (action === 'toggle-status') {
        if (!canWrite) {
          return;
        }
        handleToggleStatus(shop);
        return;
      }
      if (action === 'goto-shop-access') {
        navigate('/channels/shops');
        return;
      }
      if (action === 'goto-sync-jobs') {
        navigate('/shops/sync-jobs');
      }
    },
    [
      canWrite,
      handleAuthorize,
      handleToggleStatus,
      navigate,
      openEdit,
      refreshTokenMutation.mutate,
    ],
  );

  const closeMainContextMenu = useCallback(() => {
    setMainContextMenu({ open: false, position: null, shop: null });
  }, []);

  const handleMainGridContextMenu = useCallback((shop: Shop, event: MouseEvent | ReactMouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    setSelectedShop(shop);
    setSelectedRowKeys([shop.shop_id]);
    setMainContextMenu({
      open: true,
      position: { x: event.clientX, y: event.clientY },
      shop,
    });
  }, []);

  const mainContextMenuConfig = useMemo<MenuProps>(() => {
    const contextShop = mainContextMenu.shop;
    const nextStatusLabel = contextShop?.status === 'DISABLED' ? '启用店铺' : '停用店铺';
    return {
      items: [
        { key: 'edit', label: '编辑', disabled: !canWrite || !contextShop },
        { key: 'authorize', label: '发起授权', disabled: !canWrite || !contextShop },
        { key: 'refresh-token', label: '刷新 Token', disabled: !canWrite || !contextShop },
        { key: 'toggle-status', label: nextStatusLabel, disabled: !canWrite || !contextShop },
        { type: 'divider' },
        { key: 'goto-shop-access', label: '打开店铺接入页' },
        { key: 'goto-sync-jobs', label: '打开同步任务页' },
      ],
      onClick: ({ key }) => {
        if (!contextShop) {
          return;
        }
        runSecondaryAction(key as ShopSecondaryActionKey, contextShop);
        closeMainContextMenu();
      },
    };
  }, [canWrite, closeMainContextMenu, mainContextMenu.shop, runSecondaryAction]);

  const resolveShopFromOrderRecord = useCallback(
    (record: OrderSummary | null | undefined) => {
      const key = String(record?.id ?? record?.shop_id ?? record?.order_id ?? '').trim();
      if (!key) {
        return null;
      }
      return rowShopsById.get(key) ?? null;
    },
    [rowShopsById],
  );

  const handleReviewMainSelectionChange = useCallback(
    (keys: Key[]) => {
      const normalizedKeys = Array.from(
        new Set(
          keys
            .map((key) => String(key).trim())
            .filter(Boolean),
        ),
      );
      setSelectedRowKeys(normalizedKeys);
      const firstKey = normalizedKeys[0] ?? '';
      if (!firstKey) {
        setSelectedShop(null);
        return;
      }
      if (selectedShop && normalizedKeys.includes(selectedShop.shop_id)) {
        return;
      }
      setSelectedShop(rowShopsById.get(firstKey) ?? null);
    },
    [rowShopsById, selectedShop],
  );

  const handleReviewMainRowActive = useCallback(
    (record: OrderSummary) => {
      const shop = resolveShopFromOrderRecord(record);
      if (shop) {
        setSelectedShop(shop);
      }
    },
    [resolveShopFromOrderRecord],
  );

  const handleReviewMainRowDoubleClick = useCallback(
    (record: OrderSummary) => {
      if (!canWrite) {
        return;
      }
      const shop = resolveShopFromOrderRecord(record);
      if (shop) {
        openEdit(shop);
      }
    },
    [canWrite, openEdit, resolveShopFromOrderRecord],
  );

  const handleReviewMainFocus = useCallback((record: OrderSummary) => {
    setFocusedOrderKey(String(record.id ?? record.shop_id ?? record.order_id ?? '').trim() || null);
  }, []);

  const handleReviewMainContextMenu = useCallback(
    (record: OrderSummary, event: MouseEvent) => {
      const shop = resolveShopFromOrderRecord(record);
      if (!shop) {
        return;
      }
      handleMainGridContextMenu(shop, event);
    },
    [handleMainGridContextMenu, resolveShopFromOrderRecord],
  );
  const detailPanelContent = useMemo(() => {
    if (!selectedShopView) {
      return <Typography.Text type="secondary">请选择一行查看店铺详情</Typography.Text>;
    }

    const tokenHealthMeta = resolveTokenTag(resolveTokenHealth(selectedShopView));
    const statusMeta = resolveStatusTag(selectedShopView.status);
    const toggleLabel = selectedShopView.status === 'DISABLED' ? '启用店铺' : '停用店铺';

    return (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Descriptions column={1} bordered size="small">
          <Descriptions.Item label="店铺名称">{selectedShopView.name}</Descriptions.Item>
          <Descriptions.Item label="平台">
            <PlatformTag platform={selectedShopView.platform} />
          </Descriptions.Item>
          <Descriptions.Item label="平台店铺ID">{selectedShopView.platform_shop_id}</Descriptions.Item>
          <Descriptions.Item label="店铺ID">{selectedShopView.shop_id}</Descriptions.Item>
          <Descriptions.Item label="接入状态">
            <Tag color={statusMeta.color}>{statusMeta.label}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Token状态">
            <Tag color={tokenHealthMeta.color}>{tokenHealthMeta.label}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Token过期时间">
            {formatDateTime(selectedShopView.token_status?.expires_at)}
          </Descriptions.Item>
          <Descriptions.Item label="最近刷新">
            {formatDateTime(selectedShopView.token_status?.last_refresh_at)}
          </Descriptions.Item>
          <Descriptions.Item label="更新时间">
            {formatDateTime(selectedShopView.updated_at)}
          </Descriptions.Item>
        </Descriptions>

        <Card size="small" title="二级动作" className="channels-detail-card">
          <Space size={[8, 8]} wrap>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => runSecondaryAction('authorize', selectedShopView)}
              loading={authorizingId === selectedShopView.shop_id}
            >
              发起授权
            </Button>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => runSecondaryAction('refresh-token', selectedShopView)}
              loading={refreshingId === selectedShopView.shop_id}
            >
              刷新 Token
            </Button>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => runSecondaryAction('toggle-status', selectedShopView)}
              loading={togglingId === selectedShopView.shop_id}
            >
              {toggleLabel}
            </Button>
            <Button size="small" onClick={() => runSecondaryAction('goto-shop-access', selectedShopView)}>
              店铺接入
            </Button>
            <Button size="small" onClick={() => runSecondaryAction('goto-sync-jobs', selectedShopView)}>
              同步任务
            </Button>
          </Space>
          <Typography.Text type="secondary" style={{ display: 'block', marginTop: 8 }}>
            一级工具栏仅保留高频动作，其余动作统一下沉到右键菜单与本面板。
          </Typography.Text>
        </Card>

        <Card size="small" title="授权回调地址" className="channels-detail-card">
          {selectedShopCallbackUrl ? (
            <Typography.Paragraph className="channels-callback-url" copyable>
              {selectedShopCallbackUrl}
            </Typography.Paragraph>
          ) : (
            <Typography.Text type="secondary">暂无回调地址</Typography.Text>
          )}
        </Card>

        <Card size="small" title="授权元信息" className="channels-detail-card">
          <Typography.Paragraph className="channels-callback-url">
            {JSON.stringify(selectedShopView.auth_meta_json ?? {}, null, 2)}
          </Typography.Paragraph>
        </Card>

        {selectedShopView.token_status?.last_error ? (
          <Card size="small" title="Token异常" className="channels-detail-card">
            <Typography.Text type="danger">{selectedShopView.token_status.last_error}</Typography.Text>
          </Card>
        ) : null}
      </Space>
    );
  }, [
    authorizingId,
    canWrite,
    refreshingId,
    runSecondaryAction,
    selectedShopCallbackUrl,
    selectedShopView,
    togglingId,
  ]);

  const tableFooterSummary = useMemo(
    () => (
      <Space size={12} wrap>
        <Typography.Text>总计 {filteredItems.length} 家</Typography.Text>
        <Typography.Text type="secondary">当前页 {pagedItems.length} 家</Typography.Text>
        <Typography.Text type="secondary">已选 {selectedRowKeys.length} 家</Typography.Text>
      </Space>
    ),
    [filteredItems.length, pagedItems.length, selectedRowKeys.length],
  );

  const toolbarPrimaryActions = useMemo(
    () => (
      <Space size={6} wrap>
        <Button size="small" type="primary" disabled={!canWrite} onClick={openCreate}>
          新建店铺
        </Button>
        <Button
          size="small"
          disabled={!canWrite || !selectedShop}
          onClick={() => selectedShop && openEdit(selectedShop)}
        >
          编辑
        </Button>
        <Button size="small" icon={<ReloadOutlined />} onClick={() => void shopsQuery.refetch()}>
          刷新列表
        </Button>
      </Space>
    ),
    [canWrite, openCreate, openEdit, selectedShop, shopsQuery.refetch],
  );

  const toolbar = useMemo(
    () => (
      <WorkbenchToolbar
        className="order-review-toolbar"
        leftContent={toolbarPrimaryActions}
        searchValue={searchInput}
        onSearchChange={setSearchInput}
        onSearch={handleToolbarSearch}
        searchPlaceholder="店铺名称 / 店铺ID / 平台店铺ID"
        searchInputClassName="order-review-toolbar-search"
        searchButtonClassName="order-review-toolbar-search-trigger"
        onExport={() => message.info('导出功能待接入')}
        onPrint={() => message.info('打印功能待接入')}
        onHelp={() => message.info('帮助功能待接入')}
      />
    ),
    [handleToolbarSearch, searchInput, toolbarPrimaryActions],
  );

  const rightPanel = useMemo(
    () => (
      <div className="industrial-list-workbench-right-panel">
        <Card size="small" title="详情" className="industrial-list-workbench-card industrial-list-workbench-detail-card">
          {detailPanelContent}
        </Card>
      </div>
    ),
    [detailPanelContent],
  );

  const rightPanelSyncKey = useMemo(
    () =>
      JSON.stringify({
        selectedShopId: selectedShopView?.shop_id ?? '',
        selectedShopStatus: selectedShopView?.status ?? '',
        selectedShopUpdatedAt: selectedShopView?.updated_at ?? '',
        selectedShopTokenExpiresAt: selectedShopView?.token_status?.expires_at ?? '',
        selectedShopTokenError: selectedShopView?.token_status?.last_error ?? '',
        callbackUrl: selectedShopCallbackUrl ?? '',
        canWrite,
        authorizingId: authorizingId ?? '',
        refreshingId: refreshingId ?? '',
        togglingId: togglingId ?? '',
      }),
    [
      authorizingId,
      canWrite,
      refreshingId,
      selectedShopCallbackUrl,
      selectedShopView?.shop_id,
      selectedShopView?.status,
      selectedShopView?.token_status?.expires_at,
      selectedShopView?.token_status?.last_error,
      selectedShopView?.updated_at,
      togglingId,
    ],
  );

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        items={[
          {
            key: 'summary',
            label: '汇总',
            children: (
              <Space size={16} wrap>
                <Typography.Text>店铺总数：{filteredItems.length}</Typography.Text>
                <Typography.Text type="secondary">当前页：{pagedItems.length}</Typography.Text>
                <Typography.Text type="secondary">已选：{selectedRowKeys.length}</Typography.Text>
              </Space>
            ),
          },
          {
            key: 'selected',
            label: '当前选中',
            children: selectedShopView ? (
              <Descriptions column={2} size="small" bordered>
                <Descriptions.Item label="店铺名称">{selectedShopView.name}</Descriptions.Item>
                <Descriptions.Item label="店铺ID">{selectedShopView.shop_id}</Descriptions.Item>
                <Descriptions.Item label="平台">{selectedShopView.platform}</Descriptions.Item>
                <Descriptions.Item label="状态">{selectedShopView.status}</Descriptions.Item>
              </Descriptions>
            ) : (
              <Typography.Text type="secondary">请选择一行店铺数据</Typography.Text>
            ),
          },
        ]}
      />
    ),
    [filteredItems.length, pagedItems.length, selectedRowKeys.length, selectedShopView],
  );

  const bottomPanelSyncKey = useMemo(
    () =>
      JSON.stringify({
        filteredCount: filteredItems.length,
        pagedCount: pagedItems.length,
        selectedCount: selectedRowKeys.length,
        selectedShopId: selectedShopView?.shop_id ?? '',
        selectedShopStatus: selectedShopView?.status ?? '',
      }),
    [
      filteredItems.length,
      pagedItems.length,
      selectedRowKeys.length,
      selectedShopView?.shop_id,
      selectedShopView?.status,
    ],
  );

  const drawer = (
    <Drawer
      title={editingShop ? '编辑店铺' : '新建店铺'}
      open={drawerOpen}
      onClose={() => setDrawerOpen(false)}
      width={560}
      destroyOnHidden
      footer={
        <Space>
          <Button onClick={() => setDrawerOpen(false)}>取消</Button>
          <Button type="primary" loading={saving} onClick={() => void handleSave()}>
            保存
          </Button>
        </Space>
      }
    >
      <Form form={drawerForm} layout="vertical">
        <Form.Item name="platform" label="平台" rules={[{ required: true, message: '请选择平台' }]}>
          <Select options={platformOptions} disabled={Boolean(editingShop)} />
        </Form.Item>
        <Form.Item
          name="platform_shop_id"
          label="平台店铺ID"
          rules={[{ required: true, message: '请输入平台店铺ID' }]}
        >
          <Input placeholder="例如：taobao_9527" disabled={Boolean(editingShop)} />
        </Form.Item>
        <Form.Item name="name" label="店铺名称" rules={[{ required: true, message: '请输入店铺名称' }]}>
          <Input placeholder="例如：淘宝旗舰店-A" />
        </Form.Item>
        <Form.Item name="status" label="接入状态" rules={[{ required: true, message: '请选择接入状态' }]}>
          <Select options={editableStatusOptions} />
        </Form.Item>
        <Form.Item name="auth_meta_json_input" label="授权元信息（JSON）">
          <Input.TextArea rows={6} placeholder='例如：{"oauth_status":"SUCCESS","region":"cn-east-1"}' />
        </Form.Item>
      </Form>
    </Drawer>
  );

  return (
      <WorkbenchPageTemplate
      pageTestId="channels-workbench-grid"
      className="order-review-page channels-page"
      toolbar={toolbar}
      tableContent={
        <div className="order-review-grid-main">
          <OrderWorkbenchMainGridEntry
            variant="review"
            rows={reviewRows}
            filterOptionRows={reviewFilterOptionRows}
            quickFilterExpanded={true}
            columnFilters={reviewColumnFilters}
            shopOptions={reviewShopOptions}
            logisticsOptions={reviewLogisticsOptions}
            selectedRowKeys={selectedRowKeys}
            focusedOrderKey={focusedOrderKey}
            onColumnFiltersChange={setReviewColumnFilters}
            onSelectionChange={handleReviewMainSelectionChange}
            onRowActive={handleReviewMainRowActive}
            onRowDoubleClick={handleReviewMainRowDoubleClick}
            onFocus={handleReviewMainFocus}
            onContextMenu={handleReviewMainContextMenu}
            loading={shopsQuery.isLoading || shopsQuery.isFetching || selectedAuthStatusQuery.isFetching}
            error={shopsQuery.error}
            onRetry={() => {
              void shopsQuery.refetch();
            }}
          />
        </div>
      }
      footerSummary={tableFooterSummary}
      footerSyncKey={`channels-review-${pagination.current ?? 1}-${pagination.pageSize ?? 20}-${filteredItems.length}`}
      total={filteredItems.length}
      pagination={pagination}
      onPaginationChange={setPagination}
      stateCacheKey={channelsWorkbenchStateCacheKey}
      showFooterScrollbarRow={true}
      rightPanel={rightPanel}
      rightPanelSyncKey={rightPanelSyncKey}
      bottomPanel={bottomPanel}
      bottomPanelSyncKey={bottomPanelSyncKey}
      overlays={
        <>
          <UiModule03ContextMenu
            open={mainContextMenu.open}
            position={mainContextMenu.position}
            menu={mainContextMenuConfig}
            onClose={closeMainContextMenu}
          />
          {drawer}
        </>
      }
    />
  );
};
