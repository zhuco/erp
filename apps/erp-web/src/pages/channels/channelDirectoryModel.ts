import dayjs from 'dayjs';
import type { DateFilterModel, FilterModel } from 'ag-grid-community';

import type { Channel, ChannelStatus, ChannelType } from '@/api/channels';
import { platformLabels, type PlatformCode } from '@/features/platform';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';

export type ChannelDirectoryFilterId =
  | 'view:all'
  | 'type:ONLINE'
  | 'type:OFFLINE'
  | 'status:ACTIVE'
  | 'status:DISABLED'
  | 'capability:manual-order'
  | 'capability:manual-after-sales'
  | 'capability:has-warehouse';

export type ChannelDirectoryLeftFilters = {
  selectedFilterId: ChannelDirectoryFilterId;
  departmentKeyword: string;
  ownerKeyword: string;
  contactKeyword: string;
};

export type ChannelDirectoryColumnFilters = {
  nameKeyword: string;
  channelType: 'ALL' | ChannelType;
  platform: 'ALL' | PlatformCode | 'NONE';
  contactKeyword: string;
  phoneKeyword: string;
  status: 'ALL' | ChannelStatus;
  warehouseId: string;
  updatedAtFrom?: string;
  updatedAtTo?: string;
};

export type ChannelDirectorySummary = {
  total: number;
  online: number;
  offline: number;
  active: number;
  disabled: number;
  manualOrder: number;
  manualAfterSales: number;
  warehouseConfigured: number;
};

const trimText = (value?: string | null) => String(value ?? '').trim();

const normalizeText = (value?: string | null) => trimText(value).toLowerCase();

const normalizeDateInput = (value?: string | null) => trimText(value).slice(0, 10);

const normalizeSetValues = (values: Array<string | null | undefined>) =>
  Array.from(
    new Set(
      values
        .map((value) => trimText(value))
        .filter(Boolean),
    ),
  );

const readSetValues = (model: unknown) => {
  if (!model || typeof model !== 'object' || !('values' in model)) {
    return [] as string[];
  }
  const values = (model as { values?: unknown }).values;
  return Array.isArray(values) ? normalizeSetValues(values as string[]) : [];
};

const readTextValue = (model: unknown) => {
  if (!model || typeof model !== 'object' || !('filter' in model)) {
    return '';
  }
  return trimText(String((model as { filter?: unknown }).filter ?? ''));
};

const readDateValue = (model: unknown, key: 'dateFrom' | 'dateTo') => {
  if (!model || typeof model !== 'object' || !(key in model)) {
    return '';
  }
  return normalizeDateInput(String((model as Record<'dateFrom' | 'dateTo', unknown>)[key] ?? ''));
};

const normalizeTextModel = (value?: string) => {
  const normalized = trimText(value);
  return normalized
    ? {
        filterType: 'text' as const,
        type: 'contains' as const,
        filter: normalized,
      }
    : null;
};

const normalizeSetModel = (values: string[]) => {
  const normalized = normalizeSetValues(values);
  return normalized.length > 0
    ? {
        filterType: 'set' as const,
        values: normalized,
      }
    : null;
};

const normalizeDateRangeModel = (from?: string, to?: string): DateFilterModel | null => {
  const normalizedFrom = normalizeDateInput(from);
  const normalizedTo = normalizeDateInput(to);
  if (!normalizedFrom && !normalizedTo) {
    return null;
  }
  if (normalizedFrom && normalizedTo) {
    return {
      filterType: 'date',
      type: 'inRange',
      dateFrom: `${normalizedFrom} 00:00:00`,
      dateTo: `${normalizedTo} 23:59:59`,
    };
  }
  if (normalizedFrom) {
    return {
      filterType: 'date',
      type: 'greaterThanOrEqual',
      dateFrom: `${normalizedFrom} 00:00:00`,
    };
  }
  return {
    filterType: 'date',
    type: 'lessThanOrEqual',
    dateTo: `${normalizedTo} 23:59:59`,
  };
};

const matchesKeyword = (value: string, keyword: string) => value.includes(keyword);

export const createDefaultChannelDirectoryLeftFilters = (): ChannelDirectoryLeftFilters => ({
  selectedFilterId: 'view:all',
  departmentKeyword: '',
  ownerKeyword: '',
  contactKeyword: '',
});

export const createDefaultChannelDirectoryColumnFilters = (): ChannelDirectoryColumnFilters => ({
  nameKeyword: '',
  channelType: 'ALL',
  platform: 'ALL',
  contactKeyword: '',
  phoneKeyword: '',
  status: 'ALL',
  warehouseId: '',
  updatedAtFrom: '',
  updatedAtTo: '',
});

