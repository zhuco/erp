import dayjs from 'dayjs';

import type {
  ProductBarcode,
  ProductBarcodeCreatePayload,
  ProductBarcodeListQuery,
  ProductBarcodeListResponse,
  ProductBarcodeUpdatePayload
} from '../productBarcodes';
import type {
  ProductBrand,
  ProductBrandCreatePayload,
  ProductBrandListQuery,
  ProductBrandListResponse,
  ProductBrandUpdatePayload
} from '../productBrands';
import type {
  ProductCategory,
  ProductCategoryCreatePayload,
  ProductCategoryListQuery,
  ProductCategoryListResponse,
  ProductCategoryUpdatePayload
} from '../productCategories';
import type {
  ProductChannelAvailability,
  ProductChannelAvailabilityCreatePayload,
  ProductChannelAvailabilityListQuery,
  ProductChannelAvailabilityListResponse,
  ProductChannelAvailabilityUpdatePayload
} from '../productChannelAvailability';
import type {
  ProductChangeRequest,
  ProductChangeRequestCreatePayload,
  ProductChangeRequestListQuery,
  ProductChangeRequestListResponse,
  ProductChangeRequestUpdatePayload
} from '../productChangeRequests';
import type {
  ProductLifecycle,
  ProductLifecycleCreatePayload,
  ProductLifecycleListQuery,
  ProductLifecycleListResponse,
  ProductLifecycleUpdatePayload
} from '../productLifecycles';
import type {
  ProductUnitConversion,
  ProductUnitConversionCreatePayload,
  ProductUnitConversionListQuery,
  ProductUnitConversionListResponse,
  ProductUnitConversionUpdatePayload
} from '../productUnitConversions';
import type {
  ProductUnit,
  ProductUnitCreatePayload,
  ProductUnitListQuery,
  ProductUnitListResponse,
  ProductUnitUpdatePayload
} from '../productUnits';
import type {
  ProductWarehouse,
  ProductWarehouseCreatePayload,
  ProductWarehouseListResponse,
  ProductWarehouseUpdatePayload
} from '../productWarehouses';

import {
  applyPagination,
  mockChannels,
  mockProductBrands,
  mockProductCategories,
  mockProductUnits,
  mockProductWarehouses,
  mockSkus,
  mockTenantId,
  mockWarehouses
} from './data';

const resolveSku = (skuId: string) =>
  mockSkus.find((sku) => sku.sku_id === skuId) ?? mockSkus[0];

const resolveChannel = (channelId?: string | null) =>
  mockChannels.find((channel) => channel.channel_id === channelId);

const resolveWarehouse = (warehouseId?: string | null) =>
  mockWarehouses.find((warehouse) => warehouse.warehouse_id === warehouseId);

