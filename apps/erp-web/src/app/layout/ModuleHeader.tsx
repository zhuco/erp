import { AutoComplete, Avatar, Button, Dropdown, Input, Space, Tooltip } from 'antd';
import type { InputRef, MenuProps, SelectProps } from 'antd';
import {
  BellOutlined,
  ColumnHeightOutlined,
  DownOutlined,
  LogoutOutlined,
  SearchOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react';

import type { HeaderModuleGroupItem, HeaderModuleItem } from '@/app/layout/headerMenuModel';
import {
  resolveSharedAnchorRect,
  resolveSharedPanelGeometry,
  resolveSharedPanelMaxHeight,
  resolveSharedPanelToggle,
  resolveSharedPanelVisibleHeight,
  SHARED_SUBMENU_VERTICAL_GAP,
  type SharedAnchorRect,
} from '@/app/layout/headerSharedSubmenu';

export type FunctionSearchOption = NonNullable<SelectProps['options']>[number];

type ModuleHeaderProps = {
  logo?: ReactNode;
  modules: HeaderModuleItem[];
  activeModuleKey?: string;
  currentPath?: string;
  onModuleClick: (path: string) => void;
  onModuleIntent?: (paths: string[], source: 'hover' | 'open') => void;
  userName: string;
  userMenuItems: MenuProps['items'];
  onUserMenuClick: MenuProps['onClick'];
  onLogout: () => void;
  denseMode?: boolean;
  onToggleDense?: () => void;
  searchValue: string;
  searchOptions: FunctionSearchOption[];
  onSearchChange: (value: string) => void;
  onSearchSelect: (path: string) => void;
  onSearchEnter?: () => void;
};

const PANEL_VIEWPORT_GAP = 24;

const fallbackModules: HeaderModuleItem[] = [
  { key: 'dashboard', label: '首页', defaultPath: '/dashboard', leafPaths: ['/dashboard'], groups: [] },
  { key: 'channels', label: '渠道', leafPaths: [], groups: [] },
  { key: 'sales', label: '销售', leafPaths: [], groups: [] },
  { key: 'products', label: '商品', leafPaths: [], groups: [] },
  { key: 'procurement', label: '采购', leafPaths: [], groups: [] },
  { key: 'planning', label: '计划', leafPaths: [], groups: [] },
  { key: 'wms', label: '仓储', leafPaths: [], groups: [] },
  { key: 'logistics', label: '物流', leafPaths: [], groups: [] },
  { key: 'customers', label: '客户', leafPaths: [], groups: [] },
  { key: 'finance', label: '财务', leafPaths: [], groups: [] },
  { key: 'marketing', label: '营销', leafPaths: [], groups: [] },
  { key: 'stats', label: '分析', leafPaths: [], groups: [] },
  { key: 'integrations', label: '集成', leafPaths: [], groups: [] },
  { key: 'system', label: '系统', leafPaths: [], groups: [] },
];

const isModulePathActive = (currentPath: string | undefined, path: string) => {
  if (!currentPath) {
    return false;
  }
  return currentPath === path || currentPath.startsWith(`${path}/`);
};

const readAnchorRect = (element: HTMLElement | null): SharedAnchorRect | null => {
  if (!element) {
    return null;
  }
  const rect = element.getBoundingClientRect();
  return resolveSharedAnchorRect({
    bottom: rect.bottom,
    height: rect.height,
    left: rect.left,
    right: rect.right,
    top: rect.top,
    width: rect.width,
  });
};

const ModulePanel = ({
  contentRef,
  currentPath,
  groups,
  maxHeight,
  onSelect,
}: {
  contentRef: React.RefObject<HTMLDivElement>;
  currentPath?: string;
  groups: HeaderModuleGroupItem[];
  maxHeight?: number;
  onSelect: (path: string) => void;
}) => {
  return (
    <div
      ref={contentRef}
      className="workbench-module-panel"
      data-testid="workbench-module-panel"
      style={maxHeight ? { maxHeight } : undefined}
    >
      {groups.map((group) => (
        <section className="workbench-module-group" key={group.key}>
          <div className="workbench-module-group-title">{group.label}</div>
          <div className="workbench-module-group-links">
            {group.items.map((item) => {
              const isActive = isModulePathActive(currentPath, item.path);
              return (
                <button
                  key={item.key}
                  type="button"
                  className={`workbench-module-link ${isActive ? 'is-active' : ''}`}
                  onClick={() => onSelect(item.path)}
                  data-testid={item.testId}
                  aria-current={isActive ? 'page' : undefined}
                >
                  {item.label}
                </button>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
};

export const ModuleHeader = ({
  logo,
  modules,
  activeModuleKey,
  currentPath,
  onModuleClick,
  onModuleIntent,
  userName,
  userMenuItems,
  onUserMenuClick,
  onLogout,
  denseMode,
  onToggleDense,
  searchValue,
  searchOptions,
  onSearchChange,
  onSearchSelect,
  onSearchEnter,
}: ModuleHeaderProps) => {
  const resolvedModules = modules.length ? modules : fallbackModules;
  const [openModuleKey, setOpenModuleKey] = useState<string | null>(null);
  const [searchFocused, setSearchFocused] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [panelMetrics, setPanelMetrics] = useState<{
    left: number;
    right: number;
    top: number;
    width: number;
    maxHeight: number;
  } | null>(null);
  const [panelContentHeight, setPanelContentHeight] = useState(0);
  const searchInputRef = useRef<InputRef | null>(null);
  const searchBlurTimerRef = useRef<number | null>(null);
  const moduleMenuShellRef = useRef<HTMLDivElement | null>(null);
  const panelContentRef = useRef<HTMLDivElement>(null);

  const clearSearchBlurTimer = useCallback(() => {
    if (searchBlurTimerRef.current === null) {
      return;
    }
    window.clearTimeout(searchBlurTimerRef.current);
    searchBlurTimerRef.current = null;
  }, []);

  const openSearchDropdown = useCallback(() => {
    clearSearchBlurTimer();
    setSearchOpen(true);
  }, [clearSearchBlurTimer]);

  const closeSearchDropdown = useCallback(
    (shouldBlur = false) => {
      clearSearchBlurTimer();
      setSearchOpen(false);
      setSearchFocused(false);
      if (shouldBlur) {
        searchInputRef.current?.blur();
      }
    },
    [clearSearchBlurTimer],
  );

  const focusSearchInput = useCallback(() => {
    setSearchFocused(true);
    openSearchDropdown();
    window.requestAnimationFrame(() => {
      searchInputRef.current?.focus();
    });
  }, [openSearchDropdown]);

  const syncPanelMetrics = useCallback(() => {
    const anchorRect = readAnchorRect(moduleMenuShellRef.current);
    const geometry = resolveSharedPanelGeometry(anchorRect);
    if (!geometry) {
      setPanelMetrics(null);
      return;
    }

    setPanelMetrics({
      ...geometry,
      maxHeight: resolveSharedPanelMaxHeight(window.innerHeight, geometry.top, PANEL_VIEWPORT_GAP),
    });
  }, []);

  const openModule = useMemo(
    () => resolvedModules.find((module) => module.key === openModuleKey) ?? null,
    [openModuleKey, resolvedModules],
  );

  const visiblePanelHeight = useMemo(() => {
    if (!openModule) {
      return 0;
    }
    return resolveSharedPanelVisibleHeight(
      panelContentHeight,
      panelMetrics?.maxHeight ?? panelContentHeight,
    );
  }, [openModule, panelContentHeight, panelMetrics]);

  const handleSearchInputKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        closeSearchDropdown(true);
        return;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        onSearchEnter?.();
      }
    },
    [closeSearchDropdown, onSearchEnter],
  );

  useEffect(() => {
    setOpenModuleKey(null);
    closeSearchDropdown();
  }, [closeSearchDropdown, currentPath]);

  useEffect(() => () => clearSearchBlurTimer(), [clearSearchBlurTimer]);

  useEffect(() => {
    if (!openModuleKey) {
      setPanelMetrics(null);
      return;
    }

    syncPanelMetrics();
    const shell = moduleMenuShellRef.current;
    if (!shell) {
      return;
    }

    const observer = new ResizeObserver(syncPanelMetrics);
    observer.observe(shell);
    window.addEventListener('resize', syncPanelMetrics);

    return () => {
      observer.disconnect();
      window.removeEventListener('resize', syncPanelMetrics);
    };
  }, [openModuleKey, syncPanelMetrics]);

  useEffect(() => {
    const content = panelContentRef.current;
    if (!openModule || !content) {
      setPanelContentHeight(0);
      return;
    }

    const syncHeight = () => {
      setPanelContentHeight(content.scrollHeight);
    };

    syncHeight();
    const observer = new ResizeObserver(syncHeight);
    observer.observe(content);
    window.addEventListener('resize', syncHeight);

    return () => {
      observer.disconnect();
      window.removeEventListener('resize', syncHeight);
    };
  }, [openModule, panelMetrics?.width]);

  useEffect(() => {
    if (!openModuleKey) {
      return;
    }

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        return;
      }
      if (moduleMenuShellRef.current?.contains(target)) {
        return;
      }
      setOpenModuleKey(null);
    };

    document.addEventListener('pointerdown', handlePointerDown);
    return () => document.removeEventListener('pointerdown', handlePointerDown);
  }, [openModuleKey]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (
        !event.altKey &&
        !event.shiftKey &&
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === 'k'
      ) {
        event.preventDefault();
        focusSearchInput();
        return;
      }

      if (event.key !== 'Escape') {
        return;
      }

      if (openModuleKey) {
        setOpenModuleKey(null);
      }

      if (searchOpen || searchFocused) {
        closeSearchDropdown(true);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [closeSearchDropdown, focusSearchInput, openModuleKey, searchFocused, searchOpen]);

  const searchInput = useMemo(
    () => (
      <Input
        ref={searchInputRef}
        aria-label="Function search input"
        placeholder="搜功能"
        size="small"
        value={searchValue}
        onFocus={() => {
          setSearchFocused(true);
          openSearchDropdown();
        }}
        onBlur={() => {
          setSearchFocused(false);
          clearSearchBlurTimer();
          searchBlurTimerRef.current = window.setTimeout(() => {
            setSearchOpen(false);
            searchBlurTimerRef.current = null;
          }, 120);
        }}
        onKeyDown={handleSearchInputKeyDown}
      />
    ),
    [clearSearchBlurTimer, handleSearchInputKeyDown, openSearchDropdown, searchValue],
  );

  return (
    <div className="workbench-header-row workbench-header-primary">
      <div className="workbench-logo">{logo ?? '衡策云'}</div>
      <div
        className="workbench-module-menu-shell"
        ref={moduleMenuShellRef}
        data-testid="workbench-module-menu-anchor"
      >
        <div className="workbench-module-menu">
          {resolvedModules.map((module) => {
            const hasGroups = module.groups.length > 0;
            const isOpen = openModuleKey === module.key;
            const buttonClassName = `workbench-module-item ${
              activeModuleKey === module.key || isOpen ? 'active' : ''
            }`;

            return (
              <button
                key={module.key}
                type="button"
                className={buttonClassName}
                data-testid={`module-trigger-${module.key}`}
                aria-expanded={hasGroups ? isOpen : undefined}
                aria-controls={hasGroups ? 'workbench-shared-module-panel' : undefined}
                onClick={() => {
                  const nextState = resolveSharedPanelToggle(openModuleKey, module);
                  setOpenModuleKey(nextState.nextOpenModuleKey);

                  if (nextState.nextOpenModuleKey && module.leafPaths.length > 0) {
                    onModuleIntent?.(module.leafPaths, 'open');
                  }

                  if (nextState.navigateTo) {
                    onModuleClick(nextState.navigateTo);
                  }
                }}
                onMouseEnter={() => {
                  if (module.leafPaths.length > 0) {
                    onModuleIntent?.(module.leafPaths, 'hover');
                  }
                }}
                onFocus={() => {
                  if (module.leafPaths.length > 0) {
                    onModuleIntent?.(module.leafPaths, 'hover');
                  }
                }}
                disabled={!hasGroups && !module.defaultPath}
              >
                {module.label}
              </button>
            );
          })}
        </div>
        <div
          id="workbench-shared-module-panel"
          className={`workbench-module-panel-shell${openModule ? ' is-open' : ''}`}
          data-open-module-key={openModule?.key ?? ''}
          data-panel-left={panelMetrics ? Math.round(panelMetrics.left) : undefined}
          data-panel-right={panelMetrics ? Math.round(panelMetrics.right) : undefined}
          data-panel-top={panelMetrics ? Math.round(panelMetrics.top) : undefined}
          data-panel-width={panelMetrics ? Math.round(panelMetrics.width) : undefined}
          data-panel-height={openModule ? Math.round(visiblePanelHeight) : 0}
          style={{
            height: openModule ? visiblePanelHeight : 0,
            opacity: openModule ? 1 : 0,
            top: `calc(100% + ${SHARED_SUBMENU_VERTICAL_GAP}px)`,
          }}
        >
          {openModule ? (
            <ModulePanel
              contentRef={panelContentRef}
              groups={openModule.groups}
              currentPath={currentPath}
              maxHeight={panelMetrics?.maxHeight}
              onSelect={(path) => {
                setOpenModuleKey(null);
                onModuleClick(path);
              }}
            />
          ) : null}
        </div>
      </div>
      <div className="workbench-function-search">
        <div className="workbench-function-search-control">
          <AutoComplete
            value={searchValue}
            options={searchOptions}
            filterOption={false}
            onInputKeyDown={handleSearchInputKeyDown}
            onChange={(value) => {
              openSearchDropdown();
              onSearchChange(value);
            }}
            onOpenChange={(nextOpen) => {
              if (nextOpen) {
                openSearchDropdown();
                return;
              }
              closeSearchDropdown();
            }}
            onSelect={(path) => {
              closeSearchDropdown();
              onSearchSelect(String(path));
            }}
            popupClassName="workbench-function-search-dropdown"
            className="workbench-function-search-auto"
            defaultActiveFirstOption
            transitionName=""
            notFoundContent={
              <div className="workbench-function-search-empty" data-testid="function-search-empty">
                未找到匹配功能，请尝试模块名、分组名或路由关键词
              </div>
            }
            open={searchOpen && (searchOptions.length > 0 || Boolean(searchValue.trim()))}
          >
            {searchInput}
          </AutoComplete>
          <Tooltip title="搜索">
            <Button
              size="small"
              type="default"
              icon={<SearchOutlined />}
              aria-label="Function search"
              className="workbench-function-search-button"
              data-testid="workbench-function-search-button"
              onClick={() => {
                if (searchValue.trim()) {
                  onSearchEnter?.();
                  return;
                }
                focusSearchInput();
              }}
            />
          </Tooltip>
        </div>
      </div>
      <div className="workbench-header-actions">
        <Space size={6}>
          {onToggleDense ? (
            <Tooltip title={denseMode ? '紧凑模式：开' : '紧凑模式：关'}>
              <Button
                size="small"
                type="text"
                icon={<ColumnHeightOutlined />}
                aria-label={denseMode ? 'Disable compact mode' : 'Enable compact mode'}
                onClick={onToggleDense}
                className={`dense-toggle ${denseMode ? 'active' : ''}`}
                aria-pressed={denseMode}
              />
            </Tooltip>
          ) : null}
          <Tooltip title="通知">
            <Button size="small" type="text" icon={<BellOutlined />} aria-label="Notifications" />
          </Tooltip>
          <Tooltip title="设置">
            <Button size="small" type="text" icon={<SettingOutlined />} aria-label="Settings" />
          </Tooltip>
          <Tooltip title="退出登录">
            <Button size="small" type="text" icon={<LogoutOutlined />} onClick={onLogout} aria-label="Sign out" />
          </Tooltip>
        </Space>
        <Dropdown menu={{ items: userMenuItems, onClick: onUserMenuClick }}>
          <Button type="text" className="workbench-user-trigger" aria-label={`User menu: ${userName}`}>
            <Space size={6}>
              <Avatar size={24}>{userName.slice(0, 1).toUpperCase()}</Avatar>
              <span>{userName}</span>
              <DownOutlined aria-hidden="true" />
            </Space>
          </Button>
        </Dropdown>
      </div>
    </div>
  );
};
