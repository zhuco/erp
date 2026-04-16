import { Collapse, Empty, Tabs, Typography } from 'antd';
import type { CollapseProps } from 'antd';
import type { ReactNode } from 'react';
import { useEffect, useMemo } from 'react';

import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';

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
        label: '鏍囪',
        children: (
          <div className="workbench-detail-block">
            <Empty description="鏆傛棤鏍囪" />
          </div>
        )
      },
      {
        key: 'common',
        label: '甯哥敤鍔熻兘',
        children: (
          <div className="workbench-detail-block">
            <Empty description="鏆傛棤蹇嵎鍔熻兘" />
          </div>
        )
      },
      {
        key: 'detail',
        label: '璇︽儏',
        children: (
          <div className="workbench-detail-block">
            {detail ?? <Empty description="鏆傛棤璇︽儏鍐呭" />}
          </div>
        )
      }
    ];
  }, [detail, detailGroups]);

  const resolvedLeftPanel = useMemo(() => {
    const filterContent =
      filter ?? left ?? (
        <div className="workbench-left-placeholder">
          <Typography.Text type="secondary">鏆傛棤榛樿杩囨护</Typography.Text>
        </div>
      );
    const queryContent =
      query ?? (
        <div className="workbench-left-placeholder">
          <Typography.Text type="secondary">鏆傛棤琛ㄦ牸鏌ヨ鏉′欢</Typography.Text>
        </div>
      );
    return (
      <Tabs
        size="small"
        className="workbench-left-tabs"
        items={[
          { key: 'filters', label: '榛樿杩囨护', children: filterContent },
          { key: 'query', label: '琛ㄦ牸鏌ヨ', children: queryContent }
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
        <Typography.Text type="secondary">璇烽厤缃簳閮ㄥ瓙琛?Tabs</Typography.Text>
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
    <WorkbenchMainTableTemplate
      className={`workbench-list-page ${className ?? ''}`}
      toolbarLeft={toolbarLeft || toolbarRight ? toolbarLeft : toolbar}
      toolbarRight={toolbarLeft || toolbarRight ? toolbarRight : undefined}
      tableClassName="workbench-list-table"
      tableContent={table}
    />
  );
};

