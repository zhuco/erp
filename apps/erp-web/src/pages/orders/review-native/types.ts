import type { Key } from 'react';

import type { OrderSummary } from '@/api/orders';
import type { UiModule08StrategyScreenFilterState } from '@/components/ui-modules';
import type { OrderReviewAgLeftRow } from '@/pages/orders/orderReviewAgLeftModel';

export type ColumnFilters = {
  platform?: string[];
  shopId?: string[];
  orderIdKeyword?: string;
  platformOrderIdKeyword?: string;
  receiverNameKeyword?: string;
  receiverPhoneKeyword?: string;
  createdAtFrom?: string;
  createdAtTo?: string;
  status?: ('normal' | 'exception')[];
  timeRange?: ('today' | 'yesterday' | 'within3d' | 'within7d' | 'within30d' | 'gt30d')[];
  exceptionType?: ('frozen' | 'noLogistics' | 'stopShipping' | 'inventoryShortage' | 'invalidGoods' | 'refundReturn')[];
  warehouseId?: string[];
  logisticsCompany?: string[];
  remarkState?: ('has' | 'none')[];
  tagState?: ('has' | 'none')[];
};

export type OrderReviewQueryState = {
  paginationCurrent: number;
  paginationPageSize: number;
  searchInput: string;
  searchKeyword: string;
  columnFilters: ColumnFilters;
  leftKeyword: string;
  leftScreenFiltersDraft: UiModule08StrategyScreenFilterState;
  leftScreenFiltersApplied: UiModule08StrategyScreenFilterState;
};

export type OrderReviewAgLeftRenderRow = OrderReviewAgLeftRow & {
  checked: boolean;
};

export type OrderReviewLeftStatsItem = {
  key: string;
  label: string;
  value: number;
  ratio: number;
  color: string;
};

export type MainGridSelectionState = {
  selectedRowKeys: Key[];
  focusedOrderKey: string | null;
};

export type MainGridCallbacks = {
  onRowActive: (record: OrderSummary) => void;
  onFocus: (record: OrderSummary) => void;
  onSelectionChange: (keys: Key[]) => void;
  onContextMenu: (record: OrderSummary, event: MouseEvent) => void;
};
