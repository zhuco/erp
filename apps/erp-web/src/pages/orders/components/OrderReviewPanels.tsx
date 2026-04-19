import type { ComponentProps } from 'react';

import { WorkbenchDetailPanelTemplate } from '@/components/page-templates';
import { UiModule01QuickFiltersPanel } from '@/components/ui-modules';

export type OrderReviewQuickFiltersPanelProps = ComponentProps<typeof UiModule01QuickFiltersPanel>;
export type OrderReviewDetailPanelProps = ComponentProps<typeof WorkbenchDetailPanelTemplate>;

export const OrderReviewQuickFiltersPanel = UiModule01QuickFiltersPanel;
export const OrderReviewDetailPanel = WorkbenchDetailPanelTemplate;