export const resolveChannelRowKey = (channel: Channel | null | undefined) =>
  trimText(channel?.channel_id) || null;

export const resolveChannelTypeLabel = (channelType?: ChannelType | null) =>
  channelType === 'OFFLINE' ? '线下' : '线上';

export const resolveChannelStatusLabel = (status?: ChannelStatus | null) =>
  status === 'DISABLED' ? '停用' : '启用';

export const resolveChannelPlatformLabel = (platform?: PlatformCode | null) =>
  platform ? platformLabels[platform] ?? platform : '线下渠道';

export const resolveChannelAuthorizationEnabled = (channel: Channel | null | undefined) =>
  (channel?.channel_type ?? 'ONLINE') === 'ONLINE';

export const summarizeChannels = (channels: Channel[]): ChannelDirectorySummary => ({
  total: channels.length,
  online: channels.filter((channel) => (channel.channel_type ?? 'ONLINE') === 'ONLINE').length,
  offline: channels.filter((channel) => channel.channel_type === 'OFFLINE').length,
  active: channels.filter((channel) => channel.status === 'ACTIVE').length,
  disabled: channels.filter((channel) => channel.status === 'DISABLED').length,
  manualOrder: channels.filter((channel) => channel.allow_manual_order).length,
  manualAfterSales: channels.filter((channel) => channel.allow_manual_after_sales).length,
  warehouseConfigured: channels.filter((channel) => Boolean(channel.default_warehouse_id)).length,
});

export const buildChannelDirectoryPieBackground = (summary: ChannelDirectorySummary) => {
  const total = summary.total || 1;
  const segments = [
    { value: summary.online, color: '#2f6fed' },
    { value: summary.offline, color: '#0f9d58' },
    { value: summary.disabled, color: '#d0d7de' },
  ].filter((item) => item.value > 0);

  if (segments.length === 0) {
    return 'conic-gradient(#dce4ee 0deg 360deg)';
  }

  let start = 0;
  const stops = segments.map((segment) => {
    const end = start + (segment.value / total) * 360;
    const stop = `${segment.color} ${start.toFixed(2)}deg ${end.toFixed(2)}deg`;
    start = end;
    return stop;
  });

  if (start < 360) {
    stops.push(`#dce4ee ${start.toFixed(2)}deg 360deg`);
  }

  return `conic-gradient(${stops.join(', ')})`;
};

export const buildChannelDirectoryFilterSummary = (
  filters: ChannelDirectoryLeftFilters,
): string[] => {
  const items: string[] = [];

  if (filters.selectedFilterId !== 'view:all') {
    const labelMap: Record<ChannelDirectoryFilterId, string> = {
      'view:all': '全部渠道',
      'type:ONLINE': '线上渠道',
      'type:OFFLINE': '线下渠道',
      'status:ACTIVE': '启用中',
      'status:DISABLED': '已停用',
      'capability:manual-order': '允许手工建销售单',
      'capability:manual-after-sales': '允许手工建售后单',
      'capability:has-warehouse': '已配置默认仓库',
    };
    items.push(`视图：${labelMap[filters.selectedFilterId]}`);
  }
  if (trimText(filters.departmentKeyword)) {
    items.push(`部门：${trimText(filters.departmentKeyword)}`);
  }
  if (trimText(filters.ownerKeyword)) {
    items.push(`负责人：${trimText(filters.ownerKeyword)}`);
  }
  if (trimText(filters.contactKeyword)) {
    items.push(`联系人：${trimText(filters.contactKeyword)}`);
  }

  return items;
};

export const hasChannelDirectoryLeftManualFilter = (filters: ChannelDirectoryLeftFilters) =>
  filters.selectedFilterId !== 'view:all' ||
  Boolean(trimText(filters.departmentKeyword)) ||
  Boolean(trimText(filters.ownerKeyword)) ||
  Boolean(trimText(filters.contactKeyword));

export const buildChannelDirectoryLeftCheckedKeys = (
  filters: ChannelDirectoryLeftFilters,
) => new Set<string>([filters.selectedFilterId]);

