import dayjs from 'dayjs';

import type {
  ShopProductAutoMatchPayload,
  ShopProductCreateSkuPayload,
  ShopProductDeleteMatchPayload,
  ShopProductDownloadPayload,
  ShopProductDownloadResponse,
  ShopProductManualMatchPayload,
  ShopProductMatchFailure,
  ShopProductMatchResponse,
  ShopProductSku,
  ShopProductSkuListQuery,
  ShopProductSkuListResponse,
  ShopProductTask,
  ShopProductTaskListQuery,
  ShopProductTaskListResponse,
  ShopProductTaskResponse
} from '../shopProducts';
import { applyPagination, mockTenantId } from './data';

const mockShopProductSkus: ShopProductSku[] = [
  {
    sku_record_id: 'shop-sku-1',
    tenant_id: mockTenantId,
    shop_id: 'SHOP-taobao-1',
    platform: 'taobao',
    platform_product_id: 'product-1',
    platform_sku_id: 'TB-SKU-001',
    merchant_code: 'TB-001',
    merchant_sku_code: 'TB-001-RED',
    product_title: '春季卫衣',
    sku_title: '红色 L',
    spec_text: '红色 L',
    price_cent: 19900,
    stock_qty: 50,
    status: 'ACTIVE',
    barcode: '6900000001',
    image_url: 'https://placehold.co/64x64',
    platform_updated_at: dayjs().subtract(2, 'day').toISOString(),
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    sku_record_id: 'shop-sku-2',
    tenant_id: mockTenantId,
    shop_id: 'SHOP-taobao-1',
    platform: 'taobao',
    platform_product_id: 'product-1',
    platform_sku_id: 'TB-SKU-002',
    merchant_code: 'TB-001',
    merchant_sku_code: 'TB-001-BLUE',
    product_title: '春季卫衣',
    sku_title: '蓝色 M',
    spec_text: '蓝色 M',
    price_cent: 19900,
    stock_qty: 30,
    status: 'ACTIVE',
    barcode: '6900000002',
    image_url: 'https://placehold.co/64x64',
    platform_updated_at: dayjs().subtract(1, 'day').toISOString(),
    mapping_id: 'map-1',
    mapped_sku_id: 'sku-1',
    mapped_sku_code: 'SKU-0001',
    mapped_sku_name: '卫衣-蓝色 M',
    match_source: 'AUTO',
    mapping_status: 'ACTIVE',
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    sku_record_id: 'shop-sku-3',
    tenant_id: mockTenantId,
    shop_id: 'SHOP-jd-1',
    platform: 'jd',
    platform_product_id: 'product-2',
    platform_sku_id: 'JD-SKU-003',
    merchant_code: 'JD-002',
    merchant_sku_code: 'JD-002-BLACK',
    product_title: '轻薄外套',
    sku_title: '黑色 XL',
    spec_text: '黑色 XL',
    price_cent: 29900,
    stock_qty: 20,
    status: 'ACTIVE',
    barcode: '6900000003',
    image_url: 'https://placehold.co/64x64',
    platform_updated_at: dayjs().subtract(4, 'hour').toISOString(),
    mapping_id: 'map-2',
    mapped_sku_id: 'sku-2',
    mapped_sku_code: 'SKU-0002',
    mapped_sku_name: '外套-黑色 XL',
    match_source: 'MANUAL',
    mapping_status: 'ACTIVE',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(4, 'hour').toISOString()
  }
];

const mockTasks: ShopProductTask[] = [];

const buildTask = (
  taskType: string,
  total = 0,
  success = 0,
  failure = 0,
  status = 'SUCCESS',
  payload: Record<string, unknown> = {},
  result: Record<string, unknown> = {}
): ShopProductTask => {
  const now = dayjs().toISOString();
  return {
    task_id: `task-${Date.now()}-${Math.random().toString(16).slice(2, 6)}`,
    tenant_id: mockTenantId,
    task_type: taskType,
    status,
    shop_id: null,
    platform: null,
    total_count: total,
    success_count: success,
    failure_count: failure,
    error: failure ? '存在失败' : '',
    payload_json: payload,
    result_json: result,
    started_at: now,
    finished_at: now,
    created_at: now,
    updated_at: now
  };
};

const resolveMatchStatus = (sku: ShopProductSku): string => {
  if (!sku.mapping_id) {
    return 'UNMATCHED';
  }
  if (sku.match_source === 'MANUAL') {
    return 'MANUAL';
  }
  if (sku.match_source === 'AUTO') {
    return 'AUTO';
  }
  return 'MATCHED';
};

