import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockPurchaseReceiptDetail, getMockPurchaseReceiptList } from '@/api/mocks';

export type PurchaseReceiptSummary = {
  receipt_id: string;
  po_id: string;
  supplier_id: string;
  supplier_name?: string | null;
  warehouse_id: string;
  status: string;
  confirmed_at?: string | null;
  created_at?: string | null;
};

export type PurchaseReceipt = {
  receipt_id: string;
  po_id: string;
  supplier_id?: string | null;
  supplier_name?: string | null;
  warehouse_id: string;
  status: string;
  confirmed_at?: string | null;
  created_at?: string | null;
};

export type PurchaseReceiptItem = {
  sku_id: string;
  qty: number;
};

export type PurchaseReceiptDetail = {
  receipt: PurchaseReceipt;
  items: PurchaseReceiptItem[];
};

export type PurchaseReceiptListQuery = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type PurchaseReceiptListResponse = {
  items: PurchaseReceiptSummary[];
  total: number;
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetchWithAuth(input, init);
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError(message, response.status, payload);
  }
  return payload as T;
};

const buildQueryString = (query: PurchaseReceiptListQuery): string => {
  const params = new URLSearchParams();
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.created_from) {
    params.set('created_from', query.created_from);
  }
  if (query.created_to) {
    params.set('created_to', query.created_to);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

export const listPurchaseReceipts = async (
  tenantId: string,
  query: PurchaseReceiptListQuery
): Promise<PurchaseReceiptListResponse> => {
  if (useMocks) {
    return getMockPurchaseReceiptList(query);
  }
  return requestJson<PurchaseReceiptListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-receipts${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load purchase receipts'
  );
};

export const getPurchaseReceiptDetail = async (
  tenantId: string,
  receiptId: string
): Promise<PurchaseReceiptDetail> => {
  if (useMocks) {
    return getMockPurchaseReceiptDetail(receiptId);
  }
  return requestJson<PurchaseReceiptDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-receipts/${receiptId}`,
    { headers: buildAuthHeaders() },
    'Failed to load purchase receipt'
  );
};
