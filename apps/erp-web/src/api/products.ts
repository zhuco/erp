import { apiClient, unwrapApiResponse } from '@/api/client';
import { useMocks } from '@/utils/env';
import {
  getMockProduct,
  createMockProduct,
  deleteMockProduct,
  getMockProductList,
  updateMockProduct,
} from '@/api/mocks';

import type { components, paths } from './generated/schema';

export type Product = components['schemas']['Product'];
export type ProductListResponse = components['schemas']['ProductListResponse'];
export type ProductListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/products']['get']['parameters']['query']
>;
export type ProductCreateRequest = components['schemas']['ProductCreateRequest'];
export type ProductUpdateRequest = components['schemas']['ProductUpdateRequest'];
export type ProductDetailResponse = components['schemas']['Product'];

export const listProducts = async (
  tenantId: string,
  query: ProductListQuery,
): Promise<ProductListResponse> => {
  if (useMocks) {
    return getMockProductList(query);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/products', {
    params: {
      path: { tenant_id: tenantId },
      query,
    },
  });
  return unwrapApiResponse(result, '获取商品列表失败');
};

export const createProduct = async (
  tenantId: string,
  payload: ProductCreateRequest,
): Promise<Product> => {
  if (useMocks) {
    return createMockProduct(payload);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/products', {
    params: {
      path: { tenant_id: tenantId },
    },
    body: payload,
  });
  return unwrapApiResponse(result, '创建商品失败');
};

export const getProduct = async (
  tenantId: string,
  productId: string,
): Promise<ProductDetailResponse> => {
  if (useMocks) {
    return getMockProduct(productId);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/products/{product_id}', {
    params: {
      path: { tenant_id: tenantId, product_id: productId },
    },
  });
  return unwrapApiResponse(result, '获取商品详情失败');
};

export const updateProduct = async (
  tenantId: string,
  productId: string,
  payload: ProductUpdateRequest,
): Promise<Product> => {
  if (useMocks) {
    return updateMockProduct(productId, payload);
  }
  const result = await apiClient.PUT('/v1/tenants/{tenant_id}/products/{product_id}', {
    params: {
      path: { tenant_id: tenantId, product_id: productId },
    },
    body: payload,
  });
  return unwrapApiResponse(result, '更新商品失败');
};

export const deleteProduct = async (tenantId: string, productId: string): Promise<Product> => {
  if (useMocks) {
    return deleteMockProduct(productId);
  }
  const result = await apiClient.DELETE('/v1/tenants/{tenant_id}/products/{product_id}', {
    params: {
      path: { tenant_id: tenantId, product_id: productId },
    },
  });
  return unwrapApiResponse(result, '删除商品失败');
};
