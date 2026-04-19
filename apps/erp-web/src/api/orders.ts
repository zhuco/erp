import { apiClient, buildAuthHeaders, fetchWithAuth, unwrapApiResponse } from '@/api/client';
import {
  addMockOrderNote,
  addMockOrderTags,
  assignMockOrderException,
  getMockOrderDetail,
  getMockOrderExceptionDetail,
  getMockOrderReconciliationDetail,
  getMockOrderSettlementDetail,
  getMockOrderSettlementRule,
  getMockOrderList,
  holdMockOrderException,
  listMockOrderExceptions,
  listMockOrderReconciliations,
  listMockOrderSettlements,
  claimMockOrderException,
  cancelMockOrder,
  escalateMockOrderException,
  exportMockOrderSettlement,
  lockMockOrderSettlement,
  rejectMockOrderException,
  resolveMockOrderException,
  reserveMockOrderId,
  requeueMockOrderReview,
  requeueMockOrderAudit,
  rematchMockOrderItems,
  reviewMockOrder,
  settleMockOrderReconciliation,
  submitMockOrderFulfillment,
  returnMockOrderToDownload,
  unlockMockOrderSettlement,
  updateMockOrderSettlementRule,
  updateMockOrder,
  updateMockOrderReceiver
} from '@/api/mocks';
import { createApiError, handleApiError } from '@/api/errors';
import {
  auditStatuses,
  exceptionStatuses,
  fulfillmentStatuses,
  reserveStatuses
} from '@/features/oms-wms-main-chain/statuses';
import { apiBaseUrl } from '@/utils/env';
import { useMocks } from '@/utils/env';

import type { components, paths } from './generated/schema';

export type OrderSummary = components['schemas']['OrderSummary'];
export type OrderListResponse = components['schemas']['OrderListResponse'];
export type OrderDetail = components['schemas']['OrderDetail'];
export type OrderReviewRequest = components['schemas']['OrderReviewRequest'];
export type OrderReviewState = components['schemas']['OrderReviewState'];
export type OrderEditRequest = components['schemas']['OrderEditRequest'];
export type OrderEditItemRequest = components['schemas']['OrderEditItemRequest'];
export type OrderCancelRequest = components['schemas']['OrderCancelRequest'];
export type OrderTag = components['schemas']['OrderTag'];
export type OrderNote = components['schemas']['OrderNote'];
export type OrderForceReviewRequest = {
  password: string;
  reason?: string;
  version?: number;
};
export type OrderMergeRequest = {
  order_ids: string[];
  target_order_id?: string;
  force?: boolean;
};
export type OrderMergeResponse = {
  target_order_id: string;
  merged_order_ids: string[];
};
export type OrderSplitItemRequest = {
  line_id: string;
  qty: number;
};
export type OrderSplitChildRequest = {
  items: OrderSplitItemRequest[];
};
export type OrderSplitRequest = {
  split_count?: number;
  children?: OrderSplitChildRequest[];
};
export type OrderSplitItemResponse = {
  line_id: string;
  sku_id?: string | null;
  title?: string | null;
  qty: number;
  price_cent: number;
};
export type OrderSplitChildResponse = {
  order_id: string;
  items: OrderSplitItemResponse[];
};
export type OrderSplitResponse = {
  source_order_id: string;
  split_count: number;
  created_at: string;
  child_orders: OrderSplitChildResponse[];
};
export type OrderExceptionHandleRequest = {
  action: 'RESOLVE' | 'REJECT' | 'REVOKE' | 'REOPEN' | 'CLOSE';
  reason?: string;
  flags?: string[];
};
export type OrderListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/orders']['get']['parameters']['query']
> & {
  tagged?: boolean;
  ship_warehouse_id?: string;
  platform_order_id?: string;
  item_keyword?: string;
  total_amount_cent?: number;
  goods_qty?: number;
  customer_account?: string;
  receiver_name?: string;
  receiver_phone?: string;
  receiver_address?: string;
  remark?: string;
  order_source?: string;
  logistics_company?: string;
  estimated_postage_cent?: number;
  estimated_profit_cent?: number;
  estimated_profit_rate?: number;
};

export type OrderReviewStatsQuery = Omit<OrderListQuery, 'offset' | 'limit'>;

