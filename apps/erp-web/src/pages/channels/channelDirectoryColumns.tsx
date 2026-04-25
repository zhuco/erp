import { Input, Select, Tag, Typography } from 'antd';
import type { ReactNode } from 'react';

import type { Channel } from '@/api/channels';
import type { ErpDataGridNextColumn } from '@/components/ErpDataGridNext';
import { PlatformTag } from '@/components/PlatformTag';
import { platformOptions } from '@/features/platform';

import {
  formatChannelDateTime,
  resolveChannelPlatformLabel,
  resolveChannelStatusLabel,
  resolveChannelTypeLabel,
  type ChannelDirectoryColumnFilters,
} from './channelDirectoryModel';

type SimpleOption = {
  label: string;
  value: string;
};

type HeaderFilterBuilderOptions = {
  filters: ChannelDirectoryColumnFilters;
  warehouseOptions: SimpleOption[];
  onPatchFilters: (patch: Partial<ChannelDirectoryColumnFilters>) => void;
};

const renderBooleanTag = (value: boolean) =>
  value ? <Tag color="green">允许</Tag> : <Tag>关闭</Tag>;

export const buildChannelDirectoryColumns = (
  warehouseMap: Map<string, string>,
): ErpDataGridNextColumn<Channel>[] => [
  {
    key: 'name',
    header: '渠道名称',
    width: 220,
    minWidth: 180,
    sortValue: (channel) => channel.name,
    render: (channel) => (
      <div className="channel-directory-cell-primary">
        <Typography.Text strong>{channel.name}</Typography.Text>
        <Typography.Text type="secondary">{channel.channel_id}</Typography.Text>
      </div>
    ),
  },
  {
    key: 'channel_type',
    header: '渠道类型',
    width: 110,
    minWidth: 100,
    sortValue: (channel) => resolveChannelTypeLabel(channel.channel_type),
    render: (channel) => (
      <Tag color={channel.channel_type === 'OFFLINE' ? 'green' : 'blue'}>
        {resolveChannelTypeLabel(channel.channel_type)}
      </Tag>
    ),
  },
  {
    key: 'platform',
    header: '平台',
    width: 120,
    minWidth: 110,
    sortValue: (channel) => resolveChannelPlatformLabel(channel.platform),
    render: (channel) =>
      channel.platform ? <PlatformTag platform={channel.platform} /> : <Tag>线下渠道</Tag>,
  },
  {
    key: 'contact_name',
    header: '联系人',
    width: 120,
    minWidth: 110,
    sortValue: (channel) => channel.contact_name ?? '',
    render: (channel) => channel.contact_name || '-',
  },
  {
    key: 'contact_phone',
    header: '电话',
    width: 140,
    minWidth: 120,
    sortValue: (channel) => channel.contact_phone ?? '',
    render: (channel) => channel.contact_phone || '-',
  },
  {
    key: 'default_warehouse_id',
    header: '默认发货仓库',
    width: 160,
    minWidth: 140,
    sortValue: (channel) =>
      channel.default_warehouse_id
        ? warehouseMap.get(channel.default_warehouse_id) ?? channel.default_warehouse_id
        : '',
    render: (channel) =>
      channel.default_warehouse_id
        ? warehouseMap.get(channel.default_warehouse_id) ?? channel.default_warehouse_id
        : '-',
  },
  {
    key: 'allow_manual_order',
    header: '手工销售单',
    width: 110,
    minWidth: 100,
    sortValue: (channel) => (channel.allow_manual_order ? 1 : 0),
    render: (channel) => renderBooleanTag(channel.allow_manual_order),
  },
  {
    key: 'allow_manual_after_sales',
    header: '手工售后单',
    width: 110,
    minWidth: 100,
    sortValue: (channel) => (channel.allow_manual_after_sales ? 1 : 0),
    render: (channel) => renderBooleanTag(channel.allow_manual_after_sales),
  },
  {
    key: 'commission_formula',
    header: '平台抽佣公式',
    width: 180,
    minWidth: 150,
    sortValue: (channel) => channel.commission_formula ?? '',
    render: (channel) => channel.commission_formula || '-',
  },
  {
    key: 'status',
    header: '状态',
    width: 100,
    minWidth: 90,
    sortValue: (channel) => resolveChannelStatusLabel(channel.status),
    render: (channel) => (
      <Tag color={channel.status === 'ACTIVE' ? 'green' : 'default'}>
        {resolveChannelStatusLabel(channel.status)}
      </Tag>
    ),
  },
  {
    key: 'updated_at',
    header: '更新时间',
    width: 170,
    minWidth: 160,
    sortValue: (channel) => channel.updated_at ?? channel.created_at ?? '',
    render: (channel) => formatChannelDateTime(channel.updated_at ?? channel.created_at),
  },
];

export const buildChannelDirectoryHeaderFilters = ({
  filters,
  warehouseOptions,
  onPatchFilters,
}: HeaderFilterBuilderOptions): Record<string, ReactNode> => ({
  name: (
    <Input
      size="small"
      allowClear
      value={filters.nameKeyword}
      placeholder="筛选名称"
      onChange={(event) => onPatchFilters({ nameKeyword: event.target.value })}
    />
  ),
  channel_type: (
    <Select
      size="small"
      value={filters.channelType === 'ALL' ? undefined : filters.channelType}
      allowClear
      placeholder="全部"
      style={{ width: '100%' }}
      options={[
        { label: '线上', value: 'ONLINE' },
        { label: '线下', value: 'OFFLINE' },
      ]}
      onChange={(value) => onPatchFilters({ channelType: (value ?? 'ALL') as ChannelDirectoryColumnFilters['channelType'] })}
    />
  ),
  platform: (
    <Select
      size="small"
      value={filters.platform === 'ALL' ? undefined : filters.platform}
      allowClear
      placeholder="全部"
      style={{ width: '100%' }}
      popupMatchSelectWidth={false}
      options={[
        { label: '未设置', value: 'NONE' },
        ...platformOptions,
      ]}
      onChange={(value) => onPatchFilters({ platform: (value ?? 'ALL') as ChannelDirectoryColumnFilters['platform'] })}
    />
  ),
  contact_name: (
    <Input
      size="small"
      allowClear
      value={filters.contactKeyword}
      placeholder="筛选联系人"
      onChange={(event) => onPatchFilters({ contactKeyword: event.target.value })}
    />
  ),
  contact_phone: (
    <Input
      size="small"
      allowClear
      value={filters.phoneKeyword}
      placeholder="筛选电话"
      onChange={(event) => onPatchFilters({ phoneKeyword: event.target.value })}
    />
  ),
  default_warehouse_id: (
    <Select
      size="small"
      value={filters.warehouseId || undefined}
      allowClear
      placeholder="全部"
      style={{ width: '100%' }}
      popupMatchSelectWidth={false}
      options={warehouseOptions}
      onChange={(value) => onPatchFilters({ warehouseId: String(value ?? '') })}
    />
  ),
  status: (
    <Select
      size="small"
      value={filters.status === 'ALL' ? undefined : filters.status}
      allowClear
      placeholder="全部"
      style={{ width: '100%' }}
      options={[
        { label: '启用', value: 'ACTIVE' },
        { label: '停用', value: 'DISABLED' },
      ]}
      onChange={(value) => onPatchFilters({ status: (value ?? 'ALL') as ChannelDirectoryColumnFilters['status'] })}
    />
  ),
});
