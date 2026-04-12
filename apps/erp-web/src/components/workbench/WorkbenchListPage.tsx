import { Collapse, Empty, Tabs, Typography } from 'antd';
import type { CollapseProps } from 'antd';
import type { ReactNode } from 'react';
import { useEffect, useMemo } from 'react';

import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { UiModule02WorkbenchTemplate } from '@/components/ui-modules';

import './workbenchListPage.css';

export type WorkbenchListPageProps = {
  left?: ReactNode;
  filter?: ReactNode;
  toolbar?: ReactNode;
  toolbarLeft?: ReactNode;
  toolbarRight?: ReactNode;
  table?: ReactNode;
  detail?: ReactNode;
  bottomTabs?: ReactNode;
  tabs?: ReactNode;
  query?: ReactNode;
  detailGroups?: CollapseProps['items'];
  showLeftPanel?: boolean;
  showRightPanel?: boolean;
  showBottomPanel?: boolean;
  className?: string;
};

export const WorkbenchListPage = ({
  left,
  filter,
  toolbar,
  toolbarLeft,
  toolbarRight,
  table,
  detail,
  bottomTabs,
  tabs,
  query,
  detailGroups,
  showLeftPanel = true,
  showRightPanel = true,
  showBottomPanel = true,
  className
}: WorkbenchListPageProps) => {
  const {
    setLeftPanel,
    setRightPanel,
    setBottomPanel,
    setLeftPanelVisible,
    setRightPanelVisible,
    setBottomPanelVisible,
    clearPanels
  } = useWorkbenchPanels();

  const leftVisible = showLeftPanel;
  const rightVisible = showRightPanel;
  const bottomVisible = showBottomPanel;

  const resolvedDetailGroups = useMemo<CollapseProps['items']>(() => {
    if (detailGroups?.length) {
      return detailGroups;
    }
    return [
      {
        key: 'tags',
        label: '标记',
        children: (
          <div className="workbench-detail-block">
            <Empty description="暂无标记" />
          </div>
        )
      },
      {
        key: 'common',
        label: '常用功能',
        children: (
          <div className="workbench-detail-block">
            <Empty description="暂无快捷功能" />
          </div>
        )
      },
      {
        key: 'detail',
        label: '详情',
        children: (
          <div className="workbench-detail-block">
            {detail ?? <Empty description="暂无详情内容" />}
          </div>
        )
      }
    ];
  }, [detail, detailGroups]);

  const resolvedLeftPanel = useMemo(() => {
    const filterContent =
      filter ?? left ?? (
        <div className="workbench-left-placeholder">
          <Typography.Text type="secondary">暂无默认过滤</Typography.Text>
        </div>
      );
    const queryContent =
      query ?? (
        <div className="workbench-left-placeholder">
          <Typography.Text type="secondary">暂无表格查询条件</Typography.Text>
        </div>
      );
    return (
      <Tabs
        size="small"
        className="workbench-left-tabs"
        items={[
          { key: 'filters', label: '默认过滤', children: filterContent },
          { key: 'query', label: '表格查询', children: queryContent }
        ]}
      />
    );
  }, [filter, left, query]);

  const resolvedBottomTabs = useMemo(() => {
    const resolved = bottomTabs ?? tabs;
    if (resolved) {
      return resolved;
    }
    return (
      <div className="workbench-bottom-placeholder">
        <Typography.Text type="secondary">请配置底部子表 Tabs</Typography.Text>
      </div>
    );
  }, [bottomTabs, tabs]);

  useEffect(() => {
    setLeftPanelVisible(leftVisible);
    setRightPanelVisible(rightVisible);
    setBottomPanelVisible(bottomVisible);
    setLeftPanel(leftVisible ? resolvedLeftPanel : undefined);
    setRightPanel(
      rightVisible ? (
        <Collapse
          size="small"
          defaultActiveKey={['tags', 'common', 'detail']}
          items={resolvedDetailGroups}
        />
      ) : undefined
    );
    setBottomPanel(bottomVisible ? resolvedBottomTabs : undefined);
    return () => clearPanels();
  }, [
    bottomVisible,
    clearPanels,
    leftVisible,
    resolvedBottomTabs,
    resolvedDetailGroups,
    resolvedLeftPanel,
    rightVisible,
    setBottomPanel,
    setBottomPanelVisible,
    setLeftPanel,
    setLeftPanelVisible,
    setRightPanel,
    setRightPanelVisible
  ]);

  return (
    <UiModule02WorkbenchTemplate
      className={`workbench-list-page ${className ?? ''}`}
      toolbarLeft={toolbarLeft || toolbarRight ? toolbarLeft : toolbar}
      toolbarRight={toolbarLeft || toolbarRight ? toolbarRight : undefined}
      tableClassName="workbench-list-table"
      tableContent={table}
    />
  );
};
