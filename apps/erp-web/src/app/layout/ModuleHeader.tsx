import { AutoComplete, Avatar, Button, Dropdown, Input, Popover, Space, Tooltip } from 'antd';
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

export type ModuleLinkItem = {
  key: string;
  label: string;
  path: string;
  testId?: string;
};

export type ModuleGroupItem = {
  key: string;
  label: string;
  items: ModuleLinkItem[];
};

type ModuleItem = {
  key: string;
  label: string;
  groups?: ModuleGroupItem[];
  defaultPath?: string;
};

export type FunctionSearchOption = NonNullable<SelectProps['options']>[number];

type ModuleHeaderProps = {
  logo?: ReactNode;
  modules: ModuleItem[];
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

const fallbackModules: ModuleItem[] = [
  { key: 'home', label: '首页', defaultPath: '/dashboard' },
  { key: 'channels', label: '渠道', groups: [] },
  { key: 'sales', label: '销售', groups: [] },
  { key: 'products', label: '商品', groups: [] },
  { key: 'procurement', label: '采购', groups: [] },
  { key: 'planning', label: '计划', groups: [] },
  { key: 'warehouse', label: '仓储', groups: [] },
  { key: 'logistics', label: '物流', groups: [] },
  { key: 'customers', label: '客户', groups: [] },
  { key: 'finance', label: '财务', groups: [] },
  { key: 'marketing', label: '营销', groups: [] },
  { key: 'analytics', label: '分析', groups: [] },
  { key: 'integrations', label: '集成', groups: [] },
  { key: 'system', label: '系统', groups: [] },
];

const ModulePanel = ({
  groups,
  currentPath,
  onSelect,
}: {
  groups: ModuleGroupItem[];
  currentPath?: string;
  onSelect: (path: string) => void;
}) => {
  return (
    <div className="workbench-module-panel" data-testid="workbench-module-panel">
      {groups.map((group) => (
        <section className="workbench-module-group" key={group.key}>
          <div className="workbench-module-group-title">{group.label}</div>
          <div className="workbench-module-group-links">
            {group.items.map((item) =>
              (() => {
                const isActive =
                  currentPath === item.path ||
                  (currentPath ? currentPath.startsWith(`${item.path}/`) : false);
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
              })(),
            )}
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
  const searchInputRef = useRef<InputRef | null>(null);
  const searchBlurTimerRef = useRef<number | null>(null);

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
  }, [activeModuleKey, closeSearchDropdown]);

  useEffect(() => () => clearSearchBlurTimer(), [clearSearchBlurTimer]);

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
    [
      clearSearchBlurTimer,
      handleSearchInputKeyDown,
      openSearchDropdown,
      searchValue,
    ],
  );

  return (
    <div className="workbench-header-row workbench-header-primary">
      <div className="workbench-logo">{logo ?? '衡策云'}</div>
      <div className="workbench-module-menu">
        {resolvedModules.map((module) => {
          const groups = module.groups ?? [];
          const hasGroups = groups.length > 0;
          const isOpen = openModuleKey === module.key;
          const moduleLeafPaths = Array.from(
            new Set(
              groups.flatMap((group) => group.items.map((item) => item.path)).concat(
                module.defaultPath ? [module.defaultPath] : [],
              ),
            ),
          );
          const button = (
            <button
              key={module.key}
              className={`workbench-module-item ${
                activeModuleKey === module.key || isOpen ? 'active' : ''
              }`}
              onClick={() => {
                if (!hasGroups && module.defaultPath) {
                  onModuleClick(module.defaultPath);
                }
              }}
              onMouseEnter={() => {
                if (moduleLeafPaths.length > 0) {
                  onModuleIntent?.(moduleLeafPaths, 'hover');
                }
              }}
              onFocus={() => {
                if (moduleLeafPaths.length > 0) {
                  onModuleIntent?.(moduleLeafPaths, 'hover');
                }
              }}
              type="button"
              disabled={!hasGroups && !module.defaultPath}
              aria-expanded={hasGroups ? isOpen : undefined}
            >
              {module.label}
            </button>
          );

          if (!hasGroups) {
            return button;
          }

          return (
            <Popover
              key={module.key}
              trigger="click"
              placement="bottomLeft"
              rootClassName="workbench-module-popover"
              overlayClassName="workbench-module-popover"
              open={isOpen}
              onOpenChange={(nextOpen) => {
                setOpenModuleKey(nextOpen ? module.key : null);
                if (nextOpen && moduleLeafPaths.length > 0) {
                  onModuleIntent?.(moduleLeafPaths, 'open');
                }
              }}
              content={
                <ModulePanel
                  groups={groups}
                  currentPath={currentPath}
                  onSelect={(path) => {
                    setOpenModuleKey(null);
                    onModuleClick(path);
                  }}
                />
              }
            >
              {button}
            </Popover>
          );
        })}
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
                onClick={onToggleDense}
                className={`dense-toggle ${denseMode ? 'active' : ''}`}
                aria-pressed={denseMode}
              />
            </Tooltip>
          ) : null}
          <Tooltip title="通知">
            <Button size="small" type="text" icon={<BellOutlined />} />
          </Tooltip>
          <Tooltip title="设置">
            <Button size="small" type="text" icon={<SettingOutlined />} />
          </Tooltip>
          <Tooltip title="退出登录">
            <Button size="small" type="text" icon={<LogoutOutlined />} onClick={onLogout} />
          </Tooltip>
        </Space>
        <Dropdown menu={{ items: userMenuItems, onClick: onUserMenuClick }}>
          <Space size={6} style={{ cursor: 'pointer' }}>
            <Avatar size={24}>{userName.slice(0, 1).toUpperCase()}</Avatar>
            <span>{userName}</span>
            <DownOutlined />
          </Space>
        </Dropdown>
      </div>
    </div>
  );
};
