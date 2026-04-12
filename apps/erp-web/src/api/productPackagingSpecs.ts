import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type ProductPackagingSpec = {
  spec_id: string;
  sku_id: string;
  level_code: string;
  unit_name: string;
  units_per_level: number;
  length_mm?: number | null;
  width_mm?: number | null;
  height_mm?: number | null;
  weight_gram?: number | null;
  is_default: boolean;
  created_at: string;
  updated_at: string;
};

export type ProductPackagingSpecListResponse = {
  items: ProductPackagingSpec[];
};

export type ProductPackagingSpecUpsertPayload = {
  level_code: string;
  unit_name: string;
  units_per_level: number;
  length_mm?: number | null;
  width_mm?: number | null;
  height_mm?: number | null;
  weight_gram?: number | null;
};

const mockSpecs = new Map<string, ProductPackagingSpec[]>();

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

export const listProductPackagingSpecs = async (
  tenantId: string,
  skuId: string
): Promise<ProductPackagingSpecListResponse> => {
  if (useMocks) {
    return { items: mockSpecs.get(`${tenantId}:${skuId}`) ?? [] };
  }
  return requestJson<ProductPackagingSpecListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/${skuId}/packaging-specs`,
    { headers: buildAuthHeaders() },
    'Failed to list packaging specs'
  );
};

export const upsertDefaultProductPackagingSpec = async (
  tenantId: string,
  skuId: string,
  payload: ProductPackagingSpecUpsertPayload
): Promise<ProductPackagingSpec> => {
  if (useMocks) {
    const key = `${tenantId}:${skuId}`;
    const current = mockSpecs.get(key) ?? [];
    const normalized = current
      .filter((item) => item.level_code !== payload.level_code)
      .map((item) => ({ ...item, is_default: false }));
    const now = new Date().toISOString();
    const next: ProductPackagingSpec = {
      spec_id: crypto.randomUUID(),
      sku_id: skuId,
      level_code: payload.level_code,
      unit_name: payload.unit_name,
      units_per_level: payload.units_per_level,
      length_mm: payload.length_mm ?? null,
      width_mm: payload.width_mm ?? null,
      height_mm: payload.height_mm ?? null,
      weight_gram: payload.weight_gram ?? null,
      is_default: true,
      created_at: now,
      updated_at: now
    };
    mockSpecs.set(key, [next, ...normalized]);
    return next;
  }
  return requestJson<ProductPackagingSpec>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/${skuId}/packaging-specs/default`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to upsert packaging spec'
  );
};

