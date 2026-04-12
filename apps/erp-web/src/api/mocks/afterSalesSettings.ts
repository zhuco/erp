import dayjs from 'dayjs';

import type {
  AfterSalesQualityRule,
  AfterSalesQualityRuleListQuery,
  AfterSalesQualityRuleListResponse,
  AfterSalesQualityRulePayload,
  AfterSalesReason,
  AfterSalesReasonListQuery,
  AfterSalesReasonListResponse,
  AfterSalesReasonPayload
} from '../afterSalesSettings';
import { applyPagination } from './data';

const now = () => dayjs().toISOString();

const mockAfterSalesReasons: AfterSalesReason[] = [
  {
    reason_code: 'DEFECT',
    label: '商品破损',
    category: '质量问题',
    status: 'ACTIVE',
    sort_order: 1,
    created_at: dayjs().subtract(12, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    reason_code: 'MISSING',
    label: '少件',
    category: '物流问题',
    status: 'ACTIVE',
    sort_order: 2,
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    reason_code: 'CUSTOMER',
    label: '客户原因退货',
    category: '客户原因',
    status: 'ACTIVE',
    sort_order: 3,
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  },
  {
    reason_code: 'OTHER',
    label: '其他',
    category: '其他',
    status: 'DISABLED',
    sort_order: 99,
    created_at: dayjs().subtract(6, 'day').toISOString(),
    updated_at: dayjs().subtract(4, 'day').toISOString()
  }
];

const mockAfterSalesQualityRules: AfterSalesQualityRule[] = [
  {
    rule_id: 'QC-001',
    after_sales_type: 'RETURN_REFUND',
    result_code: 'RESALE',
    status: 'ACTIVE',
    sort_order: 1,
    created_at: dayjs().subtract(9, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    rule_id: 'QC-002',
    after_sales_type: 'RETURN_REFUND',
    result_code: 'SCRAP',
    status: 'ACTIVE',
    sort_order: 2,
    created_at: dayjs().subtract(7, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    rule_id: 'QC-003',
    after_sales_type: 'EXCHANGE',
    result_code: 'RESEND',
    status: 'DISABLED',
    sort_order: 3,
    created_at: dayjs().subtract(5, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

export const getMockAfterSalesReasonList = (
  query: AfterSalesReasonListQuery
): AfterSalesReasonListResponse => {
  const filtered = mockAfterSalesReasons.filter((item) => {
    if (query.category && item.category !== query.category) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? 50;
  return {
    items: applyPagination(filtered, offset, limit),
    total: filtered.length,
    limit,
    offset
  };
};

export const createMockAfterSalesReason = (
  payload: AfterSalesReasonPayload
): AfterSalesReason => {
  const timestamp = now();
  const record: AfterSalesReason = {
    reason_code: payload.reason_code ?? `REASON-${Date.now()}`,
    label: payload.label,
    category: payload.category ?? null,
    status: payload.status ?? 'ACTIVE',
    sort_order: payload.sort_order ?? mockAfterSalesReasons.length + 1,
    created_at: timestamp,
    updated_at: timestamp
  };
  mockAfterSalesReasons.unshift(record);
  return record;
};

export const updateMockAfterSalesReason = (
  reasonCode: string,
  payload: AfterSalesReasonPayload
): AfterSalesReason => {
  const index = mockAfterSalesReasons.findIndex((item) => item.reason_code === reasonCode);
  const base =
    index >= 0
      ? mockAfterSalesReasons[index]
      : {
          reason_code: reasonCode,
          label: payload.label ?? reasonCode,
          category: payload.category ?? null,
          status: payload.status ?? 'ACTIVE',
          sort_order: payload.sort_order ?? mockAfterSalesReasons.length + 1,
          created_at: now(),
          updated_at: now()
        };
  const updated: AfterSalesReason = {
    ...base,
    label: payload.label ?? base.label,
    category: payload.category ?? base.category ?? null,
    status: payload.status ?? base.status,
    sort_order: payload.sort_order ?? base.sort_order,
    updated_at: now()
  };
  if (index >= 0) {
    mockAfterSalesReasons[index] = updated;
  } else {
    mockAfterSalesReasons.unshift(updated);
  }
  return updated;
};

export const deleteMockAfterSalesReason = (reasonCode: string): AfterSalesReason => {
  const index = mockAfterSalesReasons.findIndex((item) => item.reason_code === reasonCode);
  if (index >= 0) {
    const [removed] = mockAfterSalesReasons.splice(index, 1);
    return removed;
  }
  return {
    reason_code: reasonCode,
    label: reasonCode,
    category: null,
    status: 'DISABLED',
    sort_order: 0,
    created_at: now(),
    updated_at: now()
  };
};

export const getMockAfterSalesQualityRuleList = (
  query: AfterSalesQualityRuleListQuery
): AfterSalesQualityRuleListResponse => {
  const filtered = mockAfterSalesQualityRules.filter((item) => {
    if (query.after_sales_type && item.after_sales_type !== query.after_sales_type) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? 50;
  return {
    items: applyPagination(filtered, offset, limit),
    total: filtered.length,
    limit,
    offset
  };
};

export const createMockAfterSalesQualityRule = (
  payload: AfterSalesQualityRulePayload
): AfterSalesQualityRule => {
  const timestamp = now();
  const record: AfterSalesQualityRule = {
    rule_id: `QC-${Date.now()}`,
    after_sales_type: payload.after_sales_type,
    result_code: payload.result_code,
    status: payload.status ?? 'ACTIVE',
    sort_order: payload.sort_order ?? mockAfterSalesQualityRules.length + 1,
    created_at: timestamp,
    updated_at: timestamp
  };
  mockAfterSalesQualityRules.unshift(record);
  return record;
};

export const updateMockAfterSalesQualityRule = (
  ruleId: string,
  payload: AfterSalesQualityRulePayload
): AfterSalesQualityRule => {
  const index = mockAfterSalesQualityRules.findIndex((item) => item.rule_id === ruleId);
  const base =
    index >= 0
      ? mockAfterSalesQualityRules[index]
      : {
          rule_id: ruleId,
          after_sales_type: payload.after_sales_type,
          result_code: payload.result_code,
          status: payload.status ?? 'ACTIVE',
          sort_order: payload.sort_order ?? mockAfterSalesQualityRules.length + 1,
          created_at: now(),
          updated_at: now()
        };
  const updated: AfterSalesQualityRule = {
    ...base,
    after_sales_type: payload.after_sales_type ?? base.after_sales_type,
    result_code: payload.result_code ?? base.result_code,
    status: payload.status ?? base.status,
    sort_order: payload.sort_order ?? base.sort_order,
    updated_at: now()
  };
  if (index >= 0) {
    mockAfterSalesQualityRules[index] = updated;
  } else {
    mockAfterSalesQualityRules.unshift(updated);
  }
  return updated;
};

export const deleteMockAfterSalesQualityRule = (ruleId: string): AfterSalesQualityRule => {
  const index = mockAfterSalesQualityRules.findIndex((item) => item.rule_id === ruleId);
  if (index >= 0) {
    const [removed] = mockAfterSalesQualityRules.splice(index, 1);
    return removed;
  }
  return {
    rule_id: ruleId,
    after_sales_type: 'RETURN_REFUND',
    result_code: 'UNKNOWN',
    status: 'DISABLED',
    sort_order: 0,
    created_at: now(),
    updated_at: now()
  };
};
