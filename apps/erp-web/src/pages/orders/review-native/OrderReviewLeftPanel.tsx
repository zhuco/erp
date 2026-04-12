import type { CellClickedEvent } from 'ag-grid-community';

import {
  UiModule08OrderReviewLeftPanelTemplate,
  UiModule08StrategyScreenFilterForm,
  type UiModule08StrategyScreenFilterChangeHandler,
  type UiModule08StrategyScreenFilterState,
} from '@/components/ui-modules';

import type { OrderReviewAgLeftRenderRow, OrderReviewLeftStatsItem } from './types';

type OrderReviewLeftPanelProps = {
  leftTab: 'filter' | 'screen';
  onLeftTabChange: (key: 'filter' | 'screen') => void;
  leftRowsForRender: OrderReviewAgLeftRenderRow[];
  onLeftRowClick: (event: CellClickedEvent<OrderReviewAgLeftRenderRow>) => void;
  leftScreenFiltersDraft: UiModule08StrategyScreenFilterState;
  onLeftScreenFieldChange: UiModule08StrategyScreenFilterChangeHandler;
  onApplyLeftScreenFilters: () => void;
  onResetLeftScreenFilters: () => void;
  leftStatsItems: OrderReviewLeftStatsItem[];
  leftStatsPieBackground: string;
  globalOrderTotal: number;
  isGlobalStatsFetching: boolean;
  shouldFallbackToAll: boolean;
};

export const OrderReviewLeftPanel = ({
  leftTab,
  onLeftTabChange,
  leftRowsForRender,
  onLeftRowClick,
  leftScreenFiltersDraft,
  onLeftScreenFieldChange,
  onApplyLeftScreenFilters,
  onResetLeftScreenFilters,
  leftStatsItems,
  leftStatsPieBackground,
  globalOrderTotal,
  isGlobalStatsFetching,
  shouldFallbackToAll,
}: OrderReviewLeftPanelProps) => (
  <UiModule08OrderReviewLeftPanelTemplate
    className="order-review-left-ag-panel"
    dataTestId="order-review-left-ag-panel"
    leftTab={leftTab}
    onLeftTabChange={onLeftTabChange}
    leftRowsForRender={leftRowsForRender}
    onLeftRowClick={onLeftRowClick}
    leftStatsItems={leftStatsItems}
    leftStatsPieBackground={leftStatsPieBackground}
    globalOrderTotal={globalOrderTotal}
    statsScopeText={isGlobalStatsFetching ? '统计更新中...' : shouldFallbackToAll ? '口径：全部订单' : '口径：待复核订单'}
    screenContent={
      <UiModule08StrategyScreenFilterForm
        value={leftScreenFiltersDraft}
        onFieldChange={onLeftScreenFieldChange}
        onApply={onApplyLeftScreenFilters}
        onReset={onResetLeftScreenFilters}
        showHeader={false}
      />
    }
  />
);

