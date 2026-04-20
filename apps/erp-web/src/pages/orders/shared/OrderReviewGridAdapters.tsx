import type { GridApi } from 'ag-grid-community';
import type { Key } from 'react';
import { useEffect, useMemo, useState } from 'react';

import type { OrderExceptionSummary, OrderSummary } from '@/api/orders';
import { OrderReviewMainGrid } from '@/pages/orders/review-native/OrderReviewMainGrid';
import { platformLabel } from '@/pages/orders/review-native/helpers';
import type { ColumnFilters as OrderReviewColumnFilters } from '@/pages/orders/review-native/types';
import { resolveOrderExceptionRowKey } from '@/pages/orders/exceptions-native/orderExceptionsRowKey';
import type { OrderExceptionsColumnFilters } from '@/pages/orders/exceptions-native/OrderExceptionsGridFilters';

type FilterOption = {
  label: string;
  value: string;
};

const buildUniqueOptions = (
  values: Array<string | null | undefined>,
  resolveLabel?: (value: string) => string,
): FilterOption[] =>
  Array.from(
    new Set(
      values
        .map((value) => String(value ?? '').trim())
        .filter(Boolean),
    ),
  ).map((value) => ({
    value,
    label: resolveLabel?.(value) ?? value,
  }));

const readLogisticsCompany = (record: Partial<OrderSummary>) =>
  String(
    record.logistics_company ??
      (record as Record<string, unknown>).carrier_name ??
      (record as Record<string, unknown>).courier_name ??
      '',
  ).trim();

export type OrderReviewQueryMainGridProps = {
  rows: OrderSummary[];
  selectedRowKeys: Key[];
  focusedOrderKey: string | null;
  columnFilters?: OrderReviewColumnFilters;
  onSelectionChange: (keys: Key[]) => void;
  onColumnFiltersChange?: (filters: OrderReviewColumnFilters) => void;
  onRowActive: (record: OrderSummary) => void;
  onRowDoubleClick?: (record: OrderSummary) => void;
  onFocus: (record: OrderSummary) => void;
  onContextMenu: (record: OrderSummary, event: MouseEvent) => void;
  onGridApiChange?: (api: GridApi<OrderSummary> | null) => void;
  loading: boolean;
  error: unknown;
  onRetry: () => void;
  quickFilterExpanded?: boolean;
};

export const OrderReviewQueryMainGrid = ({
  rows,
  selectedRowKeys,
  focusedOrderKey,
  columnFilters,
  onSelectionChange,
  onColumnFiltersChange,
  onRowActive,
  onRowDoubleClick,
  onFocus,
  onContextMenu,
  onGridApiChange,
  loading,
  error,
  onRetry,
  quickFilterExpanded = false,
}: OrderReviewQueryMainGridProps) => {
  const [internalColumnFilters, setInternalColumnFilters] = useState<OrderReviewColumnFilters>({});

  const shopOptions = useMemo(
    () => buildUniqueOptions(rows.map((record) => record.shop_id)),
    [rows],
  );
  const logisticsOptions = useMemo(
    () => buildUniqueOptions(rows.map((record) => readLogisticsCompany(record))),
    [rows],
  );
  const resolvedColumnFilters = columnFilters ?? internalColumnFilters;
  const handleColumnFiltersChange = onColumnFiltersChange ?? setInternalColumnFilters;

  return (
    <OrderReviewMainGrid
      rows={rows}
      filterOptionRows={rows}
      quickFilterExpanded={quickFilterExpanded}
      columnFilters={resolvedColumnFilters}
      shopOptions={shopOptions}
      logisticsOptions={logisticsOptions}
      selectedRowKeys={selectedRowKeys}
      focusedOrderKey={focusedOrderKey}
      onGridApiChange={onGridApiChange}
      onColumnFiltersChange={handleColumnFiltersChange}
      onSelectionChange={onSelectionChange}
      onRowActive={onRowActive}
      onRowDoubleClick={onRowDoubleClick ?? onRowActive}
      onFocus={onFocus}
      onContextMenu={onContextMenu}
      loading={loading}
      error={error}
      onRetry={onRetry}
    />
  );
};

export type OrderReviewRecheckMainGridProps = {
  rows: OrderExceptionSummary[];
  filterOptionRows: OrderExceptionSummary[];
  quickFilterExpanded: boolean;
  columnFilters: OrderExceptionsColumnFilters;
  selectedRowKeys: Key[];
  focusedOrderKey: string | null;
  onGridApiChange?: (api: GridApi<OrderExceptionSummary> | null) => void;
  onSelectionChange: (keys: Key[]) => void;
  onRowActive: (record: OrderExceptionSummary) => void;
  onRowDoubleClick: (record: OrderExceptionSummary) => void;
  onFocus: (record: OrderExceptionSummary) => void;
  onContextMenu: (record: OrderExceptionSummary, event: MouseEvent) => void;
  resolveExceptionLabel?: (status?: string | null) => string;
  loading: boolean;
  error: unknown;
  onRetry: () => void;
};

