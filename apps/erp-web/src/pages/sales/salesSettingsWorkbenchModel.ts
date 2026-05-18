import type { SalesStrategy } from '@/features/salesStrategies';

export type SalesSettingsFormValues = {
  autoCodeEnabled: boolean;
  auditMode: 'auto' | 'manual' | 'amount';
  auditThreshold: number;
  defaultCurrency: string;
  pricePrecision: number;
  requiredRemark: boolean;
  autoReview: boolean;
  enableOrderRecheck: boolean;
  mergeMode: 'consignee' | 'address' | 'none';
  mergeWindow: number;
  splitThreshold: number;
  enableExceptionIntercept: boolean;
  exceptionTag: string;
  platformIntercept: boolean;
  interceptReason: string;
  interceptDuration: number;
  interceptAutoResume: boolean;
  interceptNotePrefix: string;
  addressAutoUpdate: boolean;
  addressStatus: string;
  addressConflictPolicy: string;
  addressNotify: boolean;
  addressRemarkTemplate: string;
  remarkAutoSync: boolean;
  remarkStrategy: string;
  remarkTag: string;
  remarkReaudit: boolean;
  remarkTemplate: string;
  orderSourceDefault: string;
  orderTagMode: string[];
  orderChannelSync: boolean;
  orderNotePrefix: string;
};

export type SalesWorkbenchOverviewCard = {
  key: string;
  title: string;
  value: string;
  description: string;
  tone: 'primary' | 'success' | 'warning' | 'neutral';
  sectionKey: SalesWorkbenchSectionStatus['key'];
};

export type SalesWorkbenchSectionStatus = {
  key: 'base' | 'processing' | 'intercept' | 'sync' | 'source' | 'strategy-center';
  title: string;
  status: string;
  detail: string;
};

export type StrategyCategoryCard = {
  key: string;
  label: string;
  total: number;
  enabled: number;
  summary: string;
};

export const salesStrategyTypeDefinitions = [
  { key: 'logistics', label: '物流匹配策略' },
  { key: 'warehouse', label: '仓库匹配策略' },
  { key: 'sales', label: '业务员匹配策略' },
  { key: 'auto-review', label: '自动审核策略' },
  { key: 'auto-split', label: '自动拆分策略' },
  { key: 'auto-merge', label: '自动合并策略' },
  { key: 'goods', label: '商品解析策略' },
  { key: 'order-tags', label: '订单标记策略' },
  { key: 'sales-commission', label: '销售提成策略' },
  { key: 'shipping-commission', label: '发货提成策略' },
] as const;

const auditModeLabelMap: Record<SalesSettingsFormValues['auditMode'], string> = {
  auto: '自动审核',
  manual: '人工审核',
  amount: '金额审核',
};

const mergeModeLabelMap: Record<SalesSettingsFormValues['mergeMode'], string> = {
  consignee: '同收货人合单',
  address: '同收货地址合单',
  none: '不自动合并',
};

const interceptReasonLabelMap: Record<string, string> = {
  address_issue: '地址异常',
  stock_shortage: '库存不足',
  payment_issue: '付款异常',
};

const addressStatusLabelMap: Record<string, string> = {
  pending_review: '待审核',
  pending_ship: '待发货',
  intercepted: '拦截中',
};

const sourceLabelMap: Record<string, string> = {
  platform: '平台订单',
  manual: '手工录入',
  api: '接口导入',
};

const countDistinctStrategyTypes = (strategies: SalesStrategy[]) =>
  new Set(strategies.map((strategy) => strategy.type)).size;

const formatProcessingDetail = (values: SalesSettingsFormValues) =>
  `${mergeModeLabelMap[values.mergeMode]} / ${values.enableOrderRecheck ? '订单复核已启用' : '审核后直递 WMS'}`;