export type OrderReviewStatsResponse = {
  total: number;
  platform_counts: Record<string, number>;
  shop_counts: Record<string, number>;
};

type OrderRequestOptions = {
  signal?: AbortSignal;
};
export type OrderAuditStatus = (typeof auditStatuses)[number];
export type OrderExceptionStatus = (typeof exceptionStatuses)[number];
export type OrderFulfillmentStatus = (typeof fulfillmentStatuses)[number];
export type OrderReserveStatus = (typeof reserveStatuses)[number];

export type OrderExceptionListQuery = {
  status?: OrderExceptionStatus | string;
  keyword?: string;
  assignee_id?: string | null;
  offset?: number;
  limit?: number;
};

export type OrderExceptionActionPayload = {
  operator_id?: string | null;
  operator_name?: string | null;
  assignee_id?: string | null;
  assignee_name?: string | null;
  reason?: string | null;
};

export type OrderExceptionActionRecord = {
  action: string;
  message: string;
  operator_id?: string | null;
  operator_name?: string | null;
  created_at: string;
};

export type OrderExceptionSummary = {
  order_id: string;
  order_no: string;
  platform: string;
  shop_id: string;
  receiver_name?: string | null;
  total_amount_cent: number;
  audit_status: OrderAuditStatus | string;
  exception_status: OrderExceptionStatus | string;
  fulfillment_status: OrderFulfillmentStatus | string;
  reserve_status: OrderReserveStatus | null;
  target_warehouse_id: string | null;
  blocking_reason: string;
  assignee_id?: string | null;
  assignee_name?: string | null;
  latest_action?: string | null;
  latest_action_at?: string | null;
};

export type OrderExceptionDetail = OrderExceptionSummary & {
  blocking_reasons: string[];
  action_logs: OrderExceptionActionRecord[];
  order: OrderDetail;
};

export type OrderSettlementRule = {
  stack_mode: 'STACKED' | 'EXCLUSIVE' | 'PRIORITY';
  member_discount_rate: number;
  promo_discount_rate: number;
  max_discount_rate: number;
  version: number;
  updated_at?: string | null;
};

export type OrderSettlementRuleUpdatePayload = {
  stack_mode: 'STACKED' | 'EXCLUSIVE' | 'PRIORITY';
  member_discount_rate: number;
  promo_discount_rate: number;
  max_discount_rate: number;
  expected_version?: number;
};

