import { Tag } from 'antd';
import type { ColDef } from 'ag-grid-community';

import type { Channel } from '@/api/channels';
import type { ErpDataGridNextColumn } from '@/components/ErpDataGridNext';
import { PlatformTag } from '@/components/PlatformTag';

import {
  formatChannelDateTime,
  resolveChannelPlatformLabel,
  resolveChannelStatusLabel,
  resolveChannelTypeLabel,
} from './channelDirectoryModel';

const renderBooleanTag = (value: boolean) =>
  value ? <Tag color="green">允许</Tag> : <Tag>关闭</Tag>;

const platformFilterValues = [
  'NONE',
  'TAOBAO',
  'TMALL',
  'JD',
  'PDD',
  'DOUYIN',
  'KUAISHOU',
  'WECHAT',
  'OTHER',
];

export const buildChannelDirectoryColumns = (
  warehouseMap: Map<string, string>,
  warehouseOptions: Array<{ label: string; value: string }>,
): Array<ErpDataGridNextColumn<Channel> | ColDef<Channel>> => [
  {
    key: 'channel_id',
    field: 'channel_id',
    header: '渠道编号',
    width: 148,
    minWidth: 136,
    sortValue: (channel) => channel.channel_id,
    filter: false,
    render: (channel) => channel.channel_id,
  },
  {
    key: 'name',
    field: 'name',
    header: '渠道名称',
    width: 220,
    minWidth: 180,
    sortValue: (channel) => channel.name,
    filter: 'agTextColumnFilter',
    filterParams: {
      filterOptions: ['contains'],
      defaultOption: 'contains',
      maxNumConditions: 1,
    },
    render: (channel) => channel.name,
  },
  {
    key: 'channel_type',
    field: 'channel_type',
    header: '渠道类型',
    width: 110,
    minWidth: 100,
    sortValue: (channel) => resolveChannelTypeLabel(channel.channel_type),
    filter: 'agSetColumnFilter',
    filterParams: {
      values: ['ONLINE', 'OFFLINE'],
    },
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
    filter: 'agSetColumnFilter',
    filterParams: {
      values: platformFilterValues,
    },
    valueGetter: (params: { data?: Channel }) => params.data?.platform ?? 'NONE',
    render: (channel) =>
      channel.platform ? <PlatformTag platform={channel.platform} /> : <Tag>线下渠道</Tag>,
  },
  {
    key: 'contact_name',
    field: 'contact_name',
    header: '联系人',
    width: 120,
    minWidth: 110,
    sortValue: (channel) => channel.contact_name ?? '',
    filter: 'agTextColumnFilter',
    filterParams: {
      filterOptions: ['contains'],
      defaultOption: 'contains',
      maxNumConditions: 1,
    },
    render: (channel) => channel.contact_name || '-',
  },
  {
    key: 'contact_phone',
    field: 'contact_phone',
    header: '电话',
    width: 140,
    minWidth: 120,
    sortValue: (channel) => channel.contact_phone ?? '',
    filter: 'agTextColumnFilter',
    filterParams: {
      filterOptions: ['contains'],
      defaultOption: 'contains',
      maxNumConditions: 1,
    },
    render: (channel) => channel.contact_phone || '-',
  },
  {
    key: 'default_warehouse_id',
    field: 'default_warehouse_id',
    header: '默认发货仓库',
    width: 160,
    minWidth: 140,
    sortValue: (channel) =>
      channel.default_warehouse_id
        ? warehouseMap.get(channel.default_warehouse_id) ?? channel.default_warehouse_id
        : '',
    filter: 'agSetColumnFilter',
    filterParams: {
      values: warehouseOptions.map((option) => option.value),
    },
    render: (channel) =>
      channel.default_warehouse_id
        ? warehouseMap.get(channel.default_warehouse_id) ?? channel.default_warehouse_id
        : '-',
  },
  {
    key: 'allow_manual_order',
    field: 'allow_manual_order',
    header: '手工销售单',
    width: 110,
    minWidth: 100,
    sortValue: (channel) => (channel.allow_manual_order ? 1 : 0),
    filter: false,
    render: (channel) => renderBooleanTag(channel.allow_manual_order),
  },
  {
    key: 'allow_manual_after_sales',
    field: 'allow_manual_after_sales',
    header: '手工售后单',
    width: 110,
    minWidth: 100,
    sortValue: (channel) => (channel.allow_manual_after_sales ? 1 : 0),
    filter: false,
    render: (channel) => renderBooleanTag(channel.allow_manual_after_sales),
  },
  {
    key: 'commission_formula',
    field: 'commission_formula',
    header: '平台抽佣公式',
    width: 180,
    minWidth: 150,
    sortValue: (channel) => channel.commission_formula ?? '',
    filter: 'agTextColumnFilter',
    filterParams: {
      filterOptions: ['contains'],
      defaultOption: 'contains',
      maxNumConditions: 1,
    },
    render: (channel) => channel.commission_formula || '-',
  },
  {
    key: 'status',
    field: 'status',
    header: '状态',
    width: 100,
    minWidth: 90,
    sortValue: (channel) => resolveChannelStatusLabel(channel.status),
    filter: 'agSetColumnFilter',
    filterParams: {
      values: ['ACTIVE', 'DISABLED'],
    },
    render: (channel) => (
      <Tag color={channel.status === 'ACTIVE' ? 'green' : 'default'}>
        {resolveChannelStatusLabel(channel.status)}
      </Tag>
    ),
  },
  {
    key: 'updated_at',
    field: 'updated_at',
    header: '更新时间',
    width: 170,
    minWidth: 160,
    sortValue: (channel) => channel.updated_at ?? channel.created_at ?? '',
    filter: 'agDateColumnFilter',
    filterParams: {
      filterOptions: ['inRange', 'greaterThanOrEqual', 'lessThanOrEqual'],
      defaultOption: 'inRange',
      maxNumConditions: 1,
    },
    filterValueGetter: (params: { data?: Channel }) => {
      const raw = String(params.data?.updated_at ?? params.data?.created_at ?? '').trim();
      if (!raw) {
        return null;
      }
      const value = new Date(raw);
      return Number.isNaN(value.getTime()) ? null : value;
    },
    render: (channel) => formatChannelDateTime(channel.updated_at ?? channel.created_at),
  },
];
