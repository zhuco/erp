import { Alert, Button, Empty, Tooltip } from 'antd';
import type { MenuProps } from 'antd';
import {
  CaretDownOutlined,
  CaretLeftOutlined,
  CaretRightOutlined,
  CaretUpOutlined,
} from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';

import { getUserShortcuts } from '@/api/shortcuts';
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
import {
  buildFunctionRecommendations,
  buildFunctionSearchEntries,
  groupFunctionEntriesByModule,
  groupFunctionRecommendationsBySource,
  mergeRecentFunctionPaths,
  searchFunctionEntries,
  type FunctionRecommendation,
  type FunctionSearchEntry,
} from '@/app/layout/moduleSearch';
import { ShortcutToolbar } from '@/app/layout/ShortcutToolbar';
import { buildWorkbenchWarmupPaths } from '@/app/layout/workbenchWarmup';
import { RouteTabs } from '@/app/layout/RouteTabs';
import { useTabsStore } from '@/app/layout/tabsStore';
import {
  WorkbenchPanelsProvider,
  type WorkbenchPanelsContextValue,
} from '@/app/layout/workbenchPanels';
import type {
  FunctionSearchOption,
  ModuleGroupItem,
  ModuleLinkItem,
} from '@/app/layout/ModuleHeader';
import { ModuleHeader } from '@/app/layout/ModuleHeader';
import { useRouteLoadingSnapshot } from '@/app/navigation/routeLoadingDiagnostics';
import { RouteDiagnosticsReporter } from '@/app/navigation/RouteDiagnosticsReporter';
import { scheduleRoutePrefetchPaths } from '@/app/navigation/routePrefetch';
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

import './workbenchLayout.css';

const collapsedWidth = 44;
const collapsedBottomHeight = 0;

const filterMenuItems = (
  items: AppMenuItem[],
  permissions: Permission[],
  featureFlags: FeatureFlags,
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
        children,
      } as AppMenuItem;
    })
    .filter((item): item is AppMenuItem => Boolean(item));
};

const collectLeafPaths = (item: AppMenuItem): string[] => {
  if (item.path && (!item.children || item.children.length === 0)) {
    return [item.path];
  }
  const children = (item.children as AppMenuItem[] | undefined) ?? [];
  return children.flatMap((child) => collectLeafPaths(child));
};

const toPathTestId = (path: string, prefix: string) => {
  const normalized = path
    .replace(/^\/+/, '')
    .replace(/[^a-z0-9]+/gi, '-')
    .replace(/^-+|-+$/g, '');
  return `${prefix}-${normalized || 'home'}`;
};

const resolveFunctionSearchSourceLabel = (source: FunctionRecommendation['source']) =>
  source === 'recent' ? '\u6700\u8fd1\u4f7f\u7528' : '\u5e38\u7528\u529f\u80fd';

const buildFunctionSearchGroupLabel = (label: string, count: number) => (
  <div className="workbench-function-search-group-label">
    <span className="workbench-function-search-group-title">{label}</span>
    <span className="workbench-function-search-group-count">{count}</span>
  </div>
);

const buildFunctionSearchOption = (
  entry: FunctionSearchEntry | FunctionRecommendation,
): FunctionSearchOption => {
  const source = 'source' in entry ? entry.source : undefined;

  return {
    value: entry.path,
    label: (
      <div
        className="workbench-function-search-option"
        data-testid={toPathTestId(entry.path, 'function-search-option')}
      >
        <div className="workbench-function-search-title">
          <span>{entry.name}</span>
          {source ? (
            <span
              className={`workbench-function-search-badge is-${source}`}
              data-testid={toPathTestId(entry.path, 'function-search-source')}
            >
              {resolveFunctionSearchSourceLabel(source)}
            </span>
          ) : null}
        </div>
        <div className="workbench-function-search-meta">
          <span>
            {entry.moduleName}
            {entry.groupName ? ` / ${entry.groupName}` : ''}
          </span>
          <span>{entry.path}</span>
        </div>
      </div>
    ),
  };
};

const buildHeaderLinks = (items: AppMenuItem[], groupKey: string): ModuleLinkItem[] => {
  return flattenMenuItems(items).map((item, index) => ({
    key: `${groupKey}-${index}-${item.path}`,
    label: item.name,
    path: item.path,
    testId: toPathTestId(item.path, 'module-link'),
  }));
};