export type OrderSettlementListQuery = {
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: 'OPEN' | 'LOCKED' | 'EXPORTED' | string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type OrderSettlementActionPayload = {
  operator_id?: string | null;
  operator_name?: string | null;
  export_job_id?: string | null;
};

export type OrderSettlementPromotion = {
  name: string;
  promotion_type: string;
  rate: number;
  amount_cent: number;
};

export type OrderSettlementActionRecord = {
  action: string;
  message: string;
  operator_id?: string | null;
  operator_name?: string | null;
  created_at: string;
};

export type OrderSettlementSummary = {
  order_id: string;
  order_no: string;
  platform: string;
  shop_id: string;
  receiver_name?: string | null;
  created_at: string;
  total_amount_cent: number;
  discount_amount_cent: number;
  receivable_amount_cent: number;
  settlement_status: 'OPEN' | 'LOCKED' | 'EXPORTED' | string;
  locked_at?: string | null;
  export_job_id?: string | null;
  latest_action?: string | null;
  latest_action_at?: string | null;
};

export type OrderSettlementListSummary = {
  total_orders: number;
  open_orders: number;
  locked_orders: number;
  exported_orders: number;
  total_amount_cent: number;
  receivable_amount_cent: number;
};

export type OrderSettlementListResponse = {
  items: OrderSettlementSummary[];
  total: number;
  summary: OrderSettlementListSummary;
};

export type OrderSettlementDetail = OrderSettlementSummary & {
  promotions: OrderSettlementPromotion[];
  action_logs: OrderSettlementActionRecord[];
};

export type OrderReconciliationListQuery = {
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: 'PENDING' | 'DONE' | string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type OrderReconciliationActionPayload = {
  operator_id?: string | null;
  operator_name?: string | null;
  note?: string | null;
};

export type OrderReconciliationSummary = {
  batch_id: string;
  platform: string;
  shop_id: string;
  period_start: string;
  period_end: string;
  order_count: number;
  expected_cent: number;
  actual_cent: number;
  diff_cent: number;
  status: 'PENDING' | 'DONE' | string;
  reasons: string[];
  settled_at?: string | null;
};

export type OrderReconciliationListSummary = {
  total_batches: number;
  pending_batches: number;
  done_batches: number;
  expected_total_cent: number;
  diff_total_cent: number;
};

export type OrderReconciliationListResponse = {
  items: OrderReconciliationSummary[];
  total: number;
  summary: OrderReconciliationListSummary;
};

export type OrderReconciliationOrder = {
  order_id: string;
  order_no: string;
  receiver_name?: string | null;
  total_amount_cent: number;
  receivable_amount_cent: number;
  settlement_status: string;
  audit_status: string;
  exception_status: string;
  blocking_reason: string;
};

export type OrderReconciliationDetail = OrderReconciliationSummary & {
  note?: string | null;
  orders: OrderReconciliationOrder[];
};

export type FulfillmentPrecheckResult = {
  order_id: string;
  order_no: string;
  passed: boolean;
  reasons: string[];
  reserve_status: OrderReserveStatus | null;
  fulfillment_status: OrderFulfillmentStatus | string;
  target_warehouse_id: string | null;
  outbound_doc_count: number;
};

const normalizeOptionalTextParam = (value: unknown): string | undefined => {
  if (typeof value !== 'string') {
    return undefined;
  }
  const normalized = value.trim();
  return normalized || undefined;
};

const normalizeCsvParam = (
  value: unknown,
  normalizeToken: (token: string) => string
): string | undefined => {
  const text = normalizeOptionalTextParam(value);
  if (!text) {
    return undefined;
  }
  const tokens = text
    .split(',')
    .map((token) => normalizeToken(token.trim()))
    .filter((token) => token.length > 0);
  return tokens.length ? tokens.join(',') : undefined;
};

const normalizeOrderListQuery = <T extends OrderListQuery>(query: T): T => {
  const normalized: Record<string, unknown> = { ...query };

  const textFields = [
    'keyword',
    'tracking_no',
    'platform_order_id',
    'receiver_address',
    'item_keyword',
    'customer_account',
    'receiver_name',
    'receiver_phone',
    'remark',
    'order_source',
    'logistics_company',
    'archive_status'
  ];
  textFields.forEach((field) => {
    if (field in normalized) {
      normalized[field] = normalizeOptionalTextParam(normalized[field]);
    }
  });

  if (typeof normalized.platform === 'string') {
    normalized.platform = normalizeOptionalTextParam(normalized.platform)?.toLowerCase();
  }

  normalized.oms_status = normalizeCsvParam(normalized.oms_status, (token) => token.toUpperCase());
  normalized.exception_flags = normalizeCsvParam(normalized.exception_flags, (token) => token.toUpperCase());

  const upperCaseSingleFields = [
    'order_status',
    'review_status',
    'fulfillment_status',
    'exception_status',
    'payment_status'
  ];
  upperCaseSingleFields.forEach((field) => {
    if (typeof normalized[field] === 'string') {
      const text = normalizeOptionalTextParam(normalized[field]);
      normalized[field] = text ? text.toUpperCase() : undefined;
    }
  });

  return normalized as T;
};

const buildMainChainUrl = (
  tenantId: string,
  path: string,
  query?: Record<string, string | number | null | undefined>
) => {
  const url = new URL(`${apiBaseUrl}/v1/tenants/${tenantId}${path}`);
  if (query) {
    Object.entries(query).forEach(([key, value]) => {
      if (value === undefined || value === null || value === '') {
        return;
      }
      url.searchParams.set(key, String(value));
    });
  }
  return url.toString();
};

const requestMainChain = async <T>(
  url: string,
  method: 'GET' | 'POST' | 'PUT',
  message: string,
  body?: unknown
): Promise<T> => {
  const response = await fetchWithAuth(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...buildAuthHeaders()
    },
    body: body === undefined ? undefined : JSON.stringify(body)
  });

  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }

  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError(message, response.status, result);
  }

  return result as T;
};

