import { describe, expect, it } from 'vitest';

import type { SalesStrategy } from '@/features/salesStrategies';

import {
  buildSalesSettingsOverview,
  buildSalesSettingsSectionStatuses,
  buildStrategyCategoryCards,
  salesStrategyTypeDefinitions,
  type SalesSettingsFormValues,
} from './salesSettingsWorkbenchModel';

const baseValues: SalesSettingsFormValues = {
  autoCodeEnabled: true,
  auditMode: 'amount',
  auditThreshold: 5000,
  defaultCurrency: 'CNY',
  pricePrecision: 2,
  requiredRemark: false,
  autoReview: true,
  enableOrderRecheck: true,
  mergeMode: 'consignee',
  mergeWindow: 30,
  splitThreshold: 5,
  enableExceptionIntercept: true,
  exceptionTag: 'manual_review',
  platformIntercept: true,
  interceptReason: 'address_issue',
  interceptDuration: 24,
  interceptAutoResume: false,
  interceptNotePrefix: '平台拦截',
  addressAutoUpdate: true,
  addressStatus: 'pending_ship',
  addressConflictPolicy: 'notify',
  addressNotify: false,
  addressRemarkTemplate: '地址变更：{旧地址} -> {新地址}',
  remarkAutoSync: true,
  remarkStrategy: 'append',
  remarkTag: 'priority',
  remarkReaudit: false,
  remarkTemplate: '客户留言：{remark}',
  orderSourceDefault: 'platform',
  orderTagMode: ['channel'],
  orderChannelSync: true,
  orderNotePrefix: '渠道：{channel}',
};

const strategies: SalesStrategy[] = [
  {
    id: '1',
    type: 'logistics',
    name: '华东物流',
    priority: 90,
    validPeriod: 'forever',
    enabled: true,
    conditions: [],
    exclusions: [],
    updatedAt: '2026-04-18T09:00:00.000Z',
  },
  {
    id: '2',
    type: 'auto-merge',
    name: '夜间合单',
    priority: 60,
    mergeCount: 2,
    validPeriod: 'week',
    enabled: false,
    conditions: [],
    exclusions: [],
    updatedAt: '2026-04-17T09:00:00.000Z',
  },
  {
    id: '3',
    type: 'sales-commission',
    name: '渠道提成',
    priority: 80,
    validPeriod: 'month',
    enabled: true,
    conditions: [],
    exclusions: [],
    updatedAt: '2026-04-16T09:00:00.000Z',
  },
];

describe('salesSettingsWorkbenchModel', () => {
  it('builds executive overview cards from settings and strategies', () => {
    const overview = buildSalesSettingsOverview(baseValues, strategies);

    expect(overview).toHaveLength(6);
    expect(overview[0]).toMatchObject({
      key: 'audit',
      value: '金额审核',
      description: '阈值 5000 元',
    });
    expect(overview[1]).toMatchObject({
      key: 'processing',
      description: '同收货人合单 / 订单复核已启用',
    });
    expect(overview[5]).toMatchObject({
      key: 'strategy',
      value: '3 条策略',
      description: '覆盖 3 个策略分组',
    });
  });

  it('builds section statuses for left navigation', () => {
    const sections = buildSalesSettingsSectionStatuses(baseValues, strategies);

    expect(sections.map((item) => item.key)).toEqual([
      'base',
      'processing',
      'intercept',
      'sync',
      'source',
      'strategy-center',
    ]);
    expect(sections.find((item) => item.key === 'processing')).toMatchObject({
      status: '已启用',
      detail: '同收货人合单 / 订单复核已启用',
    });
    expect(sections.find((item) => item.key === 'intercept')).toMatchObject({
      status: '已启用',
      detail: '地址异常 / 24 小时',
    });
    expect(sections.find((item) => item.key === 'strategy-center')).toMatchObject({
      status: '3 条策略',
      detail: '3 个分组已配置',
    });
  });

  it('describes direct WMS submission when order recheck is disabled', () => {
    const values = {
      ...baseValues,
      enableOrderRecheck: false,
    };
    const overview = buildSalesSettingsOverview(values, strategies);
    const sections = buildSalesSettingsSectionStatuses(values, strategies);

    expect(overview.find((item) => item.key === 'processing')).toMatchObject({
      description: '同收货人合单 / 审核后直递 WMS',
    });
    expect(sections.find((item) => item.key === 'processing')).toMatchObject({
      detail: '同收货人合单 / 审核后直递 WMS',
    });
  });

  it('builds strategy category cards for every supported category', () => {
    const cards = buildStrategyCategoryCards(strategies);

    expect(cards).toHaveLength(salesStrategyTypeDefinitions.length);
    expect(cards.find((item) => item.key === 'logistics')).toMatchObject({
      total: 1,
      enabled: 1,
    });
    expect(cards.find((item) => item.key === 'warehouse')).toMatchObject({
      total: 0,
      enabled: 0,
    });
  });
});
