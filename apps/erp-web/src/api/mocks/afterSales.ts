import dayjs from 'dayjs';

import type { AfterSaleCaseDetail, AfterSaleListResponse } from '../afterSales';
import type {
  AfterSalesDetail,
  AfterSalesImportJob,
  AfterSalesListResponse as AfterSalesOpsListResponse,
  AfterSalesSummary,
  AfterSalesType,
  AfterSalesStatus,
  RefundChannel,
  RefundDoc,
  RefundDocListResponse,
  RefundStatus,
} from '../afterSalesOps';
import type {
  AfterSalesInspection,
  AfterSalesImportJobListQuery,
  AfterSalesIntercept,
  AfterSalesInspectionCreatePayload,
  AfterSalesInspectionListQuery,
  AfterSalesInspectionUpdatePayload,
  AfterSalesInterceptCreatePayload,
  AfterSalesInterceptListQuery,
  AfterSalesListQuery,
  AfterSalesSlaRule,
  AfterSalesSlaRuleCreatePayload,
  AfterSalesSlaRuleListQuery,
  AfterSalesSlaRuleUpdatePayload,
  AfterSalesOpsCreatePayload,
  AfterSalesOpsListQuery,
  AfterSalesOpsRecord,
  RefundDocListQuery
} from './data';
import {
  applyPagination,
  mockAfterSaleDetails,
  mockAfterSales,
  mockAfterSalesInspections,
  mockAfterSalesIntercepts,
  mockAfterSalesSlaRules,
  mockAfterSalesImportJobs,
  mockAfterSalesOps,
  mockTenantId,
  mockUserId
} from './data';

const normalizeAfterSalesType = (value?: string): AfterSalesType => {
  switch (value) {
    case 'REFUND_ONLY':
    case 'RETURN_REFUND':
    case 'EXCHANGE':
    case 'RESEND':
      return value;
    default:
      return 'RETURN_REFUND';
  }
};

const normalizeAfterSalesStatus = (value?: string): AfterSalesStatus => {
  switch (value) {
    case 'OPEN':
    case 'APPROVED':
    case 'RECEIVED':
    case 'REFUNDED':
    case 'CLOSED':
    case 'REJECTED':
      return value;
    default:
      return 'OPEN';
  }
};

const normalizeRefundStatus = (value?: string): RefundStatus => {
  switch (value) {
    case 'PENDING':
    case 'SENT':
    case 'FAILED':
    case 'DONE':
      return value;
    default:
      return 'PENDING';
  }
};

const normalizeRefundChannel = (value?: string): RefundChannel => {
  switch (value) {
    case 'PLATFORM':
    case 'MANUAL':
      return value;
    default:
      return 'MANUAL';
  }
};

const normalizeInspectionResult = (
  value?: string
): AfterSalesInspection['result_code'] => {
  switch (value) {
    case 'PENDING':
    case 'RESALE':
    case 'REPAIR':
    case 'SCRAP':
      return value;
    default:
      return 'PENDING';
  }
};

const normalizeSlaStatus = (value?: string): AfterSalesSlaRule['status'] =>
  value === 'DISABLED' ? 'DISABLED' : 'ACTIVE';

const toAfterSalesSummary = (record: AfterSalesOpsRecord): AfterSalesSummary => ({
  id: record.id,
  tenant_id: record.tenant_id,
  order_id: record.order_id,
  type: normalizeAfterSalesType(record.type),
  status: normalizeAfterSalesStatus(record.status),
  reason_code: record.reason_code ?? null,
  amount_cent: record.amount_cent,
  created_at: record.created_at,
  updated_at: record.updated_at,
});

const toRefundDoc = (doc: {
  id: string;
  after_sales_id: string;
  amount_cent: number;
  status?: string;
  channel?: string;
  created_at: string;
  updated_at: string;
}): RefundDoc => ({
  id: doc.id,
  after_sales_id: doc.after_sales_id,
  amount_cent: doc.amount_cent,
  status: normalizeRefundStatus(doc.status),
  channel: normalizeRefundChannel(doc.channel),
  created_at: doc.created_at,
  updated_at: doc.updated_at,
});

