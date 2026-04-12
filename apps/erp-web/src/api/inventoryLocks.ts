import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';
import {
  getMockInventoryLockDetail,
  getMockInventoryLockLogs,
  listMockNegativeStockPolicies,
  prioritizeMockInventoryLock,
  recalcMockInventoryLocks,
  releaseMockInventoryLock,
  updateMockNegativeStockPolicy
} from '@/api/mocks';

export type InventoryLockSourceType =
  | 'ORDER'
  | 'OUTBOUND'
  | 'PRE_SALE'
  | 'TRANSFER'
  | 'RETURN'
  | 'MANUAL';

export type InventoryLockType =
  | 'STANDARD'
  | 'PRIORITY'
  | 'PRE_SALE'
  | 'TRANSFER'
  | 'SAFETY';

export type InventoryLockStatus = 'ACTIVE' | 'RELEASED' | 'OVERRIDDEN';

export type InventoryLockAuditAction =
  | 'RECALCULATE'
  | 'PRIORITIZE'
  | 'RELEASE'
  | 'NEGATIVE_POLICY_UPDATE';

export type InventoryLockLog = {
  lock_id: string;
  sku_id: string;
  sku_code?: string | null;
  warehouse_id: string;
  locked_qty: number;
  unlocked_qty: number;
  available_after: number;
  source_type: InventoryLockSourceType;
  source_doc_no: string;
  lock_type: InventoryLockType;
  operator_name?: string | null;
  operator_type?: 'USER' | 'SYSTEM';
  occurred_at: string;
};

export type InventoryLockLogListQuery = {
  sku_id?: string;
  sku_code?: string;
  warehouse_id?: string;
  doc_no?: string;
  lock_type?: InventoryLockType;
  source_type?: InventoryLockSourceType;
  occurred_from?: string;
  occurred_to?: string;
  offset?: number;
  limit?: number;
};

export type InventoryLockLogListResponse = {
  items: InventoryLockLog[];
  total: number;
};

export type InventoryLockDocument = {
  doc_type: string;
  doc_no: string;
  qty: number;
  priority: number;
  status: string;
};

export type InventoryLockAuditEvent = {
  action: InventoryLockAuditAction;
  operator: string;
  note?: string | null;
  created_at: string;
};

export type InventoryLockDetail = {
  lock_id: string;
  sku_id: string;
  sku_code?: string | null;
  warehouse_id: string;
  locked_qty: number;
  unlocked_qty: number;
  available_after: number;
  source_type: InventoryLockSourceType;
  source_doc_no: string;
  lock_type: InventoryLockType;
  priority: number;
  status: InventoryLockStatus;
  operator_name?: string | null;
  created_at: string;
  updated_at: string;
  documents: InventoryLockDocument[];
  audit_trail: InventoryLockAuditEvent[];
};

export type InventoryLockRecalculatePayload = {
  scope?: 'GLOBAL' | 'TARGET';
  sku_id?: string;
  warehouse_id?: string;
  reason?: string;
};

export type InventoryLockRecalculateResult = {
  scope: 'GLOBAL' | 'TARGET';
  recalculated: number;
  affected_skus?: string[];
  updated_at?: string;
};

export type InventoryLockPriorityPayload = {
  lock_id: string;
  reason?: string;
};

export type InventoryLockReleasePayload = {
  lock_id: string;
  reason?: string;
};

export type NegativeStockPolicyScope = 'WAREHOUSE' | 'SKU';

export type NegativeStockPolicy = {
  policy_id: string;
  scope: NegativeStockPolicyScope;
  warehouse_id: string;
  sku_id?: string | null;
  sku_code?: string | null;
  allow_negative: boolean;
  note?: string | null;
  updated_at: string;
  updated_by?: string | null;
};

export type NegativeStockPolicyListQuery = {
  warehouse_id?: string;
  sku_id?: string;
};

export type NegativeStockPolicyListResponse = {
  items: NegativeStockPolicy[];
  total: number;
};

