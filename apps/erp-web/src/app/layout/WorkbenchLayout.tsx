import { Alert, Button, Collapse, Empty, Tabs, Tooltip } from 'antd';
import type { MenuProps } from 'antd';
import {
  CaretDownOutlined,
  CaretLeftOutlined,
  CaretRightOutlined,
  CaretUpOutlined
} from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';

import { ErrorBoundary } from '@/components/ErrorBoundary';
import { listWarehouses } from '@/api/warehouses';
import type { FeatureFlags } from '@/features/featureFlags';
import { useFeatureFlag } from '@/features/featureFlags';
import type { Permission } from '@/features/rbac';
import { hasPermission } from '@/features/rbac';
import { isFeatureEnabled } from '@/features/featureFlags';
import { useSessionStore } from '@/features/sessionStore';
import { useTenant } from '@/features/tenant';
import type { AppMenuItem } from '@/app/layout/menu';
import { appMenuItems, flattenMenuItems } from '@/app/layout/menu';
import { ShortcutToolbar } from '@/app/layout/ShortcutToolbar';
import { RouteTabs } from '@/app/layout/RouteTabs';
import { useTabsStore } from '@/app/layout/tabsStore';
import { WorkbenchPanelsProvider } from '@/app/layout/workbenchPanels';
import { ModuleHeader } from '@/app/layout/ModuleHeader';
import {
  WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
  WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT,
  WORKBENCH_DEFAULT_LEFT_PANEL_WIDTH,
  WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH,
  WORKBENCH_LEFT_PANEL_MAX_WIDTH,
  WORKBENCH_LEFT_PANEL_MIN_WIDTH,
  WORKBENCH_MAIN_TOP_MIN_HEIGHT,
  WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
  WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
  clampWorkbenchPanelSize,
} from '@/app/layout/workbenchPanelSizing';
import { useUiSettingsStore } from '@/features/uiSettings';
import brandLogo from '@/assets/brand/hengceyun-logo.png';

import './workbenchLayout.css';

const collapsedWidth = 44;
const collapsedBottomHeight = 0;
const headerModuleNames = new Set([
  '系统',
  '目录',
  '商品',
  '货品',
  '采购',
  '销售',
  '库存',
  '仓储',
  '客户',
  '服务',
  '统计'
]);
const headerModuleLabelMap = new Map([['仓库', '仓储']]);

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

const collectLeafPaths = (item: AppMenuItem): string[] => {
  if (item.path && (!item.children || item.children.length === 0)) {
    return [item.path];
  }
  const children = (item.children as AppMenuItem[] | undefined) ?? [];
  return children.flatMap((child) => collectLeafPaths(child));
};

const resolveHeaderModules = (menuRoutes: AppMenuItem[]): AppMenuItem[] => {
  const matchedByName = menuRoutes.filter(
    (item) => typeof item.name === 'string' && headerModuleNames.has(item.name)
  );
  if (matchedByName.length > 0) {
    return matchedByName;
  }
  return menuRoutes.filter((item) => {
    const children = (item.children as AppMenuItem[] | undefined) ?? [];
    if (children.length === 0) {
      return false;
    }
    return collectLeafPaths(item).length > 0;
  });
};

const isPathActive = (current: string, target: string) => {
  if (current === target) {
    return true;
  }
  return current.startsWith(`${target}/`);
};

