import { Alert, Avatar, Button, Dropdown, Input, Layout, Menu, Select, Space, Tabs } from 'antd';
import type { MenuProps } from 'antd';
import { DownOutlined, MenuOutlined, MoreOutlined, SearchOutlined } from '@ant-design/icons';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import type { ReactNode } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import { ErrorBoundary } from '@/components/ErrorBoundary';
import type { FeatureFlags } from '@/features/featureFlags';
import { useFeatureFlag } from '@/features/featureFlags';
import { useMenuCacheSettingsStore } from '@/features/menuCacheSettings';
import { useTenant } from '@/features/tenant';
import { shopOptions, tenantOptions } from '@/features/tenantOptions';
import { isFeatureEnabled } from '@/features/featureFlags';
import type { Permission } from '@/features/rbac';
import { hasPermission } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import { listWarehouses } from '@/api/warehouses';
import type { AppMenuItem } from '@/app/layout/menu';
import { appMenuItems, flattenMenuItems } from '@/app/layout/menu';
import { ShortcutToolbar } from '@/app/layout/ShortcutToolbar';
import { useTabsStore } from '@/app/layout/tabsStore';
import { resolveRouteTitle } from '@/app/layout/routeMeta';
import brandLogo from '@/assets/brand/hengceyun-logo.png';

import './appLayout.css';

const { Header, Content } = Layout;

type CachedMenuItem = {
  name?: string;
  path?: string;
  children?: CachedMenuItem[];
};

const menuCacheKeyPrefix = 'erp.menu.cache';

const encodeCacheKey = (value: string) => {
  if (typeof window === 'undefined') {
    return value;
  }
  try {
    return window.btoa(unescape(encodeURIComponent(value)));
  } catch {
    return value;
  }
};

const buildMenuCacheSignature = (permissions: Permission[], featureFlags: FeatureFlags) =>
  encodeCacheKey(
    JSON.stringify({
      permissions: [...permissions].sort(),
      featureFlags
    })
  );

const loadMenuCache = (key: string): CachedMenuItem[] | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  const raw = window.localStorage.getItem(key);
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as CachedMenuItem[];
    if (Array.isArray(parsed)) {
      return parsed;
    }
  } catch {
    return null;
  }
  return null;
};

const saveMenuCache = (key: string, value: CachedMenuItem[]) => {
  if (typeof window === 'undefined') {
    return;
  }
  window.localStorage.setItem(key, JSON.stringify(value));
};

const stripMenuIcons = (items: AppMenuItem[]): CachedMenuItem[] =>
  items.map((item) => ({
    name: item.name,
    path: item.path,
    children: item.children ? stripMenuIcons(item.children as AppMenuItem[]) : undefined
  }));

const buildIconMap = (items: AppMenuItem[], map = new Map<string, ReactNode>()) => {
  items.forEach((item) => {
    const key = item.path ?? item.name;
    if (key) {
      map.set(key, item.icon);
    }
    if (item.children) {
      buildIconMap(item.children as AppMenuItem[], map);
    }
  });
  return map;
};

const hydrateMenuIcons = (
  items: CachedMenuItem[],
  iconMap: Map<string, ReactNode>
): AppMenuItem[] =>
  items.map((item) => {
    const key = item.path ?? item.name;
    return {
      ...item,
      icon: key ? iconMap.get(key) : undefined,
      children: item.children ? hydrateMenuIcons(item.children, iconMap) : undefined
    };
  });

const filterMenuItems = (
  items: AppMenuItem[],
  permissions: Permission[],
  featureFlags: FeatureFlags
): AppMenuItem[] => {
  return items
    .map((item) => {
      if (item.feature && !isFeatureEnabled(featureFlags, item.feature)) {
        return null;
      }
      if (item.permission && !hasPermission(permissions, item.permission)) {
        return null;
      }
      const children = item.children
        ? filterMenuItems(item.children as AppMenuItem[], permissions, featureFlags)
        : undefined;
      if (!item.path && (!children || children.length === 0)) {
        return null;
      }
      return {
        ...item,
        children
      } as AppMenuItem;
    })
    .filter((item): item is AppMenuItem => Boolean(item));
};

