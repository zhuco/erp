import dayjs from 'dayjs';

import type { PrintJobDetail } from '../printJobs';
import type {
  WarehouseWorkflowConfig,
  WarehouseWorkflowConfigPayload,
} from '../workflowConfigs';
import type {
  WmsAddPackagePayload,
  WmsLogisticsUpdatePayload,
  WmsOutboundCheckPayload,
  WmsOutboundCreatePayload,
  WmsOutboundDetail,
  WmsOutboundDoc,
  WmsOutboundItem,
  WmsOutboundListQuery,
  WmsOutboundListResponse,
  WmsOutboundStatus,
  WmsOutboundWeighPayload,
  WmsPackPayload,
  WmsPickConfirmPayload,
  WmsPickTask,
  WmsPickTaskListQuery,
  WmsPickTaskListResponse,
  WmsReserveTrackingPayload,
  WmsShipmentCancelPayload,
  WmsShipPayload,
  WmsWave,
  WmsWaveCreatePayload,
  WmsWaveDetail,
  WmsWaveListQuery,
  WmsWaveListResponse,
  WmsWaveRule,
  WmsWaveRuleCreatePayload,
  WmsWaveRuleListQuery,
  WmsWaveRuleListResponse,
  WmsWaveRuleUpdatePayload,
  WmsWaveRunPayload,
  WmsWaveRunResponse
} from '../wms';
import { applyPagination, mockShops, mockSkus, mockWarehouses } from './data';
import { createMockPrintJob } from './print';

const defaultWarehouseId = mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT';
const defaultShopId = mockShops[0]?.shop_id ?? 'SHOP-DEFAULT';
let mockWaveSequence = 1;
const workflowConfigStorageKey = 'erp.mock.wms.workflow-configs';

const createDefaultWorkflowConfig = (warehouseId: string): WarehouseWorkflowConfig => ({
  warehouse_id: warehouseId,
  enable_wave: false,
  require_inspection: false,
  require_weighing: false,
  require_pack: true,
});

const normalizeWorkflowConfig = (
  warehouseId: string,
  payload: WarehouseWorkflowConfigPayload,
): WarehouseWorkflowConfig => {
  const requirePack = payload.require_pack;
  return {
    warehouse_id: warehouseId,
    enable_wave: payload.enable_wave,
    require_pack: requirePack,
    require_inspection: requirePack ? payload.require_inspection : false,
    require_weighing: requirePack ? payload.require_weighing : false,
  };
};

const createMockWaveId = () => `00000000-0000-4000-8000-${String(mockWaveSequence++).padStart(12, '0')}`;

const canUseMockStorage = () => {
  if (typeof globalThis === 'undefined') {
    return false;
  }
  return typeof globalThis.localStorage !== 'undefined';
};

const readStoredWorkflowConfigs = (): Record<string, WarehouseWorkflowConfigPayload> | null => {
  if (!canUseMockStorage()) {
    return null;
  }
  try {
    const raw = globalThis.localStorage.getItem(workflowConfigStorageKey);
    if (!raw) {
      return null;
    }
    return JSON.parse(raw) as Record<string, WarehouseWorkflowConfigPayload>;
  } catch {
    return null;
  }
};

const persistWorkflowConfigs = (configs: Map<string, WarehouseWorkflowConfig>) => {
  if (!canUseMockStorage()) {
    return;
  }
  const serialized = Object.fromEntries(
    Array.from(configs.entries()).map(([warehouseId, config]) => [
      warehouseId,
      {
        enable_wave: config.enable_wave,
        require_pack: config.require_pack,
        require_inspection: config.require_inspection,
        require_weighing: config.require_weighing,
      } satisfies WarehouseWorkflowConfigPayload,
    ]),
  );
  globalThis.localStorage.setItem(workflowConfigStorageKey, JSON.stringify(serialized));
};

const buildInitialWorkflowConfigs = () => {
  const seeded = new Map<string, WarehouseWorkflowConfig>(
    mockWarehouses.map((warehouse) => [
      warehouse.warehouse_id,
      createDefaultWorkflowConfig(warehouse.warehouse_id),
    ]),
  );
  const stored = readStoredWorkflowConfigs();
  if (!stored) {
    return seeded;
  }
  Object.entries(stored).forEach(([warehouseId, payload]) => {
    seeded.set(warehouseId, normalizeWorkflowConfig(warehouseId, payload));
  });
  return seeded;
};