export const listOrders = async (
  tenantId: string,
  query: OrderListQuery,
  options?: OrderRequestOptions
): Promise<OrderListResponse> => {
  const normalizedQuery = normalizeOrderListQuery(query);
  if (useMocks) {
    return getMockOrderList(normalizedQuery);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/orders', {
    signal: options?.signal,
    params: {
      path: { tenant_id: tenantId },
      query: normalizedQuery
    }
  });
  return unwrapApiResponse(result, '获取订单失败');
};

const normalizeOrderReviewStatsMap = (raw: unknown): Record<string, number> => {
  if (!raw) {
    return {};
  }
  if (Array.isArray(raw)) {
    return raw.reduce<Record<string, number>>((acc, item) => {
      if (!item || typeof item !== 'object') {
        return acc;
      }
      const record = item as Record<string, unknown>;
      const key = String(record.key ?? record.name ?? record.label ?? '').trim();
      const value = Number(record.count ?? record.value ?? 0);
      if (!key || !Number.isFinite(value) || value <= 0) {
        return acc;
      }
      acc[key] = Math.floor(value);
      return acc;
    }, {});
  }
  if (typeof raw === 'object') {
    return Object.entries(raw as Record<string, unknown>).reduce<Record<string, number>>((acc, [key, value]) => {
      const numericValue = Number(value ?? 0);
      if (!key || !Number.isFinite(numericValue) || numericValue <= 0) {
        return acc;
      }
      acc[key] = Math.floor(numericValue);
      return acc;
    }, {});
  }
  return {};
};

const aggregateOrderReviewStatsByScan = async (
  tenantId: string,
  query: OrderReviewStatsQuery,
  options?: OrderRequestOptions
): Promise<OrderReviewStatsResponse> => {
  const platformCounts: Record<string, number> = {};
  const shopCounts: Record<string, number> = {};
  const pageSize = 500;
  let offset = 0;
  let total = 0;

  let hasMore = true;
  while (hasMore) {
    const page = await listOrders(
      tenantId,
      {
        ...query,
        offset,
        limit: pageSize
      },
      options
    );
    const items = page.items ?? [];
    if (offset === 0) {
      total = Number(page.total ?? 0);
    }
    if (!items.length) {
      hasMore = false;
      continue;
    }

    items.forEach((item) => {
      const platform = String(item.platform ?? '').trim();
      if (platform) {
        platformCounts[platform] = (platformCounts[platform] ?? 0) + 1;
      }
      const shopId = String(item.shop_id ?? '').trim();
      if (shopId) {
        shopCounts[shopId] = (shopCounts[shopId] ?? 0) + 1;
      }
    });

    offset += items.length;
    if (total > 0 && offset >= total) {
      hasMore = false;
      continue;
    }
    if (items.length < pageSize) {
      hasMore = false;
    }
  }

  return {
    total: total > 0 ? total : offset,
    platform_counts: platformCounts,
    shop_counts: shopCounts
  };
};

export const getOrderReviewStats = async (
  tenantId: string,
  query: OrderReviewStatsQuery,
  options?: OrderRequestOptions
): Promise<OrderReviewStatsResponse> => {
  const normalizedQuery = normalizeOrderListQuery(query);
  if (useMocks) {
    return aggregateOrderReviewStatsByScan(tenantId, normalizedQuery, options);
  }

  const url = new URL(`${apiBaseUrl}/v1/tenants/${tenantId}/orders/stats`);
  Object.entries(normalizedQuery).forEach(([key, value]) => {
    if (value === undefined || value === null || value === '') {
      return;
    }
    url.searchParams.set(key, String(value));
  });

  const response = await fetchWithAuth(url.toString(), {
    method: 'GET',
    headers: buildAuthHeaders(),
    signal: options?.signal
  });

  if (!response.ok) {
    if ([404, 405, 501].includes(response.status)) {
      return aggregateOrderReviewStatsByScan(tenantId, normalizedQuery, options);
    }
    let payload: unknown = undefined;
    try {
      payload = await response.json();
    } catch {
      payload = undefined;
    }
    throw createApiError('获取订单统计失败', response.status, payload);
  }

  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  const record = (payload ?? {}) as Record<string, unknown>;
  const total = Number(record.total ?? 0);
  return {
    total: Number.isFinite(total) && total >= 0 ? Math.floor(total) : 0,
    platform_counts: normalizeOrderReviewStatsMap(
      record.platform_counts ?? record.platformCountMap ?? record.platform_distribution
    ),
    shop_counts: normalizeOrderReviewStatsMap(record.shop_counts ?? record.shopCountMap ?? record.shop_distribution)
  };
};