const buildDropdownMenuItems = (
  items: AppMenuItem[],
  parentKey = 'menu'
): { items: MenuProps['items']; keyToPath: Map<string, string> } => {
  const keyToPath = new Map<string, string>();
  const menuItems = items.map((item, index) => {
    const children = (item.children as AppMenuItem[] | undefined) ?? [];
    const hasChildren = children.length > 0;
    const key = item.path ?? `${parentKey}-${item.name ?? 'group'}-${index}`;
    if (hasChildren) {
      const childResult = buildDropdownMenuItems(children, key);
      childResult.keyToPath.forEach((value, childKey) => {
        keyToPath.set(childKey, value);
      });
      return {
        key,
        label: item.name ?? key,
        icon: item.icon,
        children: childResult.items
      };
    }
    if (item.path) {
      keyToPath.set(key, item.path);
    }
    return {
      key,
      label: item.name ?? item.path ?? key,
      icon: item.icon
    };
  });
  return { items: menuItems, keyToPath };
};

export const AppLayout = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const { tenantId, shopIds, warehouseId, setTenantId, setShopIds, setWarehouseId } = useTenant();
  const userName = useSessionStore((state) => state.userName);
  const userId = useSessionStore((state) => state.userId);
  const permissions = useSessionStore((state) => state.permissions);
  const featureFlags = useSessionStore((state) => state.featureFlags);
  const resetSession = useSessionStore((state) => state.resetSession);
  const readOnlyFlag = useFeatureFlag('system.read_only');
  const menuCacheEnabled = useMenuCacheSettingsStore((state) => state.enabled);
  const { tabs, activeKey, refreshTokens, addOrUpdateTab, closeTab, closeOtherTabs, closeAllTabs } =
    useTabsStore();
  const setTabs = useTabsStore((state) => state.setTabs);
  const setActiveKey = useTabsStore((state) => state.setActiveKey);
  const refreshTab = useTabsStore((state) => state.refreshTab);

  const iconMap = useMemo(() => buildIconMap(appMenuItems), []);
  const menuCacheKey = useMemo(() => {
    if (!tenantId || !userId) {
      return `${menuCacheKeyPrefix}.guest`;
    }
    const signature = buildMenuCacheSignature(permissions, featureFlags);
    return `${menuCacheKeyPrefix}.${tenantId}.${userId}.${signature}`;
  }, [featureFlags, permissions, tenantId, userId]);
  const [cachedMenu, setCachedMenu] = useState<CachedMenuItem[] | null>(null);

  useEffect(() => {
    if (!menuCacheEnabled) {
      setCachedMenu(null);
      return;
    }
    const cached = loadMenuCache(menuCacheKey);
    setCachedMenu(cached);
  }, [menuCacheEnabled, menuCacheKey]);

  const freshMenuRoutes = useMemo(() => {
    if (menuCacheEnabled && cachedMenu) {
      return null;
    }
    return filterMenuItems(appMenuItems, permissions, featureFlags);
  }, [cachedMenu, featureFlags, menuCacheEnabled, permissions]);

  const menuRoutes = useMemo(() => {
    if (menuCacheEnabled && cachedMenu) {
      return hydrateMenuIcons(cachedMenu, iconMap);
    }
    return freshMenuRoutes ?? filterMenuItems(appMenuItems, permissions, featureFlags);
  }, [cachedMenu, featureFlags, iconMap, menuCacheEnabled, permissions, freshMenuRoutes]);

  useEffect(() => {
    if (!menuCacheEnabled || !freshMenuRoutes) {
      return;
    }
    const stripped = stripMenuIcons(freshMenuRoutes);
    saveMenuCache(menuCacheKey, stripped);
    setCachedMenu(stripped);
  }, [freshMenuRoutes, menuCacheEnabled, menuCacheKey]);
  const menuLeafItems = useMemo(() => flattenMenuItems(menuRoutes), [menuRoutes]);
  const menuTitleMap = useMemo(() => {
    return new Map(menuLeafItems.map((item) => [item.path, item.name]));
  }, [menuLeafItems]);
  const dropdownMenu = useMemo(() => buildDropdownMenuItems(menuRoutes), [menuRoutes]);
  const mainMenuItems = dropdownMenu.items;
  const selectedMenuKeys = useMemo(() => {
    if (dropdownMenu.keyToPath.has(location.pathname)) {
      return [location.pathname];
    }
    return [];
  }, [dropdownMenu.keyToPath, location.pathname]);

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'header'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId)
  });

  const warehouses = warehouseQuery.data?.items ?? [];
  const defaultWarehouse =
    warehouses.find((warehouse) => warehouse.is_default) ?? warehouses[0] ?? null;
  const warehouseOptions = useMemo(() => {
    if (!defaultWarehouse) {
      return [];
    }
    return [{ label: defaultWarehouse.name, value: defaultWarehouse.warehouse_id }];
  }, [defaultWarehouse]);

  useEffect(() => {
    if (!tenantId || !defaultWarehouse) {
      return;
    }
    if (warehouseId !== defaultWarehouse.warehouse_id) {
      setWarehouseId(defaultWarehouse.warehouse_id);
    }
  }, [defaultWarehouse, tenantId, warehouseId, setWarehouseId]);

  const userMenuItems: MenuProps['items'] = [
    { key: 'profile', label: '个人信息' },
    { key: 'logout', label: '退出登录' }
  ];

  const handleUserMenuClick: MenuProps['onClick'] = ({ key }) => {
    if (key === 'profile') {
      navigate('/system/profile');
      return;
    }
    if (key === 'logout') {
      resetSession();
    }
  };

  const tabsStorageKey = useMemo(() => {
    if (!tenantId || !userId) {
      return '';
    }
    return `erp.tabs.${tenantId}.${userId}`;
  }, [tenantId, userId]);
  const hydratedRef = useRef(false);

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

  useEffect(() => {
    const title = resolveRouteTitle(location.pathname, menuTitleMap);
    const tabKey = location.pathname;
    addOrUpdateTab({
      key: tabKey,
      path: `${location.pathname}${location.search}`,
      title,
      closable: tabKey !== '/orders'
    });
  }, [addOrUpdateTab, location.pathname, location.search, menuTitleMap]);

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
        navigate('/orders');
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
    navigate('/orders');
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

  const handleMainMenuClick: MenuProps['onClick'] = ({ key }) => {
    const targetPath = dropdownMenu.keyToPath.get(String(key));
    if (targetPath) {
      navigate(targetPath);
    }
  };

  const handleTopMenuClick: MenuProps['onClick'] = ({ key }) => {
    const targetPath = dropdownMenu.keyToPath.get(String(key));
    if (targetPath) {
      navigate(targetPath);
    }
  };

  const outletKey = `${activeKey ?? location.pathname}-${
    refreshTokens[activeKey ?? location.pathname] ?? 0
  }`;

  return (
    <Layout className="app-layout">
      <Header className="app-layout-header">
        <div className="app-layout-header-top">
          <div className="app-layout-logo">
            <img
              src={brandLogo}
              alt="衡策云"
              style={{ width: 28, height: 28, borderRadius: 6, objectFit: 'cover' }}
            />
            <span>衡策云</span>
          </div>
          <Menu
            mode="horizontal"
            className="app-layout-menu"
            items={mainMenuItems}
            selectedKeys={selectedMenuKeys}
            onClick={handleMainMenuClick}
          />
          <div className="app-layout-header-actions">
            <Space size={16}>
              <Dropdown
                menu={{ items: dropdownMenu.items, onClick: handleTopMenuClick }}
                placement="bottomRight"
                trigger={['click']}
              >
                <Button icon={<MenuOutlined />}>功能菜单</Button>
              </Dropdown>
              <Input
                allowClear
                prefix={<SearchOutlined />}
                placeholder="搜索订单/包裹/SKU"
                style={{ width: 240 }}
              />
              <Dropdown menu={{ items: userMenuItems, onClick: handleUserMenuClick }}>
                <Space style={{ cursor: 'pointer' }}>
                  <Avatar>{userName.slice(0, 1).toUpperCase()}</Avatar>
                  <span>{userName}</span>
                  <DownOutlined />
                </Space>
              </Dropdown>
            </Space>
          </div>
        </div>
        <div className="app-layout-header-toolbar">
          <Space size={12} wrap>
            <Select
              value={tenantId || undefined}
              style={{ minWidth: 200 }}
              placeholder="选择商家"
              options={tenantOptions}
              onChange={(value) => setTenantId(value)}
              data-testid="header-tenant-select"
            />
            <Select
              mode="multiple"
              allowClear
              value={shopIds.length ? shopIds : undefined}
              style={{ minWidth: 200 }}
              placeholder="全部店铺"
              options={shopOptions}
              onChange={(value) => setShopIds(value)}
              maxTagCount="responsive"
              data-testid="header-shop-select"
            />
            <Select
              value={warehouseId}
              style={{ minWidth: 160 }}
              placeholder="默认仓库"
              options={warehouseOptions}
              onChange={(value) => setWarehouseId(value)}
              data-testid="header-warehouse-select"
              disabled
            />
          </Space>
        </div>
      </Header>
      <Content className="app-layout-content">
        <ShortcutToolbar menuItems={menuRoutes} />
        <div className="app-layout-tabs">
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
        {readOnlyFlag.enabled && (
          <Alert
            banner
            showIcon
            type="warning"
            message="当前为只读模式，写操作已暂停"
            style={{ marginTop: 12 }}
          />
        )}
        <div className="app-layout-body">
          <ErrorBoundary>
            <Outlet key={outletKey} />
          </ErrorBoundary>
        </div>
      </Content>
    </Layout>
  );
};
