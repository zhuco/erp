import { SearchOutlined } from '@ant-design/icons';
import { Button, Input, Space } from 'antd';
import type { ReactNode } from 'react';

import { UiModule02ToolbarRight } from './UiModule02ToolbarRight';

type UiModule02ActionSearchToolbarProps = {
  className?: string;
  leftContent?: ReactNode;
  centerContent?: ReactNode;
  rightContent?: ReactNode;
  searchEnabled?: boolean;
  searchValue?: string;
  onSearchChange?: (value: string) => void;
  onSearch?: () => void;
  searchPlaceholder?: string;
  searchInputClassName?: string;
  searchButtonClassName?: string;
  searchInputTestId?: string;
  searchButtonTestId?: string;
  searchButtonAriaLabel?: string;
  canExport?: boolean;
  exporting?: boolean;
  onExport?: () => void;
  onPrint?: () => void;
  onHelp?: () => void;
  exportButtonTestId?: string;
  printButtonTestId?: string;
  helpButtonTestId?: string;
};

export const UiModule02ActionSearchToolbar = ({
  className,
  leftContent,
  centerContent,
  rightContent,
  searchEnabled = true,
  searchValue,
  onSearchChange,
  onSearch,
  searchPlaceholder,
  searchInputClassName,
  searchButtonClassName,
  searchInputTestId,
  searchButtonTestId,
  searchButtonAriaLabel = 'search',
  canExport = true,
  exporting = false,
  onExport,
  onPrint,
  onHelp,
  exportButtonTestId,
  printButtonTestId,
  helpButtonTestId,
}: UiModule02ActionSearchToolbarProps) => {
  const rootClassName = ['order-workbench-toolbar', className].filter(Boolean).join(' ');
  const inputClassName = [searchInputClassName].filter(Boolean).join(' ');
  const buttonClassName = [searchButtonClassName].filter(Boolean).join(' ');
  const resolvedSearchValue = searchValue ?? '';

  return (
    <div className={rootClassName}>
      <div className="order-workbench-toolbar-left">{leftContent}</div>

      <div className="order-workbench-toolbar-center">
        {centerContent ?? (searchEnabled ? (
          <Space size={8}>
            <Input
              size="small"
              className={inputClassName || undefined}
              data-testid={searchInputTestId}
              placeholder={searchPlaceholder}
              value={resolvedSearchValue}
              onChange={(event) => onSearchChange?.(event.target.value)}
              onPressEnter={() => onSearch?.()}
            />
            <Button
              size="small"
              type="primary"
              className={buttonClassName || undefined}
              icon={<SearchOutlined />}
              data-testid={searchButtonTestId}
              aria-label={searchButtonAriaLabel}
              onClick={() => onSearch?.()}
            />
          </Space>
        ) : null)}
      </div>

      <div className="order-workbench-toolbar-right">
        {rightContent ?? (
          <UiModule02ToolbarRight
            canExport={canExport}
            exporting={exporting}
            onExport={onExport}
            onPrint={onPrint}
            onHelp={onHelp}
            exportButtonTestId={exportButtonTestId}
            printButtonTestId={printButtonTestId}
            helpButtonTestId={helpButtonTestId}
          />
        )}
      </div>
    </div>
  );
};
