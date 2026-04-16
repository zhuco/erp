import type { OrderSummary } from '@/api/orders';
import { ErpDataGridNext, type ErpDataGridNextProps } from '@/components/ErpDataGridNext';
import { OrderExceptionsMainGrid, type OrderExceptionsMainGridProps } from '@/pages/orders/exceptions-native/OrderExceptionsMainGrid';
import { OrderReviewMainGrid, type OrderReviewMainGridProps } from '@/pages/orders/review-native/OrderReviewMainGrid';

export type OrderQueryMainGridProps = ErpDataGridNextProps<OrderSummary>;

export type OrderWorkbenchMainGridEntryProps =
  | ({ variant: 'review' } & OrderReviewMainGridProps)
  | ({ variant: 'recheck' } & OrderExceptionsMainGridProps)
  | ({ variant: 'query' } & OrderQueryMainGridProps);

export const OrderWorkbenchMainGridEntry = (props: OrderWorkbenchMainGridEntryProps) => {
  if (props.variant === 'review') {
    return <OrderReviewMainGrid {...props} />;
  }

  if (props.variant === 'recheck') {
    return <OrderExceptionsMainGrid {...props} />;
  }

  const { selectionColumnWidth, ...queryProps } = props;
  return <ErpDataGridNext<OrderSummary> {...queryProps} selectionColumnWidth={selectionColumnWidth ?? 36} />;
};
