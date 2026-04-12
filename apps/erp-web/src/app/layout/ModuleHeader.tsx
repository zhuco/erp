import { Avatar, Button, Dropdown, Space, Tooltip } from 'antd';
import type { MenuProps } from 'antd';
import {
  BellOutlined,
  ColumnHeightOutlined,
  DownOutlined,
  LogoutOutlined,
  SettingOutlined
} from '@ant-design/icons';
import type { ReactNode } from 'react';

type ModuleItem = {
  key: string;
  label: string;
  menuItems?: MenuProps['items'];
  menuKeyToPath?: Map<string, string>;
  defaultPath?: string;
};

type ModuleHeaderProps = {
  logo?: ReactNode;
  modules: ModuleItem[];
  activeModuleKey?: string;
  onModuleClick: (path: string) => void;
  userName: string;
  userMenuItems: MenuProps['items'];
  onUserMenuClick: MenuProps['onClick'];
  onLogout: () => void;
  denseMode?: boolean;
  onToggleDense?: () => void;
};

const fallbackModules: ModuleItem[] = [
  { key: 'system', label: '系统', menuItems: [] },
  { key: 'directory', label: '目录', menuItems: [] },
  { key: 'product', label: '商品', menuItems: [] },
  { key: 'purchase', label: '采购', menuItems: [] },
  { key: 'sales', label: '销售', menuItems: [] },
  { key: 'inventory', label: '库存', menuItems: [] },
  { key: 'warehouse', label: '仓储', menuItems: [] },
  { key: 'customer', label: '客户', menuItems: [] },
  { key: 'service', label: '服务', menuItems: [] },
  { key: 'stats', label: '统计', menuItems: [] }
];

export const ModuleHeader = ({
  logo,
  modules,
  activeModuleKey,
  onModuleClick,
  userName,
  userMenuItems,
  onUserMenuClick,
  onLogout,
  denseMode,
  onToggleDense
}: ModuleHeaderProps) => {
  const resolvedModules = modules.length ? modules : fallbackModules;

  const handleModuleMenuClick =
    (module: ModuleItem): MenuProps['onClick'] =>
    ({ key }) => {
      const targetPath = module.menuKeyToPath?.get(String(key));
      if (targetPath) {
        onModuleClick(targetPath);
      }
    };

  return (
    <div className="workbench-header-row workbench-header-primary">
      <div className="workbench-logo">{logo ?? '衡策云工作台'}</div>
      <div className="workbench-module-menu">
        {resolvedModules.map((module) => {
          const menuItems = module.menuItems ?? [];
          const hasMenu = menuItems.length > 0;
          const button = (
            <button
              key={module.key}
              className={`workbench-module-item ${activeModuleKey === module.key ? 'active' : ''}`}
              onClick={() => {
                if (!hasMenu && module.defaultPath) {
                  onModuleClick(module.defaultPath);
                }
              }}
              type="button"
              disabled={!hasMenu && !module.defaultPath}
            >
              {module.label}
            </button>
          );

          if (!hasMenu) {
            return button;
          }

          return (
            <Dropdown
              key={module.key}
              menu={{ items: menuItems, onClick: handleModuleMenuClick(module) }}
              trigger={['click']}
              placement="bottomLeft"
            >
              {button}
            </Dropdown>
          );
        })}
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