const mockWarehouseWorkflowConfigs = buildInitialWorkflowConfigs();
const outboundStatuses: WmsOutboundStatus[] = [
  'DRAFT',
  'AUDITED',
  'PICKING',
  'PACKED',
  'SHIPPED'
];

const buildOutboundDoc = (index: number): WmsOutboundDoc => {
  const timestamp = dayjs().subtract(index, 'hour').toISOString();
  return {
    outbound_doc_id: `OB-${1000 + index}`,
    warehouse_id: defaultWarehouseId,
    doc_type: 'SALES',
    shipment_id: `SHIP-${1000 + index}`,
    order_id: `ORDER-${1000 + index}`,
    platform: 'taobao',
    platform_order_id: `TB-${1000 + index}`,
    shop_id: defaultShopId,
    receiver_name: `收货人${index + 1}`,
    receiver_phone: `1380000${String(1000 + index).slice(-4)}`,
    receiver_address: `示例地址${index + 1}号`,
    carrier_code: index % 2 === 0 ? 'SF' : 'ZTO',
    tracking_no: index % 2 === 0 ? `SF${100000 + index}` : `ZT${200000 + index}`,
    exception_flags: index % 4 === 0 ? ['PACK_DELAY'] : [],
    tags: index % 3 === 0 ? ['加急'] : [],
    doc_no: `OUT-${1000 + index}`,
    status: outboundStatuses[index % outboundStatuses.length] ?? 'DRAFT',
    created_at: timestamp,
    updated_at: timestamp
  };
};

const buildOutboundItems = (docIndex: number, status: WmsOutboundStatus): WmsOutboundItem[] => {
  const baseSku = mockSkus[docIndex % mockSkus.length];
  const secondarySku = mockSkus[(docIndex + 1) % mockSkus.length];
  const plannedQty = (docIndex % 3) + 1;
  const pickedQty = status === 'PACKED' || status === 'SHIPPED' ? plannedQty : 0;
  const packedQty = status === 'PACKED' || status === 'SHIPPED' ? plannedQty : 0;
  return [
    {
      sku_id: baseSku?.sku_id ?? `SKU-${docIndex + 1}`,
      planned_qty: plannedQty,
      picked_qty: pickedQty,
      packed_qty: packedQty,
      batch_enabled: false,
      serial_enabled: false
    },
    {
      sku_id: secondarySku?.sku_id ?? `SKU-${docIndex + 2}`,
      planned_qty: 1,
      picked_qty: status === 'SHIPPED' ? 1 : 0,
      packed_qty: status === 'SHIPPED' ? 1 : 0,
      batch_enabled: false,
      serial_enabled: false
    }
  ];
};

const buildPickTasks = (docId: string, items: WmsOutboundItem[]): WmsPickTask[] => {
  const timestamp = dayjs().toISOString();
  return items.map((item, index) => ({
    pick_task_id: `PICK-${docId}-${index + 1}`,
    outbound_doc_id: docId,
    sku_id: item.sku_id,
    from_location_id: `LOC-${100 + index}`,
    container_id: null,
    qty: item.planned_qty,
    picked_qty: item.picked_qty,
    status: item.picked_qty > 0 ? 'DONE' : 'WAITING',
    created_at: timestamp,
    updated_at: timestamp
  }));
};

const mockWmsOutbounds: WmsOutboundDoc[] = Array.from({ length: 16 }, (_, index) =>
  buildOutboundDoc(index)
);
const mockWmsOutboundDetails = new Map<string, WmsOutboundDetail>();
const mockWmsPickTasks: WmsPickTask[] = [];

const ensureOutboundDetail = (doc: WmsOutboundDoc, index: number) => {
  const existing = mockWmsOutboundDetails.get(doc.outbound_doc_id);
  if (existing) {
    return existing;
  }
  const items = buildOutboundItems(index, doc.status);
  const pickTasks = buildPickTasks(doc.outbound_doc_id, items);
  const detail: WmsOutboundDetail = {
    doc,
    items,
    pick_tasks: pickTasks,
    pack_records: []
  };
  mockWmsOutboundDetails.set(doc.outbound_doc_id, detail);
  mockWmsPickTasks.push(...pickTasks);
  return detail;
};

