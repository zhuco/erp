import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  addMockWmsPackage,
  auditMockWmsOutbound,
  cancelMockWmsShipment,
  checkMockWmsOutbound,
  confirmMockWmsPickTask,
  createMockWmsOutbound,
  createMockWmsWave,
  createMockWmsWaveRule,
  deleteMockWmsWaveRule,
  getMockWmsOutbound,
  getMockWmsWave,
  inspectMockWmsOutbound,
  listMockWmsOutbounds,
  listMockWmsPickTasks,
  listMockWmsWaveRules,
  listMockWmsWaves,
  packMockWmsOutbound,
  printMockWmsOutbound,
  printMockWmsWave,
  reserveMockWmsTracking,
  runMockWmsWave,
  shipMockWmsOutbound,
  updateMockWmsOutboundLogistics,
  updateMockWmsWaveRule,
  weighMockWmsOutbound
} from '@/api/mocks';
import type { PrintJobDetail } from '@/api/printJobs';

export type WmsOutboundStatus =
  | 'DRAFT'
  | 'AUDITED'
  | 'PICKING'
  | 'PICKED'
  | 'PACKING'
  | 'PACKED'
  | 'INSPECTED'
  | 'SHIPPED'
  | 'CANCELLED';

export type WmsOutboundDoc = {
  outbound_doc_id: string;
  warehouse_id: string;
  doc_type: string;
  shipment_id?: string | null;
  order_id?: string | null;
  platform?: string | null;
  platform_order_id?: string | null;
  shop_id?: string | null;
  receiver_name?: string | null;
  receiver_phone?: string | null;
  receiver_address?: string | null;
  carrier_code?: string | null;
  tracking_no?: string | null;
  exception_flags?: string[];
  tags?: string[];
  doc_no: string;
  status: WmsOutboundStatus;
  created_at: string;
  updated_at: string;
};

export type WmsOutboundItem = {
  sku_id: string;
  planned_qty: number;
  picked_qty: number;
  packed_qty: number;
  batch_enabled: boolean;
  serial_enabled: boolean;
};

export type WmsPickTask = {
  pick_task_id: string;
  outbound_doc_id: string;
  sku_id: string;
  from_location_id?: string | null;
  container_id?: string | null;
  qty: number;
  picked_qty: number;
  status: string;
  created_at: string;
  updated_at: string;
};

export type WmsPackRecord = {
  pack_record_id: string;
  shipment_id?: string | null;
  package_no: string;
  weight_gram?: number | null;
  operator_id?: string | null;
  packed_at: string;
};

export type WmsOutboundDetail = {
  doc: WmsOutboundDoc;
  items: WmsOutboundItem[];
  pick_tasks: WmsPickTask[];
  pack_records: WmsPackRecord[];
};

