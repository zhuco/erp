import dayjs from 'dayjs';

import type {
  PrintJob,
  Sku,
  SkuBatch,
  SkuBatchCreateRequest,
  SkuBatchListQuery,
  SkuBatchListResponse,
  SkuBatchUpdateRequest,
  SkuCreatePayload,
  SkuKind,
  SkuListQuery,
  SkuListResponse,
  SkuSerial,
  SkuSerialGenerateRequest,
  SkuSerialListQuery,
  SkuSerialListResponse,
  SkuSerialPrintRequest,
  SkuUpdatePayload
} from './data';
import {
  applyPagination,
  buildSkuSearchItem,
  mockProducts,
  mockSkuBatches,
  mockSkuSearchIndex,
  mockSkuSerials,
  mockSkus,
  mockTenantId,
  resolveMockSkuCost
} from './data';

export const getMockSkuList = (query: SkuListQuery): SkuListResponse => {
  const keyword = query.keyword?.trim().toLowerCase();
  const filtered = mockSkus.filter((sku) => {
    if (query.product_id && sku.product_id !== query.product_id) {
      return false;
    }
    if (query.sku_code && sku.sku_code !== query.sku_code) {
      return false;
    }
    if (query.barcode && sku.barcode !== query.barcode) {
      return false;
    }
    if (keyword) {
      const matchesKeyword =
        sku.sku_code.toLowerCase().includes(keyword) ||
        sku.barcode?.toLowerCase().includes(keyword) ||
        sku.name?.toLowerCase().includes(keyword) ||
        sku.tags?.some((tag) => tag.toLowerCase().includes(keyword));
      if (!matchesKeyword) {
        return false;
      }
    }
    if (query.is_active != null && sku.is_active !== query.is_active) {
      return false;
    }
    return true;
  });
  const pageSize = query.limit ?? 20;
  const offset = query.cursor ? Number.parseInt(query.cursor, 10) : 0;
  const safeOffset = Number.isNaN(offset) ? 0 : offset;
  const data = applyPagination(filtered, safeOffset, pageSize).map((sku) => ({
    ...sku,
    cost_cent: resolveMockSkuCost(sku)
  }));
  const nextCursor =
    safeOffset + pageSize < filtered.length ? String(safeOffset + pageSize) : null;
  return {
    data,
    next_cursor: nextCursor
  };
};

export const createMockSku = (payload: SkuCreatePayload): Sku => {
  const now = dayjs().toISOString();
  const isActive = payload.is_active ?? true;
  const status = payload.status ?? (isActive ? 'ACTIVE' : 'INACTIVE');
  const sku: Sku = {
    sku_id: `SKU-${Date.now()}`,
    tenant_id: mockTenantId,
    product_id: payload.product_id,
    sku_code: payload.sku_code,
    barcode: payload.barcode ?? null,
    name: payload.name ?? null,
    note: payload.note ?? null,
    spec: payload.spec ?? null,
    weight_gram: payload.weight_gram ?? null,
    length_mm: payload.length_mm ?? null,
    width_mm: payload.width_mm ?? null,
    height_mm: payload.height_mm ?? null,
    volume_cm3: payload.volume_cm3 ?? null,
    sku_kind: payload.sku_kind ?? 'NORMAL',
    cost_cent: payload.cost_cent ?? null,
    is_active: isActive,
    status,
    batch_enabled: payload.batch_enabled ?? false,
    serial_enabled: payload.serial_enabled ?? false,
    created_at: now,
    updated_at: now
  };
  mockSkus.unshift(sku);
  mockSkuSearchIndex.unshift(buildSkuSearchItem(sku));
  return sku;
};

export const updateMockSku = (skuId: string, payload: SkuUpdatePayload): Sku => {
  const now = dayjs().toISOString();
  const index = mockSkus.findIndex((sku) => sku.sku_id === skuId);
  const base =
    index >= 0
      ? mockSkus[index]
      : ({
          sku_id: skuId,
          tenant_id: mockTenantId,
          product_id: mockProducts[0]?.product_id ?? 'PROD-1',
          created_at: now,
          updated_at: now,
          sku_code: payload.sku_code ?? skuId,
          is_active: payload.is_active ?? true,
          status: payload.status ?? 'ACTIVE'
        } as Sku);
  const isActive = payload.is_active ?? base.is_active;
  const status = payload.status ?? (isActive ? 'ACTIVE' : 'INACTIVE');
  const updated: Sku = {
    ...base,
    sku_code: payload.sku_code ?? base.sku_code,
    barcode: payload.barcode ?? base.barcode ?? null,
    name: payload.name ?? base.name ?? null,
    note: payload.note ?? base.note ?? null,
    spec: payload.spec ?? base.spec ?? null,
    weight_gram: payload.weight_gram ?? base.weight_gram ?? null,
    length_mm: payload.length_mm ?? base.length_mm ?? null,
    width_mm: payload.width_mm ?? base.width_mm ?? null,
    height_mm: payload.height_mm ?? base.height_mm ?? null,
    volume_cm3: payload.volume_cm3 ?? base.volume_cm3 ?? null,
    sku_kind: payload.sku_kind ?? base.sku_kind,
    cost_cent: payload.cost_cent ?? base.cost_cent ?? null,
    is_active: isActive,
    status,
    batch_enabled: payload.batch_enabled ?? base.batch_enabled ?? false,
    serial_enabled: payload.serial_enabled ?? base.serial_enabled ?? false,
    updated_at: now
  };
  if (index >= 0) {
    mockSkus[index] = updated;
  } else {
    mockSkus.unshift(updated);
  }
  const searchIndex = mockSkuSearchIndex.findIndex((sku) => sku.sku_id === skuId);
  const searchItem = buildSkuSearchItem(updated);
  if (searchIndex >= 0) {
    mockSkuSearchIndex[searchIndex] = searchItem;
  } else {
    mockSkuSearchIndex.unshift(searchItem);
  }
  return updated;
};

