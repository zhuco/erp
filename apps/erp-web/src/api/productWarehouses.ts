import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductWarehouse,
  deleteMockProductWarehouse,
  getMockProductWarehouseList,
  updateMockProductWarehouse
} from '@/api/mocks';

export type ProductWarehouse = {
  tenant_id: string;
  product_id: string;
  warehouse_id: string;
  warehouse_code: string;
  warehouse_name: string;
  is_default: boolean;
  allow_negative_stock: boolean;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductWarehouseListResponse = {
  items: ProductWarehouse[];
};

export type ProductWarehouseCreatePayload = {
  warehouse_id: string;
  is_default?: boolean;
  allow_negative_stock?: boolean;
};

export type ProductWarehouseUpdatePayload = {
  is_default?: boolean;
  allow_negative_stock?: boolean;
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

export const listProductWarehouses = async (
  tenantId: string,
  productId: string
): Promise<ProductWarehouseListResponse> => {
  if (useMocks) {
    return getMockProductWarehouseList(productId);
  }
  return requestJson<ProductWarehouseListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/products/${productId}/warehouses`,
    { headers: buildAuthHeaders() },
    '获取商品仓库失败'
  );
};

export const createProductWarehouse = async (
  tenantId: string,
  productId: string,
  payload: ProductWarehouseCreatePayload
): Promise<ProductWarehouse> => {
  if (useMocks) {
    return createMockProductWarehouse(productId, payload);
  }
  return requestJson<ProductWarehouse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/products/${productId}/warehouses`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '新增商品仓库失败'
  );
};

export const updateProductWarehouse = async (
  tenantId: string,
  productId: string,
  warehouseId: string,
  payload: ProductWarehouseUpdatePayload
): Promise<ProductWarehouse> => {
  if (useMocks) {
    return updateMockProductWarehouse(productId, warehouseId, payload);
  }
  return requestJson<ProductWarehouse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/products/${productId}/warehouses/${warehouseId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新商品仓库失败'
  );
};

export const deleteProductWarehouse = async (
  tenantId: string,
  productId: string,
  warehouseId: string
): Promise<ProductWarehouse> => {
  if (useMocks) {
    return deleteMockProductWarehouse(productId, warehouseId);
  }
  return requestJson<ProductWarehouse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/products/${productId}/warehouses/${warehouseId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除商品仓库失败'
  );
};
