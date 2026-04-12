import type { CellClickedEvent } from 'ag-grid-community';
import type { ReactNode } from 'react';
import { Typography } from 'antd';

import {
  UiModule08OrderReviewLeftPanelTemplate,
  type UiModule08OrderReviewLeftRenderRow,
  type UiModule08OrderReviewLeftStatsItem,
} from '@/components/ui-modules';

type OrderQueryLeftPanelProps = {
  leftTab: 'filter' | 'screen';
  onLeftTabChange: (key: 'filter' | 'screen') => void;
  leftRowsForRender: UiModule08OrderReviewLeftRenderRow[];
  onLeftRowClick: (event: CellClickedEvent<UiModule08OrderReviewLeftRenderRow>) => void;
  leftStatsItems: UiModule08OrderReviewLeftStatsItem[];
  leftStatsPieBackground: string;
  globalOrderTotal: number;
  isFetching: boolean;
  statsScopeText: ReactNode;
};

export const OrderQueryLeftPanel = ({
  leftTab,
  onLeftTabChange,
  leftRowsForRender,
  onLeftRowClick,
  leftStatsItems,
  leftStatsPieBackground,
  globalOrderTotal,
  isFetching,
  statsScopeText,
}: OrderQueryLeftPanelProps) => (
  <UiModule08OrderReviewLeftPanelTemplate
    dataTestId="orders-left-panel"
    leftTab={leftTab}
    onLeftTabChange={onLeftTabChange}
    leftRowsForRender={leftRowsForRender}
    onLeftRowClick={onLeftRowClick}
    leftStatsItems={leftStatsItems}
    leftStatsPieBackground={leftStatsPieBackground}
    globalOrderTotal={globalOrderTotal}
    statsScopeText={isFetching ? '统计更新中...' : statsScopeText}
    screenContent={
      <div className="order-query-left-screen-tip">
        <Typography.Text type="secondary">
          左侧支持范围、状态、渠道、标记四个维度单选筛选，再次点击同一项即可取消并回到全部订单。
        </Typography.Text>
      </div>
    }
  />
);
