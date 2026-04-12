import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockSupplier,
  deleteMockSupplier,
  getMockSupplierList,
  updateMockSupplier
} from '@/api/mocks';

export type Supplier = {
  supplier_id: string;
  tenant_id: string;
  name: string;
  contact?: string | null;
  settlement_terms?: string | null;
  status?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type SupplierListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type SupplierListResponse = {
  items: Supplier[];
  total: number;
};

export type SupplierCreatePayload = {
  name: string;
  contact?: string;
  settlement_terms?: string;
  status?: string;
};

export type SupplierUpdatePayload = {
  name?: string;
  contact?: string;
  settlement_terms?: string;
  status?: string;
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

const buildQueryString = (query: SupplierListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
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

export const listSuppliers = async (
  tenantId: string,
  query: SupplierListQuery
): Promise<SupplierListResponse> => {
  if (useMocks) {
    return getMockSupplierList(query);
  }
  return requestJson<SupplierListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/suppliers${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取供应商列表失败'
  );
};

export const createSupplier = async (
  tenantId: string,
  payload: SupplierCreatePayload
): Promise<Supplier> => {
  if (useMocks) {
    return createMockSupplier(payload);
  }
  return requestJson<Supplier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/suppliers`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建供应商失败'
  );
};

export const updateSupplier = async (
  tenantId: string,
  supplierId: string,
  payload: SupplierUpdatePayload
): Promise<Supplier> => {
  if (useMocks) {
    return updateMockSupplier(supplierId, payload);
  }
  return requestJson<Supplier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/suppliers/${supplierId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新供应商失败'
  );
};

export const deleteSupplier = async (
  tenantId: string,
  supplierId: string
): Promise<Supplier> => {
  if (useMocks) {
    return deleteMockSupplier(supplierId);
  }
  return requestJson<Supplier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/suppliers/${supplierId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除供应商失败'
  );
};
