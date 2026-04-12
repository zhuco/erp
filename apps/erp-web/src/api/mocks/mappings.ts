import dayjs from 'dayjs';

import type { MappingListQuery, MappingPayload, MappingRecord } from './data';
import {
  applyPagination,
  mockMappings,
  mockTenantId,
  resolveMappingSkuInfo
} from './data';

export const getMockMappingList = (query: MappingListQuery) => {
  const normalizedSkuCode = query.sku_code?.trim().toLowerCase();
  const normalizedBarcode = query.barcode?.trim().toLowerCase();
  const normalizedPlatformSku = query.platform_sku_id?.trim().toLowerCase();
  const normalizedShop = query.shop_id?.trim().toLowerCase();
  const filtered = mockMappings.filter((mapping) => {
    if (query.platform && mapping.platform !== query.platform) {
      return false;
    }
    if (normalizedShop && mapping.shop_id.toLowerCase() !== normalizedShop) {
      return false;
    }
    if (
      normalizedPlatformSku &&
      !mapping.platform_sku_id.toLowerCase().includes(normalizedPlatformSku)
    ) {
      return false;
    }
    if (normalizedSkuCode && !mapping.sku_code.toLowerCase().includes(normalizedSkuCode)) {
      return false;
    }
    if (normalizedBarcode && !(mapping.barcode ?? '').toLowerCase().includes(normalizedBarcode)) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? 20;
  return {
    items: applyPagination(filtered, offset, limit),
    total: filtered.length
  };
};

export const createMockMapping = (payload: MappingPayload): MappingRecord => {
  const now = dayjs().toISOString();
  const skuInfo = resolveMappingSkuInfo(payload);
  const mapping: MappingRecord = {
    mapping_id: `MAP-${Date.now()}`,
    tenant_id: mockTenantId,
    platform: payload.platform,
    shop_id: payload.shop_id,
    platform_sku_id: payload.platform_sku_id,
    outer_sku_id: payload.outer_sku_id ?? null,
    sku_id: payload.sku_id,
    sku_code: skuInfo.sku_code,
    barcode: skuInfo.barcode ?? null,
    created_at: now,
    updated_at: now
  };
  mockMappings.unshift(mapping);
  return mapping;
};

export const updateMockMapping = (
  mappingId: string,
  payload: MappingPayload
): MappingRecord => {
  const now = dayjs().toISOString();
  const skuInfo = resolveMappingSkuInfo(payload);
  const index = mockMappings.findIndex((mapping) => mapping.mapping_id === mappingId);
  const base =
    index >= 0
      ? mockMappings[index]
      : ({
          mapping_id: mappingId,
          tenant_id: mockTenantId,
          created_at: now
        } as MappingRecord);
  const updated: MappingRecord = {
    ...base,
    platform: payload.platform,
    shop_id: payload.shop_id,
    platform_sku_id: payload.platform_sku_id,
    outer_sku_id: payload.outer_sku_id ?? null,
    sku_id: payload.sku_id,
    sku_code: skuInfo.sku_code,
    barcode: skuInfo.barcode ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockMappings[index] = updated;
  } else {
    mockMappings.unshift(updated);
  }
  return updated;
};
