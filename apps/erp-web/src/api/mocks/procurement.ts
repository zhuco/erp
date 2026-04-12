import dayjs from 'dayjs';

import type {
  ProcurementInspection,
  ProcurementInspectionCreatePayload,
  ProcurementInspectionDetail,
  ProcurementInspectionItem,
  ProcurementInspectionListQuery,
  ProcurementInspectionListResponse,
  ProcurementInspectionUpdatePayload
} from '../procurementInspections';
import type {
  ProcurementPlan,
  ProcurementPlanCreatePayload,
  ProcurementPlanDetail,
  ProcurementPlanItem,
  ProcurementPlanListQuery,
  ProcurementPlanListResponse,
  ProcurementPlanUpdatePayload
} from '../procurementPlans';
import type {
  ProcurementReconciliation,
  ProcurementReconciliationCreatePayload,
  ProcurementReconciliationListQuery,
  ProcurementReconciliationListResponse,
  ProcurementReconciliationUpdatePayload
} from '../procurementReconciliations';
import type {
  ProcurementRequest,
  ProcurementRequestCreatePayload,
  ProcurementRequestDetail,
  ProcurementRequestItem,
  ProcurementRequestListQuery,
  ProcurementRequestListResponse,
  ProcurementRequestUpdatePayload
} from '../procurementRequests';
import type {
  ProcurementSourcing,
  ProcurementSourcingCreatePayload,
  ProcurementSourcingListQuery,
  ProcurementSourcingListResponse,
  ProcurementSourcingUpdatePayload
} from '../procurementSourcing';
import type {
  SupplierPerformance,
  SupplierPerformanceCreatePayload,
  SupplierPerformanceListQuery,
  SupplierPerformanceListResponse,
  SupplierPerformanceUpdatePayload
} from '../supplierPerformances';

import { applyPagination, mockSkus, mockSuppliers, mockWarehouses } from './data';

const resolveSkuCode = (skuId: string) =>
  mockSkus.find((sku) => sku.sku_id === skuId)?.sku_code ?? skuId;

const resolveSupplierName = (supplierId?: string | null) =>
  mockSuppliers.find((supplier) => supplier.supplier_id === supplierId)?.name ?? null;

const resolveWarehouseName = (warehouseId?: string | null) =>
  mockWarehouses.find((warehouse) => warehouse.warehouse_id === warehouseId)?.name ?? null;

const calcTotalCent = (items: Array<{ qty: number; price_cent: number }>) =>
  items.reduce((sum, item) => sum + item.qty * item.price_cent, 0);

const calcPerformanceScore = (delivery: number, quality: number, price: number) =>
  Math.round((delivery + quality + price) / 3);

const mockProcurementRequests: ProcurementRequest[] = [
  {
    request_id: 'PR-1001',
    department: '供应链',
    requester: '王敏',
    status: 'DRAFT',
    budget_cent: 120000,
    expected_at: dayjs().add(10, 'day').toISOString(),
    remark: '新品采购',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'hour').toISOString()
  }
];

const mockProcurementRequestItems = new Map<string, ProcurementRequestItem[]>([
  [
    'PR-1001',
    [
      {
        sku_id: 'SKU-1001',
        sku_code: resolveSkuCode('SKU-1001'),
        qty: 100,
        price_cent: 1200
      }
    ]
  ]
]);

const mockProcurementPlans: ProcurementPlan[] = [
  {
    plan_id: 'PP-2001',
    request_id: 'PR-1001',
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
    status: 'DRAFT',
    total_cent: 120000,
    planned_at: dayjs().toISOString(),
    expected_at: dayjs().add(12, 'day').toISOString(),
    remark: '首批计划',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'hour').toISOString()
  }
];

const mockProcurementPlanItems = new Map<string, ProcurementPlanItem[]>([
  [
    'PP-2001',
    [
      {
        sku_id: 'SKU-1001',
        sku_code: resolveSkuCode('SKU-1001'),
        qty: 100,
        price_cent: 1200
      }
    ]
  ]
]);