const resolveAfterSalesOpsRecord = (afterSalesId: string): AfterSalesOpsRecord =>
  mockAfterSalesOps.find((item) => item.id === afterSalesId) ?? mockAfterSalesOps[0] ?? {
    id: afterSalesId,
    tenant_id: mockTenantId,
    order_id: `ORDER-${afterSalesId}`,
    type: 'RETURN_REFUND',
    status: 'OPEN',
    reason_code: null,
    amount_cent: 0,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString(),
  };

export const getMockAfterSalesList = (query: AfterSalesListQuery): AfterSaleListResponse => {
  const filtered = mockAfterSales.filter((item) => {
    if (query.platform && item.platform !== query.platform) {
      return false;
    }
    if (query.shop_id && item.shop_id !== query.shop_id) {
      return false;
    }
    if (query.case_status && item.case_status !== query.case_status) {
      return false;
    }
    if (query.platform_order_id && item.platform_order_id !== query.platform_order_id) {
      return false;
    }
    if (query.updated_from && item.updated_at) {
      if (dayjs(item.updated_at).isBefore(query.updated_from)) {
        return false;
      }
    }
    if (query.updated_to && item.updated_at) {
      if (dayjs(item.updated_at).isAfter(query.updated_to)) {
        return false;
      }
    }
    return true;
  });
  const limit = query.page_size ?? 20;
  const offset = query.page_token ? Number(query.page_token) || 0 : 0;
  return {
    data: applyPagination(filtered, offset, limit).map((item) => ({
      case_id: item.case_id,
      tenant_id: mockTenantId,
      shop_id: item.shop_id,
      platform: item.platform,
      case_status: item.case_status,
      platform_order_id: item.platform_order_id ?? null,
      created_at: item.created_at,
      updated_at: item.updated_at,
    })),
    next_page_token: offset + limit < filtered.length ? String(offset + limit) : null,
  };
};

export const getMockAfterSaleDetail = (caseId: string): AfterSaleCaseDetail => {
  return mockAfterSaleDetails.get(caseId) ?? {
    case: {
      case_id: caseId,
      tenant_id: mockTenantId,
      platform: 'taobao',
      shop_id: 'SHOP-taobao-1',
      after_sale_id: `AFTER-${caseId}`,
      after_sale_type: 'refund',
      refund_state: 'UNKNOWN',
      after_sale_state: 'UNKNOWN',
      refund_amount_cent: 0,
      item_count: 0,
      total_qty: 0,
      order_count: 0,
      created_at: dayjs().toISOString(),
      updated_at: dayjs().toISOString(),
    },
    items: [],
  };
};

export const getMockAfterSaleRaw = (caseId: string) => {
  const detail = getMockAfterSaleDetail(caseId);
  return {
    source: 'mock',
    fetched_at: dayjs().toISOString(),
    case_id: caseId,
    payload: detail
  };
};

