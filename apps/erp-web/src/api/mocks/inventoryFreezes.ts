import dayjs from 'dayjs';

import type {
  InventoryFreezeCreatePayload,
  InventoryFreezeListQuery,
  InventoryFreezeListResponse,
} from '../inventoryFreezes';
import type { InventoryFreezeRecord, SkuBatch } from './data';
import {
  applyPagination,
  mockInventoryFreezes,
  mockWarehouses,
} from './data';

const defaultWarehouseId = mockWarehouses[0]?.warehouse_id ?? 'WH-001';

const findActiveFreeze = (skuId: string, batchNo?: string | null) =>
  mockInventoryFreezes.find(
    (item) =>
      item.sku_id === skuId &&
      item.status === 'ACTIVE' &&
      (batchNo ? item.batch_no === batchNo : true),
  );

export const getMockInventoryFreezeList = (
  query: InventoryFreezeListQuery,
): InventoryFreezeListResponse => {
  const filtered = mockInventoryFreezes.filter((item) => {
    if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.sku_id && item.sku_id !== query.sku_id) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length,
  };
};

export const createMockInventoryFreeze = (
  payload: InventoryFreezeCreatePayload,
): InventoryFreezeRecord => {
  const existing = findActiveFreeze(payload.sku_id);
  if (existing) {
    existing.qty = Math.max(existing.qty, payload.qty);
    existing.reason = payload.reason ?? existing.reason ?? null;
    existing.updated_at = dayjs().toISOString();
    return existing;
  }
  const now = dayjs().toISOString();
  const record: InventoryFreezeRecord = {
    freeze_id: `FREEZE-${Date.now()}`,
    warehouse_id: payload.warehouse_id,
    sku_id: payload.sku_id,
    qty: payload.qty,
    reason: payload.reason ?? null,
    status: 'ACTIVE',
    created_at: now,
    updated_at: now,
    batch_no: null,
  };
  mockInventoryFreezes.unshift(record);
  return record;
};

export const cancelMockInventoryFreeze = (freezeId: string): InventoryFreezeRecord => {
  const target = mockInventoryFreezes.find((item) => item.freeze_id === freezeId);
  if (!target) {
    const now = dayjs().toISOString();
    return {
      freeze_id: freezeId,
      warehouse_id: defaultWarehouseId,
      sku_id: 'SKU-UNKNOWN',
      qty: 0,
      reason: '未找到冻结记录',
      status: 'CANCELLED',
      created_at: now,
      updated_at: now,
      batch_no: null,
    };
  }
  target.status = 'CANCELLED';
  target.updated_at = dayjs().toISOString();
  return target;
};

export const syncMockBatchInventoryFreeze = (
  batch: SkuBatch,
  options: {
    shouldFreeze: boolean;
    reason: string;
    warehouse_id?: string;
  },
) => {
  const existing = findActiveFreeze(batch.sku_id, batch.batch_no);
  if (options.shouldFreeze) {
    if (existing) {
      existing.reason = options.reason;
      existing.updated_at = dayjs().toISOString();
      return existing;
    }
    const now = dayjs().toISOString();
    const record: InventoryFreezeRecord = {
      freeze_id: `FREEZE-${Date.now()}`,
      warehouse_id: options.warehouse_id ?? defaultWarehouseId,
      sku_id: batch.sku_id,
      qty: 1,
      reason: options.reason,
      status: 'ACTIVE',
      created_at: now,
      updated_at: now,
      batch_no: batch.batch_no,
    };
    mockInventoryFreezes.unshift(record);
    return record;
  }
  if (existing) {
    existing.status = 'CANCELLED';
    existing.updated_at = dayjs().toISOString();
    return existing;
  }
  return null;
};