export type WmsOutboundListQuery = {
  warehouse_id?: string;
  status?: string;
  doc_type?: string;
  doc_no?: string;
  doc_no_keyword?: string;
  shipment_id?: string;
  shop_id?: string;
  carrier_code?: string;
  tracking_no?: string;
  tag?: string;
  exception_flag?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type WmsOutboundListResponse = {
  items: WmsOutboundDoc[];
  total: number;
};

export type WmsOutboundCreatePayload = {
  warehouse_id: string;
  doc_type: string;
  shipment_id?: string;
  doc_no: string;
  items: Array<{ sku_id: string; planned_qty: number }>;
};

export type WmsWave = {
  wave_id: string;
  warehouse_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type WmsWaveRule = {
  rule_id: string;
  warehouse_id: string;
  carrier_code?: string | null;
  max_orders?: number | null;
  max_qty?: number | null;
  cutoff_time?: string | null;
  strategy?: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};

export type WmsWaveRuleListQuery = {
  warehouse_id?: string;
  carrier_code?: string;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type WmsWaveRuleListResponse = {
  items: WmsWaveRule[];
  total: number;
};

export type WmsWaveRuleCreatePayload = {
  warehouse_id: string;
  carrier_code?: string;
  max_orders?: number;
  max_qty?: number;
  cutoff_time?: string;
  strategy?: string;
  enabled?: boolean;
};

export type WmsWaveRuleUpdatePayload = {
  carrier_code?: string;
  max_orders?: number;
  max_qty?: number;
  cutoff_time?: string;
  strategy?: string;
  enabled?: boolean;
};

export type WmsWaveListQuery = {
  warehouse_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type WmsWaveListResponse = {
  items: WmsWave[];
  total: number;
};

export type WmsWaveCreatePayload = {
  warehouse_id: string;
  cutoff_at?: string;
  max_docs?: number;
  auto_pick?: boolean;
  outbound_doc_ids?: string[];
};

export type WmsWaveRunPayload = {
  warehouse_id: string;
  cutoff_at?: string;
  max_docs?: number;
  min_docs?: number;
  auto_pick?: boolean;
};

export type WmsWaveRunResponse = {
  status: string;
  wave?: WmsWaveDetail | null;
};

export type WmsWaveDetail = {
  wave: WmsWave;
  docs: WmsOutboundDoc[];
  tasks: WmsPickTask[];
};

export type WmsPickTaskListQuery = {
  warehouse_id?: string;
  outbound_doc_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type WmsPickTaskListResponse = {
  items: WmsPickTask[];
  total: number;
};

export type WmsPickConfirmPayload = {
  picked_qty: number;
  container_id?: string;
};

export type WmsPackPayload = {
  package_no: string;
  weight_gram?: number;
  operator_id?: string;
  items: Array<{ sku_id: string; packed_qty: number }>;
  serials?: Array<{ sku_id: string; serial_nos: string[] }>;
  batches?: Array<{ sku_id: string; batch_id?: string; batch_no?: string; qty: number }>;
};

export type WmsShipPayload = {
  shipped_at?: string;
};

export type WmsOutboundCheckPayload = {
  package_no: string;
  passed: boolean;
  reason?: string;
};

export type WmsOutboundWeighPayload = {
  package_no: string;
  weight_gram: number;
};

export type WmsReserveTrackingPayload = {
  carrier_code?: string;
  waybill_no?: string;
};

export type WmsLogisticsUpdatePayload = {
  carrier_code?: string;
  tracking_no?: string;
};

export type WmsAddPackagePayload = {
  doc_no: string;
  carrier_code?: string;
  tracking_no?: string;
  shipment_type?: string;
  items: Array<{ sku_id: string; planned_qty: number }>;
};

export type WmsShipmentCancelPayload = {
  reason?: string;
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

const buildQueryString = (query: Record<string, string | number | undefined>): string => {
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (value !== undefined && value !== null && `${value}`.length > 0) {
      params.set(key, String(value));
    }
  });
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

export const listWmsOutbounds = async (
  tenantId: string,
  query: WmsOutboundListQuery
): Promise<WmsOutboundListResponse> => {
  if (useMocks) {
    return listMockWmsOutbounds(query);
  }
  return requestJson<WmsOutboundListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取出库单列表失败'
  );
};

export const getWmsOutbound = async (
  tenantId: string,
  outboundDocId: string
): Promise<WmsOutboundDetail> => {
  if (useMocks) {
    return getMockWmsOutbound(outboundDocId);
  }
  return requestJson<WmsOutboundDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取出库单详情失败'
  );
};

export const auditWmsOutbound = async (
  tenantId: string,
  outboundDocId: string
): Promise<WmsOutboundDetail> => {
  if (useMocks) {
    return auditMockWmsOutbound(outboundDocId);
  }
  return requestJson<WmsOutboundDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/audit`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      }
    },
    '审核出库单失败'
  );
};

export const createWmsOutbound = async (
  tenantId: string,
  payload: WmsOutboundCreatePayload
): Promise<WmsOutboundDetail> => {
  if (useMocks) {
    return createMockWmsOutbound(payload);
  }
  return requestJson<WmsOutboundDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建出库单失败'
  );
};

export const listWmsWaves = async (
  tenantId: string,
  query: WmsWaveListQuery
): Promise<WmsWaveListResponse> => {
  if (useMocks) {
    return listMockWmsWaves(query);
  }
  return requestJson<WmsWaveListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/waves${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取波次列表失败'
  );
};

export const listWmsWaveRules = async (
  tenantId: string,
  query: WmsWaveRuleListQuery
): Promise<WmsWaveRuleListResponse> => {
  if (useMocks) {
    return listMockWmsWaveRules(query);
  }
  return requestJson<WmsWaveRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/wave-rules${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '鑾峰彇娉㈡瑙勫垯澶辫触'
  );
};

export const createWmsWaveRule = async (
  tenantId: string,
  payload: WmsWaveRuleCreatePayload
): Promise<WmsWaveRule> => {
  if (useMocks) {
    return createMockWmsWaveRule(payload);
  }
  return requestJson<WmsWaveRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/wave-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '娣诲姞娉㈡瑙勫垯澶辫触'
  );
};

export const updateWmsWaveRule = async (
  tenantId: string,
  ruleId: string,
  payload: WmsWaveRuleUpdatePayload
): Promise<WmsWaveRule> => {
  if (useMocks) {
    return updateMockWmsWaveRule(ruleId, payload);
  }
  return requestJson<WmsWaveRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/wave-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '鏇存柊娉㈡瑙勫垯澶辫触'
  );
};

export const deleteWmsWaveRule = async (
  tenantId: string,
  ruleId: string
): Promise<WmsWaveRule> => {
  if (useMocks) {
    return deleteMockWmsWaveRule(ruleId);
  }
  return requestJson<WmsWaveRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/wave-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '鍒犻櫎娉㈡瑙勫垯澶辫触'
  );
};

export const createWmsWave = async (
  tenantId: string,
  payload: WmsWaveCreatePayload
): Promise<WmsWaveDetail> => {
  if (useMocks) {
    return createMockWmsWave(payload);
  }
  return requestJson<WmsWaveDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/waves`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建波次失败'
  );
};

export const runWmsWave = async (
  tenantId: string,
  payload: WmsWaveRunPayload
): Promise<WmsWaveRunResponse> => {
  if (useMocks) {
    return runMockWmsWave(payload);
  }
  return requestJson<WmsWaveRunResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/waves/run`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '波次生成失败'
  );
};

export const getWmsWave = async (
  tenantId: string,
  waveId: string
): Promise<WmsWaveDetail> => {
  if (useMocks) {
    return getMockWmsWave(waveId);
  }
  return requestJson<WmsWaveDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/waves/${waveId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取波次详情失败'
  );
};

export const listWmsPickTasks = async (
  tenantId: string,
  query: WmsPickTaskListQuery
): Promise<WmsPickTaskListResponse> => {
  if (useMocks) {
    return listMockWmsPickTasks(query);
  }
  return requestJson<WmsPickTaskListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/pick-tasks${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取拣货任务失败'
  );
};

export const confirmWmsPickTask = async (
  tenantId: string,
  pickTaskId: string,
  payload: WmsPickConfirmPayload
): Promise<void> => {
  if (useMocks) {
    confirmMockWmsPickTask(pickTaskId, payload);
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/pick-tasks/${pickTaskId}/confirm`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '确认拣货失败'
  );
};

export const packWmsOutbound = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsPackPayload
): Promise<void> => {
  if (useMocks) {
    packMockWmsOutbound(outboundDocId, payload);
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/pack`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交打包失败'
  );
};

export const shipWmsOutbound = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsShipPayload
): Promise<void> => {
  if (useMocks) {
    shipMockWmsOutbound(outboundDocId, payload);
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/ship`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交发货失败'
  );
};

