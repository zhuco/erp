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
    summary: '聚焦订单审核与订单复核，保留重新搭建前最核心的作业入口。',
    priority: 10,
    focusTags: ['审核', '复核'],
    actions: [
      { label: '进入订单审核', path: '/orders/review' },
      { label: '进入订单复核', path: '/orders/recheck' },
    ],
  },
  {
    key: 'channels',
    title: '渠道台',
    summary: '保留渠道目录作为基础台账入口，方便后续按新方案逐步扩展。',
    priority: 20,
    focusTags: ['渠道', '目录'],
    actions: [{ label: '进入渠道目录', path: '/channels' }],
  },
  {
    key: 'settings',
    title: '设置台',
    summary: '保留销售设置页面，作为后续销售域 UI 重建的唯一配置入口。',
    priority: 30,
    focusTags: ['设置', '销售'],
    actions: [{ label: '进入销售设置', path: '/sales/settings' }],
  },
];

const quickLinkPriority = [
  '/orders/review',
  '/orders/recheck',
  '/channels',
  '/sales/settings',
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