const mockProcurementSourcing: ProcurementSourcing[] = [
  {
    sourcing_id: 'PS-3001',
    title: '春季新品供应商评估',
    category: '服饰',
    supplier_id: mockSuppliers[1]?.supplier_id ?? 'SUP-2',
    supplier_name: mockSuppliers[1]?.name ?? 'Supplier',
    status: 'OPEN',
    target_price_cent: 9800,
    remark: '准备进行资质审核',
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockProcurementInspections: ProcurementInspection[] = [
  {
    inspection_id: 'PI-4001',
    receipt_id: null,
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
    warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
    status: 'PENDING',
    inspected_at: dayjs().toISOString(),
    remark: '首批抽检',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'hour').toISOString()
  }
];

const mockProcurementInspectionItems = new Map<string, ProcurementInspectionItem[]>([
  [
    'PI-4001',
    [
      {
        sku_id: 'SKU-1001',
        sku_code: resolveSkuCode('SKU-1001'),
        qty: 100,
        passed_qty: 96,
        failed_qty: 4,
        result: 'minor_defects'
      }
    ]
  ]
]);

const mockProcurementReconciliations: ProcurementReconciliation[] = [
  {
    reconciliation_id: 'PRC-5001',
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
    period_start: dayjs().subtract(30, 'day').format('YYYY-MM-DD'),
    period_end: dayjs().subtract(1, 'day').format('YYYY-MM-DD'),
    amount_cent: 220000,
    status: 'OPEN',
    remark: '待确认',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockSupplierPerformances: SupplierPerformance[] = [
  {
    performance_id: 'SP-6001',
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
    period_start: dayjs().subtract(90, 'day').format('YYYY-MM-DD'),
    period_end: dayjs().subtract(1, 'day').format('YYYY-MM-DD'),
    delivery_score: 85,
    quality_score: 88,
    price_score: 80,
    total_score: 84,
    status: 'ACTIVE',
    remark: '整体表现良好',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

export const getMockProcurementRequestList = (
  query: ProcurementRequestListQuery
): ProcurementRequestListResponse => {
  const filtered = mockProcurementRequests.filter((request) => {
    if (query.department && !request.department?.includes(query.department)) {
      return false;
    }
    if (query.requester && !request.requester?.includes(query.requester)) {
      return false;
    }
    if (query.status && request.status !== query.status) {
      return false;
    }
    if (query.created_from && request.created_at) {
      if (dayjs(request.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && request.created_at) {
      if (dayjs(request.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockProcurementRequestDetail = (
  requestId: string
): ProcurementRequestDetail => {
  const request =
    mockProcurementRequests.find((item) => item.request_id === requestId) ??
    ({
      request_id: requestId,
      department: null,
      requester: null,
      status: 'DRAFT',
      budget_cent: 0,
      expected_at: null,
      remark: null,
      created_at: dayjs().toISOString(),
      updated_at: dayjs().toISOString()
    } as ProcurementRequest);
  return {
    request,
    items: mockProcurementRequestItems.get(requestId) ?? []
  };
};

export const createMockProcurementRequest = (
  payload: ProcurementRequestCreatePayload
): ProcurementRequestDetail => {
  const now = dayjs().toISOString();
  const items: ProcurementRequestItem[] = payload.items.map((item) => ({
    sku_id: item.sku_id,
    sku_code: resolveSkuCode(item.sku_id),
    qty: item.qty,
    price_cent: item.price_cent
  }));
  const request: ProcurementRequest = {
    request_id: `PR-${Date.now()}`,
    department: payload.department ?? null,
    requester: payload.requester ?? null,
    status: 'DRAFT',
    budget_cent: calcTotalCent(items),
    expected_at: payload.expected_at ?? null,
    remark: payload.remark ?? null,
    created_at: now,
    updated_at: now
  };
  mockProcurementRequests.unshift(request);
  mockProcurementRequestItems.set(request.request_id, items);
  return { request, items };
};

export const updateMockProcurementRequest = (
  requestId: string,
  payload: ProcurementRequestUpdatePayload
): ProcurementRequestDetail => {
  const now = dayjs().toISOString();
  const index = mockProcurementRequests.findIndex((item) => item.request_id === requestId);
  const base =
    index >= 0
      ? mockProcurementRequests[index]
      : ({
          request_id: requestId,
          department: payload.department ?? null,
          requester: payload.requester ?? null,
          status: 'DRAFT',
          budget_cent: 0,
          expected_at: payload.expected_at ?? null,
          remark: payload.remark ?? null,
          created_at: now,
          updated_at: now
        } as ProcurementRequest);
  const items =
    payload.items?.map((item) => ({
      sku_id: item.sku_id,
      sku_code: resolveSkuCode(item.sku_id),
      qty: item.qty,
      price_cent: item.price_cent
    })) ?? mockProcurementRequestItems.get(requestId) ?? [];
  const updated: ProcurementRequest = {
    ...base,
    department: payload.department ?? base.department ?? null,
    requester: payload.requester ?? base.requester ?? null,
    status: (payload.status ?? base.status) as string,
    budget_cent: calcTotalCent(items),
    expected_at: payload.expected_at ?? base.expected_at ?? null,
    remark: payload.remark ?? base.remark ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockProcurementRequests[index] = updated;
  } else {
    mockProcurementRequests.unshift(updated);
  }
  mockProcurementRequestItems.set(requestId, items);
  return { request: updated, items };
};

export const deleteMockProcurementRequest = (requestId: string): ProcurementRequest => {
  const index = mockProcurementRequests.findIndex((item) => item.request_id === requestId);
  if (index >= 0) {
    const [removed] = mockProcurementRequests.splice(index, 1);
    mockProcurementRequestItems.delete(requestId);
    return removed;
  }
  return {
    request_id: requestId,
    department: null,
    requester: null,
    status: 'DRAFT',
    budget_cent: 0,
    expected_at: null,
    remark: null,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const getMockProcurementPlanList = (
  query: ProcurementPlanListQuery
): ProcurementPlanListResponse => {
  const filtered = mockProcurementPlans.filter((plan) => {
    if (query.supplier_id && plan.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.status && plan.status !== query.status) {
      return false;
    }
    if (query.created_from && plan.created_at) {
      if (dayjs(plan.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && plan.created_at) {
      if (dayjs(plan.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockProcurementPlanDetail = (planId: string): ProcurementPlanDetail => {
  const plan =
    mockProcurementPlans.find((item) => item.plan_id === planId) ??
    ({
      plan_id: planId,
      request_id: null,
      supplier_id: null,
      supplier_name: resolveSupplierName(null),
      status: 'DRAFT',
      total_cent: 0,
      planned_at: null,
      expected_at: null,
      remark: null,
      created_at: dayjs().toISOString(),
      updated_at: dayjs().toISOString()
    } as ProcurementPlan);
  return { plan, items: mockProcurementPlanItems.get(planId) ?? [] };
};

export const createMockProcurementPlan = (
  payload: ProcurementPlanCreatePayload
): ProcurementPlanDetail => {
  const now = dayjs().toISOString();
  const items: ProcurementPlanItem[] = payload.items.map((item) => ({
    sku_id: item.sku_id,
    sku_code: resolveSkuCode(item.sku_id),
    qty: item.qty,
    price_cent: item.price_cent
  }));
  const plan: ProcurementPlan = {
    plan_id: `PP-${Date.now()}`,
    request_id: payload.request_id ?? null,
    supplier_id: payload.supplier_id ?? null,
    supplier_name: resolveSupplierName(payload.supplier_id),
    status: 'DRAFT',
    total_cent: calcTotalCent(items),
    planned_at: payload.planned_at ?? null,
    expected_at: payload.expected_at ?? null,
    remark: payload.remark ?? null,
    created_at: now,
    updated_at: now
  };
  mockProcurementPlans.unshift(plan);
  mockProcurementPlanItems.set(plan.plan_id, items);
  return { plan, items };
};

export const updateMockProcurementPlan = (
  planId: string,
  payload: ProcurementPlanUpdatePayload
): ProcurementPlanDetail => {
  const now = dayjs().toISOString();
  const index = mockProcurementPlans.findIndex((item) => item.plan_id === planId);
  const base =
    index >= 0
      ? mockProcurementPlans[index]
      : ({
          plan_id: planId,
          request_id: null,
          supplier_id: payload.supplier_id ?? null,
          supplier_name: resolveSupplierName(payload.supplier_id),
          status: 'DRAFT',
          total_cent: 0,
          planned_at: payload.planned_at ?? null,
          expected_at: payload.expected_at ?? null,
          remark: payload.remark ?? null,
          created_at: now,
          updated_at: now
        } as ProcurementPlan);
  const items =
    payload.items?.map((item) => ({
      sku_id: item.sku_id,
      sku_code: resolveSkuCode(item.sku_id),
      qty: item.qty,
      price_cent: item.price_cent
    })) ?? mockProcurementPlanItems.get(planId) ?? [];
  const updated: ProcurementPlan = {
    ...base,
    supplier_id: payload.supplier_id ?? base.supplier_id ?? null,
    supplier_name: resolveSupplierName(payload.supplier_id ?? base.supplier_id ?? null),
    status: (payload.status ?? base.status) as string,
    total_cent: calcTotalCent(items),
    planned_at: payload.planned_at ?? base.planned_at ?? null,
    expected_at: payload.expected_at ?? base.expected_at ?? null,
    remark: payload.remark ?? base.remark ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockProcurementPlans[index] = updated;
  } else {
    mockProcurementPlans.unshift(updated);
  }
  mockProcurementPlanItems.set(planId, items);
  return { plan: updated, items };
};

export const deleteMockProcurementPlan = (planId: string): ProcurementPlan => {
  const index = mockProcurementPlans.findIndex((item) => item.plan_id === planId);
  if (index >= 0) {
    const [removed] = mockProcurementPlans.splice(index, 1);
    mockProcurementPlanItems.delete(planId);
    return removed;
  }
  return {
    plan_id: planId,
    request_id: null,
    supplier_id: null,
    supplier_name: null,
    status: 'DRAFT',
    total_cent: 0,
    planned_at: null,
    expected_at: null,
    remark: null,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const getMockProcurementSourcingList = (
  query: ProcurementSourcingListQuery
): ProcurementSourcingListResponse => {
  const filtered = mockProcurementSourcing.filter((sourcing) => {
    if (query.keyword) {
      const keyword = query.keyword.toLowerCase();
      const title = sourcing.title.toLowerCase();
      const category = sourcing.category?.toLowerCase() ?? '';
      if (!title.includes(keyword) && !category.includes(keyword)) {
        return false;
      }
    }
    if (query.supplier_id && sourcing.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.status && sourcing.status !== query.status) {
      return false;
    }
    if (query.created_from && sourcing.created_at) {
      if (dayjs(sourcing.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && sourcing.created_at) {
      if (dayjs(sourcing.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const createMockProcurementSourcing = (
  payload: ProcurementSourcingCreatePayload
): ProcurementSourcing => {
  const now = dayjs().toISOString();
  const sourcing: ProcurementSourcing = {
    sourcing_id: `PS-${Date.now()}`,
    title: payload.title,
    category: payload.category ?? null,
    supplier_id: payload.supplier_id ?? null,
    supplier_name: resolveSupplierName(payload.supplier_id ?? null),
    status: payload.status ?? 'OPEN',
    target_price_cent: payload.target_price_cent,
    remark: payload.remark ?? null,
    created_at: now,
    updated_at: now
  };
  mockProcurementSourcing.unshift(sourcing);
  return sourcing;
};

export const updateMockProcurementSourcing = (
  sourcingId: string,
  payload: ProcurementSourcingUpdatePayload
): ProcurementSourcing => {
  const now = dayjs().toISOString();
  const index = mockProcurementSourcing.findIndex((item) => item.sourcing_id === sourcingId);
  const base =
    index >= 0
      ? mockProcurementSourcing[index]
      : ({
          sourcing_id: sourcingId,
          title: payload.title ?? '新寻源需求',
          category: payload.category ?? null,
          supplier_id: payload.supplier_id ?? null,
          supplier_name: resolveSupplierName(payload.supplier_id ?? null),
          status: payload.status ?? 'OPEN',
          target_price_cent: payload.target_price_cent ?? 0,
          remark: payload.remark ?? null,
          created_at: now,
          updated_at: now
        } as ProcurementSourcing);
  const updated: ProcurementSourcing = {
    ...base,
    title: payload.title ?? base.title,
    category: payload.category ?? base.category ?? null,
    supplier_id: payload.supplier_id ?? base.supplier_id ?? null,
    supplier_name: resolveSupplierName(payload.supplier_id ?? base.supplier_id ?? null),
    status: payload.status ?? base.status,
    target_price_cent: payload.target_price_cent ?? base.target_price_cent,
    remark: payload.remark ?? base.remark ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockProcurementSourcing[index] = updated;
  } else {
    mockProcurementSourcing.unshift(updated);
  }
  return updated;
};

export const deleteMockProcurementSourcing = (sourcingId: string): ProcurementSourcing => {
  const index = mockProcurementSourcing.findIndex((item) => item.sourcing_id === sourcingId);
  if (index >= 0) {
    const [removed] = mockProcurementSourcing.splice(index, 1);
    return removed;
  }
  return {
    sourcing_id: sourcingId,
    title: '寻源需求',
    category: null,
    supplier_id: null,
    supplier_name: null,
    status: 'OPEN',
    target_price_cent: 0,
    remark: null,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const getMockProcurementInspectionList = (
  query: ProcurementInspectionListQuery
): ProcurementInspectionListResponse => {
  const filtered = mockProcurementInspections.filter((inspection) => {
    if (query.supplier_id && inspection.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.warehouse_id && inspection.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && inspection.status !== query.status) {
      return false;
    }
    if (query.created_from && inspection.created_at) {
      if (dayjs(inspection.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && inspection.created_at) {
      if (dayjs(inspection.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockProcurementInspectionDetail = (
  inspectionId: string
): ProcurementInspectionDetail => {
  const inspection =
    mockProcurementInspections.find((item) => item.inspection_id === inspectionId) ??
    ({
      inspection_id: inspectionId,
      receipt_id: null,
      supplier_id: null,
      supplier_name: resolveSupplierName(null),
      warehouse_id: null,
      status: 'PENDING',
      inspected_at: null,
      remark: null,
      created_at: dayjs().toISOString(),
      updated_at: dayjs().toISOString()
    } as ProcurementInspection);
  return {
    inspection,
    items: mockProcurementInspectionItems.get(inspectionId) ?? []
  };
};

export const createMockProcurementInspection = (
  payload: ProcurementInspectionCreatePayload
): ProcurementInspectionDetail => {
  const now = dayjs().toISOString();
  const items: ProcurementInspectionItem[] = payload.items.map((item) => ({
    sku_id: item.sku_id,
    sku_code: resolveSkuCode(item.sku_id),
    qty: item.qty,
    passed_qty: item.passed_qty,
    failed_qty: item.failed_qty,
    result: item.result ?? null
  }));
  const inspection: ProcurementInspection = {
    inspection_id: `PI-${Date.now()}`,
    receipt_id: payload.receipt_id ?? null,
    supplier_id: payload.supplier_id ?? null,
    supplier_name: resolveSupplierName(payload.supplier_id ?? null),
    warehouse_id: payload.warehouse_id ?? null,
    status: 'PENDING',
    inspected_at: payload.inspected_at ?? null,
    remark: payload.remark ?? null,
    created_at: now,
    updated_at: now
  };
  mockProcurementInspections.unshift(inspection);
  mockProcurementInspectionItems.set(inspection.inspection_id, items);
  return { inspection, items };
};

export const updateMockProcurementInspection = (
  inspectionId: string,
  payload: ProcurementInspectionUpdatePayload
): ProcurementInspectionDetail => {
  const now = dayjs().toISOString();
  const index = mockProcurementInspections.findIndex(
    (item) => item.inspection_id === inspectionId
  );
  const base =
    index >= 0
      ? mockProcurementInspections[index]
      : ({
          inspection_id: inspectionId,
          receipt_id: null,
          supplier_id: null,
          supplier_name: resolveSupplierName(null),
          warehouse_id: null,
          status: payload.status ?? 'PENDING',
          inspected_at: payload.inspected_at ?? null,
          remark: payload.remark ?? null,
          created_at: now,
          updated_at: now
        } as ProcurementInspection);
  const items =
    payload.items?.map((item) => ({
      sku_id: item.sku_id,
      sku_code: resolveSkuCode(item.sku_id),
      qty: item.qty,
      passed_qty: item.passed_qty,
      failed_qty: item.failed_qty,
      result: item.result ?? null
    })) ?? mockProcurementInspectionItems.get(inspectionId) ?? [];
  const updated: ProcurementInspection = {
    ...base,
    status: payload.status ?? base.status,
    inspected_at: payload.inspected_at ?? base.inspected_at ?? null,
    remark: payload.remark ?? base.remark ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockProcurementInspections[index] = updated;
  } else {
    mockProcurementInspections.unshift(updated);
  }
  mockProcurementInspectionItems.set(inspectionId, items);
  return { inspection: updated, items };
};

export const deleteMockProcurementInspection = (
  inspectionId: string
): ProcurementInspection => {
  const index = mockProcurementInspections.findIndex(
    (item) => item.inspection_id === inspectionId
  );
  if (index >= 0) {
    const [removed] = mockProcurementInspections.splice(index, 1);
    mockProcurementInspectionItems.delete(inspectionId);
    return removed;
  }
  return {
    inspection_id: inspectionId,
    receipt_id: null,
    supplier_id: null,
    supplier_name: null,
    warehouse_id: null,
    status: 'PENDING',
    inspected_at: null,
    remark: null,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const getMockProcurementReconciliationList = (
  query: ProcurementReconciliationListQuery
): ProcurementReconciliationListResponse => {
  const filtered = mockProcurementReconciliations.filter((reconciliation) => {
    if (query.supplier_id && reconciliation.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.status && reconciliation.status !== query.status) {
      return false;
    }
    if (query.period_from) {
      if (dayjs(reconciliation.period_start).isBefore(query.period_from, 'day')) {
        return false;
      }
    }
    if (query.period_to) {
      if (dayjs(reconciliation.period_end).isAfter(query.period_to, 'day')) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const createMockProcurementReconciliation = (
  payload: ProcurementReconciliationCreatePayload
): ProcurementReconciliation => {
  const now = dayjs().toISOString();
  const reconciliation: ProcurementReconciliation = {
    reconciliation_id: `PRC-${Date.now()}`,
    supplier_id: payload.supplier_id,
    supplier_name: resolveSupplierName(payload.supplier_id),
    period_start: payload.period_start,
    period_end: payload.period_end,
    amount_cent: payload.amount_cent,
    status: payload.status ?? 'OPEN',
    remark: payload.remark ?? null,
    created_at: now,
    updated_at: now
  };
  mockProcurementReconciliations.unshift(reconciliation);
  return reconciliation;
};

export const updateMockProcurementReconciliation = (
  reconciliationId: string,
  payload: ProcurementReconciliationUpdatePayload
): ProcurementReconciliation => {
  const now = dayjs().toISOString();
  const index = mockProcurementReconciliations.findIndex(
    (item) => item.reconciliation_id === reconciliationId
  );
  const base =
    index >= 0
      ? mockProcurementReconciliations[index]
      : ({
          reconciliation_id: reconciliationId,
          supplier_id: payload.supplier_id ?? mockSuppliers[0]?.supplier_id ?? 'SUP-1',
          supplier_name: resolveSupplierName(payload.supplier_id ?? null),
          period_start: payload.period_start ?? dayjs().format('YYYY-MM-DD'),
          period_end: payload.period_end ?? dayjs().format('YYYY-MM-DD'),
          amount_cent: payload.amount_cent ?? 0,
          status: payload.status ?? 'OPEN',
          remark: payload.remark ?? null,
          created_at: now,
          updated_at: now
        } as ProcurementReconciliation);
  const updated: ProcurementReconciliation = {
    ...base,
    supplier_id: payload.supplier_id ?? base.supplier_id,
    supplier_name: resolveSupplierName(payload.supplier_id ?? base.supplier_id ?? null),
    period_start: payload.period_start ?? base.period_start,
    period_end: payload.period_end ?? base.period_end,
    amount_cent: payload.amount_cent ?? base.amount_cent,
    status: payload.status ?? base.status,
    remark: payload.remark ?? base.remark ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockProcurementReconciliations[index] = updated;
  } else {
    mockProcurementReconciliations.unshift(updated);
  }
  return updated;
};

export const deleteMockProcurementReconciliation = (
  reconciliationId: string
): ProcurementReconciliation => {
  const index = mockProcurementReconciliations.findIndex(
    (item) => item.reconciliation_id === reconciliationId
  );
  if (index >= 0) {
    const [removed] = mockProcurementReconciliations.splice(index, 1);
    return removed;
  }
  return {
    reconciliation_id: reconciliationId,
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: resolveSupplierName(mockSuppliers[0]?.supplier_id ?? null),
    period_start: dayjs().format('YYYY-MM-DD'),
    period_end: dayjs().format('YYYY-MM-DD'),
    amount_cent: 0,
    status: 'OPEN',
    remark: null,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const getMockSupplierPerformanceList = (
  query: SupplierPerformanceListQuery
): SupplierPerformanceListResponse => {
  const filtered = mockSupplierPerformances.filter((performance) => {
    if (query.supplier_id && performance.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.status && performance.status !== query.status) {
      return false;
    }
    if (query.period_from) {
      if (dayjs(performance.period_start).isBefore(query.period_from, 'day')) {
        return false;
      }
    }
    if (query.period_to) {
      if (dayjs(performance.period_end).isAfter(query.period_to, 'day')) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const createMockSupplierPerformance = (
  payload: SupplierPerformanceCreatePayload
): SupplierPerformance => {
  const now = dayjs().toISOString();
  const totalScore = calcPerformanceScore(
    payload.delivery_score,
    payload.quality_score,
    payload.price_score
  );
  const performance: SupplierPerformance = {
    performance_id: `SP-${Date.now()}`,
    supplier_id: payload.supplier_id,
    supplier_name: resolveSupplierName(payload.supplier_id),
    period_start: payload.period_start,
    period_end: payload.period_end,
    delivery_score: payload.delivery_score,
    quality_score: payload.quality_score,
    price_score: payload.price_score,
    total_score: totalScore,
    status: payload.status ?? 'ACTIVE',
    remark: payload.remark ?? null,
    created_at: now,
    updated_at: now
  };
  mockSupplierPerformances.unshift(performance);
  return performance;
};

export const updateMockSupplierPerformance = (
  performanceId: string,
  payload: SupplierPerformanceUpdatePayload
): SupplierPerformance => {
  const now = dayjs().toISOString();
  const index = mockSupplierPerformances.findIndex(
    (item) => item.performance_id === performanceId
  );
  const base =
    index >= 0
      ? mockSupplierPerformances[index]
      : ({
          performance_id: performanceId,
          supplier_id: payload.supplier_id ?? mockSuppliers[0]?.supplier_id ?? 'SUP-1',
          supplier_name: resolveSupplierName(payload.supplier_id ?? null),
          period_start: payload.period_start ?? dayjs().format('YYYY-MM-DD'),
          period_end: payload.period_end ?? dayjs().format('YYYY-MM-DD'),
          delivery_score: payload.delivery_score ?? 0,
          quality_score: payload.quality_score ?? 0,
          price_score: payload.price_score ?? 0,
          total_score: calcPerformanceScore(
            payload.delivery_score ?? 0,
            payload.quality_score ?? 0,
            payload.price_score ?? 0
          ),
          status: payload.status ?? 'ACTIVE',
          remark: payload.remark ?? null,
          created_at: now,
          updated_at: now
        } as SupplierPerformance);
  const deliveryScore = payload.delivery_score ?? base.delivery_score;
  const qualityScore = payload.quality_score ?? base.quality_score;
  const priceScore = payload.price_score ?? base.price_score;
  const updated: SupplierPerformance = {
    ...base,
    supplier_id: payload.supplier_id ?? base.supplier_id,
    supplier_name: resolveSupplierName(payload.supplier_id ?? base.supplier_id ?? null),
    period_start: payload.period_start ?? base.period_start,
    period_end: payload.period_end ?? base.period_end,
    delivery_score: deliveryScore,
    quality_score: qualityScore,
    price_score: priceScore,
    total_score: calcPerformanceScore(deliveryScore, qualityScore, priceScore),
    status: payload.status ?? base.status,
    remark: payload.remark ?? base.remark ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockSupplierPerformances[index] = updated;
  } else {
    mockSupplierPerformances.unshift(updated);
  }
  return updated;
};

export const deleteMockSupplierPerformance = (
  performanceId: string
): SupplierPerformance => {
  const index = mockSupplierPerformances.findIndex((item) => item.performance_id === performanceId);
  if (index >= 0) {
    const [removed] = mockSupplierPerformances.splice(index, 1);
    return removed;
  }
  return {
    performance_id: performanceId,
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: resolveSupplierName(mockSuppliers[0]?.supplier_id ?? null),
    period_start: dayjs().format('YYYY-MM-DD'),
    period_end: dayjs().format('YYYY-MM-DD'),
    delivery_score: 0,
    quality_score: 0,
    price_score: 0,
    total_score: 0,
    status: 'ACTIVE',
    remark: null,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const resolveInspectionSummary = (inspection: ProcurementInspection) => ({
  supplier: inspection.supplier_name ?? resolveSupplierName(inspection.supplier_id),
  warehouse: resolveWarehouseName(inspection.warehouse_id)
});