export const WorkbenchLayout = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const { tenantId, warehouseId, setWarehouseId } = useTenant();
  const userName = useSessionStore((state) => state.userName);
  const userId = useSessionStore((state) => state.userId);
  const permissions = useSessionStore((state) => state.permissions);
  const featureFlags = useSessionStore((state) => state.featureFlags);
  const resetSession = useSessionStore((state) => state.resetSession);
  const readOnlyFlag = useFeatureFlag('system.read_only');
  const activeKey = useTabsStore((state) => state.activeKey);
  const refreshTokens = useTabsStore((state) => state.refreshTokens);
  const denseMode = useUiSettingsStore((state) => state.denseMode);
  const toggleDenseMode = useUiSettingsStore((state) => state.toggleDenseMode);

  const menuRoutes = useMemo(
    () => filterMenuItems(appMenuItems, permissions, featureFlags),
    [featureFlags, permissions]
  );
  const menuLeafItems = useMemo(() => flattenMenuItems(menuRoutes), [menuRoutes]);
  const menuTitleMap = useMemo(() => {
    return new Map(menuLeafItems.map((item) => [item.path, item.name]));
  }, [menuLeafItems]);

  const headerModules = useMemo(() => resolveHeaderModules(menuRoutes), [menuRoutes]);

  const moduleItems = useMemo(() => {
    return headerModules.map((item, index) => {
      const leafPaths = collectLeafPaths(item);
      const baseLabel =
        typeof item.name === 'string' ? item.name : item.path ?? `模块${index + 1}`;
      const label = headerModuleLabelMap.get(baseLabel) ?? baseLabel;
      const dropdown = buildDropdownMenuItems(
        (item.children as AppMenuItem[] | undefined) ?? [],
        `module-${index}`
      );
      return {
        key: baseLabel,
        label,
        defaultPath: leafPaths[0],
        leafPaths,
        menuItems: dropdown.items,
        menuKeyToPath: dropdown.keyToPath
      };
    });
  }, [headerModules]);

  const activeModuleKey = useMemo(() => {
    const current = location.pathname;
    const match = moduleItems.find((module) =>
      module.leafPaths.some((path) => isPathActive(current, path))
    );
    return match?.key;
  }, [location.pathname, moduleItems]);

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'header'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId)
  });

  const warehouses = warehouseQuery.data?.items ?? [];
  const defaultWarehouse =
    warehouses.find((warehouse) => warehouse.is_default) ?? warehouses[0] ?? null;

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

  const layoutStorageKey = useMemo(() => {
    if (!tenantId || !userId) {
      return '';
    }
    return `erp.ui.workbench.layout.${tenantId}.${userId}`;
  }, [tenantId, userId]);

  const [leftWidth, setLeftWidth] = useState(WORKBENCH_DEFAULT_LEFT_PANEL_WIDTH);
  const [rightWidth, setRightWidth] = useState(WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH);
  const [bottomHeight, setBottomHeight] = useState(WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT);
  const [leftCollapsed, setLeftCollapsed] = useState(false);
  const [rightCollapsed, setRightCollapsed] = useState(false);
  const [bottomCollapsed, setBottomCollapsed] = useState(false);
  const [leftPanelVisible, setLeftPanelVisible] = useState(false);
  const [rightPanelVisible, setRightPanelVisible] = useState(false);
  const [bottomPanelVisible, setBottomPanelVisible] = useState(false);
  const [leftPanel, setLeftPanel] = useState<ReactNode>();
  const [rightPanel, setRightPanel] = useState<ReactNode>();
  const [bottomPanel, setBottomPanel] = useState<ReactNode>();
  const layoutHydratedRef = useRef(false);
  const headerRef = useRef<HTMLElement | null>(null);
  const mainSplitRef = useRef<HTMLDivElement | null>(null);
  const resizeStateRef = useRef<{
    side: 'left' | 'right' | 'bottom';
    startX: number;
    startY: number;
    startWidth: number;
    startHeight: number;
  } | null>(null);

  const clearPanels = useCallback(() => {
    setLeftPanel(undefined);
    setRightPanel(undefined);
    setBottomPanel(undefined);
    setLeftPanelVisible(false);
    setRightPanelVisible(false);
    setBottomPanelVisible(false);
  }, []);

  const handleLeftPanelWidthChange = useCallback(
    (width: number) =>
      setLeftWidth(
        clampWorkbenchPanelSize(width, WORKBENCH_LEFT_PANEL_MIN_WIDTH, WORKBENCH_LEFT_PANEL_MAX_WIDTH),
      ),
    [],
  );

  const handleRightPanelWidthChange = useCallback(
    (width: number) =>
      setRightWidth(
        clampWorkbenchPanelSize(width, WORKBENCH_RIGHT_PANEL_MIN_WIDTH, WORKBENCH_RIGHT_PANEL_MAX_WIDTH),
      ),
    [],
  );

  const handleBottomPanelHeightChange = useCallback(
    (height: number) => {
      const containerHeight = mainSplitRef.current?.clientHeight ?? 0;
      const maxBottom =
        containerHeight > 0
          ? Math.max(WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT, containerHeight - WORKBENCH_MAIN_TOP_MIN_HEIGHT)
          : Number.MAX_SAFE_INTEGER;
      setBottomHeight(
        clampWorkbenchPanelSize(height, WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT, maxBottom),
      );
    },
    [],
  );

  useEffect(() => {
    if (!layoutStorageKey) {
      layoutHydratedRef.current = true;
      return;
    }
    const raw = localStorage.getItem(layoutStorageKey);
    if (!raw) {
      layoutHydratedRef.current = true;
      return;
    }
    try {
      const parsed = JSON.parse(raw) as {
        leftWidth?: number;
        rightWidth?: number;
        bottomHeight?: number;
        leftCollapsed?: boolean;
        rightCollapsed?: boolean;
        bottomCollapsed?: boolean;
        leftPanelVisible?: boolean;
        rightPanelVisible?: boolean;
        bottomPanelVisible?: boolean;
      };
      if (parsed.leftWidth) {
        setLeftWidth(parsed.leftWidth);
      }
      if (parsed.rightWidth) {
        setRightWidth(parsed.rightWidth);
      }
      if (parsed.bottomHeight) {
        setBottomHeight(parsed.bottomHeight);
      }
      if (typeof parsed.leftCollapsed === 'boolean') {
        setLeftCollapsed(parsed.leftCollapsed);
      }
      if (typeof parsed.rightCollapsed === 'boolean') {
        setRightCollapsed(parsed.rightCollapsed);
      }
      if (typeof parsed.bottomCollapsed === 'boolean') {
        setBottomCollapsed(parsed.bottomCollapsed);
      }
      if (typeof parsed.leftPanelVisible === 'boolean') {
        setLeftPanelVisible(parsed.leftPanelVisible);
      }
      if (typeof parsed.rightPanelVisible === 'boolean') {
        setRightPanelVisible(parsed.rightPanelVisible);
      }
      if (typeof parsed.bottomPanelVisible === 'boolean') {
        setBottomPanelVisible(parsed.bottomPanelVisible);
      }
    } catch {
      // ignore malformed cache
    }
    layoutHydratedRef.current = true;
  }, [layoutStorageKey]);

  useEffect(() => {
    if (!layoutStorageKey || !layoutHydratedRef.current) {
      return;
    }
    localStorage.setItem(
      layoutStorageKey,
      JSON.stringify({
        leftWidth,
        rightWidth,
        bottomHeight,
        leftCollapsed,
        rightCollapsed,
        bottomCollapsed,
        leftPanelVisible,
        rightPanelVisible,
        bottomPanelVisible
      })
    );
  }, [
    bottomHeight,
    bottomCollapsed,
    layoutStorageKey,
    leftCollapsed,
    leftPanelVisible,
    leftWidth,
    rightCollapsed,
    rightPanelVisible,
    rightWidth,
    bottomPanelVisible
  ]);

  useLayoutEffect(() => {
    clearPanels();
  }, [location.pathname]);

  useEffect(() => {
    const header = headerRef.current;
    if (!header) {
      return;
    }
    const root = document.documentElement;
    document.body.classList.add('erp-modal-offset');
    const updateHeaderHeight = () => {
      const height = header.getBoundingClientRect().height;
      root.style.setProperty('--erp-top-bar-height', `${height}px`);
    };
    updateHeaderHeight();
    const observer = new ResizeObserver(updateHeaderHeight);
    observer.observe(header);
    return () => {
      observer.disconnect();
      document.body.classList.remove('erp-modal-offset');
      root.style.setProperty('--erp-top-bar-height', '0px');
    };
  }, []);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      const resizeState = resizeStateRef.current;
      if (!resizeState) {
        return;
      }
      if (resizeState.side === 'bottom') {
        const delta = event.clientY - resizeState.startY;
        const containerHeight = mainSplitRef.current?.clientHeight ?? 0;
        const maxBottom = Math.max(
          WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
          containerHeight - WORKBENCH_MAIN_TOP_MIN_HEIGHT,
        );
        setBottomHeight(
          clampWorkbenchPanelSize(
            resizeState.startHeight - delta,
            WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
            maxBottom,
          )
        );
        return;
      }
      const delta = event.clientX - resizeState.startX;
      if (resizeState.side === 'left') {
        setLeftWidth(
          clampWorkbenchPanelSize(
            resizeState.startWidth + delta,
            WORKBENCH_LEFT_PANEL_MIN_WIDTH,
            WORKBENCH_LEFT_PANEL_MAX_WIDTH,
          ),
        );
        return;
      }
      setRightWidth(
        clampWorkbenchPanelSize(
          resizeState.startWidth - delta,
          WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
          WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
        ),
      );
    };

    const handlePointerUp = () => {
      if (!resizeStateRef.current) {
        return;
      }
      resizeStateRef.current = null;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    return () => {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };
  }, []);

  useEffect(() => {
    if (!mainSplitRef.current) {
      return;
    }
    const observer = new ResizeObserver(() => {
      const containerHeight = mainSplitRef.current?.clientHeight ?? 0;
      if (!containerHeight) {
        return;
      }
      const maxBottom = Math.max(
        WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
        containerHeight - WORKBENCH_MAIN_TOP_MIN_HEIGHT,
      );
      setBottomHeight((prev) =>
        clampWorkbenchPanelSize(prev, WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT, maxBottom),
      );
    });
    observer.observe(mainSplitRef.current);
    return () => observer.disconnect();
  }, []);

  const handleResizeStart =
    (side: 'left' | 'right' | 'bottom') => (event: React.PointerEvent) => {
      if (side === 'bottom' && bottomCollapsed) {
        setBottomCollapsed(false);
      }
      resizeStateRef.current = {
        side,
        startX: event.clientX,
        startY: event.clientY,
        startWidth: side === 'left' ? leftWidth : rightWidth,
        startHeight: bottomHeight
      };
      if (side === 'left' && leftCollapsed) {
        setLeftCollapsed(false);
      }
      if (side === 'right' && rightCollapsed) {
        setRightCollapsed(false);
      }
      document.body.style.cursor = side === 'bottom' ? 'row-resize' : 'col-resize';
      document.body.style.userSelect = 'none';
    };

  const leftPanelWidth = leftCollapsed ? collapsedWidth : leftWidth;
  const rightPanelWidth = rightCollapsed ? collapsedWidth : rightWidth;
  const effectiveBottomHeight = bottomCollapsed ? collapsedBottomHeight : bottomHeight;
  const outletKey = `${activeKey ?? location.pathname}-${
    refreshTokens[activeKey ?? location.pathname] ?? 0
  }`;

  const defaultLeftPanel = (
    <Tabs
      size="small"
      className="workbench-left-tabs"
      items={[
        {
          key: 'filters',
          label: '默认过滤',
          children: (
            <div className="workbench-left-placeholder">
              <Empty description="暂无默认过滤" />
            </div>
          )
        },
        {
          key: 'query',
          label: '表格查询',
          children: (
            <div className="workbench-left-placeholder">
              <Empty description="暂无表格查询条件" />
            </div>
          )
        }
      ]}
    />
  );

  const defaultRightPanel = (
    <Collapse
      size="small"
      defaultActiveKey={['tags', 'common', 'detail']}
      items={[
        {
          key: 'tags',
          label: '标记',
          children: (
            <div className="workbench-empty">
              <Empty description="暂无标记" />
            </div>
          )
        },
        {
          key: 'common',
          label: '常用功能',
          children: (
            <div className="workbench-empty">
              <Empty description="暂无快捷功能" />
            </div>
          )
        },
        {
          key: 'detail',
          label: '详情',
          children: (
            <div className="workbench-empty">
              <Empty description="暂无详情内容" />
            </div>
          )
        }
      ]}
    />
  );

  const defaultBottomPanel = (
    <Tabs
      size="small"
      type="card"
      items={[
        { key: 'sub-1', label: '子表一', children: <Empty description="暂无子表数据" /> },
        { key: 'sub-2', label: '子表二', children: <Empty description="暂无子表数据" /> }
      ]}
    />
  );

  return (
    <WorkbenchPanelsProvider
      value={{
        leftPanel,
        rightPanel,
        bottomPanel,
        leftPanelWidth: leftWidth,
        rightPanelWidth: rightWidth,
        bottomPanelHeight: bottomHeight,
        leftPanelVisible,
        rightPanelVisible,
        bottomPanelVisible,
        setLeftPanel,
        setRightPanel,
        setBottomPanel,
        setLeftPanelWidth: handleLeftPanelWidthChange,
        setRightPanelWidth: handleRightPanelWidthChange,
        setBottomPanelHeight: handleBottomPanelHeightChange,
        setLeftPanelCollapsed: setLeftCollapsed,
        setLeftPanelVisible,
        setRightPanelVisible,
        setBottomPanelVisible,
        clearPanels
      }}
    >
      <div className="workbench-shell">
        <header className="workbench-header" ref={headerRef}>
          <ModuleHeader
            logo={
              <>
                <img
                  src={brandLogo}
                  alt="衡策云"
                  style={{ width: 28, height: 28, borderRadius: 6, objectFit: 'cover' }}
                />
                衡策云工作台
              </>
            }
            modules={moduleItems}
            activeModuleKey={activeModuleKey}
            onModuleClick={(path) => navigate(path)}
            userName={userName}
            userMenuItems={userMenuItems}
            onUserMenuClick={handleUserMenuClick}
            onLogout={resetSession}
            denseMode={denseMode}
            onToggleDense={toggleDenseMode}
          />
          <div className="workbench-main-toolbar">
            <ShortcutToolbar menuItems={menuRoutes} />
          </div>
          <div className="workbench-header-row workbench-tabs-row">
            <RouteTabs menuTitleMap={menuTitleMap} />
          </div>
        </header>
        <div className="workbench-body">
          {leftPanelVisible ? (
            <>
              <aside
                className="workbench-panel workbench-panel-left"
                style={{ width: leftPanelWidth }}
              >
                {leftCollapsed ? (
                  <div className="workbench-panel-collapsed" />
                ) : (
                  <>
                    <div className="workbench-panel-body">{leftPanel ?? defaultLeftPanel}</div>
                  </>
                )}
              </aside>
              <div className="workbench-resizer" onPointerDown={handleResizeStart('left')}>
                <Tooltip title={leftCollapsed ? '展开左侧面板' : '收起左侧面板'}>
                  <Button
                    size="small"
                    type="text"
                    className="workbench-resizer-toggle"
                    icon={leftCollapsed ? <CaretRightOutlined /> : <CaretLeftOutlined />}
                    onPointerDown={(event) => event.stopPropagation()}
                    onClick={() => setLeftCollapsed((prev) => !prev)}
                  />
                </Tooltip>
              </div>
            </>
          ) : null}
          <main className="workbench-main">
            {readOnlyFlag.enabled && (
              <Alert
                banner
                showIcon
                type="warning"
                message="当前为只读模式，写操作已暂停"
                style={{ margin: '12px 16px 0' }}
              />
            )}
            <div className="workbench-main-content">
              <div className="workbench-main-split" ref={mainSplitRef}>
                <section className="workbench-main-top">
                  <ErrorBoundary>
                    <Outlet key={outletKey} />
                  </ErrorBoundary>
                </section>
                {bottomPanelVisible ? (
                  <>
                    <div
                      className="workbench-resizer-horizontal"
                      onPointerDown={handleResizeStart('bottom')}
                    >
                      <Tooltip title={bottomCollapsed ? '展开底部子表' : '收起底部子表'}>
                        <Button
                          size="small"
                          type="text"
                          className="workbench-resizer-horizontal-toggle"
                          icon={bottomCollapsed ? <CaretUpOutlined /> : <CaretDownOutlined />}
                          onPointerDown={(event) => event.stopPropagation()}
                          onClick={(event) => {
                            event.stopPropagation();
                            setBottomCollapsed((prev) => !prev);
                          }}
                        />
                      </Tooltip>
                    </div>
                    <section
                      className={`workbench-main-bottom${
                        bottomCollapsed ? ' is-collapsed' : ''
                      }`}
                      style={{ height: effectiveBottomHeight }}
                    >
                      {bottomPanel ?? defaultBottomPanel}
                    </section>
                  </>
                ) : null}
              </div>
            </div>
          </main>
          {rightPanelVisible ? (
            <>
              <div className="workbench-resizer" onPointerDown={handleResizeStart('right')}>
                <Tooltip title={rightCollapsed ? '展开详情面板' : '收起详情面板'}>
                  <Button
                    size="small"
                    type="text"
                    className="workbench-resizer-toggle"
                    icon={rightCollapsed ? <CaretLeftOutlined /> : <CaretRightOutlined />}
                    onPointerDown={(event) => event.stopPropagation()}
                    onClick={() => setRightCollapsed((prev) => !prev)}
                  />
                </Tooltip>
              </div>
              <aside
                className="workbench-panel workbench-panel-right"
                style={{ width: rightPanelWidth }}
              >
                {rightCollapsed ? (
                  <div className="workbench-panel-collapsed" />
                ) : (
                  <>
                    <div className="workbench-panel-body">
                      {rightPanel ?? defaultRightPanel}
                    </div>
                  </>
                )}
              </aside>
            </>
          ) : null}
        </div>
      </div>
    </WorkbenchPanelsProvider>
  );
};
