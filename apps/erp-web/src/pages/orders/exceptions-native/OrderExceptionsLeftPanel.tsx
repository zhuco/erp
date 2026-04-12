import type { CellClickedEvent } from 'ag-grid-community';
import { Typography } from 'antd';

import {
  UiModule08OrderReviewLeftPanelTemplate,
  type UiModule08OrderReviewLeftRenderRow,
  type UiModule08OrderReviewLeftStatsItem,
} from '@/components/ui-modules';

type OrderExceptionsLeftPanelProps = {
  leftTab: 'filter' | 'screen';
  onLeftTabChange: (key: 'filter' | 'screen') => void;
  leftRowsForRender: UiModule08OrderReviewLeftRenderRow[];
  onLeftRowClick: (event: CellClickedEvent<UiModule08OrderReviewLeftRenderRow>) => void;
  leftStatsItems: UiModule08OrderReviewLeftStatsItem[];
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
  <UiModule08OrderReviewLeftPanelTemplate
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
          左侧支持四个维度单选过滤：异常状态、责任人、SLA、发货仓。再次点击同一项可取消该维度过滤。
        </Typography.Text>
      </div>
    }
  />
);
