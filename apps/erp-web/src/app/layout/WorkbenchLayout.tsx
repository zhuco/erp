import { Alert, Button, Empty, Input, Modal, Space, Tooltip, Typography } from 'antd';
import type { InputRef, MenuProps } from 'antd';
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
import {
  RouteTabs,
  ROUTE_TABS_SHORTCUT_EVENT,
  type RouteTabsShortcutAction,
} from '@/app/layout/RouteTabs';
import { useTabsStore } from '@/app/layout/tabsStore';
import {
  WorkbenchPanelsProvider,
  type WorkbenchPanelsContextValue,
} from '@/app/layout/workbenchPanels';
import type { FunctionSearchOption } from '@/app/layout/ModuleHeader';
import { ModuleHeader } from '@/app/layout/ModuleHeader';
import {
  buildHeaderModules,
  type HeaderModuleGroupItem as ModuleGroupItem,
  type HeaderModuleLinkItem as ModuleLinkItem,
  resolveActiveHeaderModuleKey,
} from '@/app/layout/headerMenuModel';
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
import { useFocusReturn } from '@/app/layout/useFocusReturn';
import { workbenchI18n } from '@/app/layout/workbenchI18n';
import { NotFoundPage } from '@/pages/NotFound';

import './workbenchLayout.css';

const collapsedWidth = 44;
const collapsedBottomHeight = 0;
const WORKBENCH_RESIZE_KEYBOARD_STEP = 16;
const WORKBENCH_RESIZE_KEYBOARD_LARGE_STEP = 48;
const reducedUiAllowedPaths = new Set([
  '/',
  '/dashboard',
  '/channels',
  '/sales/order-query',
  '/orders/review',
  '/orders/recheck',
  '/sales/settings',
]);

export const isReducedUiPathAllowed = (pathname: string) => reducedUiAllowedPaths.has(pathname);
const WORKBENCH_RESIZE_SNAP_THRESHOLD = 20;
const LAYOUT_PRESET_QUERY_KEY = 'layoutPreset';
const LAYOUT_LEFT_COLLAPSED_QUERY_KEY = 'leftCollapsed';
const LAYOUT_RIGHT_COLLAPSED_QUERY_KEY = 'rightCollapsed';
const LAYOUT_BOTTOM_COLLAPSED_QUERY_KEY = 'bottomCollapsed';
const LAYOUT_LEFT_WIDTH_QUERY_KEY = 'leftWidth';
const LAYOUT_RIGHT_WIDTH_QUERY_KEY = 'rightWidth';
const LAYOUT_BOTTOM_HEIGHT_QUERY_KEY = 'bottomHeight';
const WORKBENCH_INTERACTION_EVENT = 'erp:workbench-interaction';
const WORKBENCH_INTERACTION_SAMPLE_RATE = 0.25;
const WORKBENCH_ANNOUNCE_EVENT = 'erp:workbench-announce';

type WorkbenchResizableSide = 'left' | 'right' | 'bottom';
type WorkbenchLayoutPreset = 'compact' | 'default' | 'analysis';
type WorkbenchRouteLoadingStatus = 'idle' | 'loading' | 'ready' | 'timeout' | 'error';
type WorkbenchCommandScope = 'workbench' | 'route-tabs';

type WorkbenchCommand = {
  id: string;
  label: string;
  description: string;
  shortcut: string;
  scope: WorkbenchCommandScope;
  keywords: string[];
  run: () => void;
};

type WorkbenchLayoutQueryState = {
  hasAny: boolean;
  preset: WorkbenchLayoutPreset | null;
  leftCollapsed?: boolean;
  rightCollapsed?: boolean;
  bottomCollapsed?: boolean;
  leftWidth?: number;
  rightWidth?: number;
  bottomHeight?: number;
};

const isWorkbenchLayoutPreset = (value: unknown): value is WorkbenchLayoutPreset =>
  value === 'compact' || value === 'default' || value === 'analysis';

const resolveLayoutPresetLabel = (preset: WorkbenchLayoutPreset) => {
  if (preset === 'compact') {
    return workbenchI18n.layoutPreset.compact;
  }
  if (preset === 'analysis') {
    return workbenchI18n.layoutPreset.analysis;
  }
  return workbenchI18n.layoutPreset.default;
};

type WorkbenchInteractionName =
  | 'layout.preset.apply'
  | 'layout.preset.restore'
  | 'layout.preset.persist'
  | 'search.select'
  | 'resize.end';

const emitWorkbenchInteraction = (
  name: WorkbenchInteractionName,
  payload: Record<string, unknown>,
) => {
  if (typeof window === 'undefined') {
    return;
  }
  const debugEnabled = (window as Window & { __ERP_WORKBENCH_DEBUG__?: boolean })
    .__ERP_WORKBENCH_DEBUG__;
  const sampled = debugEnabled || Math.random() < WORKBENCH_INTERACTION_SAMPLE_RATE;
  if (!sampled) {
    return;
  }
  window.dispatchEvent(
    new CustomEvent(WORKBENCH_INTERACTION_EVENT, {
      detail: {
        name,
        payload,
        ts: Date.now(),
      },
    }),
  );
};

