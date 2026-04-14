import type { AppMenuLeaf } from '@/app/layout/menu';

export type DashboardWorkbenchAction = {
  label: string;
  path: string;
};

export type DashboardWorkbenchCard = {
  key: string;
  title: string;
  summary: string;
  focusTags: string[];
  availableCount: number;
  primaryAction: DashboardWorkbenchAction;
  actions: DashboardWorkbenchAction[];
};

type DashboardWorkbenchTemplate = {
  key: string;
  title: string;
  summary: string;
  priority: number;
  focusTags: string[];
  actions: DashboardWorkbenchAction[];
};

const dashboardWorkbenchTemplates: DashboardWorkbenchTemplate[] = [
  {
    key: 'review',
    title: '审单台',
    summary: '聚焦订单审核、异常复核与规则落地。',
    priority: 10,
    focusTags: ['审核', '异常', '规则'],
    actions: [
      { label: '进入审单', path: '/orders/review' },
      { label: '异常订单', path: '/orders/exceptions' },
      { label: '审单规则', path: '/orders/review-rules' },
    ],
  },
  {
    key: 'fulfillment',
    title: '履约台',
    summary: '串起包裹处理、打包复核与发货回传。',
    priority: 20,
    focusTags: ['包裹', '复核', '发货'],
    actions: [
      { label: '发货包裹', path: '/shipments' },
      { label: '打包复核', path: '/wms/pack' },
      { label: '按单发货', path: '/wms/ship' },
      { label: '回传失败', path: '/shipments/push-failures' },
    ],
  },
  {
    key: 'warehouse',
    title: '仓配台',
    summary: '盯住库存、波次与拣货执行的核心链路。',
    priority: 30,
    focusTags: ['库存', '波次', '拣货'],
    actions: [
      { label: '库存总览', path: '/inventory' },
      { label: '波次管理', path: '/wms/waves' },
      { label: '拣货任务', path: '/wms/pick-tasks' },
      { label: '库存预警', path: '/inventory/insights/alerts' },
    ],
  },
  {
    key: 'procurement',
    title: '采购台',
    summary: '围绕采购计划、订单与到货节奏快速处理。',
    priority: 40,
    focusTags: ['计划', '订单', '到货'],
    actions: [
      { label: '采购订单', path: '/procurement/orders' },
      { label: '采购计划', path: '/procurement/plans' },
      { label: '采购建议', path: '/procurement/suggestions' },
      { label: '采购到货', path: '/procurement/receipts' },
    ],
  },
  {
    key: 'service',
    title: '客服台',
    summary: '让售后单、退款拦截和客服工单协同处理。',
    priority: 50,
    focusTags: ['售后', '工单', 'SLA'],
    actions: [
      { label: '售后单', path: '/after-sales' },
      { label: '退款拦截', path: '/after-sales/intercepts' },
      { label: '客服工单', path: '/customers/service-tickets' },
      { label: '售后 SLA', path: '/after-sales/sla' },
    ],
  },
  {
    key: 'finance',
    title: '财务台',
    summary: '把对账、收付和开票入口压缩到一个工作面。',
    priority: 60,
    focusTags: ['对账', '收付', '开票'],
    actions: [
      { label: '对账中心', path: '/finance/reconcile' },
      { label: '应收管理', path: '/finance/ar' },
      { label: '应付管理', path: '/finance/ap' },
      { label: '开票中心', path: '/finance/invoices' },
    ],
  },
  {
    key: 'operations',
    title: '运营台',
    summary: '衔接渠道、店铺与商品接入配置，适合日常巡检。',
    priority: 70,
    focusTags: ['渠道', '店铺', '商品'],
    actions: [
      { label: '渠道目录', path: '/channels' },
      { label: '店铺接入', path: '/channels/shops' },
      { label: '商品管理', path: '/products' },
      { label: '同步任务', path: '/channels/sync-jobs' },
    ],
  },
];

const quickLinkPriority = [
  '/orders/review',
  '/orders',
  '/shipments',
  '/inventory',
  '/channels',
  '/procurement/orders',
  '/finance/reconcile',
  '/system/settings',
];

export const buildDashboardWorkbenchCards = (
  menuLeaves: Pick<AppMenuLeaf, 'path' | 'name'>[],
  limit = 4,
): DashboardWorkbenchCard[] => {
  const accessiblePaths = new Set(menuLeaves.map((item) => item.path));

  return dashboardWorkbenchTemplates
    .map((template) => {
      const accessibleActions = template.actions.filter((action) => accessiblePaths.has(action.path));
      if (!accessibleActions.length) {
        return null;
      }

      const [primaryAction, ...restActions] = accessibleActions;
      return {
        key: template.key,
        title: template.title,
        summary: template.summary,
        focusTags: template.focusTags,
        availableCount: accessibleActions.length,
        primaryAction,
        actions: restActions.slice(0, 3),
      };
    })
    .filter((card): card is DashboardWorkbenchCard => Boolean(card))
    .sort((left, right) => {
      const leftTemplate = dashboardWorkbenchTemplates.find((item) => item.key === left.key);
      const rightTemplate = dashboardWorkbenchTemplates.find((item) => item.key === right.key);
      return (leftTemplate?.priority ?? 999) - (rightTemplate?.priority ?? 999);
    })
    .slice(0, limit);
};

export const buildDashboardQuickLinks = (
  menuLeaves: Pick<AppMenuLeaf, 'path' | 'name'>[],
  limit = 6,
): DashboardWorkbenchAction[] => {
  const entryMap = new Map(menuLeaves.map((item) => [item.path, item]));

  return quickLinkPriority
    .filter((path) => entryMap.has(path))
    .slice(0, limit)
    .map((path) => ({
      path,
      label: entryMap.get(path)?.name ?? path,
    }));
};
