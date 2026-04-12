import type { ComponentProps } from 'react';

import {
  UiModule01QuickFiltersPanel,
  UiModule05OrderReviewRightPanelTemplate,
} from '@/components/ui-modules';

export type OrderReviewQuickFiltersPanelProps = ComponentProps<typeof UiModule01QuickFiltersPanel>;
export type OrderReviewDetailPanelProps = ComponentProps<typeof UiModule05OrderReviewRightPanelTemplate>;

export const OrderReviewQuickFiltersPanel = UiModule01QuickFiltersPanel;
export const OrderReviewDetailPanel = UiModule05OrderReviewRightPanelTemplate;
