import { MoreOutlined } from '@ant-design/icons';
import { Button, Dropdown, Tabs } from 'antd';
import type { MenuProps } from 'antd';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { useLocation, useMatches, useNavigate } from 'react-router-dom';

import { resolveRouteTitleInfo } from '@/app/layout/routeMeta';
import { useTabsStore } from '@/app/layout/tabsStore';
import { useSessionStore } from '@/features/sessionStore';
import { useTenant } from '@/features/tenant';

export const ROUTE_TABS_SHORTCUT_EVENT = 'erp:route-tabs-shortcut';
export type RouteTabsShortcutAction = 'next' | 'previous' | 'close' | 'refresh';

type RouteTabsShortcutEventDetail = {
  action: RouteTabsShortcutAction;
};

type RouteTabsProps = {
  menuTitleMap: Map<string, string>;
  defaultPath?: string;
};

const WORKBENCH_ANNOUNCE_EVENT = 'erp:workbench-announce';

const announceWorkbench = (messageText: string) => {
  window.dispatchEvent(
    new CustomEvent(WORKBENCH_ANNOUNCE_EVENT, {
      detail: { message: messageText },
    }),
  );
};

const buildClosedTabSavedViewKeys = (
  tabKey: string,
  tenantId?: string | null,
  userId?: string | null,
) => {
  const normalizedTenantId = String(tenantId ?? '').trim();
  const normalizedUserId = String(userId ?? '').trim() || 'anonymous';
  if (!normalizedTenantId) {
    return [];
  }

  if (tabKey === '/orders/review') {
    return [
      `orders.review.savedView.v1.${normalizedTenantId}.${normalizedUserId}`,
      `orders.review.filterSnapshot.v1.${normalizedTenantId}.${normalizedUserId}`,
    ];
  }

  if (tabKey === '/orders/exceptions' || tabKey === '/orders/recheck') {
    return [
      `orders.exceptions.savedView.v1.pool.${normalizedTenantId}.${normalizedUserId}`,
      `orders.exceptions.savedView.v1.sla.${normalizedTenantId}.${normalizedUserId}`,
    ];
  }

  if (tabKey === '/orders') {
    return ['orders.list.savedView'];
  }

  return [];
};

const isEditableTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tagName = target.tagName;
  return (
    target.isContentEditable ||
    tagName === 'INPUT' ||
    tagName === 'TEXTAREA' ||
    tagName === 'SELECT' ||
    Boolean(target.closest('.ant-select-dropdown, .ant-picker-dropdown'))
  );
};

