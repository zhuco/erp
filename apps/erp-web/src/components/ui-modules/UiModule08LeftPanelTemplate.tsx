import { Input, Tabs, Typography } from 'antd';
import type { ReactNode } from 'react';

import './UiModule08LeftPanelTemplate.css';

export type UiModule08LeftPanelTabKey = 'filter' | 'screen';

type UiModule08LeftPanelTemplateProps = {
  activeKey: UiModule08LeftPanelTabKey;
  onChange: (key: UiModule08LeftPanelTabKey) => void;
  filterContent: ReactNode;
  screenContent: ReactNode;
  filterTabLabel?: ReactNode;
  screenTabLabel?: ReactNode;
  className?: string;
  dataTestId?: string;
};

type UiModule08LeftAgFilterTabProps = {
  title?: ReactNode;
  keyword: string;
  onKeywordChange: (value: string) => void;
  searchPlaceholder?: string;
  toolsExtra?: ReactNode;
  gridShellClassName?: string;
  className?: string;
  children: ReactNode;
};

type UiModule08LeftScreenTabProps = {
  title?: ReactNode;
  toolsExtra?: ReactNode;
  className?: string;
  children: ReactNode;
};

const joinClassNames = (...classNames: Array<string | undefined>) =>
  classNames.filter((item) => typeof item === 'string' && item.trim().length > 0).join(' ');

export const UiModule08LeftPanelTemplate = ({
  activeKey,
  onChange,
  filterContent,
  screenContent,
  filterTabLabel = '过滤',
  screenTabLabel = '筛选',
  className,
  dataTestId,
}: UiModule08LeftPanelTemplateProps) => (
  <div className={joinClassNames('ui-module-08-left-panel', className)} data-testid={dataTestId}>
    <Tabs
      size="small"
      activeKey={activeKey}
      onChange={(key) => onChange(key as UiModule08LeftPanelTabKey)}
      items={[
        { key: 'filter', label: filterTabLabel, children: filterContent },
        { key: 'screen', label: screenTabLabel, children: screenContent },
      ]}
    />
  </div>
);

export const UiModule08LeftAgFilterTab = ({
  title = '过滤条件',
  keyword,
  onKeywordChange,
  searchPlaceholder = '搜索过滤节点',
  toolsExtra,
  gridShellClassName,
  className,
  children,
}: UiModule08LeftAgFilterTabProps) => (
  <div className={joinClassNames('ui-module-08-left-filter-tab', className)}>
    <div className="ui-module-08-left-filter-tools">
      <Typography.Text strong>{title}</Typography.Text>
      {toolsExtra}
    </div>
    <Input.Search
      allowClear
      size="small"
      placeholder={searchPlaceholder}
      value={keyword}
      onChange={(event) => onKeywordChange(event.target.value)}
    />
    <div className={joinClassNames('ui-module-08-left-filter-grid-shell', gridShellClassName)}>{children}</div>
  </div>
);

export const UiModule08LeftScreenTab = ({
  title = '筛选条件',
  toolsExtra,
  className,
  children,
}: UiModule08LeftScreenTabProps) => (
  <div className={joinClassNames('ui-module-08-left-screen-tab', className)}>
    <div className="ui-module-08-left-screen-tools">
      <Typography.Text className="ui-module-08-left-screen-title">{title}</Typography.Text>
      {toolsExtra}
    </div>
    {children}
  </div>
);