export const getOrderDetail = async (
  tenantId: string,
  orderId: string,
  options?: OrderRequestOptions
): Promise<OrderDetail> => {
  if (useMocks) {
    return getMockOrderDetail(orderId);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/orders/{order_id}', {
    signal: options?.signal,
    params: {
      path: { tenant_id: tenantId, order_id: orderId }
    }
  });
  return unwrapApiResponse(result, '获取订单详情失败');
};

export const updateOrder = async (
  tenantId: string,
  orderId: string,
  payload: OrderEditRequest
): Promise<OrderDetail> => {
  if (useMocks) {
    return updateMockOrder(orderId, payload);
  }
  const result = await apiClient.PUT('/v1/tenants/{tenant_id}/orders/{order_id}', {
    params: {
      path: { tenant_id: tenantId, order_id: orderId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '订单编辑失败');
};

export const reviewOrder = async (
  tenantId: string,
  orderId: string,
  payload: OrderReviewRequest
): Promise<OrderReviewState> => {
  if (useMocks) {
    return reviewMockOrder(orderId, payload);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/orders/{order_id}/review', {
    params: {
      path: { tenant_id: tenantId, order_id: orderId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '订单审核失败');
};

export const cancelOrder = async (
  tenantId: string,
  orderId: string,
  payload: OrderCancelRequest
): Promise<OrderReviewState> => {
  if (useMocks) {
    return cancelMockOrder(orderId, payload);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/orders/{order_id}/cancel', {
    params: {
      path: { tenant_id: tenantId, order_id: orderId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '取消订单失败');
};

export const forceReviewOrder = async (
  tenantId: string,
  orderId: string,
  payload: OrderForceReviewRequest
): Promise<OrderReviewState> => {
  if (useMocks) {
    return {
      oms_status: 'PENDING_SHIPMENT',
      review_status: 'PASSED',
      exception_flags: [],
      version: 1
    } as OrderReviewState;
  }
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/force-review`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('强制审核失败', response.status, result);
  }
  return result as OrderReviewState;
};

export const mergeOrders = async (
  tenantId: string,
  payload: OrderMergeRequest
): Promise<OrderMergeResponse> => {
  if (useMocks) {
    const [target, ...rest] = payload.order_ids;
    return {
      target_order_id: payload.target_order_id ?? target ?? '',
      merged_order_ids: rest
    };
  }
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/merge`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('订单合并失败', response.status, result);
  }
  return result as OrderMergeResponse;
};

export const splitOrder = async (
  tenantId: string,
  orderId: string,
  payload: OrderSplitRequest
): Promise<OrderSplitResponse> => {
  if (useMocks) {
    const splitCount = payload.children?.length ?? payload.split_count ?? 2;
    return {
      source_order_id: orderId,
      split_count: splitCount,
      created_at: new Date().toISOString(),
      child_orders:
        payload.children?.map((child) => ({
          order_id: reserveMockOrderId(),
          items: child.items.map((item) => ({
            line_id: item.line_id,
            sku_id: null,
            title: null,
            qty: item.qty,
            price_cent: 0
          }))
        })) ??
        Array.from({ length: splitCount }, () => ({
          order_id: reserveMockOrderId(),
          items: []
        }))
    };
  }
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/split`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('订单拆分失败', response.status, result);
  }
  return result as OrderSplitResponse;
};

export const handleOrderException = async (
  tenantId: string,
  orderId: string,
  payload: OrderExceptionHandleRequest
): Promise<OrderReviewState> => {
  const response = await fetch(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/exceptions/handle`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('异常处理失败', response.status, result);
  }
  return result as OrderReviewState;
};

export const addOrderTags = async (
  tenantId: string,
  orderId: string,
  tags: string[]
): Promise<OrderTag[]> => {
  if (useMocks) {
    return addMockOrderTags(orderId, tags);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/orders/{order_id}/tags', {
    params: {
      path: { tenant_id: tenantId, order_id: orderId }
    },
    body: { tags }
  });
  return unwrapApiResponse(result, '添加标签失败');
};

export const addOrderNote = async (
  tenantId: string,
  orderId: string,
  note: string
): Promise<OrderNote> => {
  if (useMocks) {
    return addMockOrderNote(orderId, note);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/orders/{order_id}/notes', {
    params: {
      path: { tenant_id: tenantId, order_id: orderId }
    },
    body: { note }
  });
  return unwrapApiResponse(result, '添加备注失败');
};

export type OrderReceiverUpdatePayload = {
  receiver_name: string;
  receiver_phone: string;
  receiver_address: string;
};

export const updateOrderReceiver = async (
  tenantId: string,
  orderId: string,
  payload: OrderReceiverUpdatePayload
): Promise<OrderDetail> => {
  if (useMocks) {
    return updateMockOrderReceiver(orderId, payload);
  }
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/receiver`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('修改收货信息失败', response.status, result);
  }
  return result as OrderDetail;
};

export const rematchOrderItems = async (
  tenantId: string,
  orderId: string
): Promise<OrderDetail> => {
  if (useMocks) {
    return rematchMockOrderItems(orderId);
  }
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/items/rematch`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({})
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('货品匹配失败', response.status, result);
  }
  return result as OrderDetail;
};

export const returnOrderToDownload = async (
  tenantId: string,
  orderId: string
): Promise<OrderReviewState> => {
  if (useMocks) {
    return returnMockOrderToDownload(orderId);
  }
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/return-to-download`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({})
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('驳回到网店订单失败', response.status, result);
  }
  return result as OrderReviewState;
};

export const requeueOrderReview = async (
  tenantId: string,
  orderId: string
): Promise<OrderReviewState> => {
  if (useMocks) {
    return requeueMockOrderReview(orderId);
  }
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/review/requeue`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({})
    }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('转审核失败', response.status, result);
  }
  return result as OrderReviewState;
};

