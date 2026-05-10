import dayjs from 'dayjs';

import { Platform } from '@/features/platform';

import type {
  Channel,
  ChannelCreatePayload,
  ChannelListQuery,
  ChannelListResponse,
  ChannelUpdatePayload
} from '../channels';
import type {
  Courier,
  CourierCreatePayload,
  CourierListQuery,
  CourierListResponse,
  CourierUpdatePayload
} from '../couriers';
import type {
  Customer,
  CustomerCreatePayload,
  CustomerDetail,
  CustomerListQuery,
  CustomerListResponse,
  CustomerTagReplacePayload,
  CustomerUpdatePayload
} from '../customers';
import type {
  CustomerCategory,
  CustomerCategoryCreatePayload,
  CustomerCategoryListQuery,
  CustomerCategoryListResponse,
  CustomerCategoryUpdatePayload
} from '../customerCategories';
import type {
  CustomerTag,
  CustomerTagCreatePayload,
  CustomerTagListQuery,
  CustomerTagListResponse,
  CustomerTagUpdatePayload
} from '../customerTags';
import type {
  MemberLevel,
  MemberLevelCreatePayload,
  MemberLevelListQuery,
  MemberLevelListResponse,
  MemberLevelUpdatePayload
} from '../memberLevels';
import type {
  SalesShopConfig,
  SalesShopConfigListQuery,
  SalesShopConfigListResponse,
  SalesShopConfigRequest
} from '../salesShops';
import type {
  Shop,
  ShopAuthCallbackPayload,
  ShopAuthorizePayload,
  ShopAuthorizeResponse,
  ShopCreatePayload,
  ShopListQuery,
  ShopListResponse,
  ShopUpdatePayload,
  TokenRefreshRequestResponse
} from '../shops';
import type {
  SyncJob,
  SyncJobListQuery,
  SyncJobListResponse,
  SyncJobRunPayload
} from '../syncJobs';
import type { SyncJobRunListQuery, SyncJobRunListResponse } from '../syncJobRuns';
import type {
  Supplier,
  SupplierCreatePayload,
  SupplierListQuery,
  SupplierListResponse,
  SupplierUpdatePayload
} from '../suppliers';
import type {
  Warehouse,
  WarehouseCreatePayload,
  WarehouseUpdatePayload,
  WarehouseListQuery,
  WarehouseListResponse
} from '../warehouses';

import {
  applyPagination,
  buildCustomer,
  buildSalesShopConfigView,
  buildShopTokenStatus,
  mockChannels,
  mockCouriers,
  mockCustomerCategories,
  mockCustomerTagLinks,
  mockCustomerTags,
  mockCustomers,
  mockMemberLevels,
  mockSalesShopConfigs,
  mockShops,
  mockSuppliers,
  mockSyncJobRuns,
  mockSyncJobs,
  mockTenantId,
  mockWarehouses
} from './data';

