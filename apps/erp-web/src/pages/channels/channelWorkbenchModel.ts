import dayjs from 'dayjs';

import type { Channel, ChannelListQuery, ChannelStatus } from '@/api/channels';
import { platformLabels } from '@/features/platform';
import type { PlatformCode } from '@/features/platform';

export type ChannelQuickFilterId =
  | 'all'
  | ChannelStatus
  | 'manual-order'
  | 'manual-after-sales'
  | 'has-warehouse';

export type ChannelStatusFilter = ChannelStatus | 'all';

export type ChannelQuickFilter = {
  id: ChannelQuickFilterId;
  label: string;
  description: string;
  count: number;
};

export type ChannelAuthCallbackState = {
  channelId?: string;
  code?: string;
  error?: string;
  errorDescription?: string;
  hasCode: boolean;
  hasError: boolean;
  platform?: PlatformCode;
  state?: string;
};

export type ChannelWorkbenchSummary = {
  total: number;
  active: number;
  disabled: number;
  manualOrder: number;
  manualAfterSales: number;
  defaultWarehouseConfigured: number;
  platformCount: number;
};

export type ChannelManualFilter = 'allow' | 'deny';

export type ChannelDateRangeFilter = {
  start?: string | null;
  end?: string | null;
};

export type ChannelWorkbenchColumnFilters = {
  platform?: PlatformCode;
  department?: string;
  owner?: string;
  warehouseId?: string;
  hasDefaultWarehouse?: boolean;
  allowManualOrder?: ChannelManualFilter;
  allowManualAfterSales?: ChannelManualFilter;
  updatedRange?: ChannelDateRangeFilter | null;
};

export type ChannelWorkbenchFilterState = {
  statusFilter: ChannelStatusFilter;
  columnFilters: Pick<
    ChannelWorkbenchColumnFilters,
    'allowManualOrder' | 'allowManualAfterSales' | 'hasDefaultWarehouse'
  >;
};

type BuildChannelAuthCallbackUrlInput = {
  origin: string;
  routerMode: 'browser' | 'hash';
  routerBasePath: string;
  channelId: string;
  platform: PlatformCode;
};

type FilterChannelsInput = {
  statusFilter: ChannelStatusFilter;
  searchKeyword: string;
  columnFilters: ChannelWorkbenchColumnFilters;
};

type BuildChannelActiveFilterSummaryInput = {
  activeQuickFilterId: ChannelQuickFilterId | null;
  columnFilters: ChannelWorkbenchColumnFilters;
  quickFilters: ChannelQuickFilter[];
  searchKeyword: string;
  statusFilter: ChannelStatusFilter;
  warehouseNameResolver: (warehouseId: string) => string;
};

const normalizeBasePath = (value: string) => {
  if (!value || value === '/') {
    return '/';
  }

  const withLeadingSlash = value.startsWith('/') ? value : `/${value}`;
  return withLeadingSlash.endsWith('/')
    ? withLeadingSlash.slice(0, withLeadingSlash.length - 1)
    : withLeadingSlash;
};

const trimText = (value?: string | null) => value?.trim() ?? '';

const buildStatusLabel = (status: ChannelStatus) => {
  if (status === 'ACTIVE') {
    return '启用中';
  }

  return '已停用';
};

const matchManualFilter = (
  expected: ChannelManualFilter | undefined,
  actual: boolean,
): boolean => {
  if (!expected) {
    return true;
  }

  return expected === 'allow' ? actual : !actual;
};

export const buildChannelAuthCallbackUrl = ({
  origin,
  routerMode,
  routerBasePath,
  channelId,
  platform,
}: BuildChannelAuthCallbackUrlInput): string => {
  const normalizedOrigin = origin.replace(/\/+$/, '');
  const normalizedBasePath = normalizeBasePath(routerBasePath);
  const params = new URLSearchParams({
    auth_callback: '1',
    channel_id: channelId,
    platform,
  });

  if (routerMode === 'hash') {
    const baseSegment = normalizedBasePath === '/' ? '' : normalizedBasePath;
    return `${normalizedOrigin}${baseSegment}#/channels?${params.toString()}`;
  }

  const browserPath =
    normalizedBasePath === '/' ? '/channels' : `${normalizedBasePath}/channels`;
  return `${normalizedOrigin}${browserPath}?${params.toString()}`;
};

