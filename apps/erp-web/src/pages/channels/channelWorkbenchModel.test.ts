import { describe, expect, it } from 'vitest';

import type { Channel } from '@/api/channels';

import {
  buildChannelActiveFilterSummary,
  buildChannelAuthCallbackUrl,
  buildChannelListQuery,
  buildChannelQuickFilters,
  filterChannels,
  parseChannelAuthCallbackSearch,
  resolveChannelQuickFilterState,
  summarizeChannels,
} from './channelWorkbenchModel';

const channels: Channel[] = [
  {
    channel_id: 'CH-TAOBAO',
    tenant_id: 'TENANT-1',
    platform: 'taobao',
    name: '淘宝旗舰店',
    department: '电商事业部',
    owner: '张三',
    allow_manual_order: true,
    allow_manual_after_sales: false,
    default_warehouse_id: 'WH-1',
    commission_formula: 'amount * 0.05',
    status: 'ACTIVE',
    created_at: '2026-04-10T08:00:00.000Z',
    updated_at: '2026-04-13T08:00:00.000Z',
  },
  {
    channel_id: 'CH-JD',
    tenant_id: 'TENANT-1',
    platform: 'jd',
    name: '京东自营',
    department: '销售中心',
    owner: '李四',
    allow_manual_order: false,
    allow_manual_after_sales: true,
    default_warehouse_id: null,
    commission_formula: null,
    status: 'DISABLED',
    created_at: '2026-04-09T08:00:00.000Z',
    updated_at: '2026-04-12T08:00:00.000Z',
  },
  {
    channel_id: 'CH-PDD',
    tenant_id: 'TENANT-1',
    platform: 'pdd',
    name: '拼多多专营',
    department: '电商事业部',
    owner: '王五',
    allow_manual_order: true,
    allow_manual_after_sales: true,
    default_warehouse_id: 'WH-2',
    commission_formula: 'amount * 0.03',
    status: 'ACTIVE',
    created_at: '2026-04-08T08:00:00.000Z',
    updated_at: '2026-04-11T08:00:00.000Z',
  },
];

describe('channelWorkbenchModel', () => {
  it('builds browser callback urls on /channels', () => {
    expect(
      buildChannelAuthCallbackUrl({
        origin: 'https://erp.example.com',
        routerMode: 'browser',
        routerBasePath: '/',
        channelId: 'CH-TAOBAO',
        platform: 'taobao',
      }),
    ).toBe(
      'https://erp.example.com/channels?auth_callback=1&channel_id=CH-TAOBAO&platform=taobao',
    );
  });

  it('builds hash callback urls with router basename', () => {
    expect(
      buildChannelAuthCallbackUrl({
        origin: 'https://erp.example.com/',
        routerMode: 'hash',
        routerBasePath: '/erp/',
        channelId: 'CH-JD',
        platform: 'jd',
      }),
    ).toBe('https://erp.example.com/erp#/channels?auth_callback=1&channel_id=CH-JD&platform=jd');
  });

  it('parses callback query strings and exposes callback state', () => {
    expect(
      parseChannelAuthCallbackSearch(
        '?auth_callback=1&channel_id=CH-PDD&platform=pdd&code=abc123&state=st-1',
      ),
    ).toEqual({
      channelId: 'CH-PDD',
      code: 'abc123',
      error: undefined,
      errorDescription: undefined,
      hasCode: true,
      hasError: false,
      platform: 'pdd',
      state: 'st-1',
    });
  });

  it('returns null when the current search is not an auth callback', () => {
    expect(parseChannelAuthCallbackSearch('?platform=taobao')).toBeNull();
  });

  it('summarizes channel capability counts for the workbench overview', () => {
    expect(summarizeChannels(channels)).toEqual({
      active: 2,
      defaultWarehouseConfigured: 2,
      disabled: 1,
      manualAfterSales: 2,
      manualOrder: 2,
      platformCount: 3,
      total: 3,
    });
  });

  it('builds left-panel quick filters with counts', () => {
    expect(buildChannelQuickFilters(channels)).toEqual([
      {
        count: 3,
        description: '查看全部渠道',
        id: 'all',
        label: '全部渠道',
      },
      {
        count: 2,
        description: '仅看启用中的渠道',
        id: 'ACTIVE',
        label: '启用中',
      },
      {
        count: 1,
        description: '仅看已停用渠道',
        id: 'DISABLED',
        label: '已停用',
      },
      {
        count: 2,
        description: '支持手工建单',
        id: 'manual-order',
        label: '手工建单',
      },
      {
        count: 2,
        description: '支持手工售后',
        id: 'manual-after-sales',
        label: '手工售后',
      },
      {
        count: 2,
        description: '已配置默认仓',
        id: 'has-warehouse',
        label: '已配置默认仓',
      },
    ]);
  });

  it('keeps status on the server query for default active views', () => {
    expect(buildChannelListQuery('  taobao  ', 'ACTIVE')).toEqual({
      keyword: 'taobao',
      limit: 200,
      offset: 0,
      status: 'ACTIVE',
    });
  });

  it('omits status only when the page is explicitly in all-status mode', () => {
    expect(buildChannelListQuery('', 'all')).toEqual({
      keyword: undefined,
      limit: 200,
      offset: 0,
      status: undefined,
    });
  });

  it('resolves quick filters into workbench filter state', () => {
    expect(resolveChannelQuickFilterState('manual-after-sales')).toEqual({
      columnFilters: {
        allowManualAfterSales: 'allow',
        allowManualOrder: undefined,
        hasDefaultWarehouse: undefined,
      },
      statusFilter: 'all',
    });

    expect(resolveChannelQuickFilterState('DISABLED')).toEqual({
      columnFilters: {
        allowManualAfterSales: undefined,
        allowManualOrder: undefined,
        hasDefaultWarehouse: undefined,
      },
      statusFilter: 'DISABLED',
    });
  });

  it('filters channels with the shared workbench rules', () => {
    expect(
      filterChannels(channels, {
        columnFilters: {
          allowManualOrder: 'allow',
          updatedRange: {
            start: '2026-04-11',
            end: '2026-04-13',
          },
        },
        searchKeyword: '事业部',
        statusFilter: 'all',
      }).map((channel) => channel.channel_id),
    ).toEqual(['CH-TAOBAO', 'CH-PDD']);

    expect(
      filterChannels(channels, {
        columnFilters: {
          allowManualOrder: 'allow',
          updatedRange: {
            start: '2026-04-12',
            end: '2026-04-13',
          },
        },
        searchKeyword: '张三',
        statusFilter: 'all',
      }).map((channel) => channel.channel_id),
    ).toEqual(['CH-TAOBAO']);
  });

  it('builds active filter summary for the workbench side panel', () => {
    expect(
      buildChannelActiveFilterSummary({
        activeQuickFilterId: 'has-warehouse',
        columnFilters: {
          allowManualOrder: 'allow',
          owner: '张三',
          warehouseId: 'WH-1',
        },
        quickFilters: buildChannelQuickFilters(channels),
        searchKeyword: '旗舰',
        statusFilter: 'ACTIVE',
        warehouseNameResolver: (warehouseId) => (warehouseId === 'WH-1' ? '华东一仓' : warehouseId),
      }),
    ).toEqual([
      '视图：已配置默认仓',
      '搜索：旗舰',
      '负责人：张三',
      '默认仓：华东一仓',
      '手工建单：允许',
      '状态：启用中',
    ]);
  });
});
