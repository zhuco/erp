import { MoreOutlined } from '@ant-design/icons';
import { Button, Dropdown, Tabs } from 'antd';
import type { MenuProps } from 'antd';
import { useEffect, useMemo, useRef } from 'react';
import { useLocation, useMatches, useNavigate } from 'react-router-dom';

import { resolveRouteTitleInfo } from '@/app/layout/routeMeta';
import { useTabsStore } from '@/app/layout/tabsStore';
import { useSessionStore } from '@/features/sessionStore';
import { useTenant } from '@/features/tenant';

type RouteTabsProps = {
  menuTitleMap: Map<string, string>;
  defaultPath?: string;
};

export const RouteTabs = ({ menuTitleMap, defaultPath = '/orders' }: RouteTabsProps) => {
  const location = useLocation();
  const matches = useMatches();
  const navigate = useNavigate();
  const { tenantId } = useTenant();
  const userId = useSessionStore((state) => state.userId);
  const { tabs, activeKey, addOrUpdateTab, closeTab, closeOtherTabs, closeAllTabs } =
    useTabsStore();
  const setTabs = useTabsStore((state) => state.setTabs);
  const setActiveKey = useTabsStore((state) => state.setActiveKey);
  const refreshTab = useTabsStore((state) => state.refreshTab);
  const hydratedRef = useRef(false);

  const tabsStorageKey = useMemo(() => {
    if (!tenantId || !userId) {
      return '';
    }
    return `erp.tabs.${tenantId}.${userId}`;
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
        activeKey
      })
    );
  }, [activeKey, tabs, tabsStorageKey]);

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
      closable: tabKey !== defaultPath
    });
  }, [
    addOrUpdateTab,
    defaultPath,
    isNotFoundRoute,
    location.pathname,
    location.search,
    menuTitleMap,
    routeTitleFromMeta
  ]);

  const tabItems = useMemo(
    () =>
      tabs.map((tab) => ({
        key: tab.key,
        label: tab.title,
        closable: tab.closable
      })),
    [tabs]
  );

  const handleTabChange = (key: string) => {
    const target = tabs.find((tab) => tab.key === key);
    if (target) {
      setActiveKey(key);
      navigate(target.path);
    }
  };

  const handleCloseTab = (key: string) => {
    const nextActiveKey = closeTab(key);
    if (activeKey === key) {
      const nextTab = tabs.find((tab) => tab.key === nextActiveKey);
      if (nextTab) {
        navigate(nextTab.path);
      } else {
        navigate(defaultPath);
      }
    }
  };

  const handleCloseOther = () => {
    if (!activeKey) {
      return;
    }
    closeOtherTabs(activeKey);
    const current = tabs.find((tab) => tab.key === activeKey);
    if (current) {
      navigate(current.path);
    }
  };

  const handleCloseAll = () => {
    closeAllTabs();
    navigate(defaultPath);
  };

  const tabActions: MenuProps['items'] = [
    { key: 'refresh', label: '刷新当前' },
    { key: 'close', label: '关闭当前' },
    { key: 'closeOthers', label: '关闭其他' },
    { key: 'closeAll', label: '关闭全部' }
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
            <Button size="small" icon={<MoreOutlined />} />
          </Dropdown>
        }
      />
    </div>
  );
};