export type NegativeStockPolicyUpdatePayload = {
  scope: NegativeStockPolicyScope;
  warehouse_id: string;
  sku_id?: string;
  sku_code?: string;
  allow_negative: boolean;
  note?: string;
};

type InventoryLockLogListApiResponse = {
  items?: InventoryLockLog[];
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

const buildQueryString = (query: InventoryLockLogListQuery): string => {
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
  if (query.doc_no) {
    params.set('doc_no', query.doc_no);
  }
  if (query.lock_type) {
    params.set('lock_type', query.lock_type);
  }
  if (query.source_type) {
    params.set('source_type', query.source_type);
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

export const listInventoryLockLogs = async (
  tenantId: string,
  query: InventoryLockLogListQuery
): Promise<InventoryLockLogListResponse> => {
  if (useMocks) {
    return getMockInventoryLockLogs(query);
  }
  const response = await requestJson<InventoryLockLogListApiResponse | InventoryLockLogListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/locks${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取库存锁定日志失败'
  );
  return {
    items: response.items ?? [],
    total: response.total ?? 0
  };
};

export const getInventoryLockDetail = async (
  tenantId: string,
  lockId: string
): Promise<InventoryLockDetail> => {
  if (useMocks) {
    return getMockInventoryLockDetail(lockId);
  }
  return requestJson<InventoryLockDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/locks/${lockId}`,
    { headers: buildAuthHeaders() },
    '获取锁定详情失败'
  );
};

export const recalculateInventoryLocks = async (
  tenantId: string,
  payload: InventoryLockRecalculatePayload
): Promise<InventoryLockRecalculateResult> => {
  if (useMocks) {
    return recalcMockInventoryLocks(payload);
  }
  return requestJson<InventoryLockRecalculateResult>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/locks/recalculate`,
    {
      method: 'POST',
      headers: buildAuthHeaders(),
      body: JSON.stringify(payload)
    },
    '重算锁定失败'
  );
};

export const prioritizeInventoryLock = async (
  tenantId: string,
  payload: InventoryLockPriorityPayload
): Promise<InventoryLockDetail> => {
  if (useMocks) {
    return prioritizeMockInventoryLock(payload);
  }
  return requestJson<InventoryLockDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/locks/${payload.lock_id}/prioritize`,
    {
      method: 'POST',
      headers: buildAuthHeaders(),
      body: JSON.stringify(payload)
    },
    '抢占锁定失败'
  );
};

export const releaseInventoryLock = async (
  tenantId: string,
  payload: InventoryLockReleasePayload
): Promise<InventoryLockDetail> => {
  if (useMocks) {
    return releaseMockInventoryLock(payload);
  }
  return requestJson<InventoryLockDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/locks/${payload.lock_id}/release`,
    {
      method: 'POST',
      headers: buildAuthHeaders(),
      body: JSON.stringify(payload)
    },
    '释放锁定失败'
  );
};

export const listNegativeStockPolicies = async (
  tenantId: string,
  query: NegativeStockPolicyListQuery
): Promise<NegativeStockPolicyListResponse> => {
  if (useMocks) {
    return listMockNegativeStockPolicies(query);
  }
  const response = await requestJson<NegativeStockPolicyListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/negative-stock-policies${buildQueryString(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '获取负库存策略失败'
  );
  return {
    items: response.items ?? [],
    total: response.total ?? 0
  };
};

export const updateNegativeStockPolicy = async (
  tenantId: string,
  payload: NegativeStockPolicyUpdatePayload
): Promise<NegativeStockPolicy> => {
  if (useMocks) {
    return updateMockNegativeStockPolicy(payload);
  }
  return requestJson<NegativeStockPolicy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/negative-stock-policies`,
    {
      method: 'POST',
      headers: buildAuthHeaders(),
      body: JSON.stringify(payload)
    },
    '更新负库存策略失败'
  );
};