export const getMockProductCategoryList = (
  query: ProductCategoryListQuery
): ProductCategoryListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockProductCategories.filter((category) => {
    if (!normalizedKeyword) {
      return true;
    }
    return category.name.toLowerCase().includes(normalizedKeyword);
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const createMockProductCategory = (
  payload: ProductCategoryCreatePayload
): ProductCategory => {
  const now = dayjs().toISOString();
  const existing = mockProductCategories.find((item) => item.name === payload.name);
  if (existing) {
    existing.updated_at = now;
    return existing;
  }
  const record: ProductCategory = {
    category_id: `PCAT-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    created_at: now,
    updated_at: now
  };
  mockProductCategories.unshift(record);
  return record;
};

export const updateMockProductCategory = (
  categoryId: string,
  payload: ProductCategoryUpdatePayload
): ProductCategory => {
  const now = dayjs().toISOString();
  const index = mockProductCategories.findIndex((item) => item.category_id === categoryId);
  const base =
    index >= 0
      ? mockProductCategories[index]
      : {
          category_id: categoryId,
          tenant_id: mockTenantId,
          name: payload.name ?? categoryId,
          created_at: now,
          updated_at: now
        };
  const updated: ProductCategory = {
    ...base,
    name: payload.name ?? base.name,
    updated_at: now
  };
  if (index >= 0) {
    mockProductCategories[index] = updated;
  } else {
    mockProductCategories.unshift(updated);
  }
  return updated;
};

export const deleteMockProductCategory = (categoryId: string): ProductCategory => {
  const index = mockProductCategories.findIndex((item) => item.category_id === categoryId);
  if (index >= 0) {
    const [removed] = mockProductCategories.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    category_id: categoryId,
    tenant_id: mockTenantId,
    name: '未命名分类',
    created_at: now,
    updated_at: now
  };
};

export const getMockProductBrandList = (
  query: ProductBrandListQuery
): ProductBrandListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockProductBrands.filter((brand) => {
    if (!normalizedKeyword) {
      return true;
    }
    return brand.name.toLowerCase().includes(normalizedKeyword);
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const createMockProductBrand = (payload: ProductBrandCreatePayload): ProductBrand => {
  const now = dayjs().toISOString();
  const existing = mockProductBrands.find((item) => item.name === payload.name);
  if (existing) {
    existing.updated_at = now;
    return existing;
  }
  const record: ProductBrand = {
    brand_id: `PBR-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    created_at: now,
    updated_at: now
  };
  mockProductBrands.unshift(record);
  return record;
};

export const updateMockProductBrand = (
  brandId: string,
  payload: ProductBrandUpdatePayload
): ProductBrand => {
  const now = dayjs().toISOString();
  const index = mockProductBrands.findIndex((item) => item.brand_id === brandId);
  const base =
    index >= 0
      ? mockProductBrands[index]
      : {
          brand_id: brandId,
          tenant_id: mockTenantId,
          name: payload.name ?? brandId,
          created_at: now,
          updated_at: now
        };
  const updated: ProductBrand = {
    ...base,
    name: payload.name ?? base.name,
    updated_at: now
  };
  if (index >= 0) {
    mockProductBrands[index] = updated;
  } else {
    mockProductBrands.unshift(updated);
  }
  return updated;
};

export const deleteMockProductBrand = (brandId: string): ProductBrand => {
  const index = mockProductBrands.findIndex((item) => item.brand_id === brandId);
  if (index >= 0) {
    const [removed] = mockProductBrands.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    brand_id: brandId,
    tenant_id: mockTenantId,
    name: '未命名品牌',
    created_at: now,
    updated_at: now
  };
};

export const getMockProductUnitList = (
  query: ProductUnitListQuery
): ProductUnitListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockProductUnits.filter((unit) => {
    if (!normalizedKeyword) {
      return true;
    }
    return unit.name.toLowerCase().includes(normalizedKeyword);
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const createMockProductUnit = (payload: ProductUnitCreatePayload): ProductUnit => {
  const now = dayjs().toISOString();
  const existing = mockProductUnits.find((item) => item.name === payload.name);
  if (existing) {
    existing.updated_at = now;
    return existing;
  }
  const record: ProductUnit = {
    unit_id: `PUNIT-${Date.now()}`,
    tenant_id: mockTenantId,
    name: payload.name,
    created_at: now,
    updated_at: now
  };
  mockProductUnits.unshift(record);
  return record;
};

export const updateMockProductUnit = (
  unitId: string,
  payload: ProductUnitUpdatePayload
): ProductUnit => {
  const now = dayjs().toISOString();
  const index = mockProductUnits.findIndex((item) => item.unit_id === unitId);
  const base =
    index >= 0
      ? mockProductUnits[index]
      : {
          unit_id: unitId,
          tenant_id: mockTenantId,
          name: payload.name ?? unitId,
          created_at: now,
          updated_at: now
        };
  const updated: ProductUnit = {
    ...base,
    name: payload.name ?? base.name,
    updated_at: now
  };
  if (index >= 0) {
    mockProductUnits[index] = updated;
  } else {
    mockProductUnits.unshift(updated);
  }
  return updated;
};

export const getMockProductWarehouseList = (
  productId: string
): ProductWarehouseListResponse => ({
  items: mockProductWarehouses.filter((item) => item.product_id === productId)
});

export const createMockProductWarehouse = (
  productId: string,
  payload: ProductWarehouseCreatePayload
): ProductWarehouse => {
  const now = dayjs().toISOString();
  const warehouse = resolveWarehouse(payload.warehouse_id);
  if (!warehouse) {
    throw new Error('warehouse not found');
  }
  if (payload.is_default) {
    mockProductWarehouses.forEach((item) => {
      if (item.product_id === productId) {
        item.is_default = false;
        item.updated_at = now;
      }
    });
  }
  const record: ProductWarehouse = {
    tenant_id: mockTenantId,
    product_id: productId,
    warehouse_id: warehouse.warehouse_id,
    warehouse_code: warehouse.code,
    warehouse_name: warehouse.name,
    is_default: payload.is_default ?? false,
    allow_negative_stock: payload.allow_negative_stock ?? false,
    created_at: now,
    updated_at: now
  };
  mockProductWarehouses.unshift(record);
  return record;
};

export const updateMockProductWarehouse = (
  productId: string,
  warehouseId: string,
  payload: ProductWarehouseUpdatePayload
): ProductWarehouse => {
  const now = dayjs().toISOString();
  const index = mockProductWarehouses.findIndex(
    (item) => item.product_id === productId && item.warehouse_id === warehouseId
  );
  const base =
    index >= 0
      ? mockProductWarehouses[index]
      : {
          tenant_id: mockTenantId,
          product_id: productId,
          warehouse_id: warehouseId,
          warehouse_code: warehouseId,
          warehouse_name: warehouseId,
          is_default: false,
          allow_negative_stock: false,
          created_at: now,
          updated_at: now
        };
  if (payload.is_default) {
    mockProductWarehouses.forEach((item) => {
      if (item.product_id === productId) {
        item.is_default = false;
        item.updated_at = now;
      }
    });
  }
  const updated: ProductWarehouse = {
    ...base,
    is_default: payload.is_default ?? base.is_default,
    allow_negative_stock: payload.allow_negative_stock ?? base.allow_negative_stock,
    updated_at: now
  };
  if (index >= 0) {
    mockProductWarehouses[index] = updated;
  } else {
    mockProductWarehouses.unshift(updated);
  }
  return updated;
};

export const deleteMockProductWarehouse = (
  productId: string,
  warehouseId: string
): ProductWarehouse => {
  const index = mockProductWarehouses.findIndex(
    (item) => item.product_id === productId && item.warehouse_id === warehouseId
  );
  if (index >= 0) {
    const [removed] = mockProductWarehouses.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    tenant_id: mockTenantId,
    product_id: productId,
    warehouse_id: warehouseId,
    warehouse_code: warehouseId,
    warehouse_name: warehouseId,
    is_default: false,
    allow_negative_stock: false,
    created_at: now,
    updated_at: now
  };
};

const mockProductBarcodes: ProductBarcode[] = [
  {
    barcode_id: 'PB-001',
    tenant_id: mockTenantId,
    sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
    sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
    sku_name: mockSkus[0]?.name ?? null,
    barcode: '6921001000012',
    level: '单品',
    unit: '件',
    ratio: 1,
    is_default: true,
    status: 'ACTIVE',
    print_template: '标准热敏',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    barcode_id: 'PB-002',
    tenant_id: mockTenantId,
    sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
    sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
    sku_name: mockSkus[0]?.name ?? null,
    barcode: '6921001000029',
    level: '内盒',
    unit: '盒',
    ratio: 6,
    is_default: false,
    status: 'ACTIVE',
    print_template: '仓配联动模板',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    barcode_id: 'PB-003',
    tenant_id: mockTenantId,
    sku_id: mockSkus[1]?.sku_id ?? 'SKU-2',
    sku_code: mockSkus[1]?.sku_code ?? 'SKU-2',
    sku_name: mockSkus[1]?.name ?? null,
    barcode: '6938007000134',
    level: '单品',
    unit: '件',
    ratio: 1,
    is_default: true,
    status: 'ACTIVE',
    print_template: '标准热敏',
    created_at: dayjs().subtract(4, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    barcode_id: 'PB-004',
    tenant_id: mockTenantId,
    sku_id: mockSkus[2]?.sku_id ?? 'SKU-3',
    sku_code: mockSkus[2]?.sku_code ?? 'SKU-3',
    sku_name: mockSkus[2]?.name ?? null,
    barcode: '6949005000233',
    level: '外箱',
    unit: '箱',
    ratio: 12,
    is_default: false,
    status: 'INACTIVE',
    print_template: '批次条码模板',
    created_at: dayjs().subtract(6, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'day').toISOString()
  }
];

const mockProductUnitConversions: ProductUnitConversion[] = [
  {
    conversion_id: 'UC-001',
    tenant_id: mockTenantId,
    sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
    sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
    sku_name: mockSkus[0]?.name ?? null,
    base_unit: '件',
    aux_unit: '盒',
    ratio: 6,
    rounding: 'ROUND',
    scenario: '采购',
    status: 'ACTIVE',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    conversion_id: 'UC-002',
    tenant_id: mockTenantId,
    sku_id: mockSkus[1]?.sku_id ?? 'SKU-2',
    sku_code: mockSkus[1]?.sku_code ?? 'SKU-2',
    sku_name: mockSkus[1]?.name ?? null,
    base_unit: '件',
    aux_unit: '袋',
    ratio: 10,
    rounding: 'CEIL',
    scenario: '仓储',
    status: 'ACTIVE',
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    conversion_id: 'UC-003',
    tenant_id: mockTenantId,
    sku_id: mockSkus[2]?.sku_id ?? 'SKU-3',
    sku_code: mockSkus[2]?.sku_code ?? 'SKU-3',
    sku_name: mockSkus[2]?.name ?? null,
    base_unit: '件',
    aux_unit: '箱',
    ratio: 12,
    rounding: 'FLOOR',
    scenario: '销售',
    status: 'INACTIVE',
    created_at: dayjs().subtract(4, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  }
];

const mockProductChannelAvailability: ProductChannelAvailability[] = [
  {
    availability_id: 'CA-001',
    tenant_id: mockTenantId,
    sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
    sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
    sku_name: mockSkus[0]?.name ?? null,
    channel_id: mockChannels[0]?.channel_id ?? null,
    channel_platform: mockChannels[0]?.platform ?? null,
    warehouse_id: mockWarehouses[0]?.warehouse_id ?? null,
    warehouse_name: mockWarehouses[0]?.name ?? null,
    status: 'ON',
    available_qty: 520,
    rule: '库存<20自动下架',
    on_shelf_at: dayjs().subtract(3, 'day').toISOString(),
    created_at: dayjs().subtract(4, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    availability_id: 'CA-002',
    tenant_id: mockTenantId,
    sku_id: mockSkus[1]?.sku_id ?? 'SKU-2',
    sku_code: mockSkus[1]?.sku_code ?? 'SKU-2',
    sku_name: mockSkus[1]?.name ?? null,
    channel_id: mockChannels[1]?.channel_id ?? null,
    channel_platform: mockChannels[1]?.platform ?? null,
    warehouse_id: mockWarehouses[1]?.warehouse_id ?? null,
    warehouse_name: mockWarehouses[1]?.name ?? null,
    status: 'ON',
    available_qty: 120,
    rule: '新品预售到货后上架',
    on_shelf_at: dayjs().subtract(1, 'day').toISOString(),
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    availability_id: 'CA-003',
    tenant_id: mockTenantId,
    sku_id: mockSkus[2]?.sku_id ?? 'SKU-3',
    sku_code: mockSkus[2]?.sku_code ?? 'SKU-3',
    sku_name: mockSkus[2]?.name ?? null,
    channel_id: mockChannels[2]?.channel_id ?? null,
    channel_platform: mockChannels[2]?.platform ?? null,
    warehouse_id: mockWarehouses[2]?.warehouse_id ?? null,
    warehouse_name: mockWarehouses[2]?.name ?? null,
    status: 'OFF',
    available_qty: 0,
    rule: '季节性下架',
    on_shelf_at: dayjs().subtract(7, 'day').toISOString(),
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'day').toISOString()
  }
];

const mockProductChangeRequests: ProductChangeRequest[] = [
  {
    request_id: 'CR-001',
    tenant_id: mockTenantId,
    sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
    sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
    sku_name: mockSkus[0]?.name ?? null,
    title: '新增批次条码',
    change_type: '规格条码',
    status: 'REVIEWING',
    applicant: '王蕾',
    approver: '陈涛',
    reason: '仓配中心要求增加批次识别',
    history: [
      {
        time: dayjs().subtract(2, 'day').toISOString(),
        action: '提交申请',
        actor: '王蕾'
      },
      {
        time: dayjs().subtract(1, 'day').toISOString(),
        action: '进入审批',
        actor: '陈涛',
        note: '等待补充包装图示'
      }
    ],
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    request_id: 'CR-002',
    tenant_id: mockTenantId,
    sku_id: mockSkus[1]?.sku_id ?? 'SKU-2',
    sku_code: mockSkus[1]?.sku_code ?? 'SKU-2',
    sku_name: mockSkus[1]?.name ?? null,
    title: '调整尺码属性字段',
    change_type: '基础信息',
    status: 'APPROVED',
    applicant: '刘茜',
    approver: '王珊',
    reason: '增加夏季尺码分组',
    history: [
      {
        time: dayjs().subtract(4, 'day').toISOString(),
        action: '提交申请',
        actor: '刘茜'
      },
      {
        time: dayjs().subtract(2, 'day').toISOString(),
        action: '审批通过',
        actor: '王珊'
      }
    ],
    created_at: dayjs().subtract(4, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    request_id: 'CR-003',
    tenant_id: mockTenantId,
    sku_id: mockSkus[2]?.sku_id ?? 'SKU-3',
    sku_code: mockSkus[2]?.sku_code ?? 'SKU-3',
    sku_name: mockSkus[2]?.name ?? null,
    title: '下架冬季渠道配置',
    change_type: '渠道配置',
    status: 'DRAFT',
    applicant: '赵鹏',
    approver: null,
    reason: '季节性调整',
    history: [
      {
        time: dayjs().subtract(1, 'day').toISOString(),
        action: '保存草稿',
        actor: '赵鹏'
      }
    ],
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockProductLifecycles: ProductLifecycle[] = [
  {
    lifecycle_id: 'LC-001',
    tenant_id: mockTenantId,
    sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
    sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
    sku_name: mockSkus[0]?.name ?? null,
    status: 'ACTIVE',
    inventory_qty: 320,
    remark: '新品上架',
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    lifecycle_id: 'LC-002',
    tenant_id: mockTenantId,
    sku_id: mockSkus[1]?.sku_id ?? 'SKU-2',
    sku_code: mockSkus[1]?.sku_code ?? 'SKU-2',
    sku_name: mockSkus[1]?.name ?? null,
    status: 'DISABLED',
    inventory_qty: 0,
    remark: '季节性停用',
    created_at: dayjs().subtract(4, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  },
  {
    lifecycle_id: 'LC-003',
    tenant_id: mockTenantId,
    sku_id: mockSkus[2]?.sku_id ?? 'SKU-3',
    sku_code: mockSkus[2]?.sku_code ?? 'SKU-3',
    sku_name: mockSkus[2]?.name ?? null,
    status: 'ARCHIVED',
    inventory_qty: 0,
    remark: '归档待删除',
    created_at: dayjs().subtract(6, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'day').toISOString()
  }
];

export const getMockProductBarcodeList = (
  query: ProductBarcodeListQuery
): ProductBarcodeListResponse => {
  const filtered = mockProductBarcodes.filter((barcode) => {
    if (query.sku_id && barcode.sku_id !== query.sku_id) {
      return false;
    }
    if (query.barcode && barcode.barcode !== query.barcode) {
      return false;
    }
    if (query.status && barcode.status !== query.status) {
      return false;
    }
    if (query.level && barcode.level !== query.level) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const getMockProductBarcode = (barcodeId: string): ProductBarcode =>
  mockProductBarcodes.find((barcode) => barcode.barcode_id === barcodeId) ?? {
    barcode_id: barcodeId,
    tenant_id: mockTenantId,
    sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
    sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
    sku_name: mockSkus[0]?.name ?? null,
    barcode: 'UNKNOWN',
    level: '单品',
    unit: '件',
    ratio: 1,
    is_default: false,
    status: 'ACTIVE',
    print_template: null,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };

export const createMockProductBarcode = (
  payload: ProductBarcodeCreatePayload
): ProductBarcode => {
  const now = dayjs().toISOString();
  const sku = resolveSku(payload.sku_id);
  const record: ProductBarcode = {
    barcode_id: `PB-${Date.now()}`,
    tenant_id: mockTenantId,
    sku_id: payload.sku_id,
    sku_code: sku?.sku_code ?? payload.sku_id,
    sku_name: sku?.name ?? null,
    barcode: payload.barcode,
    level: payload.level,
    unit: payload.unit,
    ratio: payload.ratio ?? 1,
    is_default: payload.is_default ?? false,
    status: payload.status ?? 'ACTIVE',
    print_template: payload.print_template ?? null,
    created_at: now,
    updated_at: now
  };
  if (record.is_default) {
    mockProductBarcodes.forEach((item) => {
      if (item.sku_id === record.sku_id) {
        item.is_default = false;
        item.updated_at = now;
      }
    });
  }
  mockProductBarcodes.unshift(record);
  return record;
};

export const updateMockProductBarcode = (
  barcodeId: string,
  payload: ProductBarcodeUpdatePayload
): ProductBarcode => {
  const now = dayjs().toISOString();
  const index = mockProductBarcodes.findIndex((barcode) => barcode.barcode_id === barcodeId);
  const base =
    index >= 0
      ? mockProductBarcodes[index]
      : ({
          barcode_id: barcodeId,
          tenant_id: mockTenantId,
          sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
          sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
          sku_name: mockSkus[0]?.name ?? null,
          barcode: payload.barcode ?? 'UNKNOWN',
          level: payload.level ?? '单品',
          unit: payload.unit ?? '件',
          ratio: payload.ratio ?? 1,
          is_default: payload.is_default ?? false,
          status: payload.status ?? 'ACTIVE',
          print_template: payload.print_template ?? null,
          created_at: now,
          updated_at: now
        } as ProductBarcode);
  const updated: ProductBarcode = {
    ...base,
    barcode: payload.barcode ?? base.barcode,
    level: payload.level ?? base.level,
    unit: payload.unit ?? base.unit,
    ratio: payload.ratio ?? base.ratio,
    is_default: payload.is_default ?? base.is_default,
    status: payload.status ?? base.status,
    print_template:
      payload.print_template !== undefined ? payload.print_template : base.print_template,
    updated_at: now
  };
  if (updated.is_default) {
    mockProductBarcodes.forEach((item) => {
      if (item.sku_id === updated.sku_id) {
        item.is_default = item.barcode_id === updated.barcode_id;
        item.updated_at = now;
      }
    });
  }
  if (index >= 0) {
    mockProductBarcodes[index] = updated;
  } else {
    mockProductBarcodes.unshift(updated);
  }
  return updated;
};

export const deleteMockProductBarcode = (barcodeId: string): ProductBarcode => {
  const index = mockProductBarcodes.findIndex((barcode) => barcode.barcode_id === barcodeId);
  if (index >= 0) {
    const [removed] = mockProductBarcodes.splice(index, 1);
    return removed;
  }
  return getMockProductBarcode(barcodeId);
};

export const getMockProductUnitConversionList = (
  query: ProductUnitConversionListQuery
): ProductUnitConversionListResponse => {
  const filtered = mockProductUnitConversions.filter((conversion) => {
    if (query.sku_id && conversion.sku_id !== query.sku_id) {
      return false;
    }
    if (query.scenario && conversion.scenario !== query.scenario) {
      return false;
    }
    if (query.status && conversion.status !== query.status) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const getMockProductUnitConversion = (
  conversionId: string
): ProductUnitConversion =>
  mockProductUnitConversions.find((conversion) => conversion.conversion_id === conversionId) ??
  mockProductUnitConversions[0];

export const createMockProductUnitConversion = (
  payload: ProductUnitConversionCreatePayload
): ProductUnitConversion => {
  const now = dayjs().toISOString();
  const sku = resolveSku(payload.sku_id);
  const record: ProductUnitConversion = {
    conversion_id: `UC-${Date.now()}`,
    tenant_id: mockTenantId,
    sku_id: payload.sku_id,
    sku_code: sku?.sku_code ?? payload.sku_id,
    sku_name: sku?.name ?? null,
    base_unit: payload.base_unit,
    aux_unit: payload.aux_unit,
    ratio: payload.ratio ?? 1,
    rounding: payload.rounding ?? 'ROUND',
    scenario: payload.scenario,
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  mockProductUnitConversions.unshift(record);
  return record;
};

export const updateMockProductUnitConversion = (
  conversionId: string,
  payload: ProductUnitConversionUpdatePayload
): ProductUnitConversion => {
  const now = dayjs().toISOString();
  const index = mockProductUnitConversions.findIndex(
    (conversion) => conversion.conversion_id === conversionId
  );
  const base =
    index >= 0
      ? mockProductUnitConversions[index]
      : ({
          conversion_id: conversionId,
          tenant_id: mockTenantId,
          sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
          sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
          sku_name: mockSkus[0]?.name ?? null,
          base_unit: payload.base_unit ?? '件',
          aux_unit: payload.aux_unit ?? '盒',
          ratio: payload.ratio ?? 1,
          rounding: payload.rounding ?? 'ROUND',
          scenario: payload.scenario ?? '采购',
          status: payload.status ?? 'ACTIVE',
          created_at: now,
          updated_at: now
        } as ProductUnitConversion);
  const updated: ProductUnitConversion = {
    ...base,
    base_unit: payload.base_unit ?? base.base_unit,
    aux_unit: payload.aux_unit ?? base.aux_unit,
    ratio: payload.ratio ?? base.ratio,
    rounding: payload.rounding ?? base.rounding,
    scenario: payload.scenario ?? base.scenario,
    status: payload.status ?? base.status,
    updated_at: now
  };
  if (index >= 0) {
    mockProductUnitConversions[index] = updated;
  } else {
    mockProductUnitConversions.unshift(updated);
  }
  return updated;
};

export const deleteMockProductUnitConversion = (
  conversionId: string
): ProductUnitConversion => {
  const index = mockProductUnitConversions.findIndex(
    (conversion) => conversion.conversion_id === conversionId
  );
  if (index >= 0) {
    const [removed] = mockProductUnitConversions.splice(index, 1);
    return removed;
  }
  return getMockProductUnitConversion(conversionId);
};

export const getMockProductChannelAvailabilityList = (
  query: ProductChannelAvailabilityListQuery
): ProductChannelAvailabilityListResponse => {
  const filtered = mockProductChannelAvailability.filter((availability) => {
    if (query.sku_id && availability.sku_id !== query.sku_id) {
      return false;
    }
    if (query.channel_id && availability.channel_id !== query.channel_id) {
      return false;
    }
    if (query.warehouse_id && availability.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && availability.status !== query.status) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const getMockProductChannelAvailability = (
  availabilityId: string
): ProductChannelAvailability =>
  mockProductChannelAvailability.find(
    (availability) => availability.availability_id === availabilityId
  ) ?? mockProductChannelAvailability[0];

export const createMockProductChannelAvailability = (
  payload: ProductChannelAvailabilityCreatePayload
): ProductChannelAvailability => {
  const now = dayjs().toISOString();
  const sku = resolveSku(payload.sku_id);
  const channel = resolveChannel(payload.channel_id);
  const warehouse = resolveWarehouse(payload.warehouse_id);
  const record: ProductChannelAvailability = {
    availability_id: `CA-${Date.now()}`,
    tenant_id: mockTenantId,
    sku_id: payload.sku_id,
    sku_code: sku?.sku_code ?? payload.sku_id,
    sku_name: sku?.name ?? null,
    channel_id: payload.channel_id ?? null,
    channel_platform: channel?.platform ?? null,
    warehouse_id: payload.warehouse_id ?? null,
    warehouse_name: warehouse?.name ?? null,
    status: payload.status ?? 'ON',
    available_qty: payload.available_qty ?? 0,
    rule: payload.rule ?? null,
    on_shelf_at: payload.on_shelf_at ?? null,
    created_at: now,
    updated_at: now
  };
  mockProductChannelAvailability.unshift(record);
  return record;
};

export const updateMockProductChannelAvailability = (
  availabilityId: string,
  payload: ProductChannelAvailabilityUpdatePayload
): ProductChannelAvailability => {
  const now = dayjs().toISOString();
  const index = mockProductChannelAvailability.findIndex(
    (availability) => availability.availability_id === availabilityId
  );
  const base =
    index >= 0
      ? mockProductChannelAvailability[index]
      : ({
          availability_id: availabilityId,
          tenant_id: mockTenantId,
          sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
          sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
          sku_name: mockSkus[0]?.name ?? null,
          channel_id: payload.channel_id ?? null,
          channel_platform: null,
          warehouse_id: payload.warehouse_id ?? null,
          warehouse_name: null,
          status: payload.status ?? 'ON',
          available_qty: payload.available_qty ?? 0,
          rule: payload.rule ?? null,
          on_shelf_at: payload.on_shelf_at ?? null,
          created_at: now,
          updated_at: now
        } as ProductChannelAvailability);
  const nextChannelId = payload.channel_id ?? base.channel_id;
  const nextWarehouseId = payload.warehouse_id ?? base.warehouse_id;
  const channel = resolveChannel(nextChannelId);
  const warehouse = resolveWarehouse(nextWarehouseId);
  const updated: ProductChannelAvailability = {
    ...base,
    channel_id: nextChannelId ?? null,
    channel_platform: channel?.platform ?? base.channel_platform,
    warehouse_id: nextWarehouseId ?? null,
    warehouse_name: warehouse?.name ?? base.warehouse_name,
    status: payload.status ?? base.status,
    available_qty: payload.available_qty ?? base.available_qty,
    rule: payload.rule !== undefined ? payload.rule : base.rule,
    on_shelf_at: payload.on_shelf_at !== undefined ? payload.on_shelf_at : base.on_shelf_at,
    updated_at: now
  };
  if (index >= 0) {
    mockProductChannelAvailability[index] = updated;
  } else {
    mockProductChannelAvailability.unshift(updated);
  }
  return updated;
};

export const deleteMockProductChannelAvailability = (
  availabilityId: string
): ProductChannelAvailability => {
  const index = mockProductChannelAvailability.findIndex(
    (availability) => availability.availability_id === availabilityId
  );
  if (index >= 0) {
    const [removed] = mockProductChannelAvailability.splice(index, 1);
    return removed;
  }
  return getMockProductChannelAvailability(availabilityId);
};

export const getMockProductChangeRequestList = (
  query: ProductChangeRequestListQuery
): ProductChangeRequestListResponse => {
  const filtered = mockProductChangeRequests.filter((request) => {
    if (query.sku_id && request.sku_id !== query.sku_id) {
      return false;
    }
    if (query.status && request.status !== query.status) {
      return false;
    }
    if (query.change_type && request.change_type !== query.change_type) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const getMockProductChangeRequest = (requestId: string): ProductChangeRequest =>
  mockProductChangeRequests.find((request) => request.request_id === requestId) ??
  mockProductChangeRequests[0];

export const createMockProductChangeRequest = (
  payload: ProductChangeRequestCreatePayload
): ProductChangeRequest => {
  const now = dayjs().toISOString();
  const sku = resolveSku(payload.sku_id);
  const history =
    payload.history && payload.history.length > 0
      ? payload.history
      : [
          {
            time: now,
            action: '创建申请',
            actor: payload.applicant,
            note: null
          }
        ];
  const record: ProductChangeRequest = {
    request_id: `CR-${Date.now()}`,
    tenant_id: mockTenantId,
    sku_id: payload.sku_id,
    sku_code: sku?.sku_code ?? payload.sku_id,
    sku_name: sku?.name ?? null,
    title: payload.title,
    change_type: payload.change_type,
    status: payload.status ?? 'DRAFT',
    applicant: payload.applicant,
    approver: payload.approver ?? null,
    reason: payload.reason ?? null,
    history,
    created_at: now,
    updated_at: now
  };
  mockProductChangeRequests.unshift(record);
  return record;
};

export const updateMockProductChangeRequest = (
  requestId: string,
  payload: ProductChangeRequestUpdatePayload
): ProductChangeRequest => {
  const now = dayjs().toISOString();
  const index = mockProductChangeRequests.findIndex(
    (request) => request.request_id === requestId
  );
  const base =
    index >= 0
      ? mockProductChangeRequests[index]
      : ({
          request_id: requestId,
          tenant_id: mockTenantId,
          sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
          sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
          sku_name: mockSkus[0]?.name ?? null,
          title: payload.title ?? '未命名变更',
          change_type: payload.change_type ?? '基础信息',
          status: payload.status ?? 'DRAFT',
          applicant: '系统',
          approver: payload.approver ?? null,
          reason: payload.reason ?? null,
          history: payload.history ?? [],
          created_at: now,
          updated_at: now
        } as ProductChangeRequest);
  const updated: ProductChangeRequest = {
    ...base,
    title: payload.title ?? base.title,
    change_type: payload.change_type ?? base.change_type,
    status: payload.status ?? base.status,
    approver: payload.approver ?? base.approver,
    reason: payload.reason ?? base.reason,
    history: payload.history ?? base.history,
    updated_at: now
  };
  if (index >= 0) {
    mockProductChangeRequests[index] = updated;
  } else {
    mockProductChangeRequests.unshift(updated);
  }
  return updated;
};

export const deleteMockProductChangeRequest = (requestId: string): ProductChangeRequest => {
  const index = mockProductChangeRequests.findIndex(
    (request) => request.request_id === requestId
  );
  if (index >= 0) {
    const [removed] = mockProductChangeRequests.splice(index, 1);
    return removed;
  }
  return getMockProductChangeRequest(requestId);
};

export const getMockProductLifecycleList = (
  query: ProductLifecycleListQuery
): ProductLifecycleListResponse => {
  const filtered = mockProductLifecycles.filter((lifecycle) => {
    if (query.sku_id && lifecycle.sku_id !== query.sku_id) {
      return false;
    }
    if (query.status && lifecycle.status !== query.status) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return { items, total: filtered.length };
};

export const getMockProductLifecycle = (lifecycleId: string): ProductLifecycle =>
  mockProductLifecycles.find((lifecycle) => lifecycle.lifecycle_id === lifecycleId) ??
  mockProductLifecycles[0];

export const createMockProductLifecycle = (
  payload: ProductLifecycleCreatePayload
): ProductLifecycle => {
  const now = dayjs().toISOString();
  const sku = resolveSku(payload.sku_id);
  const existingIndex = mockProductLifecycles.findIndex(
    (lifecycle) => lifecycle.sku_id === payload.sku_id
  );
  const record: ProductLifecycle = {
    lifecycle_id: existingIndex >= 0 ? mockProductLifecycles[existingIndex].lifecycle_id : `LC-${Date.now()}`,
    tenant_id: mockTenantId,
    sku_id: payload.sku_id,
    sku_code: sku?.sku_code ?? payload.sku_id,
    sku_name: sku?.name ?? null,
    status: payload.status ?? 'ACTIVE',
    inventory_qty: payload.inventory_qty ?? 0,
    remark: payload.remark ?? null,
    created_at:
      existingIndex >= 0 ? mockProductLifecycles[existingIndex].created_at ?? now : now,
    updated_at: now
  };
  if (existingIndex >= 0) {
    mockProductLifecycles[existingIndex] = record;
  } else {
    mockProductLifecycles.unshift(record);
  }
  return record;
};

export const updateMockProductLifecycle = (
  lifecycleId: string,
  payload: ProductLifecycleUpdatePayload
): ProductLifecycle => {
  const now = dayjs().toISOString();
  const index = mockProductLifecycles.findIndex(
    (lifecycle) => lifecycle.lifecycle_id === lifecycleId
  );
  const base =
    index >= 0
      ? mockProductLifecycles[index]
      : ({
          lifecycle_id: lifecycleId,
          tenant_id: mockTenantId,
          sku_id: mockSkus[0]?.sku_id ?? 'SKU-1',
          sku_code: mockSkus[0]?.sku_code ?? 'SKU-1',
          sku_name: mockSkus[0]?.name ?? null,
          status: payload.status ?? 'ACTIVE',
          inventory_qty: payload.inventory_qty ?? 0,
          remark: payload.remark ?? null,
          created_at: now,
          updated_at: now
        } as ProductLifecycle);
  const updated: ProductLifecycle = {
    ...base,
    status: payload.status ?? base.status,
    inventory_qty: payload.inventory_qty ?? base.inventory_qty,
    remark: payload.remark !== undefined ? payload.remark : base.remark,
    updated_at: now
  };
  if (index >= 0) {
    mockProductLifecycles[index] = updated;
  } else {
    mockProductLifecycles.unshift(updated);
  }
  return updated;
};

export const deleteMockProductLifecycle = (lifecycleId: string): ProductLifecycle => {
  const index = mockProductLifecycles.findIndex(
    (lifecycle) => lifecycle.lifecycle_id === lifecycleId
  );
  if (index >= 0) {
    const [removed] = mockProductLifecycles.splice(index, 1);
    return removed;
  }
  return getMockProductLifecycle(lifecycleId);
};