export const buildChannelDirectoryLeftNodes = (channels: Channel[]): OrderReviewAgLeftNode[] => {
  const count = (predicate: (channel: Channel) => boolean) =>
    channels.filter((channel) => predicate(channel)).length;

  return [
    {
      key: 'group:overview',
      label: '目录视图',
      nodeType: 'group',
      count: channels.length,
      children: [
        {
          key: 'view:all',
          label: '全部渠道',
          nodeType: 'status',
          count: channels.length,
        },
        {
          key: 'status:ACTIVE',
          label: '启用中',
          nodeType: 'status',
          count: count((channel) => channel.status === 'ACTIVE'),
        },
        {
          key: 'status:DISABLED',
          label: '已停用',
          nodeType: 'status',
          count: count((channel) => channel.status === 'DISABLED'),
        },
      ],
    },
    {
      key: 'group:type',
      label: '渠道类型',
      nodeType: 'group',
      count: channels.length,
      children: [
        {
          key: 'type:ONLINE',
          label: '线上渠道',
          nodeType: 'platform',
          count: count((channel) => (channel.channel_type ?? 'ONLINE') === 'ONLINE'),
        },
        {
          key: 'type:OFFLINE',
          label: '线下渠道',
          nodeType: 'platform',
          count: count((channel) => channel.channel_type === 'OFFLINE'),
        },
      ],
    },
    {
      key: 'group:capability',
      label: '运营能力',
      nodeType: 'group',
      count: channels.length,
      children: [
        {
          key: 'capability:manual-order',
          label: '允许手工建销售单',
          nodeType: 'exception',
          count: count((channel) => channel.allow_manual_order),
        },
        {
          key: 'capability:manual-after-sales',
          label: '允许手工建售后单',
          nodeType: 'exception',
          count: count((channel) => channel.allow_manual_after_sales),
        },
        {
          key: 'capability:has-warehouse',
          label: '已配置默认仓库',
          nodeType: 'exception',
          count: count((channel) => Boolean(channel.default_warehouse_id)),
        },
      ],
    },
  ];
};

export const reduceChannelDirectoryLeftFiltersByNode = (
  previous: ChannelDirectoryLeftFilters,
  node: { key: string },
): ChannelDirectoryLeftFilters => {
  const nextSelectedFilterId =
    previous.selectedFilterId === node.key
      ? 'view:all'
      : (node.key as ChannelDirectoryFilterId);

  return {
    ...previous,
    selectedFilterId: nextSelectedFilterId,
  };
};

export const filterChannelsByKeyword = (channels: Channel[], keyword: string) => {
  const normalizedKeyword = normalizeText(keyword);
  if (!normalizedKeyword) {
    return channels;
  }

  return channels.filter((channel) => {
    const fields = [
      channel.channel_id,
      channel.name,
      resolveChannelPlatformLabel(channel.platform),
      channel.contact_name,
      channel.contact_phone,
      channel.contact_address,
      channel.department,
      channel.owner,
      channel.commission_formula,
    ]
      .map((value) => normalizeText(value))
      .filter(Boolean);

    return fields.some((field) => matchesKeyword(field, normalizedKeyword));
  });
};

export const filterChannelsByLeftFilters = (
  channels: Channel[],
  filters: ChannelDirectoryLeftFilters,
) => {
  const departmentKeyword = normalizeText(filters.departmentKeyword);
  const ownerKeyword = normalizeText(filters.ownerKeyword);
  const contactKeyword = normalizeText(filters.contactKeyword);

  return channels.filter((channel) => {
    const selectedFilterId = filters.selectedFilterId;

    if (selectedFilterId === 'type:ONLINE' && (channel.channel_type ?? 'ONLINE') !== 'ONLINE') {
      return false;
    }
    if (selectedFilterId === 'type:OFFLINE' && channel.channel_type !== 'OFFLINE') {
      return false;
    }
    if (selectedFilterId === 'status:ACTIVE' && channel.status !== 'ACTIVE') {
      return false;
    }
    if (selectedFilterId === 'status:DISABLED' && channel.status !== 'DISABLED') {
      return false;
    }
    if (selectedFilterId === 'capability:manual-order' && !channel.allow_manual_order) {
      return false;
    }
    if (
      selectedFilterId === 'capability:manual-after-sales' &&
      !channel.allow_manual_after_sales
    ) {
      return false;
    }
    if (selectedFilterId === 'capability:has-warehouse' && !channel.default_warehouse_id) {
      return false;
    }

    if (
      departmentKeyword &&
      !normalizeText(channel.department).includes(departmentKeyword)
    ) {
      return false;
    }
    if (ownerKeyword && !normalizeText(channel.owner).includes(ownerKeyword)) {
      return false;
    }
    if (
      contactKeyword &&
      ![channel.contact_name, channel.contact_phone, channel.contact_address]
        .map((item) => normalizeText(item))
        .some((item) => item.includes(contactKeyword))
    ) {
      return false;
    }

    return true;
  });
};