export const getMockAfterSalesOpsList = (
  query: AfterSalesOpsListQuery
): AfterSalesOpsListResponse => {
  const filtered = mockAfterSalesOps.filter((item) => {
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.after_sales_type && item.type !== query.after_sales_type) {
      return false;
    }
    if (query.order_id && !item.order_id.includes(query.order_id)) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? filtered.length;
  const items = filtered.slice(offset, offset + limit);
  return {
    items: items.map(toAfterSalesSummary),
    total: filtered.length,
    limit,
    offset
  };
};

export const getMockAfterSalesOpsDetail = (afterSalesId: string): AfterSalesDetail => {
  const record = resolveAfterSalesOpsRecord(afterSalesId);
  const now = new Date().toISOString();
  return {
    after_sales: toAfterSalesSummary(record),
    items: [
      {
        sku_id: 'SKU-001',
        qty: 1,
        refund_cent: record.amount_cent
      }
    ],
    return_receipt:
      record.status === 'RECEIVED'
        ? {
            id: `RR-${record.id}`,
            after_sales_id: record.id,
            warehouse_id: 'WH-001',
            status: 'PUTAWAY_DONE',
            received_at: now,
            putaway_at: now,
            created_at: now,
            updated_at: now
          }
        : null,
    refund_doc:
      record.status === 'REFUNDED'
        ? {
            id: `RF-${record.id}`,
            after_sales_id: record.id,
            amount_cent: record.amount_cent,
            status: normalizeRefundStatus('DONE'),
            channel: normalizeRefundChannel('MANUAL'),
            created_at: now,
            updated_at: now
          }
        : null,
    raw_json: {
      source: 'mock',
      after_sales_id: record.id,
      order_id: record.order_id
    },
    audit_logs: [
      {
        audit_id: `audit-${record.id}`,
        action: 'AFTERSALES.CREATE',
        user_id: mockUserId,
        before_json: null,
        after_json: {
          status: record.status
        },
        created_at: now
      }
    ]
  };
};

export const putawayMockAfterSales = (afterSalesId: string) => {
  const target = mockAfterSalesOps.find((item) => item.id === afterSalesId);
  const now = new Date().toISOString();
  if (target) {
    target.status = 'RECEIVED';
    target.updated_at = now;
  }
  const detail = getMockAfterSalesOpsDetail(afterSalesId);
  if (detail.return_receipt) {
    detail.return_receipt.status = 'PUTAWAY_DONE';
    detail.return_receipt.putaway_at = now;
    detail.return_receipt.updated_at = now;
  }
  return detail;
};

export const getMockAfterSalesOpsCreate = (payload: AfterSalesOpsCreatePayload) => {
  const now = new Date().toISOString();
  return {
    after_sales: {
      id: `AS-${Date.now()}`,
      tenant_id: mockTenantId,
      order_id: payload.order_id,
      type: normalizeAfterSalesType(payload.type),
      status: normalizeAfterSalesStatus('OPEN'),
      reason_code: payload.reason_code ?? null,
      amount_cent:
        payload.amount_cent ?? payload.items.reduce((sum, item) => sum + item.refund_cent, 0),
      created_at: now,
      updated_at: now
    },
    items: payload.items,
    return_receipt: null,
    refund_doc: null,
    raw_json: payload,
    audit_logs: [
      {
        audit_id: `audit-${Date.now()}`,
        action: 'AFTERSALES.CREATE',
        user_id: mockUserId,
        before_json: null,
        after_json: payload,
        created_at: now
      }
    ]
  };
};

export const getMockRefundDoc = (
  afterSalesId: string,
  payload: { amount_cent?: number; channel?: string }
): RefundDoc => {
  const now = new Date().toISOString();
  return toRefundDoc({
    id: `RF-${afterSalesId}-${Date.now()}`,
    after_sales_id: afterSalesId,
    amount_cent: payload.amount_cent ?? 1200,
    status: 'PENDING',
    channel: payload.channel ?? 'MANUAL',
    created_at: now,
    updated_at: now
  });
};

export const getMockRefundDocs = (query: RefundDocListQuery): RefundDocListResponse => {
  const now = new Date().toISOString();
  const docs = [
    toRefundDoc({
      id: 'RF-001',
      after_sales_id: 'AS-1',
      amount_cent: 1200,
      status: 'FAILED',
      channel: 'MANUAL',
      created_at: now,
      updated_at: now
    })
  ];
  const filtered = docs.filter((item) => {
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.after_sales_id && item.after_sales_id !== query.after_sales_id) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? filtered.length;
  return {
    items: filtered.slice(offset, offset + limit),
    total: filtered.length,
    limit,
    offset
  };
};

export const getMockAfterSalesImportJob = (fileName?: string): AfterSalesImportJob => {
  const now = new Date().toISOString();
  const job = {
    job_id: `AS-IMPORT-${Date.now()}`,
    file_name: fileName ?? 'after_sales_import.csv',
    file_type: 'CSV',
    status: 'PENDING',
    progress: 0,
    total_rows: 0,
    success_rows: 0,
    failed_rows: 0,
    attempts: 0,
    last_error: '',
    created_at: now,
    updated_at: now
  };
  mockAfterSalesImportJobs.unshift(job);
  return job;
};

export const getMockAfterSalesImportJobs = (query: AfterSalesImportJobListQuery) => {
  const filtered = mockAfterSalesImportJobs.filter((job) => {
    if (query.status && job.status !== query.status) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? filtered.length;
  return {
    items: filtered.slice(offset, offset + limit),
    total: filtered.length,
    limit,
    offset
  };
};

export const listMockAfterSalesIntercepts = (query: AfterSalesInterceptListQuery) => {
  const filtered = mockAfterSalesIntercepts.filter((item) => {
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.order_id && !item.order_id.includes(query.order_id)) {
      return false;
    }
    if (query.tracking_no && !(item.tracking_no ?? '').includes(query.tracking_no)) {
      return false;
    }
    if (query.requested_from && item.requested_at) {
      if (dayjs(item.requested_at).isBefore(query.requested_from)) {
        return false;
      }
    }
    if (query.requested_to && item.requested_at) {
      if (dayjs(item.requested_at).isAfter(query.requested_to)) {
        return false;
      }
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? filtered.length;
  return {
    items: applyPagination(filtered, offset, limit),
    total: filtered.length,
    limit,
    offset
  };
};

export const createMockAfterSalesIntercept = (
  payload: AfterSalesInterceptCreatePayload
): AfterSalesIntercept => {
  const now = dayjs().toISOString();
  const status = payload.status ?? 'PROCESSING';
  const record: AfterSalesIntercept = {
    intercept_id: `INT-${Date.now()}`,
    order_id: payload.order_id,
    after_sales_id: payload.after_sales_id ?? null,
    carrier_name: payload.carrier_name ?? null,
    tracking_no: payload.tracking_no ?? null,
    status,
    reason: payload.reason ?? null,
    requested_at: payload.requested_at ?? now,
    completed_at: status === 'SUCCESS' || status === 'FAILED' ? now : null,
    created_at: now,
    updated_at: now
  };
  mockAfterSalesIntercepts.unshift(record);
  return record;
};

export const retryMockAfterSalesIntercept = (interceptId: string): AfterSalesIntercept => {
  const target = mockAfterSalesIntercepts.find((item) => item.intercept_id === interceptId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: AfterSalesIntercept = {
      intercept_id: interceptId,
      order_id: 'ORDER-UNKNOWN',
      after_sales_id: null,
      carrier_name: null,
      tracking_no: null,
      status: 'PROCESSING',
      reason: null,
      requested_at: now,
      completed_at: null,
      created_at: now,
      updated_at: now
    };
    mockAfterSalesIntercepts.unshift(fallback);
    return fallback;
  }
  target.status = 'PROCESSING';
  target.completed_at = null;
  target.updated_at = now;
  return target;
};

export const escalateMockAfterSalesIntercept = (interceptId: string): AfterSalesIntercept => {
  const target = mockAfterSalesIntercepts.find((item) => item.intercept_id === interceptId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: AfterSalesIntercept = {
      intercept_id: interceptId,
      order_id: 'ORDER-UNKNOWN',
      after_sales_id: null,
      carrier_name: null,
      tracking_no: null,
      status: 'ESCALATED',
      reason: null,
      requested_at: now,
      completed_at: null,
      created_at: now,
      updated_at: now
    };
    mockAfterSalesIntercepts.unshift(fallback);
    return fallback;
  }
  target.status = 'ESCALATED';
  target.completed_at = null;
  target.updated_at = now;
  return target;
};

export const listMockAfterSalesInspections = (query: AfterSalesInspectionListQuery) => {
  const filtered = mockAfterSalesInspections.filter((item) => {
    if (query.after_sales_id && item.after_sales_id !== query.after_sales_id) {
      return false;
    }
    if (query.sku_id && item.sku_id !== query.sku_id) {
      return false;
    }
    if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.result_code && item.result_code !== query.result_code) {
      return false;
    }
    if (query.inspected_from && item.inspected_at) {
      if (dayjs(item.inspected_at).isBefore(query.inspected_from)) {
        return false;
      }
    }
    if (query.inspected_to && item.inspected_at) {
      if (dayjs(item.inspected_at).isAfter(query.inspected_to)) {
        return false;
      }
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? filtered.length;
  return {
    items: applyPagination(filtered, offset, limit),
    total: filtered.length,
    limit,
    offset
  };
};

export const createMockAfterSalesInspection = (
  payload: AfterSalesInspectionCreatePayload
): AfterSalesInspection => {
  const now = dayjs().toISOString();
  const record: AfterSalesInspection = {
    inspection_id: `AS-QC-${Date.now()}`,
    after_sales_id: payload.after_sales_id,
    sku_id: payload.sku_id,
    warehouse_id: payload.warehouse_id ?? null,
    qty: payload.qty,
    result_code: normalizeInspectionResult(payload.result_code),
    inspector_name: payload.inspector_name ?? null,
    inspected_at: payload.inspected_at ?? null,
    created_at: now,
    updated_at: now
  };
  mockAfterSalesInspections.unshift(record);
  return record;
};

export const updateMockAfterSalesInspection = (
  inspectionId: string,
  payload: AfterSalesInspectionUpdatePayload
): AfterSalesInspection => {
  const target = mockAfterSalesInspections.find((item) => item.inspection_id === inspectionId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: AfterSalesInspection = {
      inspection_id: inspectionId,
      after_sales_id: 'AS-UNKNOWN',
      sku_id: 'SKU-UNKNOWN',
      warehouse_id: payload.warehouse_id ?? null,
      qty: payload.qty ?? 1,
      result_code: normalizeInspectionResult(payload.result_code),
      inspector_name: payload.inspector_name ?? null,
      inspected_at: payload.inspected_at ?? null,
      created_at: now,
      updated_at: now
    };
    mockAfterSalesInspections.unshift(fallback);
    return fallback;
  }
  if (payload.warehouse_id !== undefined) {
    target.warehouse_id = payload.warehouse_id;
  }
  if (payload.qty != null) {
    target.qty = payload.qty;
  }
  if (payload.result_code) {
    target.result_code = normalizeInspectionResult(payload.result_code);
  }
  if (payload.inspector_name !== undefined) {
    target.inspector_name = payload.inspector_name ?? null;
  }
  if (payload.inspected_at !== undefined) {
    target.inspected_at = payload.inspected_at ?? null;
  }
  target.updated_at = now;
  return target;
};

export const listMockAfterSalesSlaRules = (query: AfterSalesSlaRuleListQuery) => {
  const filtered = mockAfterSalesSlaRules.filter((rule) => {
    if (query.after_sales_type && rule.after_sales_type !== query.after_sales_type) {
      return false;
    }
    if (query.status && rule.status !== query.status) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? filtered.length;
  return {
    items: applyPagination(filtered, offset, limit),
    total: filtered.length,
    limit,
    offset
  };
};

export const createMockAfterSalesSlaRule = (
  payload: AfterSalesSlaRuleCreatePayload
): AfterSalesSlaRule => {
  const now = dayjs().toISOString();
  const record: AfterSalesSlaRule = {
    rule_id: `SLA-${Date.now()}`,
    after_sales_type: normalizeAfterSalesType(payload.after_sales_type),
    response_hours: payload.response_hours,
    resolution_hours: payload.resolution_hours,
    escalation_role: payload.escalation_role ?? null,
    status: normalizeSlaStatus(payload.status),
    created_at: now,
    updated_at: now
  };
  mockAfterSalesSlaRules.unshift(record);
  return record;
};

export const updateMockAfterSalesSlaRule = (
  ruleId: string,
  payload: AfterSalesSlaRuleUpdatePayload
): AfterSalesSlaRule => {
  const target = mockAfterSalesSlaRules.find((rule) => rule.rule_id === ruleId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: AfterSalesSlaRule = {
      rule_id: ruleId,
      after_sales_type: normalizeAfterSalesType(payload.after_sales_type),
      response_hours: payload.response_hours ?? 4,
      resolution_hours: payload.resolution_hours ?? 24,
      escalation_role: payload.escalation_role ?? null,
      status: normalizeSlaStatus(payload.status),
      created_at: now,
      updated_at: now
    };
    mockAfterSalesSlaRules.unshift(fallback);
    return fallback;
  }
  if (payload.after_sales_type) {
    target.after_sales_type = normalizeAfterSalesType(payload.after_sales_type);
  }
  if (payload.response_hours != null) {
    target.response_hours = payload.response_hours;
  }
  if (payload.resolution_hours != null) {
    target.resolution_hours = payload.resolution_hours;
  }
  if (payload.escalation_role !== undefined) {
    target.escalation_role = payload.escalation_role ?? null;
  }
  if (payload.status) {
    target.status = normalizeSlaStatus(payload.status);
  }
  target.updated_at = now;
  return target;
};

export const deleteMockAfterSalesSlaRule = (ruleId: string) => {
  const index = mockAfterSalesSlaRules.findIndex((rule) => rule.rule_id === ruleId);
  if (index >= 0) {
    mockAfterSalesSlaRules.splice(index, 1);
  }
};
