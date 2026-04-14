import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import type { Channel } from '@/api/channels';

import type {
  ChannelAuthCallbackState,
  ChannelQuickFilter,
  ChannelWorkbenchSummary,
} from './channelWorkbenchModel';
import {
  ChannelsWorkbenchLeftPanel,
  ChannelsWorkbenchOverview,
  ChannelsWorkbenchRightPanel,
} from './ChannelWorkbenchPanels';

const selectedChannel: Channel = {
  channel_id: 'CH-TAOBAO',
  tenant_id: 'TENANT-1',
  platform: 'taobao',
  name: '淘宝旗舰店渠道',
  department: '电商事业部',
  owner: '张三',
  allow_manual_order: true,
  allow_manual_after_sales: false,
  default_warehouse_id: 'WH-1',
  commission_formula: 'amount * 0.05',
  status: 'ACTIVE',
  created_at: '2026-04-10T08:00:00.000Z',
  updated_at: '2026-04-13T08:00:00.000Z',
};

const summary: ChannelWorkbenchSummary = {
  total: 12,
  active: 9,
  disabled: 3,
  manualOrder: 7,
  manualAfterSales: 5,
  defaultWarehouseConfigured: 8,
  platformCount: 4,
};

const quickFilters: ChannelQuickFilter[] = [
  { id: 'all', label: '全部渠道', description: '查看全部渠道', count: 12 },
  { id: 'ACTIVE', label: '启用中', description: '仅看启用中的渠道', count: 9 },
  { id: 'manual-order', label: '手工建单', description: '支持手工建单', count: 7 },
];

const callbackState: ChannelAuthCallbackState = {
  channelId: 'CH-TAOBAO',
  platform: 'taobao',
  code: 'auth-code',
  state: 'state-1',
  error: undefined,
  errorDescription: undefined,
  hasCode: true,
  hasError: false,
};

describe('ChannelWorkbenchPanels', () => {
  it('renders overview with callback alert', () => {
    const markup = renderToStaticMarkup(
      createElement(ChannelsWorkbenchOverview, {
        callbackState,
        summary,
      }),
    );

    expect(markup).toContain('data-testid="channels-workbench-overview"');
    expect(markup).toContain('data-testid="channels-callback-alert"');
  });

  it('renders left panel quick filters and selected channel', () => {
    const markup = renderToStaticMarkup(
      createElement(ChannelsWorkbenchLeftPanel, {
        activeFilterSummary: ['状态：启用中', '平台：淘宝'],
        activeQuickFilterId: 'ACTIVE',
        onSelectQuickFilter: () => undefined,
        quickFilters,
        selectedChannel,
      }),
    );

    expect(markup).toContain('data-testid="channels-left-panel"');
    expect(markup).toContain('data-testid="channels-left-quick-filter-ACTIVE"');
    expect(markup).toContain('淘宝旗舰店渠道');
  });

  it('renders right panel callback and action links without channel mapping entry', () => {
    const markup = renderToStaticMarkup(
      createElement(ChannelsWorkbenchRightPanel, {
        callbackState,
        callbackUrl:
          'https://erp.example.com/channels?auth_callback=1&channel_id=CH-TAOBAO&platform=taobao',
        onCopyCallbackUrl: () => undefined,
        onOpenShops: () => undefined,
        onOpenSyncJobs: () => undefined,
        selectedChannel,
        warehouseMap: new Map([['WH-1', '华东一仓']]),
      }),
    );

    expect(markup).toContain('data-testid="channels-right-panel"');
    expect(markup).toContain('data-testid="channels-auth-callback-url"');
    expect(markup).toContain('https://erp.example.com/channels?auth_callback=1');
    expect(markup).not.toContain('渠道映射');
  });
});
