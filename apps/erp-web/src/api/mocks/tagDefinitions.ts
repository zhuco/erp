import dayjs from 'dayjs';

import type {
  OrderTagDefinition,
  OrderTagDefinitionCreatePayload,
  OrderTagDefinitionListQuery,
  OrderTagDefinitionListResponse,
  OrderTagDefinitionUpdatePayload,
} from '../orderTagDefinitions';
import type {
  SkuTagDefinition,
  SkuTagDefinitionCreatePayload,
  SkuTagDefinitionListQuery,
  SkuTagDefinitionListResponse,
  SkuTagDefinitionUpdatePayload,
} from '../skuTagDefinitions';

const mockTenantId = '00000000-0000-0000-0000-000000000000';

const orderTagSeeds: Array<{
  tag_id: string;
  name: string;
  color: string;
  priority: number;
  status: 'ACTIVE' | 'DISABLED';
  auto_apply: boolean;
  conditions_json: Record<string, unknown> | unknown[];
  createdOffsetDays: number;
  updatedOffsetHours: number;
}> = [
  {
    tag_id: 'OTAG-001',
    name: 'vip-priority',
    color: '#cf1322',
    priority: 90,
    status: 'ACTIVE',
    auto_apply: true,
    conditions_json: [{ field_path: 'customer_level', op: 'in', value: ['VIP', 'SVIP'] }],
    createdOffsetDays: 16,
    updatedOffsetHours: 4,
  },
  {
    tag_id: 'OTAG-002',
    name: 'high-amount',
    color: '#1677ff',
    priority: 80,
    status: 'ACTIVE',
    auto_apply: true,
    conditions_json: [{ field_path: 'total_cent', op: '>=', value: 500000 }],
    createdOffsetDays: 14,
    updatedOffsetHours: 8,
  },
  {
    tag_id: 'OTAG-003',
    name: 'address-review',
    color: '#d48806',
    priority: 70,
    status: 'ACTIVE',
    auto_apply: true,
    conditions_json: [{ field_path: 'address_invalid', op: '=', value: true }],
    createdOffsetDays: 12,
    updatedOffsetHours: 12,
  },
  {
    tag_id: 'OTAG-004',
    name: 'manual-hold',
    color: '#722ed1',
    priority: 65,
    status: 'DISABLED',
    auto_apply: false,
    conditions_json: [{ field_path: 'remark', op: 'contains', value: '人工复核' }],
    createdOffsetDays: 10,
    updatedOffsetHours: 18,
  },
  {
    tag_id: 'OTAG-005',
    name: 'refund-risk',
    color: '#eb2f96',
    priority: 60,
    status: 'ACTIVE',
    auto_apply: true,
    conditions_json: [{ field_path: 'refund_count_30d', op: '>=', value: 2 }],
    createdOffsetDays: 9,
    updatedOffsetHours: 20,
  },
  {
    tag_id: 'OTAG-006',
    name: 'fraud-check',
    color: '#fa541c',
    priority: 55,
    status: 'ACTIVE',
    auto_apply: true,
    conditions_json: [{ field_path: 'risk_level', op: 'in', value: ['MEDIUM', 'HIGH'] }],
    createdOffsetDays: 7,
    updatedOffsetHours: 24,
  },
  {
    tag_id: 'OTAG-007',
    name: 'vip-gift',
    color: '#13c2c2',
    priority: 50,
    status: 'ACTIVE',
    auto_apply: false,
    conditions_json: [{ field_path: 'gift_required', op: '=', value: true }],
    createdOffsetDays: 5,
    updatedOffsetHours: 30,
  },
  {
    tag_id: 'OTAG-008',
    name: 'late-shipment',
    color: '#2f54eb',
    priority: 45,
    status: 'DISABLED',
    auto_apply: false,
    conditions_json: [{ field_path: 'ship_delay_hours', op: '>=', value: 24 }],
    createdOffsetDays: 3,
    updatedOffsetHours: 36,
  },
];

