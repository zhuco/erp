import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockInventoryList } from '@/api/mocks';

export type InventoryAtpAllocation = {
  batch_id: string;
  batch_no: string;
  expire_at?: string | null;
  allocated_qty: number;
};

export type InventoryAtpResponse = {
  warehouse_id: string;
  sku_id: string;
  on_hand: number;
  reserved: number;
  frozen: number;
  available: number;
  request_qty: number;
  atp_qty: number;
  shortage_qty: number;
  allocations: InventoryAtpAllocation[];
};

export type InventoryAtpQuery = {
  warehouse_id: string;
  sku_id: string;
  request_qty?: number;
};

export type InventoryReconciliationCheckPayload = {
  warehouse_id: string;
  sku_id: string;
  expected_on_hand: number;
  source_ref?: string | null;
};

export type InventoryReconciliationResult = {
  reconciliation_id: string;
  warehouse_id: string;
  sku_id: string;
  expected_on_hand: number;
  actual_on_hand: number;
  diff_qty: number;
  status: string;
  source_ref?: string | null;
  created_at: string;
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

export const getInventoryAtp = async (
  tenantId: string,
  query: InventoryAtpQuery
): Promise<InventoryAtpResponse> => {
  if (useMocks) {
    const mock = getMockInventoryList({
      warehouse_id: query.warehouse_id,
      sku_id: query.sku_id,
      offset: 0,
      limit: 1
    });
    const item = mock.items[0];
    const available = item?.available ?? 0;
    const requestQty = Math.max(0, query.request_qty ?? 0);
    return {
      warehouse_id: query.warehouse_id,
      sku_id: query.sku_id,
      on_hand: item?.on_hand ?? 0,
      reserved: item?.reserved ?? 0,
      frozen: item?.frozen ?? 0,
      available,
      request_qty: requestQty,
      atp_qty: Math.min(available, requestQty),
      shortage_qty: Math.max(0, requestQty - available),
      allocations: []
    };
  }

  const params = new URLSearchParams();
  params.set('warehouse_id', query.warehouse_id);
  params.set('sku_id', query.sku_id);
  if (query.request_qty != null) {
    params.set('request_qty', String(query.request_qty));
  }

  return requestJson<InventoryAtpResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/atp?${params.toString()}`,
    {
      headers: buildAuthHeaders()
    },
    'Failed to fetch ATP quote'
  );
};

export const checkInventoryReconciliation = async (
  tenantId: string,
  payload: InventoryReconciliationCheckPayload
): Promise<InventoryReconciliationResult> => {
  if (useMocks) {
    const mock = getMockInventoryList({
      warehouse_id: payload.warehouse_id,
      sku_id: payload.sku_id,
      offset: 0,
      limit: 1
    });
    const actualOnHand = mock.items[0]?.on_hand ?? 0;
    return {
      reconciliation_id: crypto.randomUUID(),
      warehouse_id: payload.warehouse_id,
      sku_id: payload.sku_id,
      expected_on_hand: payload.expected_on_hand,
      actual_on_hand: actualOnHand,
      diff_qty: actualOnHand - payload.expected_on_hand,
      status: actualOnHand === payload.expected_on_hand ? 'MATCH' : 'MISMATCH',
      source_ref: payload.source_ref ?? null,
      created_at: new Date().toISOString()
    };
  }

  return requestJson<InventoryReconciliationResult>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/reconciliation-center/check`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to execute inventory reconciliation'
  );
};