const buildHeaderGroups = (items: AppMenuItem[], moduleKey: string): ModuleGroupItem[] => {
  const directLinks: AppMenuItem[] = [];
  const groups: ModuleGroupItem[] = [];

  items.forEach((item, index) => {
    const children = (item.children as AppMenuItem[] | undefined) ?? [];
    if (!item.path && children.length > 0) {
      const links = buildHeaderLinks(children, `${moduleKey}-group-${index}`);
      if (links.length > 0) {
        groups.push({
          key: `${moduleKey}-group-${index}`,
          label: item.name ?? `分组${index + 1}`,
          items: links,
        });
      }
      return;
    }

    if (item.path) {
      directLinks.push(item);
    }
  });

  if (directLinks.length > 0) {
    groups.unshift({
      key: `${moduleKey}-common`,
      label: '\u5e38\u7528\u529f\u80fd',
      items: buildHeaderLinks(directLinks, `${moduleKey}-common`),
    });
  }

  return groups;
};

const resolveHeaderModules = (menuRoutes: AppMenuItem[]): AppMenuItem[] => {
  return menuRoutes.filter((item) => {
    if (item.path) {
      return true;
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

type WorkbenchRoutePanelsState = {
  leftPanel?: ReactNode;
  rightPanel?: ReactNode;
  bottomPanel?: ReactNode;
  leftPanelVisible: boolean;
  rightPanelVisible: boolean;
  bottomPanelVisible: boolean;
};

type WorkbenchRoutePanelActions = Pick<
  WorkbenchPanelsContextValue,
  | 'setLeftPanel'
  | 'setRightPanel'
  | 'setBottomPanel'
  | 'setLeftPanelWidth'
  | 'setRightPanelWidth'
  | 'setBottomPanelHeight'
  | 'setLeftPanelCollapsed'
  | 'setLeftPanelVisible'
  | 'setRightPanelVisible'
  | 'setBottomPanelVisible'
  | 'clearPanels'
>;

const EMPTY_WORKBENCH_ROUTE_PANELS: WorkbenchRoutePanelsState = {
  leftPanel: undefined,
  rightPanel: undefined,
  bottomPanel: undefined,
  leftPanelVisible: false,
  rightPanelVisible: false,
  bottomPanelVisible: false,
};

const areWorkbenchRoutePanelsEqual = (
  left: WorkbenchRoutePanelsState,
  right: WorkbenchRoutePanelsState,
) =>
  left.leftPanel === right.leftPanel &&
  left.rightPanel === right.rightPanel &&
  left.bottomPanel === right.bottomPanel &&
  left.leftPanelVisible === right.leftPanelVisible &&
  left.rightPanelVisible === right.rightPanelVisible &&
  left.bottomPanelVisible === right.bottomPanelVisible;

type WorkbenchPlaceholderTabItem = {
  key: string;
  label: string;
  content: ReactNode;
};

type WorkbenchPlaceholderSectionItem = {
  key: string;
  label: string;
  content: ReactNode;
};

const WorkbenchPlaceholderTabs = ({
  items,
  activeKey,
  onChange,
}: {
  items: WorkbenchPlaceholderTabItem[];
  activeKey: string;
  onChange: (key: string) => void;
}) => (
  <div className="workbench-placeholder-tabs">
    <div className="workbench-placeholder-tablist" role="tablist">
      {items.map((item) => {
        const active = item.key === activeKey;
        return (
          <button
            key={item.key}
            type="button"
            role="tab"
            aria-selected={active}
            className={`workbench-placeholder-tab${active ? ' is-active' : ''}`}
            onClick={() => onChange(item.key)}
          >
            {item.label}
          </button>
        );
      })}
    </div>
    <div className="workbench-placeholder-tabpanel">
      {items.find((item) => item.key === activeKey)?.content ?? null}
    </div>
  </div>
);

const WorkbenchPlaceholderSections = ({
  items,
  openKeys,
  onToggle,
}: {
  items: WorkbenchPlaceholderSectionItem[];
  openKeys: string[];
  onToggle: (key: string) => void;
}) => (
  <div className="workbench-placeholder-sections">
    {items.map((item) => {
      const open = openKeys.includes(item.key);
      return (
        <section className={`workbench-placeholder-section${open ? ' is-open' : ''}`} key={item.key}>
          <button
            type="button"
            className="workbench-placeholder-section-toggle"
            aria-expanded={open}
            onClick={() => onToggle(item.key)}
          >
            <span>{item.label}</span>
            <span>{open ? '-' : '+'}</span>
          </button>
          {open ? <div className="workbench-placeholder-section-body">{item.content}</div> : null}
        </section>
      );
    })}
  </div>
);

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
  const tabs = useTabsStore((state) => state.tabs);
  const refreshTokens = useTabsStore((state) => state.refreshTokens);
  const denseMode = useUiSettingsStore((state) => state.denseMode);
  const toggleDenseMode = useUiSettingsStore((state) => state.toggleDenseMode);

  const menuRoutes = useMemo(
    () => filterMenuItems(appMenuItems, permissions, featureFlags),
    [featureFlags, permissions],
  );
  const menuLeafItems = useMemo(() => flattenMenuItems(menuRoutes), [menuRoutes]);
  const menuTitleMap = useMemo(() => {
    return new Map(menuLeafItems.map((item) => [item.path, item.name]));
  }, [menuLeafItems]);

  const headerModules = useMemo(() => resolveHeaderModules(menuRoutes), [menuRoutes]);
  const [functionSearch, setFunctionSearch] = useState('');
  const [recentFunctionPaths, setRecentFunctionPaths] = useState<string[]>([]);

  const moduleItems = useMemo(() => {
    return headerModules.map((item, index) => {
      const nestedLeafPaths = collectLeafPaths(item);
      const defaultPath = item.path ?? nestedLeafPaths[0];
      const leafPaths = defaultPath
        ? [defaultPath, ...nestedLeafPaths.filter((path) => path !== defaultPath)]
        : nestedLeafPaths;
      const label = typeof item.name === 'string' ? item.name : (item.path ?? `模块${index + 1}`);
      return {
        key: item.path ?? label,
        label,
        defaultPath,
        leafPaths,
        groups: buildHeaderGroups(
          (item.children as AppMenuItem[] | undefined) ?? [],
          `module-${index}`,
        ),
      };
    });
  }, [headerModules]);

  const activeModuleKey = useMemo(() => {
    const current = location.pathname;
    const match = moduleItems.find((module) =>
      module.leafPaths.some((path) => isPathActive(current, path)),
    );
    return match?.key;
  }, [location.pathname, moduleItems]);
  const moduleIntentPrefetchCleanupRef = useRef<() => void>(() => undefined);
  const currentRouteKey = location.pathname;
  const currentRouteSnapshot = useRouteLoadingSnapshot(currentRouteKey);

  const shortcutQuery = useQuery({
    queryKey: ['shortcuts', tenantId],
    queryFn: () => getUserShortcuts(tenantId),
    enabled: Boolean(tenantId),
    retry: false,
  });

  const shortcutPaths = useMemo(
    () => shortcutQuery.data?.shortcuts.map((item) => item.path) ?? [],
    [shortcutQuery.data?.shortcuts],
  );

  const functionSearchEntries = useMemo(() => buildFunctionSearchEntries(menuRoutes), [menuRoutes]);
  const functionSearchResults = useMemo(
    () => searchFunctionEntries(functionSearchEntries, functionSearch),
    [functionSearch, functionSearchEntries],
  );
  const functionRecommendations = useMemo(
    () =>
      buildFunctionRecommendations(functionSearchEntries, {
        recentPaths: recentFunctionPaths,
        shortcutPaths,
        limit: 8,
      }),
    [functionSearchEntries, recentFunctionPaths, shortcutPaths],
  );
  const functionSearchOptions = useMemo<FunctionSearchOption[]>(
    () =>
      functionSearch.trim()
        ? groupFunctionEntriesByModule(functionSearchResults).map((group) => ({
            label: buildFunctionSearchGroupLabel(group.label, group.items.length),
            options: group.items.map((entry) => buildFunctionSearchOption(entry)),
          }))
        : groupFunctionRecommendationsBySource(functionRecommendations).map((group) => ({
            label: buildFunctionSearchGroupLabel(group.label, group.items.length),
            options: group.items.map((entry) => buildFunctionSearchOption(entry)),
          })),
    [functionRecommendations, functionSearch, functionSearchResults],
  );
  const routeWarmupCandidates = useMemo(
    () =>
      buildWorkbenchWarmupPaths({
        currentPath: currentRouteKey,
        activeModuleKey,
        modules: moduleItems,
        recommendedPaths: functionRecommendations.map((entry) => entry.path),
        moduleLimit: currentRouteKey === '/dashboard' ? 0 : 3,
        limit: 5,
      }),
    [activeModuleKey, currentRouteKey, functionRecommendations, moduleItems],
  );

  const handleFunctionSearchSelect = useCallback(
    (path: string) => {
      setFunctionSearch('');
      navigate(path);
    },
    [navigate],
  );

  const handleFunctionSearchEnter = useCallback(() => {
    if (!functionSearch.trim()) {
      return;
    }
    const firstResult = functionSearchResults[0];
    if (!firstResult) {
      return;
    }
    handleFunctionSearchSelect(firstResult.path);
  }, [functionSearch, functionSearchResults, handleFunctionSearchSelect]);

  const handleModuleIntent = useCallback(
    (paths: string[], source: 'hover' | 'open') => {
      moduleIntentPrefetchCleanupRef.current();
      moduleIntentPrefetchCleanupRef.current = scheduleRoutePrefetchPaths(paths, {
        source,
        limit: source === 'open' ? 6 : 3,
        excludePath: currentRouteKey,
      });
    },
    [currentRouteKey],
  );

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'header'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
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
    { key: 'logout', label: '退出登录' },
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
  const recentFunctionStorageKey = useMemo(() => {
    if (!tenantId || !userId) {
      return '';
    }
    return `erp.ui.workbench.search.recent.${tenantId}.${userId}`;
  }, [tenantId, userId]);

  const [leftWidth, setLeftWidth] = useState(WORKBENCH_DEFAULT_LEFT_PANEL_WIDTH);
  const [rightWidth, setRightWidth] = useState(WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH);
  const [bottomHeight, setBottomHeight] = useState(WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT);
  const [leftCollapsed, setLeftCollapsed] = useState(false);
  const [rightCollapsed, setRightCollapsed] = useState(false);
  const [bottomCollapsed, setBottomCollapsed] = useState(false);
  const [leftPlaceholderTab, setLeftPlaceholderTab] = useState('filters');
  const [bottomPlaceholderTab, setBottomPlaceholderTab] = useState('sub-1');
  const [rightPlaceholderOpenKeys, setRightPlaceholderOpenKeys] = useState<string[]>([
    'tags',
    'common',
    'detail',
  ]);
  const [routePanelsByKey, setRoutePanelsByKey] = useState<
    Record<string, WorkbenchRoutePanelsState>
  >({});
  const layoutHydratedRef = useRef(false);
  const headerRef = useRef<HTMLElement | null>(null);
  const mainSplitRef = useRef<HTMLDivElement | null>(null);
  const routePanelActionsRef = useRef<Record<string, WorkbenchRoutePanelActions>>({});
  const resizeStateRef = useRef<{
    side: 'left' | 'right' | 'bottom';
    startX: number;
    startY: number;
    startWidth: number;
    startHeight: number;
  } | null>(null);

  const handleLeftPanelWidthChange = useCallback(
    (width: number) =>
      setLeftWidth(
        clampWorkbenchPanelSize(
          width,
          WORKBENCH_LEFT_PANEL_MIN_WIDTH,
          WORKBENCH_LEFT_PANEL_MAX_WIDTH,
        ),
      ),
    [],
  );

  const handleRightPanelWidthChange = useCallback(
    (width: number) =>
      setRightWidth(
        clampWorkbenchPanelSize(
          width,
          WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
          WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
        ),
      ),
    [],
  );

  const handleBottomPanelHeightChange = useCallback((height: number) => {
    const containerHeight = mainSplitRef.current?.clientHeight ?? 0;
    const maxBottom =
      containerHeight > 0
        ? Math.max(
            WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
            containerHeight - WORKBENCH_MAIN_TOP_MIN_HEIGHT,
          )
        : Number.MAX_SAFE_INTEGER;
    setBottomHeight(clampWorkbenchPanelSize(height, WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT, maxBottom));
  }, []);

  const updateRoutePanelsState = useCallback(
    (
      routeKey: string,
      updater: (current: WorkbenchRoutePanelsState) => WorkbenchRoutePanelsState,
    ) => {
      setRoutePanelsByKey((currentMap) => {
        const currentState = currentMap[routeKey] ?? EMPTY_WORKBENCH_ROUTE_PANELS;
        const nextState = updater(currentState);
        if (areWorkbenchRoutePanelsEqual(currentState, nextState)) {
          return currentMap;
        }
        return {
          ...currentMap,
          [routeKey]: nextState,
        };
      });
    },
    [],
  );

  const resetRoutePanelsState = useCallback((routeKey: string) => {
    setRoutePanelsByKey((currentMap) => {
      const currentState = currentMap[routeKey];
      if (!currentState) {
        return currentMap;
      }
      if (areWorkbenchRoutePanelsEqual(currentState, EMPTY_WORKBENCH_ROUTE_PANELS)) {
        return currentMap;
      }
      return {
        ...currentMap,
        [routeKey]: EMPTY_WORKBENCH_ROUTE_PANELS,
      };
    });
  }, []);

  const getRoutePanelActions = useCallback(
    (routeKey: string): WorkbenchRoutePanelActions => {
      const existing = routePanelActionsRef.current[routeKey];
      if (existing) {
        return existing;
      }
      const actions: WorkbenchRoutePanelActions = {
        setLeftPanel: (panel) =>
          updateRoutePanelsState(routeKey, (current) => ({
            ...current,
            leftPanel: panel,
          })),
        setRightPanel: (panel) =>
          updateRoutePanelsState(routeKey, (current) => ({
            ...current,
            rightPanel: panel,
          })),
        setBottomPanel: (panel) =>
          updateRoutePanelsState(routeKey, (current) => ({
            ...current,
            bottomPanel: panel,
          })),
        setLeftPanelWidth: handleLeftPanelWidthChange,
        setRightPanelWidth: handleRightPanelWidthChange,
        setBottomPanelHeight: handleBottomPanelHeightChange,
        setLeftPanelCollapsed: setLeftCollapsed,
        setLeftPanelVisible: (visible) =>
          updateRoutePanelsState(routeKey, (current) => ({
            ...current,
            leftPanelVisible: visible,
          })),
        setRightPanelVisible: (visible) =>
          updateRoutePanelsState(routeKey, (current) => ({
            ...current,
            rightPanelVisible: visible,
          })),
        setBottomPanelVisible: (visible) =>
          updateRoutePanelsState(routeKey, (current) => ({
            ...current,
            bottomPanelVisible: visible,
          })),
        clearPanels: () => {
          resetRoutePanelsState(routeKey);
        },
      };
      routePanelActionsRef.current[routeKey] = actions;
      return actions;
    },
    [
      handleBottomPanelHeightChange,
      handleLeftPanelWidthChange,
      handleRightPanelWidthChange,
      resetRoutePanelsState,
      updateRoutePanelsState,
    ],
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
      }),
    );
  }, [
    bottomHeight,
    bottomCollapsed,
    layoutStorageKey,
    leftCollapsed,
    leftWidth,
    rightCollapsed,
    rightWidth,
  ]);

  useEffect(() => {
    setFunctionSearch('');
  }, [location.pathname]);

  useEffect(() => {
    let frameId = 0;
    let nestedFrameId = 0;
    frameId = window.requestAnimationFrame(() => {
      nestedFrameId = window.requestAnimationFrame(() => {
        window.dispatchEvent(new Event('resize'));
      });
    });
    return () => {
      window.cancelAnimationFrame(frameId);
      window.cancelAnimationFrame(nestedFrameId);
    };
  }, [location.pathname]);

  useEffect(() => {
    if (!recentFunctionStorageKey) {
      setRecentFunctionPaths([]);
      return;
    }

    try {
      const raw = localStorage.getItem(recentFunctionStorageKey);
      if (!raw) {
        setRecentFunctionPaths([]);
        return;
      }
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        setRecentFunctionPaths([]);
        return;
      }
      setRecentFunctionPaths(parsed.filter((item): item is string => typeof item === 'string'));
    } catch {
      setRecentFunctionPaths([]);
    }
  }, [recentFunctionStorageKey]);

  useEffect(() => {
    if (!recentFunctionStorageKey || !menuTitleMap.has(location.pathname)) {
      return;
    }

    setRecentFunctionPaths((currentPaths) => {
      const nextPaths = mergeRecentFunctionPaths(currentPaths, location.pathname, 8);
      localStorage.setItem(recentFunctionStorageKey, JSON.stringify(nextPaths));
      return nextPaths;
    });
  }, [location.pathname, menuTitleMap, recentFunctionStorageKey]);

  useEffect(() => {
    const candidatePaths = functionSearch.trim()
      ? functionSearchResults.slice(0, 3).map((entry) => entry.path)
      : functionRecommendations.slice(0, 2).map((entry) => entry.path);

    return scheduleRoutePrefetchPaths(candidatePaths, {
      source: functionSearch.trim() ? 'search' : 'recommendation',
      limit: functionSearch.trim() ? 3 : 2,
      excludePath: currentRouteKey,
    });
  }, [currentRouteKey, functionRecommendations, functionSearch, functionSearchResults]);

  useEffect(() => {
    if (functionSearch.trim()) {
      return;
    }
    if (currentRouteSnapshot.status !== 'ready') {
      return;
    }
    return scheduleRoutePrefetchPaths(routeWarmupCandidates, {
      source: 'programmatic',
      limit: routeWarmupCandidates.length,
      excludePath: currentRouteKey,
    });
  }, [currentRouteKey, currentRouteSnapshot.status, functionSearch, routeWarmupCandidates]);

  useEffect(
    () => () => {
      moduleIntentPrefetchCleanupRef.current();
    },
    [],
  );

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
          ),
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

  const handleResizeStart = (side: 'left' | 'right' | 'bottom') => (event: React.PointerEvent) => {
    if (side === 'bottom' && bottomCollapsed) {
      setBottomCollapsed(false);
    }
    resizeStateRef.current = {
      side,
      startX: event.clientX,
      startY: event.clientY,
      startWidth: side === 'left' ? leftWidth : rightWidth,
      startHeight: bottomHeight,
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
  const activeRouteRefreshToken = refreshTokens[currentRouteKey] ?? 0;
  const validRouteKeys = useMemo(() => {
    const next = new Set<string>(tabs.map((tab) => tab.key));
    next.add(currentRouteKey);
    return next;
  }, [currentRouteKey, tabs]);

  Object.keys(routePanelActionsRef.current).forEach((routeKey) => {
    if (!validRouteKeys.has(routeKey)) {
      delete routePanelActionsRef.current[routeKey];
    }
  });
  const activeRoutePanels = routePanelsByKey[currentRouteKey] ?? EMPTY_WORKBENCH_ROUTE_PANELS;
  const leftPanelVisible = activeRoutePanels.leftPanelVisible;
  const rightPanelVisible = activeRoutePanels.rightPanelVisible;
  const bottomPanelVisible = activeRoutePanels.bottomPanelVisible;
  const leftPanel = activeRoutePanels.leftPanel;
  const rightPanel = activeRoutePanels.rightPanel;
  const bottomPanel = activeRoutePanels.bottomPanel;
  const currentRouteActions = getRoutePanelActions(currentRouteKey);
  const currentRouteProviderValue: WorkbenchPanelsContextValue = {
    leftPanel: activeRoutePanels.leftPanel,
    rightPanel: activeRoutePanels.rightPanel,
    bottomPanel: activeRoutePanels.bottomPanel,
    leftPanelWidth: leftWidth,
    rightPanelWidth: rightWidth,
    bottomPanelHeight: bottomHeight,
    leftPanelVisible: activeRoutePanels.leftPanelVisible,
    rightPanelVisible: activeRoutePanels.rightPanelVisible,
    bottomPanelVisible: activeRoutePanels.bottomPanelVisible,
    ...currentRouteActions,
  };

  const defaultLeftPanel = (
    <WorkbenchPlaceholderTabs
      activeKey={leftPlaceholderTab}
      onChange={setLeftPlaceholderTab}
      items={[
        {
          key: 'filters',
          label: '默认过滤',
          content: (
            <div className="workbench-left-placeholder">
              <Empty description="暂无默认过滤" />
            </div>
          ),
        },
        {
          key: 'query',
          label: '表格查询',
          content: (
            <div className="workbench-left-placeholder">
              <Empty description="暂无表格查询条件" />
            </div>
          ),
        },
      ]}
    />
  );

  const defaultRightPanel = (
    <WorkbenchPlaceholderSections
      openKeys={rightPlaceholderOpenKeys}
      onToggle={(key) =>
        setRightPlaceholderOpenKeys((currentKeys) =>
          currentKeys.includes(key)
            ? currentKeys.filter((item) => item !== key)
            : [...currentKeys, key],
        )
      }
      items={[
        {
          key: 'tags',
          label: '标记',
          content: (
            <div className="workbench-empty">
              <Empty description="暂无标记" />
            </div>
          ),
        },
        {
          key: 'common',
          label: '\u5e38\u7528\u529f\u80fd',
          content: (
            <div className="workbench-empty">
              <Empty description="暂无快捷功能" />
            </div>
          ),
        },
        {
          key: 'detail',
          label: '详情',
          content: (
            <div className="workbench-empty">
              <Empty description="暂无详情内容" />
            </div>
          ),
        },
      ]}
    />
  );

  const defaultBottomPanel = (
    <WorkbenchPlaceholderTabs
      activeKey={bottomPlaceholderTab}
      onChange={setBottomPlaceholderTab}
      items={[
        { key: 'sub-1', label: '子表一', content: <Empty description="暂无子表数据" /> },
        { key: 'sub-2', label: '子表二', content: <Empty description="暂无子表数据" /> },
      ]}
    />
  );

  useEffect(() => {
    setRoutePanelsByKey((currentMap) => {
      const nextEntries = Object.entries(currentMap).filter(([routeKey]) =>
        validRouteKeys.has(routeKey),
      );
      if (nextEntries.length === Object.keys(currentMap).length) {
        return currentMap;
      }
      return Object.fromEntries(nextEntries);
    });
  }, [validRouteKeys]);

  return (
    <div className="workbench-shell">
      <RouteDiagnosticsReporter />
      <header className="workbench-header" ref={headerRef}>
        <ModuleHeader
          logo="衡策云"
          modules={moduleItems}
          activeModuleKey={activeModuleKey}
          currentPath={location.pathname}
          onModuleClick={(path) => navigate(path)}
          onModuleIntent={handleModuleIntent}
          userName={userName}
          userMenuItems={userMenuItems}
          onUserMenuClick={handleUserMenuClick}
          onLogout={resetSession}
          denseMode={denseMode}
          onToggleDense={toggleDenseMode}
          searchValue={functionSearch}
          searchOptions={functionSearchOptions}
          onSearchChange={setFunctionSearch}
          onSearchSelect={handleFunctionSearchSelect}
          onSearchEnter={handleFunctionSearchEnter}
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
                <div className="workbench-panel-body">{leftPanel ?? defaultLeftPanel}</div>
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
                <div
                  key={`${currentRouteKey}-${activeRouteRefreshToken}`}
                  data-workbench-route-key={currentRouteKey}
                  data-workbench-route-status={currentRouteSnapshot.status}
                  data-workbench-route-duration-ms={
                    currentRouteSnapshot.lastDurationMs !== undefined
                      ? String(currentRouteSnapshot.lastDurationMs)
                      : undefined
                  }
                  data-workbench-route-prefetch-status={currentRouteSnapshot.lastPrefetchStatus}
                  style={{
                    display: 'flex',
                    flex: 1,
                    minHeight: 0,
                    height: '100%',
                  }}
                >
                  <WorkbenchPanelsProvider value={currentRouteProviderValue}>
                    <ErrorBoundary>
                      <Outlet />
                    </ErrorBoundary>
                  </WorkbenchPanelsProvider>
                </div>
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
                    className={`workbench-main-bottom${bottomCollapsed ? ' is-collapsed' : ''}`}
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
                <div className="workbench-panel-body">{rightPanel ?? defaultRightPanel}</div>
              )}
            </aside>
          </>
        ) : null}
      </div>
    </div>
  );
};




