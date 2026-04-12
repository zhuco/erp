import dayjs from 'dayjs';

import type { InventoryActionKind, InventoryActionPayload } from '../inventory';
import type {
  InventoryLedger,
  InventoryLedgerReason,
  InventoryLedgerListQuery,
  InventoryLedgerListResponse
} from '../inventoryLedgers';
import type {
  StockSyncJob,
  StockSyncJobListQuery,
  StockSyncJobListResponse
} from '../inventorySyncJobs';

import type { InventoryListQuery, InventorySnapshot } from './data';
import {
  applyPagination,
  mockInventory,
  mockInventoryActionKeys,
  mockInventoryLedgers,
  mockStockSyncJobs
} from './data';

export const getMockInventoryList = (query: InventoryListQuery) => {
  const normalizedSkuCode = query.sku_code?.trim().toLowerCase();
  const normalizedBarcode = query.barcode?.trim().toLowerCase();
  const normalizedWarehouseId = query.warehouse_id?.trim().toLowerCase();
  const filtered = mockInventory.filter((item) => {
    if (query.platform && item.platform !== query.platform) {
      return false;
    }
    if (normalizedWarehouseId && item.warehouse_id?.toLowerCase() !== normalizedWarehouseId) {
      return false;
    }
    if (query.sku_id && item.sku_id !== query.sku_id) {
      return false;
    }
    if (normalizedSkuCode && item.sku_code?.toLowerCase() !== normalizedSkuCode) {
      return false;
    }
    if (normalizedBarcode && item.barcode?.toLowerCase() !== normalizedBarcode) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const applyMockInventoryAction = (
  kind: InventoryActionKind,
  payload: InventoryActionPayload
): InventorySnapshot | null => {
  const target = mockInventory.find(
    (item) =>
      item.sku_id === payload.sku_id &&
      (item.warehouse_id ? item.warehouse_id === payload.warehouse_id : true)
  );
  if (!target) {
    return null;
  }
  const dedupKey =
    payload.ref_type && payload.ref_id
      ? `${kind}-${payload.ref_type}-${payload.ref_id}-${payload.warehouse_id}-${target.sku_id}`
      : null;
  if (dedupKey && mockInventoryActionKeys.has(dedupKey)) {
    return target;
  }
  const requestedQty = Math.abs(payload.qty);
  let appliedQty = 0;
  if (kind === 'ADJUST') {
    const currentOnHand = target.on_hand ?? 0;
    const nextOnHand = Math.max(0, currentOnHand + payload.qty);
    const onHandDelta = nextOnHand - currentOnHand;
    target.on_hand = nextOnHand;
    target.available = Math.max(0, target.available + onHandDelta);
    appliedQty = onHandDelta;
  } else if (kind === 'RESERVE') {
    const reservableQty = Math.max(0, target.available);
    appliedQty = Math.min(requestedQty, reservableQty);
    target.reserved = Math.max(0, target.reserved + appliedQty);
    target.available = Math.max(0, target.available - appliedQty);
  } else {
    appliedQty = Math.min(requestedQty, Math.max(0, target.reserved));
    target.reserved = Math.max(0, target.reserved - appliedQty);
    target.available = Math.max(0, target.available + appliedQty);
  }
  target.updated_at = dayjs().toISOString();
  if (dedupKey) {
    mockInventoryActionKeys.add(dedupKey);
  }
  if (appliedQty === 0) {
    return target;
  }
  const reasonType: InventoryLedgerReason =
    kind === 'ADJUST'
      ? 'STOCK_COUNT_ADJUST'
      : kind === 'RESERVE'
        ? 'RESERVATION_CREATE'
        : 'RESERVATION_RELEASE';
  const ledger: InventoryLedger = {
    ledger_id: `LEDGER-${Date.now()}`,
    sku_id: target.sku_id,
    sku_code: target.sku_code ?? target.sku_id,
    warehouse_id: payload.warehouse_id,
    delta_on_hand: kind === 'ADJUST' ? appliedQty : 0,
    delta_reserved: kind === 'RESERVE' ? appliedQty : kind === 'RELEASE' ? -appliedQty : 0,
    delta_in_transit: 0,
    delta_frozen: 0,
    reason_type: reasonType,
    ref_type: payload.ref_type ?? null,
    ref_id: payload.ref_id ?? null,
    occurred_at: dayjs().toISOString()
  };
  mockInventoryLedgers.unshift(ledger);
  return target;
};

export const getMockInventoryLedgers = (
  query: InventoryLedgerListQuery
): InventoryLedgerListResponse => {
  const normalizedSkuCode = query.sku_code?.trim().toLowerCase();
  const filtered = mockInventoryLedgers.filter((item) => {
    if (query.sku_id && item.sku_id !== query.sku_id) {
      return false;
    }
    if (normalizedSkuCode && item.sku_code.toLowerCase() !== normalizedSkuCode) {
      return false;
    }
    if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.occurred_from && dayjs(item.occurred_at).isBefore(query.occurred_from)) {
      return false;
    }
    if (query.occurred_to && dayjs(item.occurred_at).isAfter(query.occurred_to)) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockStockSyncJobs = (
  query: StockSyncJobListQuery
): StockSyncJobListResponse => {
  const filtered = mockStockSyncJobs.filter((job) => {
    if (query.shop_id && job.shop_id !== query.shop_id) {
      return false;
    }
    if (query.sku_id && job.sku_id !== query.sku_id) {
      return false;
    }
    if (query.warehouse_id && job.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && job.status !== query.status) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const retryMockStockSyncJob = (jobId: string): StockSyncJob => {
  const job = mockStockSyncJobs.find((item) => item.job_id === jobId);
  const now = dayjs().toISOString();
  if (!job) {
    return {
      job_id: jobId,
      shop_id: 'SHOP-taobao-1',
      sku_id: 'SKU-UNKNOWN',
      warehouse_id: 'WH-DEFAULT',
      target_qty: 0,
      status: 'PENDING',
      attempts: 0,
      last_error: null,
      created_at: now,
      updated_at: now
    };
  }
  job.status = 'PENDING';
  job.last_error = null;
  job.updated_at = now;
  return job;
};

export const runMockStockSyncJob = (jobId: string): StockSyncJob => {
  const job = mockStockSyncJobs.find((item) => item.job_id === jobId);
  const now = dayjs().toISOString();
  if (!job) {
    return {
      job_id: jobId,
      shop_id: 'SHOP-taobao-1',
      sku_id: 'SKU-UNKNOWN',
      warehouse_id: 'WH-DEFAULT',
      target_qty: 0,
      status: 'PENDING',
      attempts: 0,
      last_error: null,
      created_at: now,
      updated_at: now
    };
  }
  job.status = 'PENDING';
  job.last_error = null;
  job.updated_at = now;
  return job;
};
