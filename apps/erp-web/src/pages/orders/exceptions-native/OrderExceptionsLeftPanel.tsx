import type { CellClickedEvent } from 'ag-grid-community';
import type { ReactNode } from 'react';

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
  statsScopeText: ReactNode;
  screenContent: ReactNode;
};

export const OrderExceptionsLeftPanel = ({
  leftTab,
  onLeftTabChange,
  leftRowsForRender,
  onLeftRowClick,
  leftStatsItems,
  leftStatsPieBackground,
  globalOrderTotal,
  statsScopeText,
  screenContent,
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
    statsScopeText={statsScopeText}
    screenContent={screenContent}
  />
);
