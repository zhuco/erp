import dayjs from 'dayjs';

import type {
  LogisticsSettlementBatch,
  LogisticsSettlementBatchConfirmPayload,
  LogisticsSettlementBatchListQuery,
  LogisticsSettlementBatchListResponse,
  LogisticsSettlementRule,
  LogisticsSettlementRuleUpdatePayload
} from '../logisticsSettlements';
import { applyPagination } from './data';

let mockRule: LogisticsSettlementRule = {
  tenant_id: 'TENANT-DEFAULT',
  weight_rounding: '按 0.5KG 进位',
  min_fee_cent: 800,
  oversize_fee_cent: 1200,
  overtime_fee_cent: 500
};

const mockBatches: LogisticsSettlementBatch[] = [
  {
    batch_id: 'SET-202401',
    tenant_id: 'TENANT-DEFAULT',
    carrier: '顺丰速运',
    period_start: dayjs().subtract(30, 'day').format('YYYY-MM-DD'),
    period_end: dayjs().subtract(1, 'day').format('YYYY-MM-DD'),
    total_shipments: 320,
    expected_cent: 182000,
    actual_cent: 176500,
    diff_cent: -5500,
    status: 'DRAFT',
    deductions: ['超重调整', '偏远地区附加费'],
    created_at: dayjs().subtract(1, 'day').toISOString(),
    payment_plan_id: null
  },
  {
    batch_id: 'SET-202402',
    tenant_id: 'TENANT-DEFAULT',
    carrier: '京东物流',
    period_start: dayjs().subtract(60, 'day').format('YYYY-MM-DD'),
    period_end: dayjs().subtract(31, 'day').format('YYYY-MM-DD'),
    total_shipments: 280,
    expected_cent: 168000,
    actual_cent: 168000,
    diff_cent: 0,
    status: 'CONFIRMED',
    deductions: [],
    created_at: dayjs().subtract(10, 'day').toISOString(),
    payment_plan_id: 'PAY-9021'
  }
];

export const getMockLogisticsSettlementRule = (): LogisticsSettlementRule => ({ ...mockRule });

export const updateMockLogisticsSettlementRule = (
  payload: LogisticsSettlementRuleUpdatePayload
): LogisticsSettlementRule => {
  mockRule = {
    ...mockRule,
    ...payload
  };
  return { ...mockRule };
};

export const listMockLogisticsSettlementBatches = (
  query: LogisticsSettlementBatchListQuery
): LogisticsSettlementBatchListResponse => {
  const filtered = query.status
    ? mockBatches.filter((item) => item.status === query.status)
    : mockBatches;
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const confirmMockLogisticsSettlementBatch = (
  batchId: string,
  payload: LogisticsSettlementBatchConfirmPayload
): LogisticsSettlementBatch => {
  const target = mockBatches.find((item) => item.batch_id === batchId);
  const planId = payload.payment_plan_id ?? `PAY-${Date.now()}`;
  if (!target) {
    const fallback: LogisticsSettlementBatch = {
      batch_id: batchId,
      tenant_id: 'TENANT-DEFAULT',
      carrier: '未知承运商',
      period_start: dayjs().subtract(30, 'day').format('YYYY-MM-DD'),
      period_end: dayjs().format('YYYY-MM-DD'),
      total_shipments: 0,
      expected_cent: 0,
      actual_cent: 0,
      diff_cent: 0,
      status: 'CONFIRMED',
      deductions: [],
      created_at: dayjs().toISOString(),
      payment_plan_id: planId
    };
    mockBatches.unshift(fallback);
    return fallback;
  }
  target.status = 'CONFIRMED';
  target.payment_plan_id = target.payment_plan_id ?? planId;
  return target;
};