const mapExceptionFiltersToReviewFilters = (
  filters: OrderExceptionsColumnFilters,
): OrderReviewColumnFilters => ({
  platform: filters.platform,
  shopId: filters.shopId,
  platformOrderIdKeyword: String(filters.orderNoKeyword ?? '').trim() || undefined,
  receiverNameKeyword: String(filters.receiverNameKeyword ?? '').trim() || undefined,
  warehouseId: filters.warehouseId,
});

const mapExceptionRowToReviewRow = (
  record: OrderExceptionSummary,
  resolveExceptionLabel?: (status?: string | null) => string,
): OrderSummary => {
  const rowKey = resolveOrderExceptionRowKey(record);
  const exceptionLabel = resolveExceptionLabel?.(record.exception_status) ?? record.exception_status;

  return {
    id: rowKey || record.order_id,
    platform_order_id: record.order_no,
    platform: record.platform,
    shop_id: record.shop_id,
    receiver_name: record.receiver_name ?? undefined,
    receiver_phone: undefined,
    created_at: record.latest_action_at ?? undefined,
    total_amount_cent: record.total_amount_cent,
    audit_status: record.audit_status,
    review_status: record.audit_status,
    oms_status: record.fulfillment_status,
    order_status: record.fulfillment_status,
    ship_warehouse_id: record.target_warehouse_id ?? undefined,
    target_warehouse_id: record.target_warehouse_id ?? undefined,
    logistics_company: undefined,
    tracking_no: undefined,
    remark: record.blocking_reason,
    buyer_message: record.blocking_reason,
    exception_reason: record.blocking_reason,
    tags: exceptionLabel ? [String(exceptionLabel)] : undefined,
  } as OrderSummary;
};

export const OrderReviewRecheckMainGrid = ({
  rows,
  filterOptionRows,
  quickFilterExpanded,
  columnFilters,
  selectedRowKeys,
  focusedOrderKey,
  onGridApiChange,
  onSelectionChange,
  onRowActive,
  onRowDoubleClick,
  onFocus,
  onContextMenu,
  resolveExceptionLabel,
  loading,
  error,
  onRetry,
}: OrderReviewRecheckMainGridProps) => {
  const mappedFilters = useMemo(
    () => mapExceptionFiltersToReviewFilters(columnFilters),
    [columnFilters],
  );
  const [reviewColumnFilters, setReviewColumnFilters] =
    useState<OrderReviewColumnFilters>(mappedFilters);

  useEffect(() => {
    setReviewColumnFilters(mappedFilters);
  }, [mappedFilters]);

  const mappedRows = useMemo(
    () => rows.map((record) => mapExceptionRowToReviewRow(record, resolveExceptionLabel)),
    [resolveExceptionLabel, rows],
  );
  const mappedFilterOptionRows = useMemo(
    () =>
      filterOptionRows.map((record) => mapExceptionRowToReviewRow(record, resolveExceptionLabel)),
    [filterOptionRows, resolveExceptionLabel],
  );
  const originalRowMap = useMemo(() => {
    const nextMap = new Map<string, OrderExceptionSummary>();
    rows.forEach((record) => {
      const rowKey = resolveOrderExceptionRowKey(record);
      if (rowKey) {
        nextMap.set(rowKey, record);
      }
    });
    return nextMap;
  }, [rows]);

  const shopOptions = useMemo(
    () => buildUniqueOptions(filterOptionRows.map((record) => record.shop_id)),
    [filterOptionRows],
  );
  const logisticsOptions = useMemo(
    () => buildUniqueOptions([], (value) => value),
    [],
  );
  const platformOptions = useMemo(
    () => buildUniqueOptions(filterOptionRows.map((record) => record.platform), platformLabel),
    [filterOptionRows],
  );
  void platformOptions;

  const resolveOriginalRecord = (record: OrderSummary) =>
    originalRowMap.get(String(record.id ?? '').trim()) ?? null;

  return (
    <OrderReviewMainGrid
      rows={mappedRows}
      filterOptionRows={mappedFilterOptionRows}
      quickFilterExpanded={quickFilterExpanded}
      columnFilters={reviewColumnFilters}
      shopOptions={shopOptions}
      logisticsOptions={logisticsOptions}
      selectedRowKeys={selectedRowKeys}
      focusedOrderKey={focusedOrderKey}
      onGridApiChange={(api) =>
        onGridApiChange?.(api as unknown as GridApi<OrderExceptionSummary> | null)
      }
      onColumnFiltersChange={setReviewColumnFilters}
      onSelectionChange={onSelectionChange}
      onRowActive={(record) => {
        const originalRecord = resolveOriginalRecord(record);
        if (originalRecord) {
          onRowActive(originalRecord);
        }
      }}
      onRowDoubleClick={(record) => {
        const originalRecord = resolveOriginalRecord(record);
        if (originalRecord) {
          onRowDoubleClick(originalRecord);
        }
      }}
      onFocus={(record) => {
        const originalRecord = resolveOriginalRecord(record);
        if (originalRecord) {
          onFocus(originalRecord);
        }
      }}
      onContextMenu={(record, event) => {
        const originalRecord = resolveOriginalRecord(record);
        if (originalRecord) {
          onContextMenu(originalRecord, event);
        }
      }}
      loading={loading}
      error={error}
      onRetry={onRetry}
    />
  );
};