export const RouteTabs = ({ menuTitleMap, defaultPath = '/dashboard' }: RouteTabsProps) => {
  const location = useLocation();
  const matches = useMatches();
  const navigate = useNavigate();
  const { tenantId } = useTenant();
  const userId = useSessionStore((state) => state.userId);
  const { tabs, activeKey, addOrUpdateTab, closeTab, closeOtherTabs, closeAllTabs } = useTabsStore();
  const setTabs = useTabsStore((state) => state.setTabs);
  const setActiveKey = useTabsStore((state) => state.setActiveKey);
  const refreshTab = useTabsStore((state) => state.refreshTab);
  const hydratedRef = useRef(false);

  const tabsStorageKey = useMemo(() => {
    if (!tenantId || !userId) {
      return '';
    }
    return `erp.tabs.v2.${tenantId}.${userId}`;
  }, [tenantId, userId]);

  useEffect(() => {
    if (!tabsStorageKey) {
      setTabs([], undefined);
      return;
    }
    const raw = localStorage.getItem(tabsStorageKey);
    if (!raw) {
      setTabs([], undefined);
      hydratedRef.current = true;
      return;
    }
    try {
      const parsed = JSON.parse(raw) as { tabs?: typeof tabs; activeKey?: string };
      if (Array.isArray(parsed.tabs)) {
        setTabs(parsed.tabs, parsed.activeKey);
      }
    } catch {
      // ignore malformed cache
    }
    hydratedRef.current = true;
  }, [setTabs, tabsStorageKey]);

  useEffect(() => {
    if (!tabsStorageKey || !hydratedRef.current) {
      return;
    }
    localStorage.setItem(
      tabsStorageKey,
      JSON.stringify({
        tabs,
        activeKey,
      }),
    );
  }, [activeKey, tabs, tabsStorageKey]);

  const clearSavedViewForClosedTab = useCallback(
    (tabKey: string) => {
      const keys = buildClosedTabSavedViewKeys(tabKey, tenantId, userId);
      if (!keys.length) {
        return;
      }
      keys.forEach((key) => {
        localStorage.removeItem(key);
      });
    },
    [tenantId, userId],
  );

  const routeTitleFromMeta = useMemo(() => {
    for (let index = matches.length - 1; index >= 0; index -= 1) {
      const route = matches[index].route as { handle?: { title?: string }; meta?: { title?: string } };
      if (route?.handle?.title) {
        return route.handle.title;
      }
      if (route?.meta?.title) {
        return route.meta.title;
      }
    }
    return undefined;
  }, [matches]);

  const isNotFoundRoute = useMemo(() => {
    const lastMatch = matches[matches.length - 1];
    return lastMatch?.route?.path === '*';
  }, [matches]);

  useEffect(() => {
    if (isNotFoundRoute || location.pathname === '/login') {
      return;
    }
    const titleInfo = resolveRouteTitleInfo(location.pathname, menuTitleMap);
    const title = routeTitleFromMeta ?? titleInfo.title;
    const tabKey = location.pathname;
    addOrUpdateTab({
      key: tabKey,
      path: `${location.pathname}${location.search}`,
      title,
      closable: tabKey !== defaultPath,
    });
  }, [
    addOrUpdateTab,
    defaultPath,
    isNotFoundRoute,
    location.pathname,
    location.search,
    menuTitleMap,
    routeTitleFromMeta,
  ]);

  const tabItems = useMemo(
    () =>
      tabs.map((tab) => ({
        key: tab.key,
        label: tab.title,
        closable: tab.closable,
      })),
    [tabs],
  );

  const handleTabChange = useCallback(
    (key: string) => {
      const target = tabs.find((tab) => tab.key === key);
      if (target) {
        setActiveKey(key);
        navigate(target.path);
      }
    },
    [navigate, setActiveKey, tabs],
  );

  const handleCloseTab = useCallback(
    (key: string) => {
      const closingTab = tabs.find((tab) => tab.key === key) ?? null;
      if (!closingTab || !closingTab.closable) {
        return;
      }

      const nextActiveKey = closeTab(key);
      clearSavedViewForClosedTab(key);
      announceWorkbench(`已关闭页签：${closingTab.title}`);
      if (activeKey === key) {
        const nextTab = tabs.find((tab) => tab.key === nextActiveKey);
        if (nextTab) {
          navigate(nextTab.path);
        } else {
          navigate(defaultPath);
        }
      }
    },
    [activeKey, clearSavedViewForClosedTab, closeTab, defaultPath, navigate, tabs],
  );

  const handleCloseOther = useCallback(() => {
    if (!activeKey) {
      return;
    }
    tabs
      .filter((tab) => tab.key !== activeKey)
      .forEach((tab) => {
        clearSavedViewForClosedTab(tab.key);
      });
    closeOtherTabs(activeKey);
    const current = tabs.find((tab) => tab.key === activeKey);
    if (current) {
      navigate(current.path);
    }
    announceWorkbench('已关闭其他页签');
  }, [activeKey, clearSavedViewForClosedTab, closeOtherTabs, navigate, tabs]);

  const handleCloseAll = useCallback(() => {
    tabs.forEach((tab) => {
      clearSavedViewForClosedTab(tab.key);
    });
    closeAllTabs();
    navigate(defaultPath);
    announceWorkbench('已关闭全部页签');
  }, [clearSavedViewForClosedTab, closeAllTabs, defaultPath, navigate, tabs]);

  const cycleActiveTab = useCallback(
    (step: 1 | -1) => {
      if (tabs.length <= 1) {
        return;
      }
      const currentActiveKey = activeKey ?? location.pathname;
      const currentIndex = tabs.findIndex((tab) => tab.key === currentActiveKey);
      const baseIndex = currentIndex >= 0 ? currentIndex : 0;
      const nextIndex = (baseIndex + step + tabs.length) % tabs.length;
      const nextTab = tabs[nextIndex];
      if (!nextTab) {
        return;
      }
      setActiveKey(nextTab.key);
      navigate(nextTab.path);
    },
    [activeKey, location.pathname, navigate, setActiveKey, tabs],
  );

  const runShortcutAction = useCallback(
    (action: RouteTabsShortcutAction) => {
      if (action === 'next') {
        cycleActiveTab(1);
        announceWorkbench('已切换到下一个页签');
        return;
      }
      if (action === 'previous') {
        cycleActiveTab(-1);
        announceWorkbench('已切换到上一个页签');
        return;
      }
      const currentKey = activeKey ?? location.pathname;
      if (!currentKey) {
        return;
      }
      if (action === 'close') {
        handleCloseTab(currentKey);
        return;
      }
      refreshTab(currentKey);
      announceWorkbench('已刷新当前页签');
    },
    [activeKey, cycleActiveTab, handleCloseTab, location.pathname, refreshTab],
  );

  useEffect(() => {
    const handleGlobalKeyDown = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) {
        return;
      }
      const withCtrlOrMeta = event.ctrlKey || event.metaKey;
      if (!withCtrlOrMeta || event.altKey) {
        return;
      }

      if (event.key === 'Tab') {
        event.preventDefault();
        runShortcutAction(event.shiftKey ? 'previous' : 'next');
        return;
      }

      const key = event.key.toLowerCase();
      if (key === 'w') {
        event.preventDefault();
        runShortcutAction('close');
        return;
      }

      if (key === 'r') {
        event.preventDefault();
        runShortcutAction('refresh');
      }
    };

    const handleShortcutEvent = (event: Event) => {
      const detail = (event as CustomEvent<RouteTabsShortcutEventDetail>).detail;
      if (!detail?.action) {
        return;
      }
      runShortcutAction(detail.action);
    };

    window.addEventListener('keydown', handleGlobalKeyDown);
    window.addEventListener(ROUTE_TABS_SHORTCUT_EVENT, handleShortcutEvent as EventListener);
    return () => {
      window.removeEventListener('keydown', handleGlobalKeyDown);
      window.removeEventListener(ROUTE_TABS_SHORTCUT_EVENT, handleShortcutEvent as EventListener);
    };
  }, [runShortcutAction]);

  const tabActions: MenuProps['items'] = [
    { key: 'refresh', label: '刷新当前' },
    { key: 'close', label: '关闭当前' },
    { key: 'closeOthers', label: '关闭其他' },
    { key: 'closeAll', label: '关闭全部' },
  ];

  const handleTabActionClick: MenuProps['onClick'] = ({ key }) => {
    if (!activeKey) {
      return;
    }
    if (key === 'refresh') {
      refreshTab(activeKey);
    }
    if (key === 'close') {
      handleCloseTab(activeKey);
    }
    if (key === 'closeOthers') {
      handleCloseOther();
    }
    if (key === 'closeAll') {
      handleCloseAll();
    }
  };

  return (
    <div className="workbench-tabs">
      <Tabs
        type="editable-card"
        hideAdd
        size="small"
        activeKey={activeKey ?? location.pathname}
        items={tabItems}
        onChange={handleTabChange}
        onEdit={(key, action) => {
          if (action === 'remove') {
            handleCloseTab(key as string);
          }
        }}
        tabBarExtraContent={
          <Dropdown menu={{ items: tabActions, onClick: handleTabActionClick }}>
            <Button size="small" icon={<MoreOutlined />} aria-label="Tab actions" />
          </Dropdown>
        }
      />
    </div>
  );
};