export const getMockWarehouseList = (query: WarehouseListQuery): WarehouseListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = mockWarehouses.filter((warehouse) => {
    if (normalizedStatus && warehouse.status !== normalizedStatus) {
      return false;
    }
    if (!normalizedKeyword) {
      return true;
    }
    return (
      warehouse.name.toLowerCase().includes(normalizedKeyword) ||
      warehouse.code.toLowerCase().includes(normalizedKeyword)
    );
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockWarehouse = (payload: WarehouseCreatePayload): Warehouse => {
  const now = dayjs().toISOString();
  const warehouse: Warehouse = {
    warehouse_id: `WH-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    code: payload.code,
    is_default: payload.is_default ?? mockWarehouses.length === 0,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  if (warehouse.is_default) {
    mockWarehouses.forEach((item) => {
      item.is_default = false;
      item.updated_at = now;
    });
  }
  mockWarehouses.unshift(warehouse);
  return warehouse;
};

export const updateMockWarehouse = (
  warehouseId: string,
  payload: WarehouseUpdatePayload
): Warehouse => {
  const now = dayjs().toISOString();
  const index = mockWarehouses.findIndex((warehouse) => warehouse.warehouse_id === warehouseId);
  const base =
    index >= 0
      ? mockWarehouses[index]
      : ({
          warehouse_id: warehouseId,
          tenant_id: mockTenantId,
          name: payload.name ?? '未命名仓库',
          code: payload.code ?? warehouseId,
          is_default: payload.is_default ?? false,
          status: payload.status ?? 'ACTIVE',
          created_at: now,
          updated_at: now
        } as Warehouse);
  const updated: Warehouse = {
    ...base,
    name: payload.name ?? base.name,
    code: payload.code ?? base.code,
    is_default: payload.is_default ?? base.is_default,
    status: payload.status ?? base.status ?? 'ACTIVE',
    updated_at: now
  };
  if (updated.is_default) {
    mockWarehouses.forEach((warehouse) => {
      warehouse.is_default = warehouse.warehouse_id === updated.warehouse_id;
      warehouse.updated_at = now;
    });
  }
  if (index >= 0) {
    mockWarehouses[index] = updated;
  } else {
    mockWarehouses.unshift(updated);
  }
  return updated;
};

export const deleteMockWarehouse = (warehouseId: string): Warehouse => {
  const index = mockWarehouses.findIndex((warehouse) => warehouse.warehouse_id === warehouseId);
  if (index >= 0) {
    const [removed] = mockWarehouses.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    warehouse_id: warehouseId,
    tenant_id: mockTenantId,
    name: '未知仓库',
    code: warehouseId,
    is_default: false,
    status: 'DISABLED',
    created_at: now,
    updated_at: now
  };
};

export const setMockDefaultWarehouse = (warehouseId: string): Warehouse | null => {
  const target = mockWarehouses.find((warehouse) => warehouse.warehouse_id === warehouseId);
  if (!target) {
    return null;
  }
  const now = dayjs().toISOString();
  mockWarehouses.forEach((warehouse) => {
    warehouse.is_default = warehouse.warehouse_id === warehouseId;
    warehouse.updated_at = now;
  });
  return target;
};

export const getMockCourierList = (query: CourierListQuery): CourierListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockCouriers.filter((courier) => {
    if (!normalizedKeyword) {
      return true;
    }
    return (
      courier.name.toLowerCase().includes(normalizedKeyword) ||
      courier.code.toLowerCase().includes(normalizedKeyword)
    );
  });
  const statusFilter = query.status?.trim().toUpperCase();
  const scoped = statusFilter
    ? filtered.filter((courier) => courier.status === statusFilter)
    : filtered;
  const sorted = [...scoped].sort((left, right) => {
    const leftRank = left.status === 'ACTIVE' ? 0 : 1;
    const rightRank = right.status === 'ACTIVE' ? 0 : 1;
    if (leftRank !== rightRank) {
      return leftRank - rightRank;
    }
    const leftTime = left.updated_at ?? '';
    const rightTime = right.updated_at ?? '';
    if (leftTime !== rightTime) {
      return rightTime.localeCompare(leftTime);
    }
    return left.name.localeCompare(right.name);
  });
  const items = applyPagination(sorted, query.offset, query.limit);
  return {
    items,
    total: scoped.length
  };
};

export const createMockCourier = (payload: CourierCreatePayload): Courier => {
  const now = dayjs().toISOString();
  const existingIndex = mockCouriers.findIndex((courier) => courier.code === payload.code);
  if (existingIndex >= 0) {
    const updated = {
      ...mockCouriers[existingIndex],
      name: payload.name,
      status: payload.status ?? mockCouriers[existingIndex].status,
      updated_at: now
    };
    mockCouriers[existingIndex] = updated;
    return updated;
  }
  const courier: Courier = {
    courier_id: `CR-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    code: payload.code,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  mockCouriers.unshift(courier);
  return courier;
};

export const updateMockCourier = (
  courierId: string,
  payload: CourierUpdatePayload
): Courier => {
  const now = dayjs().toISOString();
  const index = mockCouriers.findIndex((courier) => courier.courier_id === courierId);
  const base =
    index >= 0
      ? mockCouriers[index]
      : ({
          courier_id: courierId,
          tenant_id: mockTenantId,
          name: payload.name ?? '未命名',
          code: payload.code ?? courierId,
          status: payload.status ?? 'ACTIVE',
          created_at: now
        } as Courier);
  const updated: Courier = {
    ...base,
    name: payload.name ?? base.name,
    code: payload.code ?? base.code,
    status: payload.status ?? base.status,
    updated_at: now
  };
  if (index >= 0) {
    mockCouriers[index] = updated;
  } else {
    mockCouriers.unshift(updated);
  }
  return updated;
};

export const deleteMockCourier = (courierId: string): Courier => {
  const index = mockCouriers.findIndex((courier) => courier.courier_id === courierId);
  if (index >= 0) {
    const [removed] = mockCouriers.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    courier_id: courierId,
    tenant_id: mockTenantId,
    name: '未知快递',
    code: courierId,
    created_at: now,
    updated_at: now
  };
};

export const getMockChannelList = (query: ChannelListQuery): ChannelListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockChannels.filter((channel) => {
    if (query.status && channel.status !== query.status) {
      return false;
    }
    if (!normalizedKeyword) {
      return true;
    }
    return (
      String(channel.platform ?? '').toLowerCase().includes(normalizedKeyword) ||
      String(channel.channel_type ?? '').toLowerCase().includes(normalizedKeyword) ||
      channel.name.toLowerCase().includes(normalizedKeyword) ||
      channel.contact_name?.toLowerCase()?.includes(normalizedKeyword) ||
      channel.contact_phone?.toLowerCase()?.includes(normalizedKeyword) ||
      channel.contact_address?.toLowerCase()?.includes(normalizedKeyword) ||
      channel.department?.toLowerCase()?.includes(normalizedKeyword) ||
      channel.owner?.toLowerCase()?.includes(normalizedKeyword)
    );
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockChannel = (payload: ChannelCreatePayload): Channel => {
  const now = dayjs().toISOString();
  const channelType = payload.channel_type ?? (payload.platform ? 'ONLINE' : 'OFFLINE');
  const channel: Channel = {
    channel_id: `CH-${Date.now()}`,
    tenant_id: mockTenantId,
    channel_type: channelType,
    platform: payload.platform ?? null,
    name: payload.name,
    contact_name: payload.contact_name ?? null,
    contact_phone: payload.contact_phone ?? null,
    contact_address: payload.contact_address ?? null,
    department: payload.department ?? null,
    owner: payload.owner ?? null,
    allow_manual_order: payload.allow_manual_order ?? false,
    allow_manual_after_sales: payload.allow_manual_after_sales ?? false,
    default_warehouse_id: payload.default_warehouse_id ?? null,
    commission_formula: payload.commission_formula ?? null,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  mockChannels.unshift(channel);
  return channel;
};

export const updateMockChannel = (
  channelId: string,
  payload: ChannelUpdatePayload
): Channel => {
  const now = dayjs().toISOString();
  const hasOwn = <T extends object>(key: keyof T) => Object.prototype.hasOwnProperty.call(payload, key);
  const index = mockChannels.findIndex((channel) => channel.channel_id === channelId);
  const base =
    index >= 0
      ? mockChannels[index]
      : ({
          channel_id: channelId,
          tenant_id: mockTenantId,
          channel_type: payload.channel_type ?? (payload.platform ? 'ONLINE' : 'OFFLINE'),
          platform: payload.platform ?? null,
          name: payload.name ?? '未命名渠道',
          contact_name: payload.contact_name ?? null,
          contact_phone: payload.contact_phone ?? null,
          contact_address: payload.contact_address ?? null,
          department: payload.department ?? null,
          owner: payload.owner ?? null,
          allow_manual_order: payload.allow_manual_order ?? false,
          allow_manual_after_sales: payload.allow_manual_after_sales ?? false,
          default_warehouse_id: payload.default_warehouse_id ?? null,
          commission_formula: payload.commission_formula ?? null,
          status: payload.status ?? 'ACTIVE',
          created_at: now
        } as Channel);
  const resolvedChannelType =
    payload.channel_type ?? base.channel_type ?? (hasOwn('platform') ? (payload.platform ? 'ONLINE' : 'OFFLINE') : base.platform ? 'ONLINE' : 'OFFLINE');
  const updated: Channel = {
    ...base,
    channel_type: resolvedChannelType,
    platform: hasOwn('platform') ? payload.platform ?? null : base.platform,
    name: payload.name ?? base.name,
    contact_name: hasOwn('contact_name') ? payload.contact_name ?? null : base.contact_name,
    contact_phone: hasOwn('contact_phone') ? payload.contact_phone ?? null : base.contact_phone,
    contact_address: hasOwn('contact_address')
      ? payload.contact_address ?? null
      : base.contact_address,
    department: payload.department ?? base.department,
    owner: payload.owner ?? base.owner,
    allow_manual_order: payload.allow_manual_order ?? base.allow_manual_order,
    allow_manual_after_sales: payload.allow_manual_after_sales ?? base.allow_manual_after_sales,
    default_warehouse_id: hasOwn('default_warehouse_id')
      ? payload.default_warehouse_id ?? null
      : base.default_warehouse_id,
    commission_formula: hasOwn('commission_formula')
      ? payload.commission_formula ?? null
      : base.commission_formula,
    status: payload.status ?? base.status,
    updated_at: now
  };
  if (index >= 0) {
    mockChannels[index] = updated;
  } else {
    mockChannels.unshift(updated);
  }
  return updated;
};

export const deleteMockChannel = (channelId: string): Channel => {
  const index = mockChannels.findIndex((channel) => channel.channel_id === channelId);
  if (index >= 0) {
    const [removed] = mockChannels.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    channel_id: channelId,
    tenant_id: mockTenantId,
    channel_type: 'OFFLINE',
    platform: null,
    name: '未命名渠道',
    contact_name: null,
    contact_phone: null,
    contact_address: null,
    department: null,
    owner: null,
    allow_manual_order: false,
    allow_manual_after_sales: false,
    default_warehouse_id: null,
    commission_formula: null,
    status: 'DISABLED',
    created_at: now,
    updated_at: now
  };
};

export const getMockSupplierList = (query: SupplierListQuery): SupplierListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockSuppliers.filter((supplier) => {
    if (!normalizedKeyword) {
      return true;
    }
    return [supplier.name, supplier.contact, supplier.settlement_terms, supplier.status]
      .map((field) => String(field ?? '').toLowerCase())
      .some((field) => field.includes(normalizedKeyword));
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockSupplier = (payload: SupplierCreatePayload): Supplier => {
  const now = dayjs().toISOString();
  const existingIndex = mockSuppliers.findIndex((supplier) => supplier.name === payload.name);
  if (existingIndex >= 0) {
    const updated = {
      ...mockSuppliers[existingIndex],
      contact: payload.contact ?? mockSuppliers[existingIndex].contact ?? null,
      settlement_terms:
        payload.settlement_terms ?? mockSuppliers[existingIndex].settlement_terms ?? null,
      status: payload.status ?? mockSuppliers[existingIndex].status ?? 'ACTIVE',
      updated_at: now
    };
    mockSuppliers[existingIndex] = updated;
    return updated;
  }
  const supplier: Supplier = {
    supplier_id: `SUP-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    contact: payload.contact ?? null,
    settlement_terms: payload.settlement_terms ?? null,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  mockSuppliers.unshift(supplier);
  return supplier;
};

export const updateMockSupplier = (
  supplierId: string,
  payload: SupplierUpdatePayload
): Supplier => {
  const now = dayjs().toISOString();
  const index = mockSuppliers.findIndex((supplier) => supplier.supplier_id === supplierId);
  const base =
    index >= 0
      ? mockSuppliers[index]
      : ({
          supplier_id: supplierId,
          tenant_id: mockTenantId,
          name: payload.name ?? '未命名',
          created_at: now
        } as Supplier);
  const updated: Supplier = {
    ...base,
    name: payload.name ?? base.name,
    contact: payload.contact ?? base.contact ?? null,
    settlement_terms: payload.settlement_terms ?? base.settlement_terms ?? null,
    status: payload.status ?? base.status ?? 'ACTIVE',
    updated_at: now
  };
  if (index >= 0) {
    mockSuppliers[index] = updated;
  } else {
    mockSuppliers.unshift(updated);
  }
  return updated;
};

export const deleteMockSupplier = (supplierId: string): Supplier => {
  const index = mockSuppliers.findIndex((supplier) => supplier.supplier_id === supplierId);
  if (index >= 0) {
    const [removed] = mockSuppliers.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    supplier_id: supplierId,
    tenant_id: mockTenantId,
    name: '未知供应商',
    created_at: now,
    updated_at: now
  };
};

export const getMockCustomerCategoryList = (
  query: CustomerCategoryListQuery
): CustomerCategoryListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = mockCustomerCategories.filter((category) => {
    if (
      normalizedKeyword &&
      ![category.name, category.description ?? '']
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(normalizedKeyword))
    ) {
      return false;
    }
    if (normalizedStatus && category.status !== normalizedStatus) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockCustomerCategory = (
  payload: CustomerCategoryCreatePayload
): CustomerCategory => {
  const now = dayjs().toISOString();
  const existingIndex = mockCustomerCategories.findIndex(
    (category) => category.name === payload.name
  );
  if (existingIndex >= 0) {
    const updated = {
      ...mockCustomerCategories[existingIndex],
      description: payload.description ?? mockCustomerCategories[existingIndex].description ?? null,
      status: payload.status ?? mockCustomerCategories[existingIndex].status ?? 'ACTIVE',
      updated_at: now
    };
    mockCustomerCategories[existingIndex] = updated;
    return updated;
  }
  const category: CustomerCategory = {
    category_id: `CAT-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    description: payload.description ?? null,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  mockCustomerCategories.unshift(category);
  return category;
};

export const updateMockCustomerCategory = (
  categoryId: string,
  payload: CustomerCategoryUpdatePayload
): CustomerCategory => {
  const now = dayjs().toISOString();
  const index = mockCustomerCategories.findIndex((category) => category.category_id === categoryId);
  const base =
    index >= 0
      ? mockCustomerCategories[index]
      : ({
          category_id: categoryId,
          tenant_id: mockTenantId,
          name: payload.name ?? categoryId,
          created_at: now
        } as CustomerCategory);
  const updated: CustomerCategory = {
    ...base,
    name: payload.name ?? base.name,
    description: payload.description ?? base.description ?? null,
    status: payload.status ?? base.status ?? 'ACTIVE',
    updated_at: now
  };
  if (index >= 0) {
    mockCustomerCategories[index] = updated;
  } else {
    mockCustomerCategories.unshift(updated);
  }
  return updated;
};

export const deleteMockCustomerCategory = (categoryId: string): CustomerCategory => {
  const index = mockCustomerCategories.findIndex((category) => category.category_id === categoryId);
  if (index >= 0) {
    const [removed] = mockCustomerCategories.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    category_id: categoryId,
    tenant_id: mockTenantId,
    name: '未分类',
    status: 'INACTIVE',
    created_at: now,
    updated_at: now
  };
};

export const getMockMemberLevelList = (query: MemberLevelListQuery): MemberLevelListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = mockMemberLevels.filter((level) => {
    if (
      normalizedKeyword &&
      ![level.name, level.rules ?? '']
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(normalizedKeyword))
    ) {
      return false;
    }
    if (normalizedStatus && level.status !== normalizedStatus) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockMemberLevel = (payload: MemberLevelCreatePayload): MemberLevel => {
  const now = dayjs().toISOString();
  const existingIndex = mockMemberLevels.findIndex((level) => level.name === payload.name);
  if (existingIndex >= 0) {
    const updated = {
      ...mockMemberLevels[existingIndex],
      level_rank: payload.level_rank ?? mockMemberLevels[existingIndex].level_rank ?? 0,
      discount_rate: payload.discount_rate ?? mockMemberLevels[existingIndex].discount_rate ?? null,
      rules: payload.rules ?? mockMemberLevels[existingIndex].rules ?? null,
      status: payload.status ?? mockMemberLevels[existingIndex].status ?? 'ACTIVE',
      updated_at: now
    };
    mockMemberLevels[existingIndex] = updated;
    return updated;
  }
  const level: MemberLevel = {
    level_id: `LEVEL-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    level_rank: payload.level_rank ?? 0,
    discount_rate: payload.discount_rate ?? null,
    rules: payload.rules ?? null,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  mockMemberLevels.unshift(level);
  return level;
};

export const updateMockMemberLevel = (
  levelId: string,
  payload: MemberLevelUpdatePayload
): MemberLevel => {
  const now = dayjs().toISOString();
  const index = mockMemberLevels.findIndex((level) => level.level_id === levelId);
  const base =
    index >= 0
      ? mockMemberLevels[index]
      : ({
          level_id: levelId,
          tenant_id: mockTenantId,
          name: payload.name ?? levelId,
          level_rank: payload.level_rank ?? 0,
          created_at: now
        } as MemberLevel);
  const updated: MemberLevel = {
    ...base,
    name: payload.name ?? base.name,
    level_rank: payload.level_rank ?? base.level_rank ?? 0,
    discount_rate: payload.discount_rate ?? base.discount_rate ?? null,
    rules: payload.rules ?? base.rules ?? null,
    status: payload.status ?? base.status ?? 'ACTIVE',
    updated_at: now
  };
  if (index >= 0) {
    mockMemberLevels[index] = updated;
  } else {
    mockMemberLevels.unshift(updated);
  }
  return updated;
};

export const deleteMockMemberLevel = (levelId: string): MemberLevel => {
  const index = mockMemberLevels.findIndex((level) => level.level_id === levelId);
  if (index >= 0) {
    const [removed] = mockMemberLevels.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    level_id: levelId,
    tenant_id: mockTenantId,
    name: '未设置',
    level_rank: 0,
    discount_rate: null,
    rules: null,
    status: 'INACTIVE',
    created_at: now,
    updated_at: now
  };
};

export const getMockCustomerTagList = (query: CustomerTagListQuery): CustomerTagListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockCustomerTags.filter((tag) => {
    if (!normalizedKeyword) {
      return true;
    }
    return [tag.name, tag.color ?? '']
      .map((value) => value.toLowerCase())
      .some((value) => value.includes(normalizedKeyword));
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockCustomerTag = (payload: CustomerTagCreatePayload): CustomerTag => {
  const now = dayjs().toISOString();
  const existingIndex = mockCustomerTags.findIndex((tag) => tag.name === payload.name);
  if (existingIndex >= 0) {
    const updated = {
      ...mockCustomerTags[existingIndex],
      color: payload.color ?? mockCustomerTags[existingIndex].color ?? null,
      updated_at: now
    };
    mockCustomerTags[existingIndex] = updated;
    return updated;
  }
  const tag: CustomerTag = {
    tag_id: `CTAG-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    color: payload.color ?? null,
    created_at: now,
    updated_at: now
  };
  mockCustomerTags.unshift(tag);
  return tag;
};

export const updateMockCustomerTag = (
  tagId: string,
  payload: CustomerTagUpdatePayload
): CustomerTag => {
  const now = dayjs().toISOString();
  const index = mockCustomerTags.findIndex((tag) => tag.tag_id === tagId);
  const base =
    index >= 0
      ? mockCustomerTags[index]
      : ({
          tag_id: tagId,
          tenant_id: mockTenantId,
          name: payload.name ?? tagId,
          created_at: now
        } as CustomerTag);
  const updated: CustomerTag = {
    ...base,
    name: payload.name ?? base.name,
    color: payload.color ?? base.color ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockCustomerTags[index] = updated;
  } else {
    mockCustomerTags.unshift(updated);
  }
  return updated;
};

export const deleteMockCustomerTag = (tagId: string): CustomerTag => {
  const index = mockCustomerTags.findIndex((tag) => tag.tag_id === tagId);
  if (index >= 0) {
    const [removed] = mockCustomerTags.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    tag_id: tagId,
    tenant_id: mockTenantId,
    name: '未命名标签',
    color: null,
    created_at: now,
    updated_at: now
  };
};

export const getMockCustomerList = (query: CustomerListQuery): CustomerListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = mockCustomers.filter((customer) => {
    if (normalizedKeyword) {
      const phone = customer.phone?.toLowerCase() ?? '';
      if (
        !customer.name.toLowerCase().includes(normalizedKeyword) &&
        !phone.includes(normalizedKeyword)
      ) {
        return false;
      }
    }
    if (normalizedStatus && customer.status !== normalizedStatus) {
      return false;
    }
    if (query.category_id && customer.category_id !== query.category_id) {
      return false;
    }
    if (query.member_level_id && customer.member_level_id !== query.member_level_id) {
      return false;
    }
    if (query.tag_id) {
      const tags = mockCustomerTagLinks.get(customer.customer_id) ?? [];
      if (!tags.includes(query.tag_id)) {
        return false;
      }
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const getMockCustomerDetail = (customerId: string): CustomerDetail => {
  const customer =
    mockCustomers.find((item) => item.customer_id === customerId) ?? buildCustomer(0);
  const tagIds = mockCustomerTagLinks.get(customer.customer_id) ?? [];
  const tags = tagIds
    .map((tagId) => mockCustomerTags.find((tag) => tag.tag_id === tagId))
    .filter(Boolean) as CustomerTag[];
  return {
    customer,
    tags
  };
};

export const createMockCustomer = (payload: CustomerCreatePayload): Customer => {
  const now = dayjs().toISOString();
  const existingIndex = mockCustomers.findIndex((customer) => customer.name === payload.name);
  const category = payload.category_id
    ? mockCustomerCategories.find((item) => item.category_id === payload.category_id)
    : undefined;
  const level = payload.member_level_id
    ? mockMemberLevels.find((item) => item.level_id === payload.member_level_id)
    : undefined;
  if (existingIndex >= 0) {
    const updated = {
      ...mockCustomers[existingIndex],
      phone: payload.phone ?? mockCustomers[existingIndex].phone ?? null,
      address: payload.address ?? mockCustomers[existingIndex].address ?? null,
      note: payload.note ?? mockCustomers[existingIndex].note ?? null,
      status: payload.status ?? mockCustomers[existingIndex].status ?? 'ACTIVE',
      category_id: payload.category_id ?? mockCustomers[existingIndex].category_id ?? null,
      category_name: category?.name ?? mockCustomers[existingIndex].category_name ?? null,
      member_level_id:
        payload.member_level_id ?? mockCustomers[existingIndex].member_level_id ?? null,
      member_level_name: level?.name ?? mockCustomers[existingIndex].member_level_name ?? null,
      updated_at: now
    };
    mockCustomers[existingIndex] = updated;
    return updated;
  }
  const customer: Customer = {
    customer_id: `CUST-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    phone: payload.phone ?? null,
    address: payload.address ?? null,
    note: payload.note ?? null,
    status: payload.status ?? 'ACTIVE',
    category_id: payload.category_id ?? null,
    category_name: category?.name ?? null,
    member_level_id: payload.member_level_id ?? null,
    member_level_name: level?.name ?? null,
    created_at: now,
    updated_at: now
  };
  mockCustomers.unshift(customer);
  mockCustomerTagLinks.set(customer.customer_id, []);
  return customer;
};

export const updateMockCustomer = (
  customerId: string,
  payload: CustomerUpdatePayload
): Customer => {
  const now = dayjs().toISOString();
  const index = mockCustomers.findIndex((customer) => customer.customer_id === customerId);
  const base =
    index >= 0
      ? mockCustomers[index]
      : ({
          customer_id: customerId,
          tenant_id: mockTenantId,
          name: payload.name ?? customerId,
          created_at: now
        } as Customer);
  const category = payload.category_id
    ? mockCustomerCategories.find((item) => item.category_id === payload.category_id)
    : undefined;
  const level = payload.member_level_id
    ? mockMemberLevels.find((item) => item.level_id === payload.member_level_id)
    : undefined;
  const updated: Customer = {
    ...base,
    name: payload.name ?? base.name,
    phone: payload.phone ?? base.phone ?? null,
    address: payload.address ?? base.address ?? null,
    note: payload.note ?? base.note ?? null,
    status: payload.status ?? base.status ?? 'ACTIVE',
    category_id: payload.category_id ?? base.category_id ?? null,
    category_name: category?.name ?? base.category_name ?? null,
    member_level_id: payload.member_level_id ?? base.member_level_id ?? null,
    member_level_name: level?.name ?? base.member_level_name ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockCustomers[index] = updated;
  } else {
    mockCustomers.unshift(updated);
  }
  return updated;
};

export const deleteMockCustomer = (customerId: string): Customer => {
  const index = mockCustomers.findIndex((customer) => customer.customer_id === customerId);
  if (index >= 0) {
    const [removed] = mockCustomers.splice(index, 1);
    mockCustomerTagLinks.delete(customerId);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    customer_id: customerId,
    tenant_id: mockTenantId,
    name: '未知客户',
    status: 'INACTIVE',
    created_at: now,
    updated_at: now
  };
};

export const replaceMockCustomerTags = (
  customerId: string,
  payload: CustomerTagReplacePayload
): CustomerTag[] => {
  const tagIds = payload.tag_ids ?? [];
  mockCustomerTagLinks.set(customerId, [...tagIds]);
  return tagIds
    .map((tagId) => mockCustomerTags.find((tag) => tag.tag_id === tagId))
    .filter(Boolean) as CustomerTag[];
};

export const getMockShopList = (query: ShopListQuery): ShopListResponse => {
  const normalizedPlatform = query.platform?.trim().toLowerCase();
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = mockShops.filter((shop) => {
    if (normalizedPlatform && shop.platform !== normalizedPlatform) {
      return false;
    }
    if (normalizedStatus && shop.status !== normalizedStatus) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const getMockShop = (shopId: string): Shop => {
  const shop = mockShops.find((item) => item.shop_id === shopId);
  if (shop) {
    return shop;
  }
  const now = dayjs().toISOString();
  return {
    shop_id: shopId,
    tenant_id: mockTenantId,
    platform: Platform.Taobao,
    platform_shop_id: `PLAT-TAOBAO-${shopId}`,
    name: '??a??￥?o—é“o',
    status: 'AUTHORIZED',
    auth_meta_json: {},
    created_at: now,
    updated_at: now,
    token_status: buildShopTokenStatus(1)
  };
};

export const getMockShopAuthStatus = (shopId: string): Shop => getMockShop(shopId);

export const createMockShop = (payload: ShopCreatePayload): Shop => {
  const now = dayjs().toISOString();
  const existingIndex = mockShops.findIndex(
    (shop) => shop.platform_shop_id === payload.platform_shop_id
  );
  if (existingIndex >= 0) {
    const updated = {
      ...mockShops[existingIndex],
      name: payload.name,
      status: payload.status ?? mockShops[existingIndex].status,
      updated_at: now
    };
    mockShops[existingIndex] = updated;
    return updated;
  }
  const shop: Shop = {
    shop_id: `SHOP-${Date.now()}`,
    tenant_id: mockTenantId,
    platform: payload.platform,
    platform_shop_id: payload.platform_shop_id,
    name: payload.name,
    status: payload.status ?? 'AUTHORIZED',
    auth_meta_json: payload.auth_meta_json ?? {},
    created_at: now,
    updated_at: now,
    token_status: null
  };
  mockShops.unshift(shop);
  return shop;
};

export const updateMockShop = (shopId: string, payload: ShopUpdatePayload): Shop => {
  const now = dayjs().toISOString();
  const index = mockShops.findIndex((shop) => shop.shop_id === shopId);
  const base =
    index >= 0
      ? mockShops[index]
      : ({
          shop_id: shopId,
          tenant_id: mockTenantId,
          platform: Platform.Taobao,
          platform_shop_id: `PLAT-TAOBAO-${shopId}`,
          name: payload.name ?? '??a?‘????',
          status: payload.status ?? 'AUTHORIZED',
          auth_meta_json: {},
          created_at: now
        } as Shop);
  const updated: Shop = {
    ...base,
    name: payload.name ?? base.name,
    status: payload.status ?? base.status,
    auth_meta_json: payload.auth_meta_json ?? base.auth_meta_json,
    updated_at: now
  };
  if (index >= 0) {
    mockShops[index] = updated;
  } else {
    mockShops.unshift(updated);
  }
  return updated;
};

export const deleteMockShop = (shopId: string): Shop => {
  const updated = updateMockShop(shopId, { status: 'DISABLED' });
  return updated;
};

export const startMockShopAuthorize = (
  shopId: string,
  _payload: ShopAuthorizePayload
): ShopAuthorizeResponse => {
  void _payload;
  return {
    authorize_url: `https://mock.auth/${shopId}`,
    state: `state-${Date.now()}`
  };
};

export const submitMockShopAuth = (shopId: string, payload: ShopAuthCallbackPayload): Shop => {
  const hasError = Boolean(payload.error?.trim());
  const derivedAccessToken =
    payload.access_token ?? (payload.code ? `mock-access-${payload.code}` : 'token');
  const derivedRefreshToken =
    payload.refresh_token ?? (payload.code ? `mock-refresh-${payload.code}` : 'refresh');
  const expiresAt = payload.expires_at ?? dayjs().add(2, 'hour').toISOString();
  const now = dayjs();
  const tokenStatus: NonNullable<Shop['token_status']> = {
    has_access_token: !hasError && Boolean(derivedAccessToken),
    has_refresh_token: !hasError && Boolean(derivedRefreshToken),
    expires_at: expiresAt,
    expires_in_minutes: hasError ? 0 : dayjs(expiresAt).diff(now, 'minute'),
    is_expired: hasError || dayjs(expiresAt).isBefore(now),
    is_expiring_soon: !hasError && dayjs(expiresAt).isBefore(now.add(30, 'minute')),
    scopes: payload.scopes ?? 'orders,shipping',
    last_refresh_at: now.toISOString(),
    last_error: payload.error ?? '',
    updated_at: now.toISOString()
  };
  const updated = updateMockShop(shopId, {
    status: hasError ? 'EXPIRED' : 'AUTHORIZED',
    auth_meta_json: {
      oauth_status: hasError ? 'FAILED' : 'SUCCESS',
      oauth_state: payload.state ?? null,
      oauth_error: payload.error ?? null,
      oauth_error_description: payload.error_description ?? null,
      oauth_updated_at: now.toISOString()
    }
  });
  updated.token_status = tokenStatus;
  return updated;
};

export const requestMockTokenRefresh = (shopId: string): TokenRefreshRequestResponse => {
  const shop = mockShops.find((item) => item.shop_id === shopId);
  if (shop?.token_status) {
    shop.token_status = {
      ...shop.token_status,
      last_error: '',
      updated_at: dayjs().toISOString()
    };
  }
  return { outbox_id: `OUT-${Date.now()}` };
};

export const getMockSalesShopConfigList = (
  query: SalesShopConfigListQuery
): SalesShopConfigListResponse => {
  const normalizedPlatform = query.platform?.trim().toLowerCase();
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = mockShops.filter((shop) => {
    if (query.shop_id && shop.shop_id !== query.shop_id) {
      return false;
    }
    if (normalizedPlatform && shop.platform !== normalizedPlatform) {
      return false;
    }
    if (normalizedStatus && shop.status !== normalizedStatus) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit).map((shop) =>
    buildSalesShopConfigView(shop)
  );
  return {
    items,
    total: filtered.length
  };
};

export const getMockSalesShopConfig = (shopId: string): SalesShopConfig => {
  const shop = getMockShop(shopId);
  return buildSalesShopConfigView(shop);
};

export const updateMockSalesShopConfig = (
  shopId: string,
  payload: SalesShopConfigRequest
): SalesShopConfig => {
  const shop = getMockShop(shopId);
  const existing = buildSalesShopConfigView(shop);
  const updated: SalesShopConfig = {
    ...existing,
    ...payload,
    shop_id: shop.shop_id,
    platform: shop.platform,
    platform_shop_id: shop.platform_shop_id,
    name: shop.name,
    status: shop.status
  };
  mockSalesShopConfigs.set(shop.shop_id, updated);
  return updated;
};

export const getMockSyncJobList = (query: SyncJobListQuery): SyncJobListResponse => {
  const normalizedStatus = query.status?.trim().toUpperCase();
  const filtered = mockSyncJobs.filter((job) => {
    if (query.shop_id && job.shop_id !== query.shop_id) {
      return false;
    }
    if (query.job_type && job.job_type !== query.job_type) {
      return false;
    }
    if (normalizedStatus && job.status !== normalizedStatus) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const runMockSyncJob = (shopId: string, payload: SyncJobRunPayload): SyncJob => {
  const now = dayjs().toISOString();
  const index = mockSyncJobs.findIndex(
    (job) => job.shop_id === shopId && job.job_type === payload.job_type
  );
  if (index >= 0) {
    const updated = {
      ...mockSyncJobs[index],
      cursor: payload.cursor ?? mockSyncJobs[index].cursor ?? null,
      last_time: payload.last_time ?? mockSyncJobs[index].last_time ?? null,
      status: 'PENDING',
      last_error: '',
      updated_at: now
    };
    mockSyncJobs[index] = updated;
    return updated;
  }
  const job: SyncJob = {
    job_id: `SYNC-${shopId}-${payload.job_type}-${Date.now()}`,
    tenant_id: mockTenantId,
    shop_id: shopId,
    job_type: payload.job_type,
    cursor: payload.cursor ?? null,
    last_time: payload.last_time ?? null,
    status: 'PENDING',
    last_error: '',
    created_at: now,
    updated_at: now
  };
  mockSyncJobs.unshift(job);
  return job;
};

export const getMockSyncJobRunList = (
  query: SyncJobRunListQuery
): SyncJobRunListResponse => {
  const normalizedStatus = query.status?.trim().toUpperCase();
  const startedFrom = query.started_from ? dayjs(query.started_from) : null;
  const startedTo = query.started_to ? dayjs(query.started_to) : null;
  const filtered = mockSyncJobRuns.filter((run) => {
    if (query.job_id && run.job_id !== query.job_id) {
      return false;
    }
    if (query.shop_id && run.shop_id !== query.shop_id) {
      return false;
    }
    if (query.job_type && run.job_type !== query.job_type) {
      return false;
    }
    if (normalizedStatus && run.status !== normalizedStatus) {
      return false;
    }
    const startedAt = dayjs(run.started_at);
    if (startedFrom?.isValid() && startedAt.isBefore(startedFrom)) {
      return false;
    }
    if (startedTo?.isValid() && startedAt.isAfter(startedTo)) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};
