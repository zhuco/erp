import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductBarcode,
  deleteMockProductBarcode,
  getMockProductBarcode,
  getMockProductBarcodeList,
  updateMockProductBarcode
} from '@/api/mocks';

export type ProductBarcode = {
  barcode_id: string;
  tenant_id: string;
  sku_id: string;
  sku_code?: string | null;
  sku_name?: string | null;
  barcode: string;
  level: string;
  unit: string;
  ratio: number;
  is_default: boolean;
  status: string;
  print_template?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductBarcodeListQuery = {
  sku_id?: string;
  barcode?: string;
  status?: string;
  level?: string;
  offset?: number;
  limit?: number;
};

export type ProductBarcodeListResponse = {
  items: ProductBarcode[];
  total: number;
};

export type ProductBarcodeCreatePayload = {
  sku_id: string;
  barcode: string;
  level: string;
  unit: string;
  ratio?: number;
  is_default?: boolean;
  status?: string;
  print_template?: string | null;
};

export type ProductBarcodeUpdatePayload = {
  barcode?: string;
  level?: string;
  unit?: string;
  ratio?: number;
  is_default?: boolean;
  status?: string;
  print_template?: string | null;
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

const buildQueryString = (query: ProductBarcodeListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.barcode) {
    params.set('barcode', query.barcode);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.level) {
    params.set('level', query.level);
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

export const listProductBarcodes = async (
  tenantId: string,
  query: ProductBarcodeListQuery
): Promise<ProductBarcodeListResponse> => {
  if (useMocks) {
    return getMockProductBarcodeList(query);
  }
  return requestJson<ProductBarcodeListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-barcodes${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取商品条码失败'
  );
};

export const getProductBarcode = async (
  tenantId: string,
  barcodeId: string
): Promise<ProductBarcode> => {
  if (useMocks) {
    return getMockProductBarcode(barcodeId);
  }
  return requestJson<ProductBarcode>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-barcodes/${barcodeId}`,
    { headers: buildAuthHeaders() },
    '获取条码详情失败'
  );
};

export const createProductBarcode = async (
  tenantId: string,
  payload: ProductBarcodeCreatePayload
): Promise<ProductBarcode> => {
  if (useMocks) {
    return createMockProductBarcode(payload);
  }
  return requestJson<ProductBarcode>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-barcodes`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建条码失败'
  );
};

export const updateProductBarcode = async (
  tenantId: string,
  barcodeId: string,
  payload: ProductBarcodeUpdatePayload
): Promise<ProductBarcode> => {
  if (useMocks) {
    return updateMockProductBarcode(barcodeId, payload);
  }
  return requestJson<ProductBarcode>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-barcodes/${barcodeId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新条码失败'
  );
};

export const deleteProductBarcode = async (
  tenantId: string,
  barcodeId: string
): Promise<ProductBarcode> => {
  if (useMocks) {
    return deleteMockProductBarcode(barcodeId);
  }
  return requestJson<ProductBarcode>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-barcodes/${barcodeId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除条码失败'
  );
};