const skuTagSeeds: Array<{
  tag_id: string;
  name: string;
  color: string;
  priority: number;
  status: 'ACTIVE' | 'DISABLED';
  createdOffsetDays: number;
  updatedOffsetHours: number;
}> = [
  {
    tag_id: 'STAG-001',
    name: 'fragile-glass',
    color: '#1677ff',
    priority: 90,
    status: 'ACTIVE',
    createdOffsetDays: 16,
    updatedOffsetHours: 4,
  },
  {
    tag_id: 'STAG-002',
    name: 'cold-chain',
    color: '#13c2c2',
    priority: 82,
    status: 'ACTIVE',
    createdOffsetDays: 15,
    updatedOffsetHours: 8,
  },
  {
    tag_id: 'STAG-003',
    name: 'serial-control',
    color: '#722ed1',
    priority: 76,
    status: 'ACTIVE',
    createdOffsetDays: 13,
    updatedOffsetHours: 12,
  },
  {
    tag_id: 'STAG-004',
    name: 'oversize',
    color: '#fa8c16',
    priority: 68,
    status: 'ACTIVE',
    createdOffsetDays: 11,
    updatedOffsetHours: 18,
  },
  {
    tag_id: 'STAG-005',
    name: 'fragile-ceramic',
    color: '#cf1322',
    priority: 64,
    status: 'ACTIVE',
    createdOffsetDays: 9,
    updatedOffsetHours: 24,
  },
  {
    tag_id: 'STAG-006',
    name: 'restricted-sale',
    color: '#eb2f96',
    priority: 58,
    status: 'DISABLED',
    createdOffsetDays: 7,
    updatedOffsetHours: 28,
  },
  {
    tag_id: 'STAG-007',
    name: 'seasonal-promo',
    color: '#52c41a',
    priority: 50,
    status: 'ACTIVE',
    createdOffsetDays: 5,
    updatedOffsetHours: 34,
  },
  {
    tag_id: 'STAG-008',
    name: 'fragile-display',
    color: '#d48806',
    priority: 46,
    status: 'DISABLED',
    createdOffsetDays: 4,
    updatedOffsetHours: 38,
  },
];

const buildOrderTagDefinition = (
  seed: (typeof orderTagSeeds)[number],
): OrderTagDefinition => ({
  tag_id: seed.tag_id,
  tenant_id: mockTenantId,
  name: seed.name,
  color: seed.color,
  priority: seed.priority,
  status: seed.status,
  auto_apply: seed.auto_apply,
  conditions_json: seed.conditions_json,
  created_at: dayjs().subtract(seed.createdOffsetDays, 'day').toISOString(),
  updated_at: dayjs().subtract(seed.updatedOffsetHours, 'hour').toISOString(),
});

const buildSkuTagDefinition = (
  seed: (typeof skuTagSeeds)[number],
): SkuTagDefinition => ({
  tag_id: seed.tag_id,
  tenant_id: mockTenantId,
  name: seed.name,
  color: seed.color,
  priority: seed.priority,
  status: seed.status,
  created_at: dayjs().subtract(seed.createdOffsetDays, 'day').toISOString(),
  updated_at: dayjs().subtract(seed.updatedOffsetHours, 'hour').toISOString(),
});

const mockOrderTagDefinitions: OrderTagDefinition[] = orderTagSeeds.map(buildOrderTagDefinition);
const mockSkuTagDefinitions: SkuTagDefinition[] = skuTagSeeds.map(buildSkuTagDefinition);

const matchKeyword = (keyword: string | undefined, fields: Array<string | number | boolean | null | undefined>) => {
  const normalizedKeyword = keyword?.trim().toLowerCase();
  if (!normalizedKeyword) {
    return true;
  }
  return fields
    .map((field) => String(field ?? '').toLowerCase())
    .some((field) => field.includes(normalizedKeyword));
};

const applyPagination = <TRow,>(rows: TRow[], offset?: number, limit?: number) => {
  const start = Math.max(0, offset ?? 0);
  const pageSize = Math.max(1, limit ?? (rows.length || 1));
  return rows.slice(start, start + pageSize);
};

const sortByPriority = <TRow extends { priority: number; updated_at: string }>(rows: TRow[]) =>
  [...rows].sort((left, right) => {
    if (left.priority !== right.priority) {
      return right.priority - left.priority;
    }
    return right.updated_at.localeCompare(left.updated_at);
  });