export const submitOrdersForFulfillment = async (
  tenantId: string,
  orderIds: string[],
  options?: { dryRun?: boolean }
): Promise<FulfillmentPrecheckResult[]> => {
  if (useMocks) {
    return orderIds.map((orderId) => submitMockOrderFulfillment(orderId, options));
  }

  return requestMainChain<FulfillmentPrecheckResult[]>(
    buildMainChainUrl(tenantId, '/oms/orders/fulfillment/submit'),
    'POST',
    '提交发货失败',
    {
      order_ids: orderIds,
      dry_run: options?.dryRun ?? false
    }
  );
};

export const listOrderExceptions = async (
  tenantId: string,
  query?: OrderExceptionListQuery
): Promise<{ items: OrderExceptionSummary[]; total: number }> => {
  if (useMocks) {
    return listMockOrderExceptions(query);
  }

  return requestMainChain<{ items: OrderExceptionSummary[]; total: number }>(
    buildMainChainUrl(tenantId, '/oms/orders/exceptions', {
      status: query?.status,
      keyword: query?.keyword,
      assignee_id: query?.assignee_id,
      offset: query?.offset,
      limit: query?.limit
    }),
    'GET',
    '获取订单异常池失败'
  );
};

export const getOrderExceptionDetail = async (
  tenantId: string,
  orderId: string
): Promise<OrderExceptionDetail> => {
  if (useMocks) {
    return getMockOrderExceptionDetail(orderId);
  }

  return requestMainChain<OrderExceptionDetail>(
    buildMainChainUrl(tenantId, `/oms/orders/exceptions/${orderId}`),
    'GET',
    '获取订单异常详情失败'
  );
};

export const claimOrderException = async (
  tenantId: string,
  orderId: string,
  payload?: OrderExceptionActionPayload
): Promise<OrderExceptionSummary> => {
  if (useMocks) {
    return claimMockOrderException(orderId, payload);
  }

  return requestMainChain<OrderExceptionSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/exceptions/${orderId}/claim`),
    'POST',
    '领取订单异常失败',
    payload ?? {}
  );
};

export const assignOrderException = async (
  tenantId: string,
  orderId: string,
  payload?: OrderExceptionActionPayload
): Promise<OrderExceptionSummary> => {
  if (useMocks) {
    return assignMockOrderException(orderId, payload);
  }

  return requestMainChain<OrderExceptionSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/exceptions/${orderId}/assign`),
    'POST',
    '分派订单异常失败',
    payload ?? {}
  );
};