export const parseChannelAuthCallbackSearch = (
  search: string,
): ChannelAuthCallbackState | null => {
  const params = new URLSearchParams(search.startsWith('?') ? search : `?${search}`);
  if (params.get('auth_callback') !== '1') {
    return null;
  }

  const code = params.get('code') ?? undefined;
  const error = params.get('error') ?? undefined;
  const errorDescription = params.get('error_description') ?? undefined;
  const channelId = params.get('channel_id') ?? undefined;
  const platform = (params.get('platform') ?? undefined) as PlatformCode | undefined;
  const state = params.get('state') ?? undefined;

  return {
    channelId,
    code,
    error,
    errorDescription,
    hasCode: Boolean(code),
    hasError: Boolean(error),
    platform,
    state,
  };
};

export const summarizeChannels = (channels: Channel[]): ChannelWorkbenchSummary => {
  const active = channels.filter((item) => item.status === 'ACTIVE').length;
  const disabled = channels.filter((item) => item.status === 'DISABLED').length;
  const manualOrder = channels.filter((item) => item.allow_manual_order).length;
  const manualAfterSales = channels.filter((item) => item.allow_manual_after_sales).length;
  const defaultWarehouseConfigured = channels.filter((item) => item.default_warehouse_id).length;
  const platformCount = new Set(channels.map((item) => item.platform)).size;

  return {
    total: channels.length,
    active,
    disabled,
    manualOrder,
    manualAfterSales,
    defaultWarehouseConfigured,
    platformCount,
  };
};

export const buildChannelQuickFilters = (channels: Channel[]): ChannelQuickFilter[] => {
  const summary = summarizeChannels(channels);
  return [
    {
      id: 'all',
      label: '全部渠道',
      description: '查看全部渠道',
      count: summary.total,
    },
    {
      id: 'ACTIVE',
      label: '启用中',
      description: '仅看启用中的渠道',
      count: summary.active,
    },
    {
      id: 'DISABLED',
      label: '已停用',
      description: '仅看已停用渠道',
      count: summary.disabled,
    },
    {
      id: 'manual-order',
      label: '手工建单',
      description: '支持手工建单',
      count: summary.manualOrder,
    },
    {
      id: 'manual-after-sales',
      label: '手工售后',
      description: '支持手工售后',
      count: summary.manualAfterSales,
    },
    {
      id: 'has-warehouse',
      label: '已配置默认仓',
      description: '已配置默认仓',
      count: summary.defaultWarehouseConfigured,
    },
  ];
};

export const buildChannelListQuery = (
  keyword: string,
  statusFilter: ChannelStatusFilter,
): ChannelListQuery => {
  const trimmedKeyword = trimText(keyword);
  return {
    keyword: trimmedKeyword || undefined,
    limit: 200,
    offset: 0,
    status: statusFilter === 'all' ? undefined : statusFilter,
  };
};

export const resolveChannelQuickFilterState = (
  filterId: ChannelQuickFilterId,
): ChannelWorkbenchFilterState => {
  if (filterId === 'ACTIVE' || filterId === 'DISABLED') {
    return {
      statusFilter: filterId,
      columnFilters: {
        allowManualAfterSales: undefined,
        allowManualOrder: undefined,
        hasDefaultWarehouse: undefined,
      },
    };
  }

  return {
    statusFilter: 'all',
    columnFilters: {
      allowManualAfterSales: filterId === 'manual-after-sales' ? 'allow' : undefined,
      allowManualOrder: filterId === 'manual-order' ? 'allow' : undefined,
      hasDefaultWarehouse: filterId === 'has-warehouse' ? true : undefined,
    },
  };
};