export const listMockOrderTagDefinitions = (
  query: OrderTagDefinitionListQuery,
): OrderTagDefinitionListResponse => {
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = sortByPriority(mockOrderTagDefinitions).filter((item) => {
    if (normalizedStatus && item.status !== normalizedStatus) {
      return false;
    }
    return matchKeyword(query.keyword, [
      item.name,
      item.color,
      item.status,
      item.auto_apply,
      JSON.stringify(item.conditions_json),
    ]);
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length,
  };
};

export const createMockOrderTagDefinition = (
  tenantId: string,
  payload: OrderTagDefinitionCreatePayload,
): OrderTagDefinition => {
  const now = dayjs().toISOString();
  const created: OrderTagDefinition = {
    tag_id: `OTAG-${Date.now()}`,
    tenant_id: tenantId || mockTenantId,
    name: payload.name.trim(),
    color: payload.color ?? null,
    priority: payload.priority ?? 0,
    status: payload.status ?? 'ACTIVE',
    auto_apply: payload.auto_apply ?? false,
    conditions_json: payload.conditions_json ?? [],
    created_at: now,
    updated_at: now,
  };
  mockOrderTagDefinitions.unshift(created);
  return created;
};

export const updateMockOrderTagDefinition = (
  tenantId: string,
  tagId: string,
  payload: OrderTagDefinitionUpdatePayload,
): OrderTagDefinition => {
  const now = dayjs().toISOString();
  const index = mockOrderTagDefinitions.findIndex((item) => item.tag_id === tagId);
  const base =
    index >= 0
      ? mockOrderTagDefinitions[index]
      : ({
          tag_id: tagId,
          tenant_id: tenantId || mockTenantId,
          name: 'mock-order-tag',
          color: null,
          priority: 0,
          status: 'ACTIVE',
          auto_apply: false,
          conditions_json: [],
          created_at: now,
          updated_at: now,
        } as OrderTagDefinition);
  const updated: OrderTagDefinition = {
    ...base,
    name: payload.name ?? base.name,
    color: payload.color ?? base.color ?? null,
    priority: payload.priority ?? base.priority,
    status: payload.status ?? base.status,
    auto_apply: payload.auto_apply ?? base.auto_apply,
    conditions_json: payload.conditions_json ?? base.conditions_json,
    updated_at: now,
  };
  if (index >= 0) {
    mockOrderTagDefinitions[index] = updated;
  } else {
    mockOrderTagDefinitions.unshift(updated);
  }
  return updated;
};

export const deleteMockOrderTagDefinition = (
  tenantId: string,
  tagId: string,
): OrderTagDefinition => {
  const index = mockOrderTagDefinitions.findIndex((item) => item.tag_id === tagId);
  if (index >= 0) {
    const [removed] = mockOrderTagDefinitions.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    tag_id: tagId,
    tenant_id: tenantId || mockTenantId,
    name: 'mock-order-tag',
    color: null,
    priority: 0,
    status: 'ACTIVE',
    auto_apply: false,
    conditions_json: [],
    created_at: now,
    updated_at: now,
  };
};

export const listMockSkuTagDefinitions = (
  query: SkuTagDefinitionListQuery,
): SkuTagDefinitionListResponse => {
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = sortByPriority(mockSkuTagDefinitions).filter((item) => {
    if (normalizedStatus && item.status !== normalizedStatus) {
      return false;
    }
    return matchKeyword(query.keyword, [item.name, item.color, item.status]);
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length,
  };
};

export const createMockSkuTagDefinition = (
  tenantId: string,
  payload: SkuTagDefinitionCreatePayload,
): SkuTagDefinition => {
  const now = dayjs().toISOString();
  const created: SkuTagDefinition = {
    tag_id: `STAG-${Date.now()}`,
    tenant_id: tenantId || mockTenantId,
    name: payload.name.trim(),
    color: payload.color ?? null,
    priority: payload.priority ?? 0,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now,
  };
  mockSkuTagDefinitions.unshift(created);
  return created;
};

export const updateMockSkuTagDefinition = (
  tenantId: string,
  tagId: string,
  payload: SkuTagDefinitionUpdatePayload,
): SkuTagDefinition => {
  const now = dayjs().toISOString();
  const index = mockSkuTagDefinitions.findIndex((item) => item.tag_id === tagId);
  const base =
    index >= 0
      ? mockSkuTagDefinitions[index]
      : ({
          tag_id: tagId,
          tenant_id: tenantId || mockTenantId,
          name: 'mock-sku-tag',
          color: null,
          priority: 0,
          status: 'ACTIVE',
          created_at: now,
          updated_at: now,
        } as SkuTagDefinition);
  const updated: SkuTagDefinition = {
    ...base,
    name: payload.name ?? base.name,
    color: payload.color ?? base.color ?? null,
    priority: payload.priority ?? base.priority,
    status: payload.status ?? base.status,
    updated_at: now,
  };
  if (index >= 0) {
    mockSkuTagDefinitions[index] = updated;
  } else {
    mockSkuTagDefinitions.unshift(updated);
  }
  return updated;
};

export const deleteMockSkuTagDefinition = (
  tenantId: string,
  tagId: string,
): SkuTagDefinition => {
  const index = mockSkuTagDefinitions.findIndex((item) => item.tag_id === tagId);
  if (index >= 0) {
    const [removed] = mockSkuTagDefinitions.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    tag_id: tagId,
    tenant_id: tenantId || mockTenantId,
    name: 'mock-sku-tag',
    color: null,
    priority: 0,
    status: 'ACTIVE',
    created_at: now,
    updated_at: now,
  };
};