export const getMockShopProductSkuList = (
  query: ShopProductSkuListQuery
): ShopProductSkuListResponse => {
  const normalizedKeyword = query.keyword?.trim().toLowerCase();
  const filtered = mockShopProductSkus.filter((sku) => {
    if (query.platform && sku.platform !== query.platform) {
      return false;
    }
    if (query.shop_ids?.length && !query.shop_ids.includes(sku.shop_id)) {
      return false;
    }
    if (query.status && sku.status !== query.status) {
      return false;
    }
    if (query.match_status) {
      const status = resolveMatchStatus(sku);
      if (query.match_status === 'MATCHED') {
        if (!sku.mapping_id) {
          return false;
        }
      } else if (status !== query.match_status) {
        return false;
      }
    }
    if (normalizedKeyword) {
      const combined =
        `${sku.platform_sku_id} ${sku.merchant_sku_code ?? ''} ${sku.product_title} ${sku.sku_title ?? ''}`.toLowerCase();
      if (!combined.includes(normalizedKeyword)) {
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

export const getMockShopProductTaskList = (
  query: ShopProductTaskListQuery
): ShopProductTaskListResponse => {
  const filtered = mockTasks.filter((task) => {
    if (query.task_type && task.task_type !== query.task_type) {
      return false;
    }
    if (query.status && task.status !== query.status) {
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

export const createMockShopProductDownload = (
  payload: ShopProductDownloadPayload
): ShopProductDownloadResponse => {
  const task = buildTask('DOWNLOAD', 3, 3, 0, 'SUCCESS', payload);
  mockTasks.unshift(task);
  const autoMatch = payload.auto_match
    ? buildTask('AUTO_MATCH', 2, 2, 0, 'SUCCESS', payload)
    : null;
  if (autoMatch) {
    mockTasks.unshift(autoMatch);
  }
  return {
    task,
    auto_match_task: autoMatch,
    auto_match_failures: []
  };
};

export const createMockShopProductSkuTask = (
  payload: ShopProductCreateSkuPayload
): ShopProductTaskResponse => {
  const ids = payload.sku_record_ids ?? mockShopProductSkus.map((sku) => sku.sku_record_id);
  ids.forEach((id) => {
    const target = mockShopProductSkus.find((sku) => sku.sku_record_id === id);
    if (target && !target.mapping_id) {
      target.mapping_id = `map-${Date.now()}`;
      target.mapped_sku_id = 'sku-new';
      target.mapped_sku_code = 'SKU-NEW';
      target.mapped_sku_name = `${target.product_title}-${target.spec_text ?? ''}`;
      target.match_source = 'AUTO';
      target.mapping_status = 'ACTIVE';
    }
  });
  const task = buildTask('CREATE_SKUS', ids.length, ids.length, 0, 'SUCCESS', payload);
  mockTasks.unshift(task);
  return { task };
};

export const createMockShopProductMatchTask = (
  payload: ShopProductAutoMatchPayload
): ShopProductMatchResponse => {
  const ids = payload.sku_record_ids ?? mockShopProductSkus.map((sku) => sku.sku_record_id);
  const failures: ShopProductMatchFailure[] = [];
  ids.forEach((id) => {
    const target = mockShopProductSkus.find((sku) => sku.sku_record_id === id);
    if (!target) {
      failures.push({
        sku_record_id: id,
        platform_sku_id: id,
        reason: '未找到记录'
      });
      return;
    }
    if (!target.mapping_id) {
      target.mapping_id = `map-${Date.now()}`;
      target.mapped_sku_id = 'sku-auto';
      target.mapped_sku_code = 'SKU-AUTO';
      target.mapped_sku_name = `${target.product_title}-${target.spec_text ?? ''}`;
      target.match_source = 'AUTO';
      target.mapping_status = 'ACTIVE';
    }
  });
  const task = buildTask(
    'AUTO_MATCH',
    ids.length,
    ids.length - failures.length,
    failures.length,
    failures.length ? 'FAILED' : 'SUCCESS',
    payload,
    { failures }
  );
  mockTasks.unshift(task);
  return { task, failures };
};

export const manualMockShopProductMatch = (
  payload: ShopProductManualMatchPayload
): ShopProductTaskResponse => {
  payload.sku_record_ids.forEach((id) => {
    const target = mockShopProductSkus.find((sku) => sku.sku_record_id === id);
    if (!target) {
      return;
    }
    target.mapping_id = `map-${Date.now()}`;
    target.mapped_sku_id = payload.sku_id;
    target.mapped_sku_code = `SKU-${payload.sku_id.slice(0, 6)}`;
    target.mapped_sku_name = target.product_title;
    target.match_source = 'MANUAL';
    target.mapping_status = 'ACTIVE';
  });
  const task = buildTask(
    'MANUAL_MATCH',
    payload.sku_record_ids.length,
    payload.sku_record_ids.length,
    0,
    'SUCCESS',
    payload
  );
  mockTasks.unshift(task);
  return { task };
};

export const deleteMockShopProductMatches = (
  payload: ShopProductDeleteMatchPayload
): ShopProductTaskResponse => {
  payload.sku_record_ids.forEach((id) => {
    const target = mockShopProductSkus.find((sku) => sku.sku_record_id === id);
    if (!target) {
      return;
    }
    target.mapping_id = null;
    target.mapped_sku_id = null;
    target.mapped_sku_code = null;
    target.mapped_sku_name = null;
    target.match_source = null;
    target.mapping_status = null;
  });
  const task = buildTask(
    'DELETE_MATCH',
    payload.sku_record_ids.length,
    payload.sku_record_ids.length,
    0,
    'SUCCESS',
    payload
  );
  mockTasks.unshift(task);
  return { task };
};