export const filterChannels = (
  channels: Channel[],
  { statusFilter, searchKeyword, columnFilters }: FilterChannelsInput,
): Channel[] => {
  const normalizedKeyword = trimText(searchKeyword).toLowerCase();
  const normalizedDepartment = trimText(columnFilters.department).toLowerCase();
  const normalizedOwner = trimText(columnFilters.owner).toLowerCase();
  const updatedRange = columnFilters.updatedRange ?? null;

  return channels.filter((item) => {
    if (statusFilter !== 'all' && item.status !== statusFilter) {
      return false;
    }

    if (normalizedKeyword) {
      const searchableValues = [
        item.channel_id,
        item.name,
        item.department ?? '',
        item.owner ?? '',
      ].map((value) => value.toLowerCase());
      if (!searchableValues.some((value) => value.includes(normalizedKeyword))) {
        return false;
      }
    }

    if (columnFilters.platform && item.platform !== columnFilters.platform) {
      return false;
    }

    if (normalizedDepartment && !(item.department ?? '').toLowerCase().includes(normalizedDepartment)) {
      return false;
    }

    if (normalizedOwner && !(item.owner ?? '').toLowerCase().includes(normalizedOwner)) {
      return false;
    }

    if (columnFilters.warehouseId && item.default_warehouse_id !== columnFilters.warehouseId) {
      return false;
    }

    if (columnFilters.hasDefaultWarehouse && !item.default_warehouse_id) {
      return false;
    }

    if (!matchManualFilter(columnFilters.allowManualOrder, item.allow_manual_order)) {
      return false;
    }

    if (!matchManualFilter(columnFilters.allowManualAfterSales, item.allow_manual_after_sales)) {
      return false;
    }

    if (updatedRange) {
      const target = dayjs(item.updated_at ?? item.created_at ?? '');
      if (!target.isValid()) {
        return false;
      }

      if (updatedRange.start && target.isBefore(dayjs(updatedRange.start).startOf('day'))) {
        return false;
      }

      if (updatedRange.end && target.isAfter(dayjs(updatedRange.end).endOf('day'))) {
        return false;
      }
    }

    return true;
  });
};

export const buildChannelActiveFilterSummary = ({
  activeQuickFilterId,
  columnFilters,
  quickFilters,
  searchKeyword,
  statusFilter,
  warehouseNameResolver,
}: BuildChannelActiveFilterSummaryInput): string[] => {
  const summary: string[] = [];
  const activeQuickFilter = quickFilters.find((item) => item.id === activeQuickFilterId);
  if (activeQuickFilter) {
    summary.push(`视图：${activeQuickFilter.label}`);
  }

  const trimmedKeyword = trimText(searchKeyword);
  if (trimmedKeyword) {
    summary.push(`搜索：${trimmedKeyword}`);
  }

  if (columnFilters.platform) {
    summary.push(`平台：${platformLabels[columnFilters.platform] ?? columnFilters.platform}`);
  }

  if (trimText(columnFilters.department)) {
    summary.push(`部门：${trimText(columnFilters.department)}`);
  }

  if (trimText(columnFilters.owner)) {
    summary.push(`负责人：${trimText(columnFilters.owner)}`);
  }

  if (columnFilters.warehouseId) {
    summary.push(`默认仓：${warehouseNameResolver(columnFilters.warehouseId)}`);
  }

  if (columnFilters.allowManualOrder) {
    summary.push(`手工建单：${columnFilters.allowManualOrder === 'allow' ? '允许' : '不允许'}`);
  }

  if (columnFilters.allowManualAfterSales) {
    summary.push(
      `手工售后：${columnFilters.allowManualAfterSales === 'allow' ? '允许' : '不允许'}`,
    );
  }

  if (columnFilters.hasDefaultWarehouse && activeQuickFilterId !== 'has-warehouse') {
    summary.push('默认仓：已配置');
  }

  if (statusFilter !== 'all') {
    summary.push(`状态：${buildStatusLabel(statusFilter)}`);
  }

  if (columnFilters.updatedRange) {
    const startText = trimText(columnFilters.updatedRange.start) || '-';
    const endText = trimText(columnFilters.updatedRange.end) || '-';
    summary.push(`更新时间：${startText} 至 ${endText}`);
  }

  return summary;
};
