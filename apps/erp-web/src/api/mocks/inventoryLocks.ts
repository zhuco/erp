import dayjs from 'dayjs';

import type {
  InventoryLockDetail,
  InventoryLockLog,
  InventoryLockLogListQuery,
  InventoryLockLogListResponse,
  InventoryLockPriorityPayload,
  InventoryLockRecalculatePayload,
  InventoryLockRecalculateResult,
  InventoryLockReleasePayload,
  NegativeStockPolicy,
  NegativeStockPolicyListQuery,
  NegativeStockPolicyListResponse,
  NegativeStockPolicyUpdatePayload
} from '../inventoryLocks';

import { applyPagination, mockWarehouseSeeds } from './data';

const skuSeeds = ['SKU-1001', 'SKU-1002', 'SKU-1003', 'SKU-1004', 'SKU-1005'];
const skuCodes = ['SKU-1001', 'SKU-1002', 'SKU-1003', 'SKU-1004', 'SKU-1005'];
const warehouseIds =
  mockWarehouseSeeds.map((seed) => seed.warehouse_id) ?? ['WH-DEFAULT', 'WH-EAST'];
const sourceTypes: InventoryLockLog['source_type'][] = [
  'ORDER',
  'OUTBOUND',
  'PRE_SALE',
  'TRANSFER',
  'RETURN',
  'MANUAL'
];
const lockTypes: InventoryLockLog['lock_type'][] = [
  'STANDARD',
  'PRIORITY',
  'PRE_SALE',
  'TRANSFER',
  'SAFETY'
];
const operatorNames = ['系统', '陈敏', '李雷', '韩梅梅'];

const buildLockLog = (index: number): InventoryLockLog => {
  const skuIndex = index % skuSeeds.length;
  const warehouseId = warehouseIds[index % warehouseIds.length] ?? 'WH-DEFAULT';
  const lockedQty = (index % 4) + 1;
  const unlockedQty = index % 3 === 0 ? 1 : 0;
  const availableAfter = Math.max(0, 40 - index * 2 + unlockedQty - lockedQty);
  const sourceType = sourceTypes[index % sourceTypes.length];
  const lockType = lockTypes[index % lockTypes.length];
  const operatorName = operatorNames[index % operatorNames.length] ?? '系统';
  const operatorType = operatorName === '系统' ? 'SYSTEM' : 'USER';
  return {
    lock_id: `LOCK-${1000 + index}`,
    sku_id: skuSeeds[skuIndex] ?? 'SKU-1001',
    sku_code: skuCodes[skuIndex] ?? 'SKU-1001',
    warehouse_id: warehouseId,
    locked_qty: lockedQty,
    unlocked_qty: unlockedQty,
    available_after: availableAfter,
    source_type: sourceType,
    source_doc_no: `${sourceType}-${3000 + index}`,
    lock_type: lockType,
    operator_name: operatorName,
    operator_type: operatorType,
    occurred_at: dayjs().subtract(index, 'hour').toISOString()
  };
};

const buildLockDetail = (log: InventoryLockLog, index: number): InventoryLockDetail => {
  const createdAt = dayjs(log.occurred_at);
  const documents = [
    {
      doc_type: log.source_type,
      doc_no: log.source_doc_no,
      qty: log.locked_qty,
      priority: 80 - index,
      status: log.unlocked_qty > 0 ? 'PARTIAL_RELEASE' : 'LOCKED'
    },
    {
      doc_type: 'PICK',
      doc_no: `PICK-${9000 + index}`,
      qty: Math.max(1, log.locked_qty - 1),
      priority: 50,
      status: 'ALLOCATED'
    }
  ];
  return {
    lock_id: log.lock_id,
    sku_id: log.sku_id,
    sku_code: log.sku_code ?? log.sku_id,
    warehouse_id: log.warehouse_id,
    locked_qty: log.locked_qty,
    unlocked_qty: log.unlocked_qty,
    available_after: log.available_after,
    source_type: log.source_type,
    source_doc_no: log.source_doc_no,
    lock_type: log.lock_type,
    priority: 60 - index,
    status: log.unlocked_qty > 0 ? 'RELEASED' : 'ACTIVE',
    operator_name: log.operator_name ?? '系统',
    created_at: createdAt.subtract(2, 'hour').toISOString(),
    updated_at: createdAt.toISOString(),
    documents,
    audit_trail: [
      {
        action: 'RECALCULATE',
        operator: log.operator_name ?? '系统',
        note: '自动校验锁定',
        created_at: createdAt.toISOString()
      }
    ]
  };
};