export const buildSalesSettingsOverview = (
  values: SalesSettingsFormValues,
  strategies: SalesStrategy[],
): SalesWorkbenchOverviewCard[] => {
  const strategyTypes = countDistinctStrategyTypes(strategies);
  return [
    {
      key: 'audit',
      title: '审核机制',
      value: auditModeLabelMap[values.auditMode],
      description:
        values.auditMode === 'amount'
          ? `阈值 ${values.auditThreshold} 元`
          : values.requiredRemark
            ? '备注必填'
            : '按标准规则处理',
      tone: values.auditMode === 'manual' ? 'warning' : 'primary',
      sectionKey: 'base',
    },
    {
      key: 'processing',
      title: '自动处理',
      value: values.autoReview ? '自动处理开启' : '自动处理关闭',
      description: formatProcessingDetail(values),
      tone: values.autoReview ? 'success' : 'warning',
      sectionKey: 'processing',
    },
    {
      key: 'risk',
      title: '平台风控',
      value: values.platformIntercept ? '平台拦截启用' : '平台拦截关闭',
      description: `${interceptReasonLabelMap[values.interceptReason] ?? values.interceptReason} / ${values.interceptDuration} 小时`,
      tone: values.platformIntercept ? 'warning' : 'neutral',
      sectionKey: 'intercept',
    },
    {
      key: 'sync',
      title: '同步治理',
      value: values.addressAutoUpdate ? '地址同步开启' : '地址同步关闭',
      description: values.remarkAutoSync ? '备注同步已启用' : '备注同步需人工处理',
      tone: values.addressAutoUpdate && values.remarkAutoSync ? 'success' : 'warning',
      sectionKey: 'sync',
    },
    {
      key: 'source',
      title: '标识治理',
      value: sourceLabelMap[values.orderSourceDefault] ?? values.orderSourceDefault,
      description: `自动标签 ${values.orderTagMode.length} 类 / 渠道同步${values.orderChannelSync ? '开启' : '关闭'}`,
      tone: 'neutral',
      sectionKey: 'source',
    },
    {
      key: 'strategy',
      title: '策略覆盖',
      value: `${strategies.length} 条策略`,
      description: `覆盖 ${strategyTypes} 个策略分组`,
      tone: strategies.some((item) => item.enabled) ? 'primary' : 'neutral',
      sectionKey: 'strategy-center',
    },
  ];
};

export const buildSalesSettingsSectionStatuses = (
  values: SalesSettingsFormValues,
  strategies: SalesStrategy[],
): SalesWorkbenchSectionStatus[] => {
  const strategyTypes = countDistinctStrategyTypes(strategies);

  return [
    {
      key: 'base',
      title: '基础设置',
      status: auditModeLabelMap[values.auditMode],
      detail: `${values.defaultCurrency} / 价格精度 ${values.pricePrecision} 位`,
    },
    {
      key: 'processing',
      title: '自动处理',
      status: values.autoReview ? '已启用' : '待人工确认',
      detail: formatProcessingDetail(values),
    },
    {
      key: 'intercept',
      title: '平台拦截',
      status: values.platformIntercept ? '已启用' : '未启用',
      detail: `${interceptReasonLabelMap[values.interceptReason] ?? values.interceptReason} / ${values.interceptDuration} 小时`,
    },
    {
      key: 'sync',
      title: '地址与备注同步',
      status: values.addressAutoUpdate || values.remarkAutoSync ? '同步治理中' : '需关注',
      detail: `${addressStatusLabelMap[values.addressStatus] ?? values.addressStatus} / 备注${values.remarkAutoSync ? '自动' : '人工'}`,
    },
    {
      key: 'source',
      title: '来源与标签',
      status: `${values.orderTagMode.length} 项标签`,
      detail: `${sourceLabelMap[values.orderSourceDefault] ?? values.orderSourceDefault} / 渠道同步${values.orderChannelSync ? '开启' : '关闭'}`,
    },
    {
      key: 'strategy-center',
      title: '策略中心',
      status: `${strategies.length} 条策略`,
      detail: `${strategyTypes} 个分组已配置`,
    },
  ];
};

export const buildStrategyCategoryCards = (strategies: SalesStrategy[]): StrategyCategoryCard[] =>
  salesStrategyTypeDefinitions.map((definition) => {
    const grouped = strategies.filter((item) => item.type === definition.key);
    const enabled = grouped.filter((item) => item.enabled).length;
    return {
      key: definition.key,
      label: definition.label,
      total: grouped.length,
      enabled,
      summary: grouped.length > 0 ? `${enabled} 条生效 / ${grouped.length} 条配置` : '尚未配置',
    };
  });
