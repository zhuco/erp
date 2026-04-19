import type { CellClickedEvent } from 'ag-grid-community';
import { Typography } from 'antd';

import {
  WorkbenchLeftPanelTemplate,
  type WorkbenchLeftRenderRow,
  type WorkbenchLeftStatsItem,
} from '@/components/page-templates';

type OrderExceptionsLeftPanelProps = {
  leftTab: 'filter' | 'screen';
  onLeftTabChange: (key: 'filter' | 'screen') => void;
  leftRowsForRender: WorkbenchLeftRenderRow[];
  onLeftRowClick: (event: CellClickedEvent<WorkbenchLeftRenderRow>) => void;
  leftStatsItems: WorkbenchLeftStatsItem[];
  leftStatsPieBackground: string;
  globalOrderTotal: number;
  isFetching: boolean;
  hasManualFilterActive: boolean;
};

export const OrderExceptionsLeftPanel = ({
  leftTab,
  onLeftTabChange,
  leftRowsForRender,
  onLeftRowClick,
  leftStatsItems,
  leftStatsPieBackground,
  globalOrderTotal,
  isFetching,
  hasManualFilterActive,
}: OrderExceptionsLeftPanelProps) => (
  <WorkbenchLeftPanelTemplate
    dataTestId="order-exceptions-left-panel"
    leftTab={leftTab}
    onLeftTabChange={onLeftTabChange}
    leftRowsForRender={leftRowsForRender}
    onLeftRowClick={onLeftRowClick}
    leftStatsItems={leftStatsItems}
    leftStatsPieBackground={leftStatsPieBackground}
    globalOrderTotal={globalOrderTotal}
    statsScopeText={isFetching ? '统计更新中...' : hasManualFilterActive ? '口径：左侧过滤后' : '口径：复核结果'}
    screenContent={
      <div className="order-exceptions-left-screen-tip">
        <Typography.Text type="secondary">
          左侧支持四个维度单选过滤：异常状态、责任人、SLA（含即将超时）、发货仓。再次点击同一项可取消该维度过滤。
        </Typography.Text>
      </div>
    }
  />
);