const mockInventoryLockLogs: InventoryLockLog[] = Array.from({ length: 28 }, (_, index) =>
  buildLockLog(index)
);
const mockInventoryLockDetails = new Map<string, InventoryLockDetail>(
  mockInventoryLockLogs.map((log, index) => [log.lock_id, buildLockDetail(log, index)])
);

const mockNegativeStockPolicies: NegativeStockPolicy[] = [
  {
    policy_id: 'NEG-001',
    scope: 'WAREHOUSE',
    warehouse_id: warehouseIds[0] ?? 'WH-DEFAULT',
    sku_id: null,
    sku_code: null,
    allow_negative: false,
    note: '默认禁止负库存',
    updated_at: dayjs().subtract(3, 'day').toISOString(),
    updated_by: '系统'
  },
  {
    policy_id: 'NEG-002',
    scope: 'SKU',
    warehouse_id: warehouseIds[0] ?? 'WH-DEFAULT',
    sku_id: skuSeeds[1] ?? 'SKU-1002',
    sku_code: skuCodes[1] ?? 'SKU-1002',
    allow_negative: true,
    note: '紧急补货 SKU 允许负库存',
    updated_at: dayjs().subtract(1, 'day').toISOString(),
    updated_by: '王芳'
  }
];

export const getMockInventoryLockLogs = (
  query: InventoryLockLogListQuery
): InventoryLockLogListResponse => {
  const normalizedDocNo = query.doc_no?.trim().toLowerCase();
  const normalizedSkuCode = query.sku_code?.trim().toLowerCase();
  const filtered = mockInventoryLockLogs.filter((item) => {
    if (query.sku_id && item.sku_id !== query.sku_id) {
      return false;
    }
    if (normalizedSkuCode && item.sku_code?.toLowerCase() !== normalizedSkuCode) {
      return false;
    }
    if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (normalizedDocNo && !item.source_doc_no.toLowerCase().includes(normalizedDocNo)) {
      return false;
    }
    if (query.lock_type && item.lock_type !== query.lock_type) {
      return false;
    }
    if (query.source_type && item.source_type !== query.source_type) {
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

export const getMockInventoryLockDetail = (lockId: string): InventoryLockDetail => {
  const stored = mockInventoryLockDetails.get(lockId);
  if (stored) {
    return stored;
  }
  const fallback = mockInventoryLockLogs[0] ?? buildLockLog(0);
  const detail = buildLockDetail(fallback, 0);
  detail.lock_id = lockId;
  mockInventoryLockDetails.set(lockId, detail);
  return detail;
};

export const recalcMockInventoryLocks = (
  payload: InventoryLockRecalculatePayload
): InventoryLockRecalculateResult => {
  const scope: InventoryLockRecalculateResult['scope'] =
    payload.scope ??
    (payload.sku_id || payload.warehouse_id ? 'TARGET' : 'GLOBAL');
  const targets = mockInventoryLockLogs.filter((item) => {
    if (scope === 'GLOBAL') {
      return true;
    }
    if (payload.sku_id && item.sku_id !== payload.sku_id) {
      return false;
    }
    if (payload.warehouse_id && item.warehouse_id !== payload.warehouse_id) {
      return false;
    }
    return true;
  });
  const delta = scope === 'GLOBAL' ? 1 : 2;
  targets.slice(0, 6).forEach((item) => {
    item.available_after = Math.max(0, item.available_after + delta);
  });
  const now = dayjs().toISOString();
  const reference = targets[0] ?? mockInventoryLockLogs[0];
  if (reference) {
    const newLog: InventoryLockLog = {
      lock_id: `LOCK-RECALC-${Date.now()}`,
      sku_id: payload.sku_id ?? reference.sku_id,
      sku_code: reference.sku_code ?? reference.sku_id,
      warehouse_id: payload.warehouse_id ?? reference.warehouse_id,
      locked_qty: 0,
      unlocked_qty: 0,
      available_after: reference.available_after,
      source_type: 'MANUAL',
      source_doc_no: `RECALC-${Date.now()}`,
      lock_type: 'STANDARD',
      operator_name: '系统',
      operator_type: 'SYSTEM',
      occurred_at: now
    };
    mockInventoryLockLogs.unshift(newLog);
  }
  if (reference) {
    const detail = getMockInventoryLockDetail(reference.lock_id);
    detail.updated_at = now;
    detail.audit_trail.unshift({
      action: 'RECALCULATE',
      operator: '系统',
      note: payload.reason ?? '手工重算',
      created_at: now
    });
  }
  return {
    scope,
    recalculated: targets.length,
    affected_skus: Array.from(new Set(targets.map((item) => item.sku_id))),
    updated_at: now
  };
};

export const prioritizeMockInventoryLock = (
  payload: InventoryLockPriorityPayload
): InventoryLockDetail => {
  const detail = getMockInventoryLockDetail(payload.lock_id);
  detail.lock_type = 'PRIORITY';
  detail.priority = 95;
  detail.status = 'ACTIVE';
  detail.updated_at = dayjs().toISOString();
  detail.audit_trail.unshift({
    action: 'PRIORITIZE',
    operator: '调度员',
    note: payload.reason ?? '手工抢占',
    created_at: detail.updated_at
  });
  const log = mockInventoryLockLogs.find((item) => item.lock_id === payload.lock_id);
  if (log) {
    log.lock_type = 'PRIORITY';
    log.operator_name = '调度员';
    log.operator_type = 'USER';
    log.occurred_at = detail.updated_at;
  }
  return detail;
};

export const releaseMockInventoryLock = (
  payload: InventoryLockReleasePayload
): InventoryLockDetail => {
  const detail = getMockInventoryLockDetail(payload.lock_id);
  detail.status = 'RELEASED';
  detail.unlocked_qty = detail.locked_qty;
  detail.updated_at = dayjs().toISOString();
  detail.audit_trail.unshift({
    action: 'RELEASE',
    operator: '系统',
    note: payload.reason ?? '手工释放',
    created_at: detail.updated_at
  });
  const log = mockInventoryLockLogs.find((item) => item.lock_id === payload.lock_id);
  if (log) {
    log.unlocked_qty = log.locked_qty;
    log.operator_name = '系统';
    log.operator_type = 'SYSTEM';
    log.occurred_at = detail.updated_at;
  }
  return detail;
};

export const listMockNegativeStockPolicies = (
  query: NegativeStockPolicyListQuery
): NegativeStockPolicyListResponse => {
  const filtered = mockNegativeStockPolicies.filter((item) => {
    if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.sku_id && item.sku_id !== query.sku_id) {
      return false;
    }
    return true;
  });
  return {
    items: filtered,
    total: filtered.length
  };
};

export const updateMockNegativeStockPolicy = (
  payload: NegativeStockPolicyUpdatePayload
): NegativeStockPolicy => {
  const existingIndex = mockNegativeStockPolicies.findIndex(
    (item) =>
      item.scope === payload.scope &&
      item.warehouse_id === payload.warehouse_id &&
      (payload.scope === 'SKU' ? item.sku_id === payload.sku_id : true)
  );
  const now = dayjs().toISOString();
  const record: NegativeStockPolicy = {
    policy_id:
      existingIndex >= 0
        ? mockNegativeStockPolicies[existingIndex]?.policy_id ?? `NEG-${Date.now()}`
        : `NEG-${Date.now()}`,
    scope: payload.scope,
    warehouse_id: payload.warehouse_id,
    sku_id: payload.scope === 'SKU' ? payload.sku_id ?? null : null,
    sku_code: payload.scope === 'SKU' ? payload.sku_code ?? payload.sku_id ?? null : null,
    allow_negative: payload.allow_negative,
    note: payload.note ?? null,
    updated_at: now,
    updated_by: '系统'
  };
  if (existingIndex >= 0) {
    mockNegativeStockPolicies[existingIndex] = record;
  } else {
    mockNegativeStockPolicies.unshift(record);
  }
  return record;
};
