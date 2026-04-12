import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  confirmMockPurchaseOrder,
  createMockPurchaseOrder,
  deleteMockPurchaseOrder,
  getMockPurchaseOrderDetail,
  getMockPurchaseOrderList,
  receiveMockPurchaseOrder,
  sendMockPurchaseOrder,
  updateMockPurchaseOrder
} from '@/api/mocks';

export type PurchaseOrderStatus =
  | 'DRAFT'
  | 'SENT'
  | 'CONFIRMED'
  | 'RECEIVING'
  | 'DONE'
  | 'CANCELLED';

export type PurchaseOrderSummary = {
  po_id: string;
  supplier_id: string;
  supplier_name?: string | null;
  warehouse_id?: string | null;
  expected_at?: string | null;
  remark?: string | null;
  status: PurchaseOrderStatus;
  total_cent: number;
  created_at?: string | null;
  updated_at?: string | null;
};

export type PurchaseOrderItem = {
  sku_id: string;
  sku_code?: string | null;
  qty: number;
  price_cent: number;
  received_qty: number;
};

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

export type PurchaseOrderDetail = {
  order: PurchaseOrderSummary;
  items: PurchaseOrderItem[];
  receipts: PurchaseReceiptSummary[];
};

export type PurchaseOrderListQuery = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: PurchaseOrderStatus;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type PurchaseOrderListResponse = {
  items: PurchaseOrderSummary[];
  total: number;
};

export type PurchaseOrderItemPayload = {
  sku_id: string;
  qty: number;
  price_cent: number;
};

export type PurchaseOrderCreatePayload = {
  supplier_id: string;
  warehouse_id: string;
  expected_at?: string | null;
  remark?: string | null;
  items: PurchaseOrderItemPayload[];
};

export type PurchaseOrderUpdatePayload = {
  supplier_id?: string;
  warehouse_id?: string;
  expected_at?: string | null;
  remark?: string | null;
  items?: PurchaseOrderItemPayload[];
};

export type PurchaseOrderReceivePayload = {
  warehouse_id: string;
  items: Array<{
    sku_id: string;
    qty: number;
  }>;
  update_cost?: boolean;
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

export type PurchaseOrderReceiveResponse = {
  order: PurchaseOrderSummary;
  receipt: PurchaseReceipt;
  receipt_items: PurchaseReceiptItem[];
  inbound_doc_id: string;
  ap_doc_id?: string | null;
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

const buildQueryString = (query: PurchaseOrderListQuery): string => {
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

export const listPurchaseOrders = async (
  tenantId: string,
  query: PurchaseOrderListQuery
): Promise<PurchaseOrderListResponse> => {
  if (useMocks) {
    return getMockPurchaseOrderList(query);
  }
  return requestJson<PurchaseOrderListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load purchase orders'
  );
};

export const getPurchaseOrder = async (
  tenantId: string,
  poId: string
): Promise<PurchaseOrderDetail> => {
  if (useMocks) {
    return getMockPurchaseOrderDetail(poId);
  }
  return requestJson<PurchaseOrderDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders/${poId}`,
    { headers: buildAuthHeaders() },
    'Failed to load purchase order'
  );
};

export const createPurchaseOrder = async (
  tenantId: string,
  payload: PurchaseOrderCreatePayload
): Promise<PurchaseOrderDetail> => {
  if (useMocks) {
    return createMockPurchaseOrder(payload);
  }
  return requestJson<PurchaseOrderDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create purchase order'
  );
};

export const updatePurchaseOrder = async (
  tenantId: string,
  poId: string,
  payload: PurchaseOrderUpdatePayload
): Promise<PurchaseOrderDetail> => {
  if (useMocks) {
    return updateMockPurchaseOrder(poId, payload);
  }
  return requestJson<PurchaseOrderDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders/${poId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to update purchase order'
  );
};

export const deletePurchaseOrder = async (
  tenantId: string,
  poId: string
): Promise<PurchaseOrderSummary> => {
  if (useMocks) {
    return deleteMockPurchaseOrder(poId);
  }
  return requestJson<PurchaseOrderSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders/${poId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    'Failed to delete purchase order'
  );
};

export const sendPurchaseOrder = async (
  tenantId: string,
  poId: string
): Promise<PurchaseOrderSummary> => {
  if (useMocks) {
    return sendMockPurchaseOrder(poId);
  }
  return requestJson<PurchaseOrderSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders/${poId}/send`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    'Failed to send purchase order'
  );
};

export const confirmPurchaseOrder = async (
  tenantId: string,
  poId: string
): Promise<PurchaseOrderSummary> => {
  if (useMocks) {
    return confirmMockPurchaseOrder(poId);
  }
  return requestJson<PurchaseOrderSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders/${poId}/confirm`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    'Failed to confirm purchase order'
  );
};

export const receivePurchaseOrder = async (
  tenantId: string,
  poId: string,
  payload: PurchaseOrderReceivePayload
): Promise<PurchaseOrderReceiveResponse> => {
  if (useMocks) {
    return receiveMockPurchaseOrder(poId, payload);
  }
  return requestJson<PurchaseOrderReceiveResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-orders/${poId}/receive`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to receive purchase order'
  );
};