export const escalateOrderException = async (
  tenantId: string,
  orderId: string,
  payload?: OrderExceptionActionPayload
): Promise<OrderExceptionSummary> => {
  if (useMocks) {
    return escalateMockOrderException(orderId, payload);
  }

  return requestMainChain<OrderExceptionSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/exceptions/${orderId}/escalate`),
    'POST',
    '升级订单异常失败',
    payload ?? {}
  );
};

export const resolveOrderException = async (
  tenantId: string,
  orderId: string,
  payload?: OrderExceptionActionPayload
): Promise<OrderExceptionSummary> => {
  if (useMocks) {
    return resolveMockOrderException(orderId, payload);
  }

  return requestMainChain<OrderExceptionSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/exceptions/${orderId}/resolve`),
    'POST',
    '解决订单异常失败',
    payload ?? {}
  );
};

export const holdOrderException = async (
  tenantId: string,
  orderId: string,
  payload?: OrderExceptionActionPayload
): Promise<OrderExceptionSummary> => {
  if (useMocks) {
    return holdMockOrderException(orderId, payload);
  }

  return requestMainChain<OrderExceptionSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/exceptions/${orderId}/hold`),
    'POST',
    '挂起订单异常失败',
    payload ?? {}
  );
};

export const rejectOrderException = async (
  tenantId: string,
  orderId: string,
  payload?: OrderExceptionActionPayload
): Promise<OrderExceptionSummary> => {
  if (useMocks) {
    return rejectMockOrderException(orderId, payload);
  }

  return requestMainChain<OrderExceptionSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/exceptions/${orderId}/reject`),
    'POST',
    '驳回订单异常失败',
    payload ?? {}
  );
};

export const getOrderSettlementRule = async (
  tenantId: string
): Promise<OrderSettlementRule> => {
  if (useMocks) {
    return getMockOrderSettlementRule();
  }

  return requestMainChain<OrderSettlementRule>(
    buildMainChainUrl(tenantId, '/oms/orders/settlements/rules'),
    'GET',
    '获取结算规则失败'
  );
};

export const updateOrderSettlementRule = async (
  tenantId: string,
  payload: OrderSettlementRuleUpdatePayload
): Promise<OrderSettlementRule> => {
  if (useMocks) {
    return updateMockOrderSettlementRule(payload);
  }

  return requestMainChain<OrderSettlementRule>(
    buildMainChainUrl(tenantId, '/oms/orders/settlements/rules'),
    'PUT',
    '更新结算规则失败',
    payload
  );
};

export const listOrderSettlements = async (
  tenantId: string,
  query?: OrderSettlementListQuery
): Promise<OrderSettlementListResponse> => {
  if (useMocks) {
    return listMockOrderSettlements(query);
  }

  return requestMainChain<OrderSettlementListResponse>(
    buildMainChainUrl(tenantId, '/oms/orders/settlements', {
      platform: query?.platform,
      shop_id: query?.shop_id,
      keyword: query?.keyword,
      status: query?.status,
      created_from: query?.created_from,
      created_to: query?.created_to,
      offset: query?.offset,
      limit: query?.limit
    }),
    'GET',
    '获取订单结算列表失败'
  );
};

export const getOrderSettlementDetail = async (
  tenantId: string,
  orderId: string
): Promise<OrderSettlementDetail> => {
  if (useMocks) {
    return getMockOrderSettlementDetail(orderId);
  }

  return requestMainChain<OrderSettlementDetail>(
    buildMainChainUrl(tenantId, `/oms/orders/settlements/${orderId}`),
    'GET',
    '获取订单结算详情失败'
  );
};

