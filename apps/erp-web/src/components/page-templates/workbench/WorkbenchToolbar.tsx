import {
  OrderReviewWorkbenchToolbarTemplate,
  type OrderReviewWorkbenchToolbarTemplateProps,
} from '@/components/workbench/templates/OrderReviewWorkbenchToolbarTemplate';

export type WorkbenchToolbarProps = OrderReviewWorkbenchToolbarTemplateProps;

export const WorkbenchToolbar = (props: WorkbenchToolbarProps) => <OrderReviewWorkbenchToolbarTemplate {...props} />;

WorkbenchToolbar.displayName = 'WorkbenchToolbar';