const isEditableEventTarget = (target: EventTarget | null) => {
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

const dispatchTabsShortcutAction = (action: RouteTabsShortcutAction) => {
  window.dispatchEvent(
    new CustomEvent(ROUTE_TABS_SHORTCUT_EVENT, {
      detail: { action },
    }),
  );
};

type ResizeIndicatorState = {
  x: number;
  y: number;
  size: number;
  side: WorkbenchResizableSide;
};

const resolveSnappedPanelSize = (size: number, snapPoints: number[]) => {
  const nearest = snapPoints.reduce(
    (currentNearest, point) =>
      Math.abs(size - point) < Math.abs(size - currentNearest) ? point : currentNearest,
    snapPoints[0] ?? size,
  );
  if (Math.abs(size - nearest) <= WORKBENCH_RESIZE_SNAP_THRESHOLD) {
    return nearest;
  }
  return size;
};

const parseLayoutBooleanQueryValue = (value: string | null) => {
  if (value === null) {
    return undefined;
  }
  if (value === '1' || value.toLowerCase() === 'true') {
    return true;
  }
  if (value === '0' || value.toLowerCase() === 'false') {
    return false;
  }
  return undefined;
};

const parseLayoutSizeQueryValue = (
  value: string | null,
  min: number,
  max: number = Number.MAX_SAFE_INTEGER,
) => {
  if (!value) {
    return undefined;
  }
  const nextSize = Number.parseInt(value, 10);
  if (!Number.isFinite(nextSize)) {
    return undefined;
  }
  return clampWorkbenchPanelSize(nextSize, min, max);
};

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
          label: item.name ?? `Group ${index + 1}`,
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

  if (!isReducedUiPathAllowed(location.pathname)) {
    return <NotFoundPage />;
  }

  const headerModules = useMemo(() => resolveHeaderModules(menuRoutes), [menuRoutes]);
  const [functionSearch, setFunctionSearch] = useState('');
  const [recentFunctionPaths, setRecentFunctionPaths] = useState<string[]>([]);

  const legacyModuleItems = useMemo(() => {
    return headerModules.map((item, index) => {
      const nestedLeafPaths = collectLeafPaths(item);
      const defaultPath = item.path ?? nestedLeafPaths[0];
      const leafPaths = defaultPath
        ? [defaultPath, ...nestedLeafPaths.filter((path) => path !== defaultPath)]
        : nestedLeafPaths;
      const label = typeof item.name === 'string' ? item.name : (item.path ?? `Module ${index + 1}`);
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

  const moduleItems = useMemo(() => buildHeaderModules(menuRoutes), [menuRoutes]);

  const legacyActiveModuleKey = useMemo(() => {
    const current = location.pathname;
    const match = moduleItems.find((module) =>
      module.leafPaths.some((path) => isPathActive(current, path)),
    );
    return match?.key;
  }, [location.pathname, moduleItems]);
  void legacyModuleItems;
  void legacyActiveModuleKey;

  const activeModuleKey = useMemo(
    () => resolveActiveHeaderModuleKey(moduleItems, location.pathname),
    [location.pathname, moduleItems],
  );
  const moduleIntentPrefetchCleanupRef = useRef<() => void>(() => undefined);
  const currentRouteKey = location.pathname;
  const currentRouteSnapshot = useRouteLoadingSnapshot(currentRouteKey);
  const queryLayoutState = useMemo<WorkbenchLayoutQueryState>(() => {
    const params = new URLSearchParams(location.search);
    const routePreset = params.get(LAYOUT_PRESET_QUERY_KEY);
    const leftCollapsed = parseLayoutBooleanQueryValue(params.get(LAYOUT_LEFT_COLLAPSED_QUERY_KEY));
    const rightCollapsed = parseLayoutBooleanQueryValue(
      params.get(LAYOUT_RIGHT_COLLAPSED_QUERY_KEY),
    );
    const bottomCollapsed = parseLayoutBooleanQueryValue(
      params.get(LAYOUT_BOTTOM_COLLAPSED_QUERY_KEY),
    );
    const leftWidth = parseLayoutSizeQueryValue(
      params.get(LAYOUT_LEFT_WIDTH_QUERY_KEY),
      WORKBENCH_LEFT_PANEL_MIN_WIDTH,
      WORKBENCH_LEFT_PANEL_MAX_WIDTH,
    );
    const rightWidth = parseLayoutSizeQueryValue(
      params.get(LAYOUT_RIGHT_WIDTH_QUERY_KEY),
      WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
      WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
    );
    const bottomHeight = parseLayoutSizeQueryValue(
      params.get(LAYOUT_BOTTOM_HEIGHT_QUERY_KEY),
      WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
    );
    const preset = isWorkbenchLayoutPreset(routePreset) ? routePreset : null;

    return {
      hasAny:
        preset !== null ||
        typeof leftCollapsed === 'boolean' ||
        typeof rightCollapsed === 'boolean' ||
        typeof bottomCollapsed === 'boolean' ||
        typeof leftWidth === 'number' ||
        typeof rightWidth === 'number' ||
        typeof bottomHeight === 'number',
      preset,
      leftCollapsed,
      rightCollapsed,
      bottomCollapsed,
      leftWidth,
      rightWidth,
      bottomHeight,
    };
  }, [location.search]);

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

  const resolveResizeSnapPoints = useCallback(
    (side: WorkbenchResizableSide, bottomMax: number) => {
      if (side === 'left') {
        return [
          WORKBENCH_LEFT_PANEL_MIN_WIDTH,
          WORKBENCH_DEFAULT_LEFT_PANEL_WIDTH,
          WORKBENCH_LEFT_PANEL_MAX_WIDTH,
        ];
      }
      if (side === 'right') {
        return [
          WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
          WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH,
          WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
        ];
      }
      return [
        WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
        clampWorkbenchPanelSize(
          WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT,
          WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
          bottomMax,
        ),
        bottomMax,
      ];
    },
    [],
  );

  const buildResizeIndicatorLabel = useCallback((side: WorkbenchResizableSide, size: number) => {
    if (side === 'bottom') {
      return `Bottom ${Math.round(size)}px`;
    }
    if (side === 'right') {
      return `Right ${Math.round(size)}px`;
    }
    return `Left ${Math.round(size)}px`;
  }, []);

  const focusFunctionSearchInput = useCallback(() => {
    window.dispatchEvent(new Event('erp:focus-function-search'));
  }, []);

  const openCommandPalette = useCallback(() => {
    setCommandKeyword('');
    setCommandPaletteOpen(true);
  }, []);

  const openShortcutHelp = useCallback(() => {
    setShortcutHelpOpen(true);
  }, []);

  const runRouteTabsShortcut = useCallback((action: RouteTabsShortcutAction) => {
    dispatchTabsShortcutAction(action);
  }, []);

  const announceForScreenReader = useCallback((message: string) => {
    if (!message) {
      return;
    }
    if (screenReaderAnnounceTimerRef.current !== null) {
      window.clearTimeout(screenReaderAnnounceTimerRef.current);
      screenReaderAnnounceTimerRef.current = null;
    }
    setScreenReaderAnnouncement('');
    screenReaderAnnounceTimerRef.current = window.setTimeout(() => {
      setScreenReaderAnnouncement(message);
      screenReaderAnnounceTimerRef.current = null;
    }, 40);
  }, []);

  const handleFunctionSearchSelect = useCallback(
    (path: string) => {
      emitWorkbenchInteraction('search.select', {
        path,
        route: currentRouteKey,
      });
      pendingFunctionSearchFocusRef.current = true;
      setFunctionSearch('');
      navigate(path);
    },
    [currentRouteKey, navigate],
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
    { key: 'profile', label: 'Profile' },
    { key: 'logout', label: 'Sign Out' },
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
  const layoutPresetStorageKey = useMemo(() => {
    if (!tenantId || !userId) {
      return '';
    }
    return `erp.ui.workbench.layout.preset.${tenantId}.${userId}`;
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
  const [resizeIndicator, setResizeIndicator] = useState<ResizeIndicatorState | null>(null);
  const [layoutPreset, setLayoutPreset] = useState<WorkbenchLayoutPreset>('default');
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [commandKeyword, setCommandKeyword] = useState('');
  const [shortcutHelpOpen, setShortcutHelpOpen] = useState(false);
  const [screenReaderAnnouncement, setScreenReaderAnnouncement] = useState('');
  const [resizeCommitVersion, setResizeCommitVersion] = useState(0);
  const layoutHydratedRef = useRef(false);
  const layoutPresetHydratedRef = useRef<string | null>(null);
  const layoutPresetExpectedRef = useRef<WorkbenchLayoutPreset>('default');
  const pendingFunctionSearchFocusRef = useRef(false);
  const screenReaderAnnounceTimerRef = useRef<number | null>(null);
  const previousRouteStatusRef = useRef<WorkbenchRouteLoadingStatus | null>(null);
  const previousPanelCollapsedStateRef = useRef<{
    initialized: boolean;
    leftCollapsed: boolean;
    rightCollapsed: boolean;
    bottomCollapsed: boolean;
  }>({
    initialized: false,
    leftCollapsed: false,
    rightCollapsed: false,
    bottomCollapsed: false,
  });
  const headerRef = useRef<HTMLElement | null>(null);
  const mainSplitRef = useRef<HTMLDivElement | null>(null);
  const mainRouteRegionRef = useRef<HTMLDivElement | null>(null);
  const commandInputRef = useRef<InputRef | null>(null);
  const routePanelActionsRef = useRef<Record<string, WorkbenchRoutePanelActions>>({});
  const resizeStateRef = useRef<{
    side: 'left' | 'right' | 'bottom';
    startX: number;
    startY: number;
    startWidth: number;
    startHeight: number;
  } | null>(null);
  const resizeFrameRef = useRef<number | null>(null);
  const pendingResizePointerRef = useRef<{ x: number; y: number } | null>(null);
  const isPointerResizingRef = useRef(false);

  useFocusReturn(commandPaletteOpen);
  useFocusReturn(shortcutHelpOpen);

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
    if (!pendingFunctionSearchFocusRef.current) {
      return;
    }
    pendingFunctionSearchFocusRef.current = false;
    let frameId = 0;
    frameId = window.requestAnimationFrame(() => {
      mainRouteRegionRef.current?.focus({ preventScroll: true });
    });
    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [location.pathname]);

  useEffect(
    () => () => {
      if (screenReaderAnnounceTimerRef.current !== null) {
        window.clearTimeout(screenReaderAnnounceTimerRef.current);
        screenReaderAnnounceTimerRef.current = null;
      }
    },
    [],
  );

  useEffect(() => {
    const handleAnnounceEvent = (event: Event) => {
      const detail = (event as CustomEvent<{ message?: string }>).detail;
      const messageText = detail?.message?.trim();
      if (!messageText) {
        return;
      }
      announceForScreenReader(messageText);
    };

    window.addEventListener(WORKBENCH_ANNOUNCE_EVENT, handleAnnounceEvent as EventListener);
    return () => {
      window.removeEventListener(WORKBENCH_ANNOUNCE_EVENT, handleAnnounceEvent as EventListener);
    };
  }, [announceForScreenReader]);

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
    const flushResizeFrame = () => {
      resizeFrameRef.current = null;
      const resizeState = resizeStateRef.current;
      const pointer = pendingResizePointerRef.current;
      if (!resizeState || !pointer) {
        return;
      }
      const bottomMax = Math.max(
        WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
        (mainSplitRef.current?.clientHeight ?? 0) - WORKBENCH_MAIN_TOP_MIN_HEIGHT,
      );

      if (resizeState.side === 'bottom') {
        const delta = pointer.y - resizeState.startY;
        const nextSize = resolveSnappedPanelSize(
          clampWorkbenchPanelSize(
            resizeState.startHeight - delta,
            WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
            bottomMax,
          ),
          resolveResizeSnapPoints('bottom', bottomMax),
        );
        setBottomHeight(nextSize);
        setResizeIndicator({
          side: 'bottom',
          size: nextSize,
          x: pointer.x,
          y: pointer.y,
        });
        return;
      }
      const delta = pointer.x - resizeState.startX;
      if (resizeState.side === 'left') {
        const nextSize = resolveSnappedPanelSize(
          clampWorkbenchPanelSize(
            resizeState.startWidth + delta,
            WORKBENCH_LEFT_PANEL_MIN_WIDTH,
            WORKBENCH_LEFT_PANEL_MAX_WIDTH,
          ),
          resolveResizeSnapPoints('left', bottomMax),
        );
        setLeftWidth(nextSize);
        setResizeIndicator({
          side: 'left',
          size: nextSize,
          x: pointer.x,
          y: pointer.y,
        });
        return;
      }
      const nextSize = resolveSnappedPanelSize(
        clampWorkbenchPanelSize(
          resizeState.startWidth - delta,
          WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
          WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
        ),
        resolveResizeSnapPoints('right', bottomMax),
      );
      setRightWidth(nextSize);
      setResizeIndicator({
        side: 'right',
        size: nextSize,
        x: pointer.x,
        y: pointer.y,
      });
    };

    const handlePointerMove = (event: PointerEvent) => {
      if (!resizeStateRef.current) {
        return;
      }
      pendingResizePointerRef.current = {
        x: event.clientX,
        y: event.clientY,
      };
      if (resizeFrameRef.current !== null) {
        return;
      }
      resizeFrameRef.current = window.requestAnimationFrame(flushResizeFrame);
    };

    const handlePointerUp = () => {
      const resizeState = resizeStateRef.current;
      if (!resizeState) {
        return;
      }
      if (resizeFrameRef.current !== null) {
        window.cancelAnimationFrame(resizeFrameRef.current);
        resizeFrameRef.current = null;
      }
      pendingResizePointerRef.current = null;
      resizeStateRef.current = null;
      isPointerResizingRef.current = false;
      setResizeIndicator(null);
      setResizeCommitVersion((current) => current + 1);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      emitWorkbenchInteraction('resize.end', {
        route: currentRouteKey,
        side: resizeState.side,
        leftWidth,
        rightWidth,
        bottomHeight,
      });
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    return () => {
      if (resizeFrameRef.current !== null) {
        window.cancelAnimationFrame(resizeFrameRef.current);
        resizeFrameRef.current = null;
      }
      isPointerResizingRef.current = false;
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };
  }, [bottomHeight, currentRouteKey, leftWidth, resolveResizeSnapPoints, rightWidth]);

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

  const handleResizeStart = (side: WorkbenchResizableSide) => (event: React.PointerEvent) => {
    if (resizeFrameRef.current !== null) {
      window.cancelAnimationFrame(resizeFrameRef.current);
      resizeFrameRef.current = null;
    }
    pendingResizePointerRef.current = null;
    isPointerResizingRef.current = true;
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
    const indicatorSize = side === 'left' ? leftWidth : side === 'right' ? rightWidth : bottomHeight;
    setResizeIndicator({
      side,
      size: indicatorSize,
      x: event.clientX,
      y: event.clientY,
    });
    document.body.style.cursor = side === 'bottom' ? 'row-resize' : 'col-resize';
    document.body.style.userSelect = 'none';
  };

  const resolveBottomPanelMaxHeight = useCallback(() => {
    const containerHeight = mainSplitRef.current?.clientHeight ?? 0;
    if (containerHeight <= 0) {
      return Number.MAX_SAFE_INTEGER;
    }
    return Math.max(
      WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
      containerHeight - WORKBENCH_MAIN_TOP_MIN_HEIGHT,
    );
  }, []);

  const resetResizablePanelSize = useCallback((side: WorkbenchResizableSide, maxBottom: number) => {
    if (side === 'left') {
      setLeftCollapsed(false);
      setLeftWidth(WORKBENCH_DEFAULT_LEFT_PANEL_WIDTH);
      return;
    }
    if (side === 'right') {
      setRightCollapsed(false);
      setRightWidth(WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH);
      return;
    }
    setBottomCollapsed(false);
    setBottomHeight(
      clampWorkbenchPanelSize(
        WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT,
        WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
        maxBottom,
      ),
    );
  }, []);

  const handleResizeReset = (side: WorkbenchResizableSide) => (event: React.MouseEvent) => {
    event.preventDefault();
    const maxBottom = resolveBottomPanelMaxHeight();
    resetResizablePanelSize(side, maxBottom);
  };

  const applyLayoutPreset = useCallback(
    (preset: WorkbenchLayoutPreset) => {
      const maxBottom = resolveBottomPanelMaxHeight();
      layoutPresetExpectedRef.current = preset;
      setLayoutPreset(preset);
      setResizeIndicator(null);
      emitWorkbenchInteraction('layout.preset.apply', {
        preset,
        route: currentRouteKey,
      });

      if (preset === 'compact') {
        setLeftCollapsed(true);
        setRightCollapsed(true);
        setBottomCollapsed(true);
        return;
      }

      if (preset === 'default') {
        setLeftCollapsed(false);
        setRightCollapsed(false);
        setBottomCollapsed(false);
        setLeftWidth(WORKBENCH_DEFAULT_LEFT_PANEL_WIDTH);
        setRightWidth(WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH);
        setBottomHeight(
          clampWorkbenchPanelSize(
            WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT,
            WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
            maxBottom,
          ),
        );
        return;
      }

      setLeftCollapsed(true);
      setRightCollapsed(false);
      setBottomCollapsed(false);
      setRightWidth(
        clampWorkbenchPanelSize(
          WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH + 80,
          WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
          WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
        ),
      );
      setBottomHeight(
        clampWorkbenchPanelSize(
          WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT + 120,
          WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
          maxBottom,
        ),
      );
    },
    [currentRouteKey, resolveBottomPanelMaxHeight],
  );

  const workbenchCommands = useMemo<WorkbenchCommand[]>(
    () => [
      {
        id: 'focus-search',
        label: workbenchI18n.commandLabels.focusSearch,
        description: workbenchI18n.commandDescriptions.focusSearch,
        shortcut: 'Ctrl+K -> Enter',
        scope: 'workbench',
        keywords: ['鎼滅储', '鑱氱劍', '鍔熻兘', 'search', 'focus'],
        run: () => focusFunctionSearchInput(),
      },
      {
        id: 'preset-compact',
        label: workbenchI18n.commandLabels.applyCompactLayout,
        description: workbenchI18n.commandDescriptions.applyCompactLayout,
        shortcut: 'Alt+Shift+1',
        scope: 'workbench',
        keywords: ['甯冨眬', '绱у噾', 'layout', 'compact'],
        run: () => applyLayoutPreset('compact'),
      },
      {
        id: 'preset-default',
        label: workbenchI18n.commandLabels.applyDefaultLayout,
        description: workbenchI18n.commandDescriptions.applyDefaultLayout,
        shortcut: 'Alt+Shift+2',
        scope: 'workbench',
        keywords: ['甯冨眬', '鏍囧噯', 'layout', 'default'],
        run: () => applyLayoutPreset('default'),
      },
      {
        id: 'preset-analysis',
        label: workbenchI18n.commandLabels.applyAnalysisLayout,
        description: workbenchI18n.commandDescriptions.applyAnalysisLayout,
        shortcut: 'Alt+Shift+3',
        scope: 'workbench',
        keywords: ['甯冨眬', '鍒嗘瀽', 'layout', 'analysis'],
        run: () => applyLayoutPreset('analysis'),
      },
      {
        id: 'toggle-left',
        label: leftCollapsed
          ? workbenchI18n.commandLabels.expandLeftPanel
          : workbenchI18n.commandLabels.collapseLeftPanel,
        description: workbenchI18n.commandDescriptions.toggleLeftPanel,
        shortcut: 'Command',
        scope: 'workbench',
        keywords: ['宸︿晶', '闈㈡澘', 'left', 'panel'],
        run: () => setLeftCollapsed((prev) => !prev),
      },
      {
        id: 'toggle-right',
        label: rightCollapsed
          ? workbenchI18n.commandLabels.expandRightPanel
          : workbenchI18n.commandLabels.collapseRightPanel,
        description: workbenchI18n.commandDescriptions.toggleRightPanel,
        shortcut: 'Command',
        scope: 'workbench',
        keywords: ['right', 'panel', 'toggle'],
        run: () => setRightCollapsed((prev) => !prev),
      },
      {
        id: 'toggle-bottom',
        label: bottomCollapsed
          ? workbenchI18n.commandLabels.expandBottomPanel
          : workbenchI18n.commandLabels.collapseBottomPanel,
        description: workbenchI18n.commandDescriptions.toggleBottomPanel,
        shortcut: 'Command',
        scope: 'workbench',
        keywords: ['bottom', 'panel', 'toggle'],
        run: () => setBottomCollapsed((prev) => !prev),
      },
      {
        id: 'route-refresh',
        label: workbenchI18n.commandLabels.refreshCurrentTab,
        description: workbenchI18n.commandDescriptions.refreshCurrentTab,
        shortcut: 'Ctrl+R',
        scope: 'route-tabs',
        keywords: ['tab', 'refresh', 'route'],
        run: () => runRouteTabsShortcut('refresh'),
      },
      {
        id: 'route-next',
        label: workbenchI18n.commandLabels.nextTab,
        description: workbenchI18n.commandDescriptions.nextTab,
        shortcut: 'Ctrl+Tab',
        scope: 'route-tabs',
        keywords: ['tab', 'next', 'route'],
        run: () => runRouteTabsShortcut('next'),
      },
      {
        id: 'route-prev',
        label: workbenchI18n.commandLabels.previousTab,
        description: workbenchI18n.commandDescriptions.previousTab,
        shortcut: 'Ctrl+Shift+Tab',
        scope: 'route-tabs',
        keywords: ['tab', 'previous', 'route'],
        run: () => runRouteTabsShortcut('previous'),
      },
      {
        id: 'route-close',
        label: workbenchI18n.commandLabels.closeCurrentTab,
        description: workbenchI18n.commandDescriptions.closeCurrentTab,
        shortcut: 'Ctrl+W',
        scope: 'route-tabs',
        keywords: ['tab', 'close', 'route'],
        run: () => runRouteTabsShortcut('close'),
      },
      {
        id: 'open-help',
        label: workbenchI18n.commandLabels.openShortcutHelp,
        description: workbenchI18n.commandDescriptions.openShortcutHelp,
        shortcut: '? / Ctrl+/',
        scope: 'workbench',
        keywords: ['help', 'shortcut', 'hotkey'],
        run: () => openShortcutHelp(),
      },
    ],
    [
      applyLayoutPreset,
      bottomCollapsed,
      focusFunctionSearchInput,
      leftCollapsed,
      openShortcutHelp,
      rightCollapsed,
      runRouteTabsShortcut,
    ],
  );

  const filteredWorkbenchCommands = useMemo(() => {
    const keyword = commandKeyword.trim().toLowerCase();
    if (!keyword) {
      return workbenchCommands;
    }
    return workbenchCommands.filter((command) => {
      if (command.label.toLowerCase().includes(keyword)) {
        return true;
      }
      if (command.description.toLowerCase().includes(keyword)) {
        return true;
      }
      return command.keywords.some((item) => item.toLowerCase().includes(keyword));
    });
  }, [commandKeyword, workbenchCommands]);

  const runWorkbenchCommand = useCallback(
    (command: WorkbenchCommand) => {
      command.run();
      setCommandPaletteOpen(false);
      setCommandKeyword('');
      announceForScreenReader(workbenchI18n.commandExecuted(command.label));
    },
    [announceForScreenReader],
  );

  useEffect(() => {
    if (!commandPaletteOpen) {
      return;
    }
    window.requestAnimationFrame(() => {
      commandInputRef.current?.focus();
    });
  }, [commandPaletteOpen]);

  useEffect(() => {
    const handleWorkbenchHotkeys = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      const withCtrlOrMeta = event.ctrlKey || event.metaKey;
      const editableTarget = isEditableEventTarget(event.target);

      if (event.key === 'Escape') {
        if (commandPaletteOpen) {
          event.preventDefault();
          setCommandPaletteOpen(false);
          return;
        }
        if (shortcutHelpOpen) {
          event.preventDefault();
          setShortcutHelpOpen(false);
          return;
        }
      }

      if (
        !event.altKey &&
        !event.shiftKey &&
        withCtrlOrMeta &&
        key === 'k'
      ) {
        event.preventDefault();
        openCommandPalette();
        return;
      }

      if (!event.altKey && withCtrlOrMeta && key === '/') {
        event.preventDefault();
        openShortcutHelp();
        return;
      }

      if (!withCtrlOrMeta && !event.altKey && event.key === '?') {
        event.preventDefault();
        openShortcutHelp();
        return;
      }

      if (editableTarget || commandPaletteOpen || shortcutHelpOpen) {
        return;
      }

      if (!event.altKey || !event.shiftKey || event.ctrlKey || event.metaKey) {
        return;
      }

      if (event.key === '1' || event.code === 'Digit1') {
        event.preventDefault();
        applyLayoutPreset('compact');
        return;
      }
      if (event.key === '2' || event.code === 'Digit2') {
        event.preventDefault();
        applyLayoutPreset('default');
        return;
      }
      if (event.key === '3' || event.code === 'Digit3') {
        event.preventDefault();
        applyLayoutPreset('analysis');
      }
    };

    window.addEventListener('keydown', handleWorkbenchHotkeys);
    return () => {
      window.removeEventListener('keydown', handleWorkbenchHotkeys);
    };
  }, [applyLayoutPreset, commandPaletteOpen, openCommandPalette, openShortcutHelp, shortcutHelpOpen]);

  useEffect(() => {
    if (queryLayoutState.hasAny) {
      const restoredPreset = queryLayoutState.preset ?? 'default';
      const maxBottom = resolveBottomPanelMaxHeight();
      layoutPresetHydratedRef.current = currentRouteKey;
      applyLayoutPreset(restoredPreset);

      if (typeof queryLayoutState.leftCollapsed === 'boolean') {
        setLeftCollapsed(queryLayoutState.leftCollapsed);
      }
      if (typeof queryLayoutState.rightCollapsed === 'boolean') {
        setRightCollapsed(queryLayoutState.rightCollapsed);
      }
      if (typeof queryLayoutState.bottomCollapsed === 'boolean') {
        setBottomCollapsed(queryLayoutState.bottomCollapsed);
      }
      if (typeof queryLayoutState.leftWidth === 'number') {
        setLeftWidth(queryLayoutState.leftWidth);
      }
      if (typeof queryLayoutState.rightWidth === 'number') {
        setRightWidth(queryLayoutState.rightWidth);
      }
      if (typeof queryLayoutState.bottomHeight === 'number') {
        setBottomHeight(
          clampWorkbenchPanelSize(
            queryLayoutState.bottomHeight,
            WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
            maxBottom,
          ),
        );
      }

      emitWorkbenchInteraction('layout.preset.restore', {
        preset: restoredPreset,
        route: currentRouteKey,
        source: 'query',
      });
      return;
    }

    let cachedPreset: WorkbenchLayoutPreset = 'default';
    let restoreSource: 'default' | 'storage' = 'default';
    if (layoutPresetStorageKey) {
      try {
        const raw = localStorage.getItem(layoutPresetStorageKey);
        if (raw) {
          const parsed = JSON.parse(raw) as Record<string, unknown>;
          const routePreset = parsed[currentRouteKey];
          if (isWorkbenchLayoutPreset(routePreset)) {
            cachedPreset = routePreset;
            restoreSource = 'storage';
          }
        }
      } catch {
        // ignore malformed route-preset cache
      }
    }
    layoutPresetHydratedRef.current = currentRouteKey;
    applyLayoutPreset(cachedPreset);
    emitWorkbenchInteraction('layout.preset.restore', {
      preset: cachedPreset,
      route: currentRouteKey,
      source: restoreSource,
    });
  }, [
    applyLayoutPreset,
    currentRouteKey,
    layoutPresetStorageKey,
    queryLayoutState,
    resolveBottomPanelMaxHeight,
  ]);

  useEffect(() => {
    if (!layoutPresetStorageKey) {
      return;
    }
    if (layoutPresetHydratedRef.current !== currentRouteKey) {
      return;
    }
    if (layoutPresetExpectedRef.current !== layoutPreset) {
      return;
    }

    try {
      const raw = localStorage.getItem(layoutPresetStorageKey);
      const parsed = raw ? (JSON.parse(raw) as Record<string, unknown>) : {};
      const next =
        parsed && typeof parsed === 'object' && !Array.isArray(parsed)
          ? { ...parsed }
          : ({} as Record<string, unknown>);
      next[currentRouteKey] = layoutPreset;
      localStorage.setItem(layoutPresetStorageKey, JSON.stringify(next));
      emitWorkbenchInteraction('layout.preset.persist', {
        preset: layoutPreset,
        route: currentRouteKey,
      });
    } catch {
      localStorage.setItem(
        layoutPresetStorageKey,
        JSON.stringify({
          [currentRouteKey]: layoutPreset,
        }),
      );
      emitWorkbenchInteraction('layout.preset.persist', {
        preset: layoutPreset,
        route: currentRouteKey,
      });
    }
  }, [currentRouteKey, layoutPreset, layoutPresetStorageKey]);

  useEffect(() => {
    if (layoutPresetHydratedRef.current !== currentRouteKey) {
      return;
    }
    if (isPointerResizingRef.current) {
      return;
    }
    const params = new URLSearchParams(location.search);
    params.set(LAYOUT_PRESET_QUERY_KEY, layoutPreset);
    params.set(LAYOUT_LEFT_COLLAPSED_QUERY_KEY, leftCollapsed ? '1' : '0');
    params.set(LAYOUT_RIGHT_COLLAPSED_QUERY_KEY, rightCollapsed ? '1' : '0');
    params.set(LAYOUT_BOTTOM_COLLAPSED_QUERY_KEY, bottomCollapsed ? '1' : '0');
    params.set(LAYOUT_LEFT_WIDTH_QUERY_KEY, String(Math.round(leftWidth)));
    params.set(LAYOUT_RIGHT_WIDTH_QUERY_KEY, String(Math.round(rightWidth)));
    params.set(LAYOUT_BOTTOM_HEIGHT_QUERY_KEY, String(Math.round(bottomHeight)));
    const nextQuery = params.toString();
    const nextUrl = `${location.pathname}${nextQuery ? `?${nextQuery}` : ''}${location.hash}`;
    const currentUrl = `${location.pathname}${location.search}${location.hash}`;
    if (nextUrl === currentUrl) {
      return;
    }
    window.history.replaceState(window.history.state, '', nextUrl);
  }, [
    bottomCollapsed,
    bottomHeight,
    currentRouteKey,
    layoutPreset,
    leftCollapsed,
    leftWidth,
    location.hash,
    location.pathname,
    location.search,
    resizeCommitVersion,
    rightCollapsed,
    rightWidth,
  ]);

  useEffect(() => {
    if (layoutPresetHydratedRef.current !== currentRouteKey) {
      return;
    }
    announceForScreenReader(workbenchI18n.presetApplied(resolveLayoutPresetLabel(layoutPreset)));
  }, [announceForScreenReader, currentRouteKey, layoutPreset]);

  useEffect(() => {
    const previous = previousPanelCollapsedStateRef.current;
    if (!previous.initialized) {
      previousPanelCollapsedStateRef.current = {
        initialized: true,
        leftCollapsed,
        rightCollapsed,
        bottomCollapsed,
      };
      return;
    }

    let announcement = '';
    if (previous.leftCollapsed !== leftCollapsed) {
      announcement = leftCollapsed
        ? workbenchI18n.leftPanelCollapsed
        : workbenchI18n.leftPanelExpanded;
    } else if (previous.rightCollapsed !== rightCollapsed) {
      announcement = rightCollapsed
        ? workbenchI18n.rightPanelCollapsed
        : workbenchI18n.rightPanelExpanded;
    } else if (previous.bottomCollapsed !== bottomCollapsed) {
      announcement = bottomCollapsed
        ? workbenchI18n.bottomPanelCollapsed
        : workbenchI18n.bottomPanelExpanded;
    }

    previousPanelCollapsedStateRef.current = {
      initialized: true,
      leftCollapsed,
      rightCollapsed,
      bottomCollapsed,
    };

    if (!announcement || layoutPresetHydratedRef.current !== currentRouteKey) {
      return;
    }
    announceForScreenReader(announcement);
  }, [
    announceForScreenReader,
    bottomCollapsed,
    currentRouteKey,
    leftCollapsed,
    rightCollapsed,
  ]);

  useEffect(() => {
    const currentStatus = currentRouteSnapshot.status as WorkbenchRouteLoadingStatus;
    if (previousRouteStatusRef.current === currentStatus) {
      return;
    }
    previousRouteStatusRef.current = currentStatus;

    const routeLabel = menuTitleMap.get(currentRouteKey) ?? currentRouteKey;
    if (currentStatus === 'loading') {
      announceForScreenReader(workbenchI18n.routeLoading(routeLabel));
      return;
    }
    if (currentStatus === 'ready') {
      announceForScreenReader(workbenchI18n.routeReady(routeLabel));
      return;
    }
    if (currentStatus === 'timeout') {
      announceForScreenReader(workbenchI18n.routeTimeout(routeLabel));
      return;
    }
    if (currentStatus === 'error') {
      announceForScreenReader(workbenchI18n.routeFailed(routeLabel));
    }
  }, [announceForScreenReader, currentRouteKey, currentRouteSnapshot.status, menuTitleMap]);

  const handleResizeKeyDown =
    (side: WorkbenchResizableSide) => (event: React.KeyboardEvent<HTMLDivElement>) => {
      const step = event.shiftKey
        ? WORKBENCH_RESIZE_KEYBOARD_LARGE_STEP
        : WORKBENCH_RESIZE_KEYBOARD_STEP;
      const maxBottom = resolveBottomPanelMaxHeight();
      const leftSnapPoints = resolveResizeSnapPoints('left', maxBottom);
      const rightSnapPoints = resolveResizeSnapPoints('right', maxBottom);
      const bottomSnapPoints = resolveResizeSnapPoints('bottom', maxBottom);

      if (event.key === 'Enter') {
        event.preventDefault();
        resetResizablePanelSize(side, maxBottom);
        return;
      }

      if (event.key === 'Home') {
        event.preventDefault();
        if (side === 'left') {
          setLeftCollapsed(false);
          setLeftWidth(WORKBENCH_LEFT_PANEL_MIN_WIDTH);
          return;
        }
        if (side === 'right') {
          setRightCollapsed(false);
          setRightWidth(WORKBENCH_RIGHT_PANEL_MIN_WIDTH);
          return;
        }
        setBottomCollapsed(false);
        setBottomHeight(WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT);
        return;
      }

      if (event.key === 'End') {
        event.preventDefault();
        if (side === 'left') {
          setLeftCollapsed(false);
          setLeftWidth(WORKBENCH_LEFT_PANEL_MAX_WIDTH);
          return;
        }
        if (side === 'right') {
          setRightCollapsed(false);
          setRightWidth(WORKBENCH_RIGHT_PANEL_MAX_WIDTH);
          return;
        }
        setBottomCollapsed(false);
        setBottomHeight(maxBottom);
        return;
      }

      if (side === 'left' && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) {
        event.preventDefault();
        setLeftCollapsed(false);
        setLeftWidth((current) =>
          resolveSnappedPanelSize(
            clampWorkbenchPanelSize(
              current + (event.key === 'ArrowRight' ? step : -step),
              WORKBENCH_LEFT_PANEL_MIN_WIDTH,
              WORKBENCH_LEFT_PANEL_MAX_WIDTH,
            ),
            leftSnapPoints,
          ),
        );
        return;
      }

      if (side === 'right' && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) {
        event.preventDefault();
        setRightCollapsed(false);
        setRightWidth((current) =>
          resolveSnappedPanelSize(
            clampWorkbenchPanelSize(
              current + (event.key === 'ArrowLeft' ? step : -step),
              WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
              WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
            ),
            rightSnapPoints,
          ),
        );
        return;
      }

      if (side === 'bottom' && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
        event.preventDefault();
        setBottomCollapsed(false);
        setBottomHeight((current) =>
          resolveSnappedPanelSize(
            clampWorkbenchPanelSize(
              current + (event.key === 'ArrowUp' ? step : -step),
              WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
              maxBottom,
            ),
            bottomSnapPoints,
          ),
        );
      }
    };

  const leftPanelWidth = leftCollapsed ? collapsedWidth : leftWidth;
  const rightPanelWidth = rightCollapsed ? collapsedWidth : rightWidth;
  const effectiveBottomHeight = bottomCollapsed ? collapsedBottomHeight : bottomHeight;
  const bottomPanelMaxHeight = resolveBottomPanelMaxHeight();
  const bottomAriaValue = bottomCollapsed
    ? WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT
    : clampWorkbenchPanelSize(
        bottomHeight,
        WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
        bottomPanelMaxHeight,
      );
  const resizeIndicatorLabel = resizeIndicator
    ? buildResizeIndicatorLabel(resizeIndicator.side, resizeIndicator.size)
    : '';
  const layoutPresetStatusText = workbenchI18n.layoutPresetStatus(
    resolveLayoutPresetLabel(layoutPreset),
  );
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
          label: workbenchI18n.panelPlaceholder.filtersTab,
          content: (
            <div className="workbench-left-placeholder">
              <Empty description={workbenchI18n.panelPlaceholder.filtersPanelNotReady} />
            </div>
          ),
        },
        {
          key: 'query',
          label: workbenchI18n.panelPlaceholder.quickQueryTab,
          content: (
            <div className="workbench-left-placeholder">
              <Empty description={workbenchI18n.panelPlaceholder.quickQueryNotReady} />
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
          label: workbenchI18n.panelPlaceholder.tagsTab,
          content: (
            <div className="workbench-empty">
              <Empty description={workbenchI18n.panelPlaceholder.tagsPanelNotReady} />
            </div>
          ),
        },
        {
          key: 'common',
          label: workbenchI18n.panelPlaceholder.commonActionsTab,
          content: (
            <div className="workbench-empty">
              <Empty description={workbenchI18n.panelPlaceholder.commonActionsNotReady} />
            </div>
          ),
        },
        {
          key: 'detail',
          label: workbenchI18n.panelPlaceholder.detailsTab,
          content: (
            <div className="workbench-empty">
              <Empty description={workbenchI18n.panelPlaceholder.detailsPanelNotReady} />
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
        {
          key: 'sub-1',
          label: workbenchI18n.panelPlaceholder.logsTab,
          content: <Empty description={workbenchI18n.panelPlaceholder.logsPanelNotReady} />,
        },
        {
          key: 'sub-2',
          label: workbenchI18n.panelPlaceholder.activitiesTab,
          content: <Empty description={workbenchI18n.panelPlaceholder.activitiesPanelNotReady} />,
        },
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

  const handleCommandInputKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLInputElement>) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        setCommandPaletteOpen(false);
        return;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        const firstCommand = filteredWorkbenchCommands[0];
        if (!firstCommand) {
          return;
        }
        runWorkbenchCommand(firstCommand);
      }
    },
    [filteredWorkbenchCommands, runWorkbenchCommand],
  );

  return (
    <div
      className="workbench-shell"
      data-workbench-layout-preset={layoutPreset}
      data-workbench-left-collapsed={leftCollapsed ? 'true' : 'false'}
      data-workbench-right-collapsed={rightCollapsed ? 'true' : 'false'}
      data-workbench-bottom-collapsed={bottomCollapsed ? 'true' : 'false'}
      data-workbench-left-width={Math.round(leftWidth)}
      data-workbench-right-width={Math.round(rightWidth)}
      data-workbench-bottom-height={Math.round(bottomHeight)}
    >
      <a className="workbench-skip-link" href="#workbench-main-content">
        {workbenchI18n.skipToMainContent}
      </a>
      <div
        className="workbench-sr-live-region"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {screenReaderAnnouncement}
      </div>
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
          disableCtrlKSearchHotkey
        />
        <div className="workbench-main-toolbar">
          <ShortcutToolbar menuItems={menuRoutes} />
          <div
            className="workbench-layout-presets"
            role="group"
            aria-label={workbenchI18n.layoutPresetGroup}
          >
            <Button
              size="small"
              type={layoutPreset === 'compact' ? 'primary' : 'default'}
              data-testid="workbench-layout-preset-compact"
              aria-keyshortcuts="Alt+Shift+1"
              onClick={() => applyLayoutPreset('compact')}
            >
              {workbenchI18n.layoutPreset.compact}
            </Button>
            <Button
              size="small"
              type={layoutPreset === 'default' ? 'primary' : 'default'}
              data-testid="workbench-layout-preset-default"
              aria-keyshortcuts="Alt+Shift+2"
              onClick={() => applyLayoutPreset('default')}
            >
              {workbenchI18n.layoutPreset.default}
            </Button>
            <Button
              size="small"
              type={layoutPreset === 'analysis' ? 'primary' : 'default'}
              data-testid="workbench-layout-preset-analysis"
              aria-keyshortcuts="Alt+Shift+3"
              onClick={() => applyLayoutPreset('analysis')}
            >
              {workbenchI18n.layoutPreset.analysis}
            </Button>
          </div>
          <div
            className="workbench-layout-preset-status"
            data-testid="workbench-layout-preset-status"
            aria-live="polite"
          >
            {layoutPresetStatusText}
          </div>
          <Button
            size="small"
            className="workbench-toolbar-focus-search"
            data-testid="workbench-focus-search-trigger"
            aria-keyshortcuts="Control+K"
            onClick={openCommandPalette}
          >
            {workbenchI18n.commandPaletteTrigger}
          </Button>
          <Button
            size="small"
            className="workbench-toolbar-focus-search"
            data-testid="workbench-shortcut-help-trigger"
            aria-keyshortcuts="Control+/"
            onClick={openShortcutHelp}
          >
            {workbenchI18n.shortcutHelpTrigger}
          </Button>
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
            <div
              className="workbench-resizer"
              data-testid="workbench-left-resizer"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize left panel width"
              aria-valuemin={WORKBENCH_LEFT_PANEL_MIN_WIDTH}
              aria-valuemax={WORKBENCH_LEFT_PANEL_MAX_WIDTH}
              aria-valuenow={leftCollapsed ? WORKBENCH_LEFT_PANEL_MIN_WIDTH : leftWidth}
              tabIndex={0}
              onPointerDown={handleResizeStart('left')}
              onKeyDown={handleResizeKeyDown('left')}
              onDoubleClick={handleResizeReset('left')}
            >
              <Tooltip title={leftCollapsed ? 'Expand left panel' : 'Collapse left panel'}>
                <Button
                  size="small"
                  type="text"
                  className="workbench-resizer-toggle"
                  icon={leftCollapsed ? <CaretRightOutlined /> : <CaretLeftOutlined />}
                  aria-label={leftCollapsed ? 'Expand left panel' : 'Collapse left panel'}
                  onPointerDown={(event) => event.stopPropagation()}
                  onClick={() => setLeftCollapsed((prev) => !prev)}
                />
              </Tooltip>
            </div>
          </>
        ) : null}
        <main
          id="workbench-main-content"
          className="workbench-main"
          role="main"
          aria-label="Workbench main content"
        >
          {readOnlyFlag.enabled && (
            <Alert
              banner
              showIcon
              type="warning"
              message="This route has not connected a workbench top hint area yet. Showing placeholder content."
              style={{ margin: '12px 16px 0' }}
            />
          )}
          <div className="workbench-main-content">
            <div className="workbench-main-split" ref={mainSplitRef}>
              <section className="workbench-main-top">
                <div
                  key={`${currentRouteKey}-${activeRouteRefreshToken}`}
                  ref={mainRouteRegionRef}
                  tabIndex={-1}
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
                    data-testid="workbench-bottom-resizer"
                    role="separator"
                    aria-orientation="horizontal"
                    aria-label="Resize bottom panel height"
                    aria-valuemin={WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT}
                    aria-valuemax={bottomPanelMaxHeight}
                    aria-valuenow={bottomAriaValue}
                    tabIndex={0}
                    onPointerDown={handleResizeStart('bottom')}
                    onKeyDown={handleResizeKeyDown('bottom')}
                    onDoubleClick={handleResizeReset('bottom')}
                  >
                    <Tooltip title={bottomCollapsed ? 'Expand bottom panel' : 'Collapse bottom panel'}>
                      <Button
                        size="small"
                        type="text"
                        className="workbench-resizer-horizontal-toggle"
                        icon={bottomCollapsed ? <CaretUpOutlined /> : <CaretDownOutlined />}
                        aria-label={bottomCollapsed ? 'Expand bottom panel' : 'Collapse bottom panel'}
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
            <div
              className="workbench-resizer"
              data-testid="workbench-right-resizer"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize right panel width"
              aria-valuemin={WORKBENCH_RIGHT_PANEL_MIN_WIDTH}
              aria-valuemax={WORKBENCH_RIGHT_PANEL_MAX_WIDTH}
              aria-valuenow={rightCollapsed ? WORKBENCH_RIGHT_PANEL_MIN_WIDTH : rightWidth}
              tabIndex={0}
              onPointerDown={handleResizeStart('right')}
              onKeyDown={handleResizeKeyDown('right')}
              onDoubleClick={handleResizeReset('right')}
            >
              <Tooltip title={rightCollapsed ? 'Expand right panel' : 'Collapse right panel'}>
                <Button
                  size="small"
                  type="text"
                  className="workbench-resizer-toggle"
                  icon={rightCollapsed ? <CaretLeftOutlined /> : <CaretRightOutlined />}
                  aria-label={rightCollapsed ? 'Expand right panel' : 'Collapse right panel'}
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
        {resizeIndicator ? (
          <div
            className="workbench-resize-indicator"
            style={{
              left: resizeIndicator.x,
              top: resizeIndicator.y,
            }}
          >
            {resizeIndicatorLabel}
          </div>
        ) : null}
        <Modal
          open={commandPaletteOpen}
          title="Command Palette"
          onCancel={() => setCommandPaletteOpen(false)}
          footer={null}
          width={640}
          destroyOnHidden
        >
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Input
              ref={commandInputRef}
              value={commandKeyword}
              onChange={(event) => setCommandKeyword(event.target.value)}
              onKeyDown={handleCommandInputKeyDown}
              placeholder="Type command name, description, or shortcut (e.g. Ctrl+R, layout, search)"
              allowClear
            />
            <div className="workbench-command-list" role="listbox" aria-label="Command list">
              {filteredWorkbenchCommands.length ? (
                filteredWorkbenchCommands.map((command) => (
                  <button
                    type="button"
                    key={command.id}
                    className="workbench-command-item"
                    onClick={() => runWorkbenchCommand(command)}
                  >
                    <span className="workbench-command-item-main">
                      <span className="workbench-command-item-title">{command.label}</span>
                      <span className="workbench-command-item-desc">{command.description}</span>
                    </span>
                    <span className="workbench-command-item-shortcut">{command.shortcut}</span>
                  </button>
                ))
              ) : (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description="No matching command found. Try another keyword."
                />
              )}
            </div>
          </Space>
        </Modal>
        <Modal
          open={shortcutHelpOpen}
          title={workbenchI18n.shortcutHelpTitle}
          onCancel={() => setShortcutHelpOpen(false)}
          footer={
            <Button type="primary" onClick={() => setShortcutHelpOpen(false)}>
              {workbenchI18n.shortcutHelpClose}
            </Button>
          }
          width={680}
          destroyOnHidden
        >
          <div className="workbench-shortcut-help-list">
            {workbenchI18n.shortcutHelpRows.map(([shortcut, description]) => (
              <div key={shortcut} className="workbench-shortcut-help-row">
                <Typography.Text code>{shortcut}</Typography.Text>
                <Typography.Text>{description}</Typography.Text>
              </div>
            ))}
          </div>
        </Modal>
      </div>
    </div>
  );
};