export const filterChannelsByColumnFilters = (
  channels: Channel[],
  filters: ChannelDirectoryColumnFilters,
) => {
  const nameKeyword = normalizeText(filters.nameKeyword);
  const contactKeyword = normalizeText(filters.contactKeyword);
  const phoneKeyword = normalizeText(filters.phoneKeyword);
  const updatedAtFrom = normalizeDateInput(filters.updatedAtFrom);
  const updatedAtTo = normalizeDateInput(filters.updatedAtTo);

  return channels.filter((channel) => {
    if (
      nameKeyword &&
      ![channel.name, channel.channel_id]
        .map((value) => normalizeText(value))
        .some((value) => value.includes(nameKeyword))
    ) {
      return false;
    }
    if (
      filters.channelType !== 'ALL' &&
      (channel.channel_type ?? 'ONLINE') !== filters.channelType
    ) {
      return false;
    }
    if (filters.platform === 'NONE' && channel.platform) {
      return false;
    }
    if (
      filters.platform !== 'ALL' &&
      filters.platform !== 'NONE' &&
      channel.platform !== filters.platform
    ) {
      return false;
    }
    if (contactKeyword && !normalizeText(channel.contact_name).includes(contactKeyword)) {
      return false;
    }
    if (phoneKeyword && !normalizeText(channel.contact_phone).includes(phoneKeyword)) {
      return false;
    }
    if (filters.status !== 'ALL' && channel.status !== filters.status) {
      return false;
    }
    if (trimText(filters.warehouseId) && channel.default_warehouse_id !== trimText(filters.warehouseId)) {
      return false;
    }
    if (updatedAtFrom || updatedAtTo) {
      const updatedAt = normalizeDateInput(channel.updated_at ?? channel.created_at);
      if (!updatedAt) {
        return false;
      }
      if (updatedAtFrom && updatedAt < updatedAtFrom) {
        return false;
      }
      if (updatedAtTo && updatedAt > updatedAtTo) {
        return false;
      }
    }

    return true;
  });
};

export const buildChannelDirectoryGridFilterModel = (
  filters: ChannelDirectoryColumnFilters,
): FilterModel => {
  const model: FilterModel = {};

  const nameModel = normalizeTextModel(filters.nameKeyword);
  if (nameModel) {
    model.channel_id = nameModel;
  }

  const channelTypeValues =
    filters.channelType === 'ALL' ? [] : normalizeSetValues([filters.channelType]);
  if (channelTypeValues.length > 0) {
    model.channel_type = normalizeSetModel(channelTypeValues);
  }

  const platformValues =
    filters.platform === 'ALL' ? [] : normalizeSetValues([filters.platform]);
  if (platformValues.length > 0) {
    model.platform = normalizeSetModel(platformValues);
  }

  const contactModel = normalizeTextModel(filters.contactKeyword);
  if (contactModel) {
    model.contact_name = contactModel;
  }

  const phoneModel = normalizeTextModel(filters.phoneKeyword);
  if (phoneModel) {
    model.contact_phone = phoneModel;
  }

  const statusValues = filters.status === 'ALL' ? [] : normalizeSetValues([filters.status]);
  if (statusValues.length > 0) {
    model.status = normalizeSetModel(statusValues);
  }

  const warehouseValues = trimText(filters.warehouseId)
    ? normalizeSetValues([filters.warehouseId])
    : [];
  if (warehouseValues.length > 0) {
    model.default_warehouse_id = normalizeSetModel(warehouseValues);
  }

  const updatedAtModel = normalizeDateRangeModel(filters.updatedAtFrom, filters.updatedAtTo);
  if (updatedAtModel) {
    model.updated_at = updatedAtModel;
  }

  return model;
};

export const extractChannelDirectoryGridFilterModel = (
  model: FilterModel | null | undefined,
): ChannelDirectoryColumnFilters => {
  const channelTypeValues = readSetValues(model?.channel_type);
  const platformValues = readSetValues(model?.platform);
  const statusValues = readSetValues(model?.status);
  const warehouseValues = readSetValues(model?.default_warehouse_id);

  return {
    nameKeyword: readTextValue(model?.channel_id),
    channelType: (channelTypeValues[0] as ChannelType | undefined) ?? 'ALL',
    platform: (platformValues[0] as ChannelDirectoryColumnFilters['platform'] | undefined) ?? 'ALL',
    contactKeyword: readTextValue(model?.contact_name),
    phoneKeyword: readTextValue(model?.contact_phone),
    status: (statusValues[0] as ChannelStatus | undefined) ?? 'ALL',
    warehouseId: warehouseValues[0] ?? '',
    updatedAtFrom: readDateValue(model?.updated_at, 'dateFrom'),
    updatedAtTo: readDateValue(model?.updated_at, 'dateTo'),
  };
};

export const formatChannelDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm:ss') : '-';
