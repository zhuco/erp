import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';
import { getMockInventoryLedgers } from '@/api/mocks';

export type InventoryLedgerReason =
  | 'STOCK_COUNT_ADJUST'
  | 'PURCHASE_RECEIPT'
  | 'SALES_SHIP'
  | 'RESERVATION_CREATE'
  | 'RESERVATION_RELEASE'
  | 'TRANSFER_OUT'
  | 'TRANSFER_IN'
  | 'RETURN_IN';

export type InventoryLedger = {
  ledger_id: string;
  sku_id: string;
  sku_code: string;
  warehouse_id: string;
  delta_on_hand: number;
  delta_reserved: number;
  delta_in_transit: number;
  delta_frozen: number;
  reason_type: InventoryLedgerReason;
  ref_type?: string | null;
  ref_id?: string | null;
  occurred_at: string;
};

export type InventoryLedgerListQuery = {
  sku_id?: string;
  sku_code?: string;
  warehouse_id?: string;
  occurred_from?: string;
  occurred_to?: string;
  offset?: number;
  limit?: number;
};

export type InventoryLedgerListResponse = {
  items: InventoryLedger[];
  total: number;
};

type InventoryLedgerListApiResponse = {
  items?: InventoryLedger[];
  data?: InventoryLedger[];
  total?: number;
};

const INVENTORY_REQUEST_TIMEOUT_MS = 15000;
const enableInventoryDiagnostics = import.meta.env.DEV || import.meta.env.MODE === 'test';

const logInventoryRequest = ({
  url,
  method,
  status,
  durationMs,
  errorSummary
}: {
  url: string;
  method: string;
  status?: number;
  durationMs: number;
  errorSummary?: string;
}) => {
  if (!enableInventoryDiagnostics) {
    return;
  }
  const statusLabel = status ?? 'NETWORK_ERROR';
  const summary = errorSummary ? ` error="${errorSummary}"` : '';
  console.info(
    `[inventory] ${method.toUpperCase()} ${url} status=${statusLabel} duration=${durationMs}ms${summary}`
  );
};

const buildQueryString = (query: InventoryLedgerListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.sku_code) {
    params.set('sku_code', query.sku_code);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.occurred_from) {
    params.set('occurred_from', query.occurred_from);
  }
  if (query.occurred_to) {
    params.set('occurred_to', query.occurred_to);
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

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const url = typeof input === 'string' ? input : input.toString();
  const method = init.method ?? 'GET';
  const start = Date.now();
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    INVENTORY_REQUEST_TIMEOUT_MS
  );

  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
    let payload: unknown = undefined;
    try {
      payload = await response.json();
    } catch {
      payload = undefined;
    }
    const durationMs = Date.now() - start;
    if (!response.ok) {
      const errorSummary = isErrorResponse(payload)
        ? payload.message
        : response.statusText || 'request failed';
      logInventoryRequest({
        url,
        method,
        status: response.status,
        durationMs,
        errorSummary
      });
      handleApiError(response.status);
      throw createApiError(message, response.status, payload);
    }
    logInventoryRequest({ url, method, status: response.status, durationMs });
    return payload as T;
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    const durationMs = Date.now() - start;
    const isTimeout = error instanceof DOMException && error.name === 'AbortError';
    const status = isTimeout ? 408 : undefined;
    const errorSummary = error instanceof Error ? error.message : 'request failed';
    logInventoryRequest({ url, method, status, durationMs, errorSummary });
    handleApiError(status);
    throw createApiError(isTimeout ? '请求超时，请重试' : message, status);
  } finally {
    globalThis.clearTimeout(timeoutId);
  }
};

export const listInventoryLedgers = async (
  tenantId: string,
  query: InventoryLedgerListQuery
): Promise<InventoryLedgerListResponse> => {
  if (useMocks) {
    return getMockInventoryLedgers(query);
  }
  const response = await requestJson<InventoryLedgerListApiResponse | InventoryLedgerListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/ledgers${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存流水失败'
  );
  const items = response.items ?? response.data ?? [];
  const offset = query.offset ?? 0;
  const limit = query.limit ?? items.length;
  return {
    items,
    total: response.total ?? offset + items.length + (items.length === limit ? limit : 0)
  };
};
