import {
  AppstoreOutlined,
  CarOutlined,
  ClockCircleOutlined,
  FolderOpenOutlined,
  FolderOutlined,
  MessageOutlined,
  MinusSquareOutlined,
  PieChartOutlined,
  PlusSquareOutlined,
  ShopOutlined,
  TagOutlined,
  TeamOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { CellClickedEvent, ColDef, ICellRendererParams } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';
import { Tabs, Typography } from 'antd';
import type { ReactNode } from 'react';
import { useMemo } from 'react';

import './UiModule08OrderReviewLeftPanelTemplate.css';

export type UiModule08LeftPanelTabKey = 'filter' | 'screen';

export type UiModule08OrderReviewLeftNodeType =
  | 'group'
  | 'platform'
  | 'shop'
  | 'time'
  | 'status'
  | 'exception'
  | 'assignee'
  | 'sla'
  | 'warehouse'
  | 'logistics'
  | 'remark'
  | 'tag';

export type UiModule08OrderReviewLeftRenderRow = {
  key: string;
  label: string;
  count?: number;
  nodeType: UiModule08OrderReviewLeftNodeType;
  level: number;
  hasChildren: boolean;
  expanded: boolean;
  checked: boolean;
};

export type UiModule08OrderReviewLeftStatsItem = {
  key: string;
  label: string;
  value: number;
  color: string;
};

type UiModule08OrderReviewLeftPanelTemplateProps = {
  leftTab: UiModule08LeftPanelTabKey;
  onLeftTabChange: (key: UiModule08LeftPanelTabKey) => void;
  leftRowsForRender: UiModule08OrderReviewLeftRenderRow[];
  onLeftRowClick: (event: CellClickedEvent<UiModule08OrderReviewLeftRenderRow>) => void;
  screenContent: ReactNode;
  leftStatsItems: UiModule08OrderReviewLeftStatsItem[];
  leftStatsPieBackground: string;
  globalOrderTotal: number;
  statsScopeText: ReactNode;
  className?: string;
  dataTestId?: string;
};

const LEFT_FILTER_PANEL_COPY = {
  filterColumn: '过滤',
  statsTitle: '过滤统计',
  totalOrders: '总订单',
} as const;

const joinClassNames = (...classNames: Array<string | undefined>) =>
  classNames.filter((item) => typeof item === 'string' && item.trim().length > 0).join(' ');

const resolveNodeIcon = (row: UiModule08OrderReviewLeftRenderRow): ReactNode => {
  if (row.nodeType === 'group') {
    return row.expanded ? <FolderOpenOutlined /> : <FolderOutlined />;
  }
  if (row.nodeType === 'shop') {
    return <ShopOutlined />;
  }
  if (row.nodeType === 'time') {
    return <ClockCircleOutlined />;
  }
  if (row.nodeType === 'exception') {
    return <WarningOutlined />;
  }
  if (row.nodeType === 'assignee') {
    return <TeamOutlined />;
  }
  if (row.nodeType === 'sla') {
    return <ClockCircleOutlined />;
  }
  if (row.nodeType === 'logistics') {
    return <CarOutlined />;
  }
  if (row.nodeType === 'remark') {
    return <MessageOutlined />;
  }
  if (row.nodeType === 'tag') {
    return <TagOutlined />;
  }
  return <AppstoreOutlined />;
};

export const UiModule08OrderReviewLeftPanelTemplate = ({
  leftTab,
  onLeftTabChange,
  leftRowsForRender,
  onLeftRowClick,
  screenContent,
  leftStatsItems,
  leftStatsPieBackground,
  globalOrderTotal,
  statsScopeText,
  className,
  dataTestId,
}: UiModule08OrderReviewLeftPanelTemplateProps) => {
  const leftColDefs = useMemo<ColDef<UiModule08OrderReviewLeftRenderRow>[]>(
    () => [
      {
        headerName: LEFT_FILTER_PANEL_COPY.filterColumn,
        field: 'label',
        flex: 1,
        sortable: false,
        filter: false,
        cellRenderer: (params: ICellRendererParams<UiModule08OrderReviewLeftRenderRow>) => {
          const row = params.data;
          if (!row) return null;

          return (
            <div className="order-review-left-ag-row" style={{ paddingLeft: `${row.level * 14}px` }}>
              <span className="order-review-left-ag-toggle">
                {row.hasChildren ? row.expanded ? <MinusSquareOutlined /> : <PlusSquareOutlined /> : null}
              </span>
              <span className="order-review-left-ag-node-icon">{resolveNodeIcon(row)}</span>
              <span className="order-review-left-ag-label">{row.label}</span>
              {typeof row.count === 'number' ? (
                <span className="order-review-left-ag-count" data-testid="order-review-left-node-count">
                  {row.count}
                </span>
              ) : null}
            </div>
          );
        },
      },
    ],
    [],
  );

  return (
    <div className={joinClassNames('order-review-left-ag-panel', className)} data-testid={dataTestId}>
      <Tabs
        size="small"
        activeKey={leftTab}
        onChange={(key) => onLeftTabChange(key as UiModule08LeftPanelTabKey)}
        items={[
          {
            key: 'filter',
            label: '过滤',
            children: (
              <div className="order-review-left-ag-filter-tab">
                <div className="ag-theme-quartz order-review-left-ag-grid-shell order-review-left-ag-filter-grid-area">
                  <AgGridReact<UiModule08OrderReviewLeftRenderRow>
                    theme="legacy"
                    rowData={leftRowsForRender}
                    columnDefs={leftColDefs}
                    defaultColDef={{
                      sortable: false,
                      filter: false,
                      editable: false,
                      resizable: false,
                    }}
                    getRowId={(params) => params.data.key}
                    rowHeight={30}
                    headerHeight={0}
                    sideBar={false}
                    cellSelection={false}
                    enableCellTextSelection={false}
                    animateRows={false}
                    rowBuffer={8}
                    onCellClicked={onLeftRowClick}
                    rowClassRules={{
                      'order-review-left-ag-row-checked': (params) => Boolean(params.data?.checked),
                    }}
                  />
                </div>

                <div className="order-review-left-ag-stats" data-testid="order-review-left-stats">
                  <div className="order-review-left-ag-stats-header">
                    <Typography.Text strong>{LEFT_FILTER_PANEL_COPY.statsTitle}</Typography.Text>
                    <Typography.Text type="secondary" data-testid="order-review-left-stats-total">
                      {LEFT_FILTER_PANEL_COPY.totalOrders} {globalOrderTotal}
                    </Typography.Text>
                  </div>
                  <Typography.Text type="secondary">{statsScopeText}</Typography.Text>
                  <div className="order-review-left-ag-stats-body">
                    <div className="order-review-left-ag-stats-pie-wrap">
                      <div
                        className="order-review-left-ag-stats-pie"
                        style={{ backgroundImage: leftStatsPieBackground }}
                        data-testid="order-review-left-stats-pie"
                      >
                        <span className="order-review-left-ag-stats-pie-center">
                          <PieChartOutlined />
                        </span>
                      </div>
                    </div>
                    <div className="order-review-left-ag-stats-legend">
                      {leftStatsItems.map((item) => (
                        <div className="order-review-left-ag-stats-legend-row" key={item.key}>
                          <span className="order-review-left-ag-stats-dot" style={{ backgroundColor: item.color }} />
                          <span className="order-review-left-ag-stats-label">{item.label}</span>
                          <span className="order-review-left-ag-stats-value">{item.value}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            ),
          },
          {
            key: 'screen',
            label: '筛选',
            children: <div className="order-review-left-ag-screen-tab">{screenContent}</div>,
          },
        ]}
      />
    </div>
  );
};
