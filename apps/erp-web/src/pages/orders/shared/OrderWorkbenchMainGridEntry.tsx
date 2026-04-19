import type { OrderSummary } from '@/api/orders';
import { ErpDataGridNext, type ErpDataGridNextProps } from '@/components/ErpDataGridNext';
import { OrderExceptionsMainGrid, type OrderExceptionsMainGridProps } from '@/pages/orders/exceptions-native/OrderExceptionsMainGrid';
import { OrderReviewMainGrid, type OrderReviewMainGridProps } from '@/pages/orders/review-native/OrderReviewMainGrid';

export type OrderQueryMainGridProps<TRow = OrderSummary> = ErpDataGridNextProps<TRow>;

export type OrderWorkbenchMainGridEntryProps<TRow = OrderSummary> =
  | ({ variant: 'review' } & OrderReviewMainGridProps)
  | ({ variant: 'recheck' } & OrderExceptionsMainGridProps)
  | ({ variant: 'query' } & OrderQueryMainGridProps<TRow>);

export function OrderWorkbenchMainGridEntry(props: { variant: 'review' } & OrderReviewMainGridProps): JSX.Element;
export function OrderWorkbenchMainGridEntry(props: { variant: 'recheck' } & OrderExceptionsMainGridProps): JSX.Element;
export function OrderWorkbenchMainGridEntry<TRow>(props: { variant: 'query' } & OrderQueryMainGridProps<TRow>): JSX.Element;
export function OrderWorkbenchMainGridEntry<TRow>(props: OrderWorkbenchMainGridEntryProps<TRow>) {
  if (props.variant === 'review') {
    return <OrderReviewMainGrid {...props} />;
  }

  if (props.variant === 'recheck') {
    return <OrderExceptionsMainGrid {...props} />;
  }

  const { selectionColumnWidth, ...queryProps } = props;
  return <ErpDataGridNext<TRow> {...queryProps} selectionColumnWidth={selectionColumnWidth ?? 36} />;
}