export const lockOrderSettlement = async (
  tenantId: string,
  orderId: string,
  payload?: OrderSettlementActionPayload
): Promise<OrderSettlementSummary> => {
  if (useMocks) {
    return lockMockOrderSettlement(orderId, payload);
  }

  return requestMainChain<OrderSettlementSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/settlements/${orderId}/lock`),
    'POST',
    '锁定订单结算失败',
    payload ?? {}
  );
};

export const unlockOrderSettlement = async (
  tenantId: string,
  orderId: string,
  payload?: OrderSettlementActionPayload
): Promise<OrderSettlementSummary> => {
  if (useMocks) {
    return unlockMockOrderSettlement(orderId, payload);
  }

  return requestMainChain<OrderSettlementSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/settlements/${orderId}/unlock`),
    'POST',
    '解锁订单结算失败',
    payload ?? {}
  );
};

export const exportOrderSettlement = async (
  tenantId: string,
  orderId: string,
  payload?: OrderSettlementActionPayload
): Promise<OrderSettlementSummary> => {
  if (useMocks) {
    return exportMockOrderSettlement(orderId, payload);
  }

  return requestMainChain<OrderSettlementSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/settlements/${orderId}/export`),
    'POST',
    '导出订单结算失败',
    payload ?? {}
  );
};

export const listOrderReconciliations = async (
  tenantId: string,
  query?: OrderReconciliationListQuery
): Promise<OrderReconciliationListResponse> => {
  if (useMocks) {
    return listMockOrderReconciliations(query);
  }

  return requestMainChain<OrderReconciliationListResponse>(
    buildMainChainUrl(tenantId, '/oms/orders/reconciliations', {
      platform: query?.platform,
      shop_id: query?.shop_id,
      keyword: query?.keyword,
      status: query?.status,
      created_from: query?.created_from,
      created_to: query?.created_to,
      offset: query?.offset,
      limit: query?.limit
    }),
    'GET',
    '获取销售对账列表失败'
  );
};

export const getOrderReconciliationDetail = async (
  tenantId: string,
  batchId: string
): Promise<OrderReconciliationDetail> => {
  if (useMocks) {
    return getMockOrderReconciliationDetail(batchId);
  }

  return requestMainChain<OrderReconciliationDetail>(
    buildMainChainUrl(tenantId, `/oms/orders/reconciliations/${batchId}`),
    'GET',
    '获取销售对账详情失败'
  );
};

export const settleOrderReconciliation = async (
  tenantId: string,
  batchId: string,
  payload?: OrderReconciliationActionPayload
): Promise<OrderReconciliationSummary> => {
  if (useMocks) {
    return settleMockOrderReconciliation(batchId, payload);
  }

  return requestMainChain<OrderReconciliationSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/reconciliations/${batchId}/settle`),
    'POST',
    '完成销售对账失败',
    payload ?? {}
  );
};

export const requeueOrderAudit = async (
  tenantId: string,
  orderId: string,
  payload?: OrderExceptionActionPayload
): Promise<OrderExceptionSummary> => {
  if (useMocks) {
    return requeueMockOrderAudit(orderId, payload);
  }

  return requestMainChain<OrderExceptionSummary>(
    buildMainChainUrl(tenantId, `/oms/orders/${orderId}/review/requeue`),
    'POST',
    '回退待审核失败',
    payload ?? {}
  );
};

export type OrderAutoTagApplyResponse = {
  tagged_orders: string[];
  total: number;
};

export type ExportJobResponse = {
  job_id: string;
  output_url?: string | null;
};

export const applyAutoTags = async (
  tenantId: string,
  orderIds: string[]
): Promise<OrderAutoTagApplyResponse> => {
  const response = await fetch(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/auto-tags/apply`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ order_ids: orderIds })
    }
  );
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('默认标记应用失败', response.status, payload);
  }
  return payload as OrderAutoTagApplyResponse;
};

export const exportReviewHits = async (
  tenantId: string,
  orderIds: string[]
): Promise<ExportJobResponse> => {
  const response = await fetch(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/review-hits/export`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ order_ids: orderIds })
    }
  );
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('审单命中导出失败', response.status, payload);
  }
  const data = payload as { job_id: string; output_url?: string | null };
  return { job_id: data.job_id, output_url: data.output_url ?? null };
};

export const downloadExportJob = async (
  jobId: string
): Promise<Blob> => {
  const response = await fetch(`${apiBaseUrl}/exports/jobs/${jobId}/file`, {
    headers: buildAuthHeaders()
  });
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('导出文件下载失败', response.status, null);
  }
  return response.blob();
};