export const inspectWmsOutbound = async (
  tenantId: string,
  outboundDocId: string
): Promise<void> => {
  if (useMocks) {
    inspectMockWmsOutbound(outboundDocId);
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/inspect`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      }
    },
    '提交验货失败'
  );
};

export const checkWmsOutbound = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsOutboundCheckPayload
): Promise<void> => {
  if (useMocks) {
    checkMockWmsOutbound(outboundDocId, payload);
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/check`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '鎻愪氦澶嶆牳澶辫触'
  );
};

export const weighWmsOutbound = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsOutboundWeighPayload
): Promise<void> => {
  if (useMocks) {
    weighMockWmsOutbound(outboundDocId, payload);
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/weigh`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '鎻愪氦绉伴噸澶辫触'
  );
};

export const reserveWmsTracking = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsReserveTrackingPayload
): Promise<WmsOutboundDetail> => {
  if (useMocks) {
    return reserveMockWmsTracking(outboundDocId, payload);
  }
  return requestJson<WmsOutboundDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/reserve-tracking`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '预约面单失败'
  );
};

export const updateWmsOutboundLogistics = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsLogisticsUpdatePayload
): Promise<WmsOutboundDetail> => {
  if (useMocks) {
    return updateMockWmsOutboundLogistics(outboundDocId, payload);
  }
  return requestJson<WmsOutboundDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/logistics`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新物流失败'
  );
};

export const createWmsAddPackage = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsAddPackagePayload
): Promise<WmsOutboundDetail> => {
  if (useMocks) {
    return addMockWmsPackage(outboundDocId, payload);
  }
  return requestJson<WmsOutboundDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/add-package`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '追加包裹失败'
  );
};

export const requestWmsShipmentCancel = async (
  tenantId: string,
  outboundDocId: string,
  payload: WmsShipmentCancelPayload
): Promise<void> => {
  if (useMocks) {
    cancelMockWmsShipment(outboundDocId, payload);
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/cancel-request`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交撤销请求失败'
  );
};

export const printWmsOutbound = async (
  tenantId: string,
  outboundDocId: string
): Promise<PrintJobDetail> => {
  if (useMocks) {
    return printMockWmsOutbound(outboundDocId);
  }
  return requestJson<PrintJobDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/outbounds/${outboundDocId}/print`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '打印出库单失败'
  );
};

export const printWmsWave = async (
  tenantId: string,
  waveId: string
): Promise<PrintJobDetail> => {
  if (useMocks) {
    return printMockWmsWave(waveId);
  }
  return requestJson<PrintJobDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/waves/${waveId}/print`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '打印波次失败'
  );
};