export const getMockSkuBatchList = (query: SkuBatchListQuery): SkuBatchListResponse => {
  const filtered = mockSkuBatches.filter((batch) => {
    if (query.sku_id && batch.sku_id !== query.sku_id) {
      return false;
    }
    if (query.batch_no && batch.batch_no !== query.batch_no) {
      return false;
    }
    return true;
  });
  const pageSize = query.limit ?? 20;
  const offset = query.cursor ? Number.parseInt(query.cursor, 10) : 0;
  const safeOffset = Number.isNaN(offset) ? 0 : offset;
  const data = applyPagination(filtered, safeOffset, pageSize);
  const nextCursor =
    safeOffset + pageSize < filtered.length ? String(safeOffset + pageSize) : null;
  return {
    data,
    next_cursor: nextCursor
  };
};

export const createMockSkuBatch = (payload: SkuBatchCreateRequest): SkuBatch => {
  const now = dayjs().toISOString();
  const batch: SkuBatch = {
    batch_id: `BATCH-${Date.now()}`,
    tenant_id: mockTenantId,
    sku_id: payload.sku_id,
    batch_no: payload.batch_no,
    expire_at: payload.expire_at ?? null,
    created_at: now,
    updated_at: now
  };
  mockSkuBatches.unshift(batch);
  return batch;
};

export const updateMockSkuBatch = (
  batchId: string,
  payload: SkuBatchUpdateRequest
): SkuBatch => {
  const now = dayjs().toISOString();
  const index = mockSkuBatches.findIndex((batch) => batch.batch_id === batchId);
  const base =
    index >= 0
      ? mockSkuBatches[index]
      : {
          batch_id: batchId,
          tenant_id: mockTenantId,
          sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
          batch_no: payload.batch_no ?? batchId,
          expire_at: payload.expire_at ?? null,
          created_at: now,
          updated_at: now
        };
  const updated: SkuBatch = {
    ...base,
    batch_no: payload.batch_no ?? base.batch_no,
    expire_at: payload.expire_at ?? base.expire_at,
    updated_at: now
  };
  if (index >= 0) {
    mockSkuBatches[index] = updated;
  } else {
    mockSkuBatches.unshift(updated);
  }
  return updated;
};

export const getMockSkuSerialList = (query: SkuSerialListQuery): SkuSerialListResponse => {
  const filtered = mockSkuSerials.filter((serial) => {
    if (query.sku_id && serial.sku_id !== query.sku_id) {
      return false;
    }
    if (query.status && serial.status !== query.status) {
      return false;
    }
    if (query.serial_no && serial.serial_no !== query.serial_no) {
      return false;
    }
    return true;
  });
  const pageSize = query.limit ?? 20;
  const offset = query.cursor ? Number.parseInt(query.cursor, 10) : 0;
  const safeOffset = Number.isNaN(offset) ? 0 : offset;
  const data = applyPagination(filtered, safeOffset, pageSize);
  const nextCursor =
    safeOffset + pageSize < filtered.length ? String(safeOffset + pageSize) : null;
  return {
    data,
    next_cursor: nextCursor
  };
};

export const generateMockSkuSerials = (
  payload: SkuSerialGenerateRequest
): SkuSerial[] => {
  const now = dayjs();
  const dateKey = now.format('YYYYMMDDHHmmss');
  const generated = Array.from({ length: payload.count }, (_, index) => {
    const serialNo = `${payload.prefix}${dateKey}${String(index + 1).padStart(6, '0')}`;
    const serial: SkuSerial = {
      serial_id: `SERIAL-${Date.now()}-${index}`,
      tenant_id: mockTenantId,
      sku_id: payload.sku_id,
      serial_no: serialNo,
      status: 'IN_STOCK',
      ref_doc_type: null,
      ref_doc_id: null,
      created_at: now.toISOString(),
      updated_at: now.toISOString()
    };
    mockSkuSerials.unshift(serial);
    return serial;
  });
  return generated;
};

export const printMockSkuSerials = (payload: SkuSerialPrintRequest): PrintJob => {
  const now = dayjs().toISOString();
  return {
    job_id: `JOB-${Date.now()}`,
    tenant_id: mockTenantId,
    job_type: 'SKU_SERIAL_LABEL',
    ref_type: 'SKU_SERIAL',
    ref_id: payload.serial_ids[0] ?? 'SKU_SERIAL',
    template_id: payload.template_id,
    template_version: null,
    requested_by_user_id: null,
    status: 'DONE',
    payload: { serial_ids: payload.serial_ids },
    output_url: null,
    attempts: 1,
    last_error: '',
    created_at: now,
    updated_at: now,
    events: []
  };
};

export const searchMockSkus = (keyword: string, skuKind?: SkuKind) => {
  const normalized = keyword.trim().toLowerCase();
  const filtered = mockSkuSearchIndex.filter((sku) => {
    if (skuKind && sku.sku_kind !== skuKind) {
      return false;
    }
    if (!normalized) {
      return true;
    }
    return (
      sku.sku_code.toLowerCase().includes(normalized) ||
      sku.barcode?.toLowerCase().includes(normalized) ||
      sku.name?.toLowerCase().includes(normalized) ||
      sku.tags?.some((tag) => tag.toLowerCase().includes(normalized))
    );
  });
  return {
    items: filtered.slice(0, 20)
  };
};