mockWmsOutbounds.forEach((doc, index) => ensureOutboundDetail(doc, index));

const mockWmsWaveRules: WmsWaveRule[] = [
  {
    rule_id: 'WAVE-RULE-001',
    warehouse_id: defaultWarehouseId,
    carrier_code: 'SF',
    max_orders: 50,
    max_qty: 200,
    cutoff_time: '16:00',
    strategy: '按快递分组',
    enabled: true,
    created_at: dayjs().subtract(5, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockWmsWaves: WmsWave[] = [
  {
    wave_id: createMockWaveId(),
    warehouse_id: defaultWarehouseId,
    status: 'CREATED',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'hour').toISOString()
  }
];
const mockWmsWaveDetails = new Map<string, WmsWaveDetail>();

const buildWaveDetail = (wave: WmsWave, docs?: WmsOutboundDoc[]): WmsWaveDetail => {
  const matchedDocs =
    docs ??
    mockWmsOutbounds.filter((doc) => doc.status !== 'SHIPPED').slice(0, 6);
  const tasks = mockWmsPickTasks.filter((task) =>
    matchedDocs.some((doc) => doc.outbound_doc_id === task.outbound_doc_id)
  );
  return {
    wave,
    docs: matchedDocs,
    tasks
  };
};

export const getMockWarehouseWorkflowConfig = (warehouseId: string): WarehouseWorkflowConfig => {
  const existing = mockWarehouseWorkflowConfigs.get(warehouseId);
  if (existing) {
    return { ...existing };
  }
  const created = createDefaultWorkflowConfig(warehouseId);
  mockWarehouseWorkflowConfigs.set(warehouseId, created);
  return { ...created };
};

export const updateMockWarehouseWorkflowConfig = (
  warehouseId: string,
  payload: WarehouseWorkflowConfigPayload,
): WarehouseWorkflowConfig => {
  const updated = normalizeWorkflowConfig(warehouseId, payload);
  mockWarehouseWorkflowConfigs.set(warehouseId, updated);
  persistWorkflowConfigs(mockWarehouseWorkflowConfigs);
  return { ...updated };
};

export const listMockWmsOutbounds = (query: WmsOutboundListQuery): WmsOutboundListResponse => {
  const filtered = mockWmsOutbounds.filter((doc) => {
    if (query.warehouse_id && doc.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && doc.status !== query.status) {
      return false;
    }
    if (query.doc_type && doc.doc_type !== query.doc_type) {
      return false;
    }
    if (query.doc_no && doc.doc_no !== query.doc_no) {
      return false;
    }
    if (query.doc_no_keyword && !doc.doc_no.includes(query.doc_no_keyword)) {
      return false;
    }
    if (query.shipment_id && doc.shipment_id !== query.shipment_id) {
      return false;
    }
    if (query.shop_id && doc.shop_id !== query.shop_id) {
      return false;
    }
    if (query.carrier_code && doc.carrier_code !== query.carrier_code) {
      return false;
    }
    if (query.tracking_no && doc.tracking_no !== query.tracking_no) {
      return false;
    }
    if (query.tag && !(doc.tags ?? []).includes(query.tag)) {
      return false;
    }
    if (query.exception_flag && !(doc.exception_flags ?? []).includes(query.exception_flag)) {
      return false;
    }
    if (query.created_from && doc.created_at) {
      if (dayjs(doc.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && doc.created_at) {
      if (dayjs(doc.created_at).isAfter(query.created_to)) {
        return false;
      }
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

export const getMockWmsOutbound = (outboundDocId: string): WmsOutboundDetail => {
  const docIndex = mockWmsOutbounds.findIndex((item) => item.outbound_doc_id === outboundDocId);
  const doc = mockWmsOutbounds[docIndex] ?? buildOutboundDoc(mockWmsOutbounds.length + 1);
  if (docIndex < 0) {
    mockWmsOutbounds.unshift(doc);
  }
  return ensureOutboundDetail(doc, docIndex >= 0 ? docIndex : mockWmsOutbounds.length);
};

export const auditMockWmsOutbound = (outboundDocId: string): WmsOutboundDetail => {
  const detail = getMockWmsOutbound(outboundDocId);
  detail.doc.status = 'AUDITED';
  detail.doc.updated_at = dayjs().toISOString();
  return detail;
};

export const createMockWmsOutbound = (payload: WmsOutboundCreatePayload): WmsOutboundDetail => {
  const timestamp = dayjs().toISOString();
  const doc: WmsOutboundDoc = {
    outbound_doc_id: `OB-${Date.now()}`,
    warehouse_id: payload.warehouse_id,
    doc_type: payload.doc_type,
    shipment_id: payload.shipment_id ?? null,
    order_id: null,
    platform: null,
    platform_order_id: null,
    shop_id: defaultShopId,
    receiver_name: null,
    receiver_phone: null,
    receiver_address: null,
    carrier_code: null,
    tracking_no: null,
    exception_flags: [],
    tags: [],
    doc_no: payload.doc_no,
    status: 'DRAFT',
    created_at: timestamp,
    updated_at: timestamp
  };
  mockWmsOutbounds.unshift(doc);
  const items: WmsOutboundItem[] = payload.items.map((item) => ({
    sku_id: item.sku_id,
    planned_qty: item.planned_qty,
    picked_qty: 0,
    packed_qty: 0,
    batch_enabled: false,
    serial_enabled: false
  }));
  const pickTasks = buildPickTasks(doc.outbound_doc_id, items);
  mockWmsPickTasks.push(...pickTasks);
  const detail: WmsOutboundDetail = {
    doc,
    items,
    pick_tasks: pickTasks,
    pack_records: []
  };
  mockWmsOutboundDetails.set(doc.outbound_doc_id, detail);
  return detail;
};

export const listMockWmsWaves = (query: WmsWaveListQuery): WmsWaveListResponse => {
  const filtered = mockWmsWaves.filter((wave) => {
    if (query.warehouse_id && wave.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && wave.status !== query.status) {
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

export const listMockWmsWaveRules = (
  query: WmsWaveRuleListQuery
): WmsWaveRuleListResponse => {
  const filtered = mockWmsWaveRules.filter((rule) => {
    if (query.warehouse_id && rule.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.carrier_code && rule.carrier_code !== query.carrier_code) {
      return false;
    }
    if (query.enabled != null && rule.enabled !== query.enabled) {
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

export const createMockWmsWaveRule = (payload: WmsWaveRuleCreatePayload): WmsWaveRule => {
  const timestamp = dayjs().toISOString();
  const rule: WmsWaveRule = {
    rule_id: `WAVE-RULE-${Date.now()}`,
    warehouse_id: payload.warehouse_id,
    carrier_code: payload.carrier_code ?? null,
    max_orders: payload.max_orders ?? null,
    max_qty: payload.max_qty ?? null,
    cutoff_time: payload.cutoff_time ?? null,
    strategy: payload.strategy ?? null,
    enabled: payload.enabled ?? true,
    created_at: timestamp,
    updated_at: timestamp
  };
  mockWmsWaveRules.unshift(rule);
  return rule;
};

export const updateMockWmsWaveRule = (
  ruleId: string,
  payload: WmsWaveRuleUpdatePayload
): WmsWaveRule => {
  const index = mockWmsWaveRules.findIndex((rule) => rule.rule_id === ruleId);
  const base =
    index >= 0
      ? mockWmsWaveRules[index]
      : {
          rule_id: ruleId,
          warehouse_id: defaultWarehouseId,
          carrier_code: null,
          max_orders: null,
          max_qty: null,
          cutoff_time: null,
          strategy: null,
          enabled: true,
          created_at: dayjs().toISOString(),
          updated_at: dayjs().toISOString()
        };
  const updated: WmsWaveRule = {
    ...base,
    carrier_code: payload.carrier_code ?? base.carrier_code,
    max_orders: payload.max_orders ?? base.max_orders,
    max_qty: payload.max_qty ?? base.max_qty,
    cutoff_time: payload.cutoff_time ?? base.cutoff_time,
    strategy: payload.strategy ?? base.strategy,
    enabled: payload.enabled ?? base.enabled,
    updated_at: dayjs().toISOString()
  };
  if (index >= 0) {
    mockWmsWaveRules[index] = updated;
  } else {
    mockWmsWaveRules.unshift(updated);
  }
  return updated;
};

export const deleteMockWmsWaveRule = (ruleId: string): WmsWaveRule => {
  const index = mockWmsWaveRules.findIndex((rule) => rule.rule_id === ruleId);
  if (index >= 0) {
    const [removed] = mockWmsWaveRules.splice(index, 1);
    return removed;
  }
  return {
    rule_id: ruleId,
    warehouse_id: defaultWarehouseId,
    carrier_code: null,
    max_orders: null,
    max_qty: null,
    cutoff_time: null,
    strategy: null,
    enabled: false,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const createMockWmsWave = (payload: WmsWaveCreatePayload): WmsWaveDetail => {
  const timestamp = dayjs().toISOString();
  const wave: WmsWave = {
    wave_id: createMockWaveId(),
    warehouse_id: payload.warehouse_id,
    status: 'CREATED',
    created_at: timestamp,
    updated_at: timestamp
  };
  mockWmsWaves.unshift(wave);
  const docs = payload.outbound_doc_ids
    ? mockWmsOutbounds.filter((doc) => payload.outbound_doc_ids?.includes(doc.outbound_doc_id))
    : undefined;
  const detail = buildWaveDetail(wave, docs);
  mockWmsWaveDetails.set(wave.wave_id, detail);
  return detail;
};

export const runMockWmsWave = (payload: WmsWaveRunPayload): WmsWaveRunResponse => {
  const workflowConfig = getMockWarehouseWorkflowConfig(payload.warehouse_id);
  if (!workflowConfig.enable_wave) {
    return {
      status: 'disabled',
      wave: null,
    };
  }
  const detail = createMockWmsWave({
    warehouse_id: payload.warehouse_id,
    cutoff_at: payload.cutoff_at,
    max_docs: payload.max_docs,
    auto_pick: payload.auto_pick,
    outbound_doc_ids: payload.auto_pick
      ? mockWmsOutbounds.slice(0, payload.max_docs ?? 6).map((doc) => doc.outbound_doc_id)
      : undefined
  });
  return {
    status: 'created',
    wave: detail
  };
};

export const getMockWmsWave = (waveId: string): WmsWaveDetail => {
  const existing = mockWmsWaveDetails.get(waveId);
  if (existing) {
    return existing;
  }
  const wave =
    mockWmsWaves.find((item) => item.wave_id === waveId) ??
    ({
      wave_id: waveId,
      warehouse_id: defaultWarehouseId,
      status: 'CREATED',
      created_at: dayjs().toISOString(),
      updated_at: dayjs().toISOString()
    } as WmsWave);
  const detail = buildWaveDetail(wave);
  mockWmsWaveDetails.set(wave.wave_id, detail);
  if (!mockWmsWaves.find((item) => item.wave_id === waveId)) {
    mockWmsWaves.unshift(wave);
  }
  return detail;
};

export const listMockWmsPickTasks = (
  query: WmsPickTaskListQuery
): WmsPickTaskListResponse => {
  const filtered = mockWmsPickTasks.filter((task) => {
    if (query.outbound_doc_id && task.outbound_doc_id !== query.outbound_doc_id) {
      return false;
    }
    if (query.status && task.status !== query.status) {
      return false;
    }
    if (query.warehouse_id) {
      const doc = mockWmsOutbounds.find(
        (item) => item.outbound_doc_id === task.outbound_doc_id
      );
      if (doc?.warehouse_id !== query.warehouse_id) {
        return false;
      }
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

export const confirmMockWmsPickTask = (
  pickTaskId: string,
  payload: WmsPickConfirmPayload
) => {
  const task = mockWmsPickTasks.find((item) => item.pick_task_id === pickTaskId);
  if (task) {
    task.picked_qty = payload.picked_qty;
    task.container_id = payload.container_id ?? task.container_id;
    task.status = payload.picked_qty >= task.qty ? 'DONE' : 'PICKING';
    task.updated_at = dayjs().toISOString();
  }
  const detail = task ? getMockWmsOutbound(task.outbound_doc_id) : null;
  if (detail && task) {
    detail.items = detail.items.map((item) =>
      item.sku_id === task.sku_id ? { ...item, picked_qty: task.picked_qty } : item
    );
  }
};

const updateOutboundStatus = (outboundDocId: string, status: WmsOutboundStatus) => {
  const detail = getMockWmsOutbound(outboundDocId);
  detail.doc.status = status;
  detail.doc.updated_at = dayjs().toISOString();
  return detail;
};

export const packMockWmsOutbound = (outboundDocId: string, payload: WmsPackPayload) => {
  const detail = updateOutboundStatus(outboundDocId, 'PACKED');
  const timestamp = dayjs().toISOString();
  detail.pack_records.push({
    pack_record_id: `PACK-${Date.now()}`,
    shipment_id: detail.doc.shipment_id ?? null,
    package_no: payload.package_no,
    weight_gram: payload.weight_gram ?? null,
    operator_id: payload.operator_id ?? null,
    packed_at: timestamp
  });
  detail.items = detail.items.map((item) => {
    const matched = payload.items.find((target) => target.sku_id === item.sku_id);
    if (!matched) {
      return item;
    }
    return { ...item, packed_qty: matched.packed_qty };
  });
};

export const shipMockWmsOutbound = (outboundDocId: string, payload: WmsShipPayload) => {
  void payload;
  updateOutboundStatus(outboundDocId, 'SHIPPED');
};

export const inspectMockWmsOutbound = (outboundDocId: string) => {
  updateOutboundStatus(outboundDocId, 'INSPECTED');
};

export const checkMockWmsOutbound = (outboundDocId: string, payload: WmsOutboundCheckPayload) => {
  if (!payload.passed) {
    updateOutboundStatus(outboundDocId, 'PICKING');
    return;
  }
  updateOutboundStatus(outboundDocId, 'INSPECTED');
};

export const weighMockWmsOutbound = (outboundDocId: string, payload: WmsOutboundWeighPayload) => {
  const detail = getMockWmsOutbound(outboundDocId);
  detail.pack_records.push({
    pack_record_id: `WEIGH-${Date.now()}`,
    shipment_id: detail.doc.shipment_id ?? null,
    package_no: payload.package_no,
    weight_gram: payload.weight_gram,
    operator_id: null,
    packed_at: dayjs().toISOString()
  });
};

export const reserveMockWmsTracking = (
  outboundDocId: string,
  payload: WmsReserveTrackingPayload
): WmsOutboundDetail => {
  const detail = getMockWmsOutbound(outboundDocId);
  detail.doc.carrier_code = payload.carrier_code ?? detail.doc.carrier_code ?? null;
  detail.doc.tracking_no = payload.waybill_no ?? detail.doc.tracking_no ?? null;
  detail.doc.updated_at = dayjs().toISOString();
  return detail;
};

export const updateMockWmsOutboundLogistics = (
  outboundDocId: string,
  payload: WmsLogisticsUpdatePayload
): WmsOutboundDetail => {
  const detail = getMockWmsOutbound(outboundDocId);
  detail.doc.carrier_code = payload.carrier_code ?? detail.doc.carrier_code ?? null;
  detail.doc.tracking_no = payload.tracking_no ?? detail.doc.tracking_no ?? null;
  detail.doc.updated_at = dayjs().toISOString();
  return detail;
};

export const addMockWmsPackage = (
  outboundDocId: string,
  payload: WmsAddPackagePayload
): WmsOutboundDetail => {
  const detail = getMockWmsOutbound(outboundDocId);
  detail.doc.doc_no = payload.doc_no;
  detail.doc.updated_at = dayjs().toISOString();
  return detail;
};

export const cancelMockWmsShipment = (
  outboundDocId: string,
  payload: WmsShipmentCancelPayload
) => {
  void payload;
  updateOutboundStatus(outboundDocId, 'CANCELLED');
};

export const printMockWmsOutbound = (outboundDocId: string): PrintJobDetail =>
  createMockPrintJob({
    job_type: 'WMS_OUTBOUND',
    ref_type: 'OUTBOUND',
    ref_id: outboundDocId
  });

export const printMockWmsWave = (waveId: string): PrintJobDetail =>
  createMockPrintJob({
    job_type: 'WMS_WAVE',
    ref_type: 'WAVE',
    ref_id: waveId
  });
