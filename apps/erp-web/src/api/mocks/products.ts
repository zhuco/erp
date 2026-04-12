import dayjs from 'dayjs';

import type {
  Product,
  ProductCreateRequest,
  ProductListQuery,
  ProductListResponse,
  ProductUpdateRequest,
} from './data';
import { applyPagination, mockProducts, mockTenantId } from './data';

const normalizeProductStatus = (status?: string | null): string =>
  status === 'DISABLED' ? 'INACTIVE' : (status ?? 'ACTIVE');

export const getMockProductList = (query: ProductListQuery): ProductListResponse => {
  const keyword = query.keyword?.trim();
  const filtered = mockProducts.filter((product) => {
    const normalizedStatus = normalizeProductStatus(product.status);
    if (
      keyword &&
      !product.name.includes(keyword) &&
      !(product.product_code ?? '').includes(keyword) &&
      !(product.alias ?? '').includes(keyword) &&
      !(product.primary_barcode ?? '').includes(keyword)
    ) {
      return false;
    }
    if (query.brand && product.brand !== query.brand) {
      return false;
    }
    if (query.category && product.category !== query.category) {
      return false;
    }
    if (query.status) {
      if (query.status === 'INACTIVE') {
        if (normalizedStatus === 'ACTIVE') {
          return false;
        }
      } else if (normalizedStatus !== query.status) {
        return false;
      }
    }
    return true;
  });
  const pageSize = query.limit ?? 20;
  const offset = query.cursor ? Number.parseInt(query.cursor, 10) : 0;
  const safeOffset = Number.isNaN(offset) ? 0 : offset;
  const data = applyPagination(filtered, safeOffset, pageSize);
  const nextCursor = safeOffset + pageSize < filtered.length ? String(safeOffset + pageSize) : null;
  return {
    data,
    next_cursor: nextCursor,
  };
};

export const createMockProduct = (payload: ProductCreateRequest): Product => {
  const now = dayjs().toISOString();
  const product: Product = {
    product_id: `PROD-${Date.now()}`,
    tenant_id: mockTenantId,
    product_code: payload.product_code ?? null,
    name: payload.name,
    alias: payload.alias ?? null,
    brand: payload.brand ?? null,
    category: payload.category ?? null,
    base_unit: payload.base_unit ?? null,
    second_unit: payload.second_unit ?? null,
    default_supplier_id: payload.default_supplier_id ?? null,
    default_warehouse_id: payload.default_warehouse_id ?? null,
    primary_barcode: payload.primary_barcode ?? null,
    product_tags: payload.product_tags ?? [],
    remark: payload.remark ?? null,
    warning_stock: payload.warning_stock ?? {},
    batch_enabled: payload.batch_enabled ?? false,
    shelf_life_days: payload.shelf_life_days ?? null,
    forbid_sale_days: payload.forbid_sale_days ?? null,
    serial_enabled: payload.serial_enabled ?? false,
    serial_push_platform: payload.serial_push_platform ?? false,
    attrs: payload.attrs ?? {},
    allow_negative_stock: payload.allow_negative_stock ?? false,
    status: 'ACTIVE',
    created_at: now,
    updated_at: now,
  };
  mockProducts.unshift(product);
  return product;
};

export const updateMockProduct = (productId: string, payload: ProductUpdateRequest): Product => {
  const now = dayjs().toISOString();
  const index = mockProducts.findIndex((product) => product.product_id === productId);
  const base =
    index >= 0
      ? mockProducts[index]
      : {
          product_id: productId,
          tenant_id: mockTenantId,
          product_code: payload.product_code ?? null,
          name: payload.name ?? productId,
          alias: payload.alias ?? null,
          brand: payload.brand ?? null,
          category: payload.category ?? null,
          base_unit: payload.base_unit ?? null,
          second_unit: payload.second_unit ?? null,
          default_supplier_id: payload.default_supplier_id ?? null,
          default_warehouse_id: payload.default_warehouse_id ?? null,
          primary_barcode: payload.primary_barcode ?? null,
          product_tags: payload.product_tags ?? [],
          remark: payload.remark ?? null,
          warning_stock: payload.warning_stock ?? {},
          batch_enabled: payload.batch_enabled ?? false,
          shelf_life_days: payload.shelf_life_days ?? null,
          forbid_sale_days: payload.forbid_sale_days ?? null,
          serial_enabled: payload.serial_enabled ?? false,
          serial_push_platform: payload.serial_push_platform ?? false,
          attrs: payload.attrs ?? {},
          allow_negative_stock: payload.allow_negative_stock ?? false,
          status: payload.status ?? 'ACTIVE',
          created_at: now,
          updated_at: now,
        };
  const updated: Product = {
    ...base,
    product_code: payload.product_code ?? base.product_code ?? null,
    name: payload.name ?? base.name,
    alias: payload.alias ?? base.alias ?? null,
    brand: payload.brand ?? base.brand ?? null,
    category: payload.category ?? base.category ?? null,
    base_unit: payload.base_unit ?? base.base_unit ?? null,
    second_unit: payload.second_unit ?? base.second_unit ?? null,
    default_supplier_id: payload.default_supplier_id ?? base.default_supplier_id ?? null,
    default_warehouse_id: payload.default_warehouse_id ?? base.default_warehouse_id ?? null,
    primary_barcode: payload.primary_barcode ?? base.primary_barcode ?? null,
    product_tags: payload.product_tags ?? base.product_tags ?? [],
    remark: payload.remark ?? base.remark ?? null,
    warning_stock: payload.warning_stock ?? base.warning_stock,
    batch_enabled: payload.batch_enabled ?? base.batch_enabled,
    shelf_life_days: payload.shelf_life_days ?? base.shelf_life_days ?? null,
    forbid_sale_days: payload.forbid_sale_days ?? base.forbid_sale_days ?? null,
    serial_enabled: payload.serial_enabled ?? base.serial_enabled,
    serial_push_platform: payload.serial_push_platform ?? base.serial_push_platform,
    attrs: payload.attrs ?? base.attrs,
    allow_negative_stock: payload.allow_negative_stock ?? base.allow_negative_stock,
    status: payload.status ?? base.status ?? 'ACTIVE',
    updated_at: now,
  };
  if (index >= 0) {
    mockProducts[index] = updated;
  } else {
    mockProducts.unshift(updated);
  }
  return updated;
};

export const deleteMockProduct = (productId: string): Product => {
  const index = mockProducts.findIndex((product) => product.product_id === productId);
  if (index < 0) {
    throw new Error('商品不存在');
  }
  const [deleted] = mockProducts.splice(index, 1);
  return deleted as Product;
};
