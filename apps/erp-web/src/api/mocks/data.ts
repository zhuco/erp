import dayjs from 'dayjs';

import type { PlatformCode } from '@/features/platform';
import { Platform, platformValues } from '@/features/platform';

import type { InventoryLedger } from '../inventoryLedgers';
import type { StockSyncJob } from '../inventorySyncJobs';
import type { ShipmentSummary } from '../shipments';
import type { Channel } from '../channels';
import type { Shop } from '../shops';
import type { Courier } from '../couriers';
import type { SyncJob } from '../syncJobs';
import type { SalesShopConfig } from '../salesShops';
import type { SyncJobRun } from '../syncJobRuns';
import type { Supplier } from '../suppliers';
import type { Customer } from '../customers';
import type { CustomerCategory } from '../customerCategories';
import type { MemberLevel } from '../memberLevels';
import type { CustomerTag } from '../customerTags';
import type { PurchaseOrderDetail, PurchaseOrderSummary } from '../purchaseOrders';
import type { PurchaseReceiptDetail, PurchaseReceiptSummary } from '../purchaseReceipts';
import type { PurchaseSuggestion } from '../purchaseSuggestions';
import type { PurchaseReturnDetail, PurchaseReturnSummary } from '../purchaseReturns';
import type { Warehouse } from '../warehouses';
import type { components, paths } from '../generated/schema';

type OrderListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/orders']['get']['parameters']['query']
> & {
  tagged?: boolean;
  ship_warehouse_id?: string;
  platform_order_id?: string;
  item_keyword?: string;
  total_amount_cent?: number;
  goods_qty?: number;
  customer_account?: string;
  receiver_name?: string;
  receiver_phone?: string;
  receiver_address?: string;
  remark?: string;
  order_source?: string;
  logistics_company?: string;
  estimated_postage_cent?: number;
  estimated_profit_cent?: number;
  estimated_profit_rate?: number;
};
type AfterSalesListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/after-sale-cases']['get']['parameters']['query']
>;
type InventoryListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/inventory']['get']['parameters']['query']
>;
type SkuListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/skus']['get']['parameters']['query']
>;

type OrderSummary = components['schemas']['OrderSummary'];
type OrderDetail = components['schemas']['OrderDetail'];
type OrderItem = components['schemas']['OrderItem'];
type OrderReceiverUpdatePayload = {
  receiver_name: string;
  receiver_phone: string;
  receiver_address: string;
};
type MockOmsMainChainActionSeed = {
  action: string;
  message: string;
  operator_id?: string | null;
  operator_name?: string | null;
  created_at: string;
};
type MockOmsMainChainSeed = {
  order_id: string;
  audit_status: string;
  exception_status: string;
  fulfillment_status: string;
  reserve_status?: string | null;
  reserve_precheck_passed: boolean;
  reserve_precheck_reasons: string[];
  target_warehouse_id?: string | null;
  blocking_reason: string;
  assignee_id?: string | null;
  assignee_name?: string | null;
  action_logs: MockOmsMainChainActionSeed[];
};
type AfterSaleCaseSummary = components['schemas']['AfterSaleCaseSummary'];
type AfterSaleCaseDetail = components['schemas']['AfterSaleCaseDetail'];
type AfterSaleCaseItem = components['schemas']['AfterSaleCaseItem'];
type AfterSalesIntercept = components['schemas']['AfterSalesIntercept'];
type AfterSalesInterceptStatus = components['schemas']['AfterSalesInterceptStatus'];
type AfterSalesInspection = components['schemas']['AfterSalesInspection'];
type AfterSalesInspectionResult = components['schemas']['AfterSalesInspectionResult'];
type AfterSalesSlaRule = components['schemas']['AfterSalesSlaRule'];
type AfterSalesSlaStatus = components['schemas']['AfterSalesSlaStatus'];
type InventorySnapshot = components['schemas']['InventorySnapshot'];
type Product = components['schemas']['Product'];
type ProductListResponse = components['schemas']['ProductListResponse'];
type ProductListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/products']['get']['parameters']['query']
>;
type ProductCreateRequest = components['schemas']['ProductCreateRequest'];
type ProductUpdateRequest = components['schemas']['ProductUpdateRequest'];
type Sku = components['schemas']['Sku'];
type SkuListResponse = components['schemas']['SkuListResponse'];
type SkuKind = 'KIT' | 'NORMAL';
type SkuSearchItem = components['schemas']['SkuSearchItem'];
type SkuCreatePayload = components['schemas']['SkuCreateRequest'];
type SkuUpdatePayload = components['schemas']['SkuUpdateRequest'];
type SkuBatch = components['schemas']['SkuBatch'];
type SkuBatchListResponse = components['schemas']['SkuBatchListResponse'];
type SkuBatchListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/sku-batches']['get']['parameters']['query']
>;
type SkuBatchCreateRequest = components['schemas']['SkuBatchCreateRequest'];
type SkuBatchUpdateRequest = components['schemas']['SkuBatchUpdateRequest'];
type SkuSerial = components['schemas']['SkuSerial'];
type SkuSerialListResponse = components['schemas']['SkuSerialListResponse'];
type SkuSerialListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/sku-serials']['get']['parameters']['query']
>;
type SkuSerialGenerateRequest = components['schemas']['SkuSerialGenerateRequest'];
type SkuSerialPrintRequest = components['schemas']['SkuSerialPrintRequest'];
type PrintJob = components['schemas']['PrintJob'];
type MappingListQuery = {
  platform?: PlatformCode;
  shop_id?: string;
  platform_sku_id?: string;
  sku_code?: string;
  barcode?: string;
  offset?: number;
  limit?: number;
};
type MappingPayload = {
  platform: PlatformCode;
  shop_id: string;
  platform_sku_id: string;
  outer_sku_id?: string | null;
  sku_id: string;
  sku_code?: string | null;
  barcode?: string | null;
};
type MappingRecord = {
  mapping_id: string;
  tenant_id: string;
  platform: PlatformCode;
  shop_id: string;
  platform_sku_id: string;
  outer_sku_id?: string | null;
  sku_id: string;
  sku_code: string;
  barcode?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};
type KitComponent = {
  component_sku_id: string;
  component_sku_code: string;
  component_name?: string | null;
  component_barcode?: string | null;
  qty: number;
};
type KitDetail = {
  kit_sku: SkuSearchItem;
  components: KitComponent[];
  updated_at?: string | null;
};
type KitSummary = {
  kit_sku_id: string;
  kit_sku_code: string;
  kit_name?: string | null;
  component_count: number;
  updated_at?: string | null;
};
type PrintJobStatus = 'PENDING' | 'RUNNING' | 'DONE' | 'FAILED';
type PrintJobDispatchStatus = 'QUEUED' | 'DISPATCHED' | 'PRINTED' | 'FAILED';
type PrintJobEvent = {
  status: PrintJobStatus;
  error?: string | null;
  created_at: string;
  actor_type?: string | null;
};
type PrintShippingLabelPayload = {
  order_id: string;
  platform: PlatformCode;
  shop_id: string;
  shipment_id?: string;
  tracking_no?: string | null;
  carrier_code?: string | null;
};
type PrintJobDetail = {
  job_id: string;
  job_type?: string | null;
  ref_type?: string | null;
  ref_id?: string | null;
  status: PrintJobStatus;
  dispatch_status: PrintJobDispatchStatus;
  output_url?: string | null;
  attempts?: number | null;
  last_error?: string | null;
  device_fingerprint?: string | null;
  device_name?: string | null;
  created_at: string;
  updated_at: string;
  events: PrintJobEvent[];
  payload?: PrintShippingLabelPayload | Record<string, unknown> | null;
};
type PrintJobSummary = {
  job_id: string;
  job_type?: string | null;
  ref_type?: string | null;
  ref_id?: string | null;
  status: PrintJobStatus;
  dispatch_status: PrintJobDispatchStatus;
  output_url?: string | null;
  attempts?: number | null;
  last_error?: string | null;
  device_fingerprint?: string | null;
  device_name?: string | null;
  created_at: string;
  updated_at: string;
};
type PrintTemplateType = 'SHIPPING_LABEL' | 'PICK_LIST' | 'PACK_LIST' | 'INVOICE';
type PrintTemplate = {
  template_id: string;
  platform: PlatformCode;
  shop_id: string;
  template_type: PrintTemplateType;
  version: number;
  content?: unknown;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};

const platforms: PlatformCode[] = platformValues;
const mockTenantId = '00000000-0000-0000-0000-000000000000';
const mockUserId = 'USER-0001';

const mockWarehouseSeeds = [
  { warehouse_id: 'WH-DEFAULT', name: '默认仓', code: 'DEFAULT', is_default: true },
  { warehouse_id: 'WH-EAST', name: '华东仓', code: 'EAST', is_default: false }
];

const buildWarehouse = (
  seed: (typeof mockWarehouseSeeds)[number],
  index: number
): Warehouse => ({
  warehouse_id: seed.warehouse_id,
  tenant_id: mockTenantId,
  name: seed.name,
  code: seed.code,
  is_default: seed.is_default,
  created_at: dayjs().subtract(index + 1, 'day').toISOString(),
  updated_at: dayjs().subtract(index, 'hour').toISOString()
});

const mockWarehouses: Warehouse[] = mockWarehouseSeeds.map((seed, index) =>
  buildWarehouse(seed, index)
);

const mockCourierSeeds = [
  { courier_id: 'CR-SF', name: '顺丰速运', code: 'SF', status: 'ACTIVE' },
  { courier_id: 'CR-YTO', name: '圆通速递', code: 'YTO', status: 'ACTIVE' },
  { courier_id: 'CR-ZTO', name: '中通快递', code: 'ZTO', status: 'ACTIVE' },
  { courier_id: 'CR-STO', name: '申通快递', code: 'STO', status: 'ACTIVE' },
  { courier_id: 'CR-YD', name: '韵达快递', code: 'YD', status: 'ACTIVE' },
  { courier_id: 'CR-EMS', name: '邮政EMS', code: 'EMS', status: 'ACTIVE' },
  { courier_id: 'CR-JD', name: '京东物流', code: 'JD', status: 'ACTIVE' },
  { courier_id: 'CR-JTX', name: '极兔速递', code: 'JTX', status: 'ACTIVE' },
  { courier_id: 'CR-DB', name: '德邦快递', code: 'DB', status: 'ACTIVE' },
  { courier_id: 'CR-BEST', name: '百世快递', code: 'BEST', status: 'ACTIVE' },
  { courier_id: 'CR-ZJS', name: '宅急送', code: 'ZJS', status: 'DISABLED' },
  { courier_id: 'CR-TTK', name: '天天快递', code: 'TTK', status: 'DISABLED' },
  { courier_id: 'CR-UC', name: '优速快递', code: 'UC', status: 'DISABLED' },
  { courier_id: 'CR-ANE', name: '安能快递', code: 'ANE', status: 'DISABLED' },
  { courier_id: 'CR-KYS', name: '跨越速运', code: 'KYS', status: 'DISABLED' },
  { courier_id: 'CR-POST', name: '邮政快递包裹', code: 'POST', status: 'DISABLED' },
  { courier_id: 'CR-YZPY', name: '邮政平邮', code: 'YZPY', status: 'DISABLED' },
  { courier_id: 'CR-SUNING', name: '苏宁物流', code: 'SUNING', status: 'DISABLED' },
  { courier_id: 'CR-GTO', name: '国通快递', code: 'GTO', status: 'DISABLED' },
  { courier_id: 'CR-DHL', name: 'DHL', code: 'DHL', status: 'DISABLED' },
  { courier_id: 'CR-FEDEX', name: 'FedEx', code: 'FEDEX', status: 'DISABLED' },
  { courier_id: 'CR-UPS', name: 'UPS', code: 'UPS', status: 'DISABLED' },
  { courier_id: 'CR-TNT', name: 'TNT', code: 'TNT', status: 'DISABLED' }
];

const buildCourier = (
  seed: (typeof mockCourierSeeds)[number],
  index: number
): Courier => ({
  courier_id: seed.courier_id,
  tenant_id: mockTenantId,
  name: seed.name,
  code: seed.code,
  status: seed.status ?? 'ACTIVE',
  created_at: dayjs().subtract(index + 2, 'day').toISOString(),
  updated_at: dayjs().subtract(index, 'hour').toISOString()
});

const mockCouriers: Courier[] = mockCourierSeeds.map((seed, index) =>
  buildCourier(seed, index)
);

const mockChannels: Channel[] = platformValues.map((platform, index) => ({
  channel_id: `CH-${platform}`,
  tenant_id: mockTenantId,
  platform,
  name: `渠道-${platform.toUpperCase()}`,
  department: index % 2 === 0 ? '电商事业部' : '销售中心',
  owner: index % 2 === 0 ? '李雷' : '韩梅梅',
  allow_manual_order: index % 2 === 0,
  allow_manual_after_sales: index % 3 === 0,
  default_warehouse_id: mockWarehouses[index % mockWarehouses.length]?.warehouse_id ?? null,
  commission_formula: index % 2 === 0 ? '销售额*0.05' : '成交额*0.03',
  status: index % 4 === 0 ? 'DISABLED' : 'ACTIVE',
  created_at: dayjs().subtract(index + 1, 'day').toISOString(),
  updated_at: dayjs().subtract(index, 'hour').toISOString()
}));

const mockSupplierSeeds = ['华东供应商', '华南供应商', '海外供应商'];

const mockSuppliers: Supplier[] = mockSupplierSeeds.map((name, index) => ({
  supplier_id: `SUP-${index + 1}`,
  tenant_id: mockTenantId,
  name,
  contact: `Contact ${index + 1}`,
  settlement_terms: index % 2 === 0 ? 'Monthly' : 'Weekly',
  status: 'ACTIVE',
  created_at: dayjs().subtract(index + 3, 'day').toISOString(),
  updated_at: dayjs().subtract(index, 'hour').toISOString()
}));

const mockCustomerCategorySeeds = ['零售客户', '批发客户', '会员客户'];
const mockCustomerCategories: CustomerCategory[] = mockCustomerCategorySeeds.map(
  (name, index) => ({
    category_id: `CAT-${index + 1}`,
    tenant_id: mockTenantId,
    name,
    description: index === 0 ? '线上零售' : index === 1 ? '批发合作' : '会员客户',
    status: 'ACTIVE',
    created_at: dayjs().subtract(index + 5, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString()
  })
);

const mockMemberLevels: MemberLevel[] = [
  {
    level_id: 'LEVEL-1',
    tenant_id: mockTenantId,
    name: '普通会员',
    level_rank: 1,
    discount_rate: null,
    rules: '默认等级',
    status: 'ACTIVE',
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    level_id: 'LEVEL-2',
    tenant_id: mockTenantId,
    name: '银卡会员',
    level_rank: 2,
    discount_rate: 0.95,
    rules: '消费满 2000',
    status: 'ACTIVE',
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    level_id: 'LEVEL-3',
    tenant_id: mockTenantId,
    name: '金卡会员',
    level_rank: 3,
    discount_rate: 0.9,
    rules: '消费满 5000',
    status: 'ACTIVE',
    created_at: dayjs().subtract(6, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  }
];

const mockCustomerTags: CustomerTag[] = [
  {
    tag_id: 'CTAG-1',
    tenant_id: mockTenantId,
    name: 'VIP',
    color: 'gold',
    created_at: dayjs().subtract(7, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    tag_id: 'CTAG-2',
    tenant_id: mockTenantId,
    name: '回头客',
    color: 'blue',
    created_at: dayjs().subtract(6, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    tag_id: 'CTAG-3',
    tenant_id: mockTenantId,
    name: '黑名单',
    color: 'red',
    created_at: dayjs().subtract(5, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  }
];

const buildCustomer = (index: number): Customer => {
  const category = mockCustomerCategories[index % mockCustomerCategories.length];
  const level = mockMemberLevels[index % mockMemberLevels.length];
  const status = index % 9 === 0 ? 'INACTIVE' : 'ACTIVE';
  return {
    customer_id: `CUST-${index + 1}`,
    tenant_id: mockTenantId,
    name: `客户-${index + 1}`,
    phone: `1380000${String(1000 + index).slice(-4)}`,
    address: `上海市浦东新区示例路 ${index + 1} 号`,
    note: index % 3 === 0 ? '重点客户' : null,
    status,
    category_id: category?.category_id ?? null,
    category_name: category?.name ?? null,
    member_level_id: level?.level_id ?? null,
    member_level_name: level?.name ?? null,
    created_at: dayjs().subtract(index + 2, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString()
  };
};

const mockCustomers: Customer[] = Array.from({ length: 24 }, (_, index) => buildCustomer(index));
const mockCustomerTagLinks = new Map<string, string[]>(
  mockCustomers.map((customer, index) => {
    const tags = mockCustomerTags
      .filter((_, tagIndex) => (index + tagIndex) % 2 === 0)
      .map((tag) => tag.tag_id);
    return [customer.customer_id, tags];
  })
);

const pickPlatform = (index: number): PlatformCode =>
  platforms[index % platforms.length] ?? Platform.Taobao;

const buildShopTokenStatus = (index: number): NonNullable<Shop['token_status']> => {
  const expiresAt = dayjs().add(index % 3 === 0 ? -40 : 90, 'minute');
  const now = dayjs();
  const expired = expiresAt.isBefore(now);
  const expiringSoon = !expired && expiresAt.isBefore(now.add(30, 'minute'));
  return {
    has_access_token: true,
    has_refresh_token: true,
    expires_at: expiresAt.toISOString(),
    expires_in_minutes: expiresAt.diff(now, 'minute'),
    is_expired: expired,
    is_expiring_soon: expiringSoon,
    scopes: 'orders,shipping',
    last_refresh_at: now.subtract(2, 'hour').toISOString(),
    last_error: expired ? 'token expired' : '',
    updated_at: now.subtract(10, 'minute').toISOString()
  };
};

const buildShop = (index: number): Shop => {
  const platform = pickPlatform(index);
  const status = index % 5 === 0 ? 'DISABLED' : index % 4 === 0 ? 'EXPIRED' : 'AUTHORIZED';
  return {
    shop_id: `SHOP-${platform}-${index + 1}`,
    tenant_id: mockTenantId,
    platform,
    platform_shop_id: `PLAT-${platform.toUpperCase()}-${100 + index}`,
    name: `店铺-${platform.toUpperCase()}-${index + 1}`,
    status,
    auth_meta_json: {},
    created_at: dayjs().subtract(index + 2, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString(),
    token_status: status === 'DISABLED' ? null : buildShopTokenStatus(index)
  };
};

const mockShops: Shop[] = Array.from({ length: 6 }, (_, index) => buildShop(index));

const mockSyncJobs: SyncJob[] = mockShops.flatMap((shop, index) => {
  const baseTime = dayjs().subtract(index + 1, 'hour');
  return ['order_sync', 'refund_sync', 'shipping_sync'].map((jobType, jobIndex) => ({
    job_id: `SYNC-${shop.shop_id}-${jobType}`,
    tenant_id: mockTenantId,
    shop_id: shop.shop_id,
    job_type: jobType,
    cursor: null,
    last_time: baseTime.subtract(jobIndex * 10, 'minute').toISOString(),
    status: jobIndex === 0 ? 'RUNNING' : 'PENDING',
    last_error: '',
    created_at: baseTime.subtract(1, 'day').toISOString(),
    updated_at: baseTime.toISOString()
  }));
});

const buildSalesShopConfig = (shop: Shop): SalesShopConfig => ({
  shop_id: shop.shop_id,
  platform: shop.platform,
  platform_shop_id: shop.platform_shop_id,
  name: shop.name,
  status: shop.status,
  auto_download_orders: true,
  auto_to_review: true,
  download_interval_minutes: 5,
  auto_merge_non_shipping_time: false,
  non_shipping_time_start: '00:00',
  non_shipping_time_end: '08:00',
  enable_stock_sync: true,
  enable_after_sales_download: true,
  enable_logistics_upload: true
});

const mockSalesShopConfigs = new Map<string, SalesShopConfig>(
  mockShops.map((shop) => [shop.shop_id, buildSalesShopConfig(shop)])
);

const mockSyncJobRuns: SyncJobRun[] = mockSyncJobs.flatMap((job, index) => {
  const baseTime = dayjs().subtract(index + 1, 'hour');
  const failedTime = baseTime.subtract(1, 'day');
  return [
    {
      run_id: `RUN-${job.job_id}-SUCCESS`,
      tenant_id: mockTenantId,
      job_id: job.job_id,
      shop_id: job.shop_id,
      job_type: job.job_type,
      status: 'SUCCESS',
      item_count: 6 + index,
      duration_ms: 1200 + index * 50,
      error: '',
      started_at: baseTime.toISOString(),
      finished_at: baseTime.add(2, 'minute').toISOString()
    },
    {
      run_id: `RUN-${job.job_id}-FAILED`,
      tenant_id: mockTenantId,
      job_id: job.job_id,
      shop_id: job.shop_id,
      job_type: job.job_type,
      status: 'FAILED',
      item_count: 0,
      duration_ms: 900,
      error: 'timeout',
      started_at: failedTime.toISOString(),
      finished_at: failedTime.add(1, 'minute').toISOString()
    }
  ];
});

const buildOrder = (index: number): OrderSummary => {
  const platform = pickPlatform(index);
  const receiverName = `收件人${index + 1}`;
  const receiverPhone = `1380000${String(1000 + index).slice(-4)}`;
  const trackingNo = `SF${100000 + index}`;
  const customer = mockCustomers[index % mockCustomers.length];
  const mappingMissing = index % 5 === 0;
  const inventoryShortage = index % 7 === 0;
  const shippingFailed = index % 11 === 0;
  const reviewStatus =
    index % 3 === 0 ? 'PENDING' : index % 3 === 1 ? 'PASSED' : 'REJECTED';
  const exceptionFlags = [
    ...(mappingMissing ? ['MAPPING_MISSING'] : []),
    ...(inventoryShortage ? ['INVENTORY_SHORTAGE'] : []),
    ...(shippingFailed ? ['SHIPPING_FAILED'] : []),
    ...(reviewStatus === 'REJECTED' ? ['REVIEW_REJECTED'] : [])
  ];
  const omsStatus =
    index % 4 === 0
      ? 'REVIEW_PENDING'
      : index % 4 === 1
        ? 'PENDING_SHIPMENT'
        : index % 4 === 2
          ? 'ALLOCATED'
          : 'SHIPPED';
  const orderIdSeq = String(index + 1).padStart(4, '0');
  const orderId = `DT${dayjs().format('YYYYMMDD')}${orderIdSeq}`;
  return {
    id: orderId,
    platform,
    shop_id: `SHOP-${platform}-${(index % 4) + 1}`,
    platform_order_id: `PLAT-${platform.toUpperCase()}-${1000 + index}`,
    platform_status:
      index % 3 === 0 ? 'PENDING_SHIPMENT' : index % 3 === 1 ? 'SHIPPED' : 'COMPLETED',
    oms_status: omsStatus,
    review_status: reviewStatus,
    total_amount_cent: 1999 + index * 10,
    receiver_name: receiverName,
    receiver_phone: receiverPhone,
    receiver_phone_last4: receiverPhone.slice(-4),
    customer_id: customer?.customer_id ?? null,
    customer_name: customer?.name ?? null,
    updated_at: dayjs().subtract(index, 'hour').toISOString(),
    created_at: dayjs().subtract(index + 6, 'hour').toISOString(),
    exception_flags: exceptionFlags,
    mapping_missing: mappingMissing,
    inventory_shortage: inventoryShortage,
    shipping_failed: shippingFailed,
    tracking_no: trackingNo
  };
};

const specialMockOrders: OrderSummary[] = [
  {
    id: 'TEST-ORDER-NO-ITEMS',
    platform: Platform.Taobao,
    shop_id: 'SHOP-taobao-1',
    platform_order_id: 'PLAT-TEST-NO-ITEMS',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 1200,
    receiver_name: '无货品测试',
    receiver_phone: '13800009990',
    receiver_phone_last4: '9990',
    customer_id: mockCustomers[0]?.customer_id ?? null,
    customer_name: mockCustomers[0]?.name ?? null,
    updated_at: dayjs().subtract(40, 'minute').toISOString(),
    created_at: dayjs().subtract(2, 'hour').toISOString(),
    exception_flags: [],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-NO-LOGISTICS',
    platform: Platform.Pdd,
    shop_id: 'SHOP-pdd-1',
    platform_order_id: 'PLAT-TEST-NO-LOGISTICS',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 2580,
    receiver_name: '无快递测试',
    receiver_phone: '13800009991',
    receiver_phone_last4: '9991',
    customer_id: mockCustomers[1]?.customer_id ?? null,
    customer_name: mockCustomers[1]?.name ?? null,
    updated_at: dayjs().subtract(35, 'minute').toISOString(),
    created_at: dayjs().subtract(3, 'hour').toISOString(),
    exception_flags: [],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-MISSING-SKU',
    platform: Platform.Jd,
    shop_id: 'SHOP-jd-1',
    platform_order_id: 'PLAT-TEST-MISSING-SKU',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 3290,
    receiver_name: '无SKU测试',
    receiver_phone: '13800009992',
    receiver_phone_last4: '9992',
    customer_id: mockCustomers[2]?.customer_id ?? null,
    customer_name: mockCustomers[2]?.name ?? null,
    updated_at: dayjs().subtract(30, 'minute').toISOString(),
    created_at: dayjs().subtract(4, 'hour').toISOString(),
    exception_flags: ['MAPPING_MISSING'],
    mapping_missing: true,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: 'SF998877'
  },
  {
    id: 'TEST-ORDER-SHORTAGE',
    platform: Platform.Douyin,
    shop_id: 'SHOP-douyin-1',
    platform_order_id: 'PLAT-TEST-SHORTAGE',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 4590,
    receiver_name: '缺货测试',
    receiver_phone: '13800009993',
    receiver_phone_last4: '9993',
    customer_id: mockCustomers[3]?.customer_id ?? null,
    customer_name: mockCustomers[3]?.name ?? null,
    updated_at: dayjs().subtract(25, 'minute').toISOString(),
    created_at: dayjs().subtract(5, 'hour').toISOString(),
    exception_flags: ['INVENTORY_SHORTAGE'],
    mapping_missing: false,
    inventory_shortage: true,
    shipping_failed: false,
    tracking_no: 'SF998878'
  },
  {
    id: 'TEST-ORDER-FROZEN',
    platform: Platform.Taobao,
    shop_id: 'SHOP-taobao-2',
    platform_order_id: 'PLAT-TEST-FROZEN',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 3890,
    receiver_name: '冻结测试',
    receiver_phone: '13800009994',
    receiver_phone_last4: '9994',
    customer_id: mockCustomers[4]?.customer_id ?? null,
    customer_name: mockCustomers[4]?.name ?? null,
    updated_at: dayjs().subtract(20, 'minute').toISOString(),
    created_at: dayjs().subtract(6, 'hour').toISOString(),
    exception_flags: ['ORDER_FROZEN'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: 'SF998879'
  },
  {
    id: 'TEST-ORDER-ADDRESS-INVALID',
    platform: Platform.Jd,
    shop_id: 'SHOP-jd-2',
    platform_order_id: 'PLAT-TEST-ADDRESS',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 2680,
    receiver_name: '地址异常测试',
    receiver_phone: '13800009995',
    receiver_phone_last4: '9995',
    customer_id: mockCustomers[5]?.customer_id ?? null,
    customer_name: mockCustomers[5]?.name ?? null,
    updated_at: dayjs().subtract(18, 'minute').toISOString(),
    created_at: dayjs().subtract(7, 'hour').toISOString(),
    exception_flags: ['ADDRESS_INVALID'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-DUPLICATE',
    platform: Platform.Taobao,
    shop_id: 'SHOP-taobao-3',
    platform_order_id: 'PLAT-TEST-DUPLICATE',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'REJECTED',
    total_amount_cent: 3180,
    receiver_name: '重复订单测试',
    receiver_phone: '13800009996',
    receiver_phone_last4: '9996',
    customer_id: mockCustomers[6]?.customer_id ?? null,
    customer_name: mockCustomers[6]?.name ?? null,
    updated_at: dayjs().subtract(16, 'minute').toISOString(),
    created_at: dayjs().subtract(8, 'hour').toISOString(),
    exception_flags: ['DUPLICATE_ORDER', 'REVIEW_REJECTED'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-BLACKLIST',
    platform: Platform.Pdd,
    shop_id: 'SHOP-pdd-2',
    platform_order_id: 'PLAT-TEST-BLACKLIST',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'REJECTED',
    total_amount_cent: 4280,
    receiver_name: '黑名单客户',
    receiver_phone: '13800009997',
    receiver_phone_last4: '9997',
    customer_id: mockCustomers[7]?.customer_id ?? null,
    customer_name: mockCustomers[7]?.name ?? null,
    updated_at: dayjs().subtract(15, 'minute').toISOString(),
    created_at: dayjs().subtract(9, 'hour').toISOString(),
    exception_flags: ['BLACKLISTED_CUSTOMER', 'REVIEW_REJECTED'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-REVIEW-FLAGGED',
    platform: Platform.Douyin,
    shop_id: 'SHOP-douyin-2',
    platform_order_id: 'PLAT-TEST-FLAGGED',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 5680,
    receiver_name: '拦截复核',
    receiver_phone: '13800009998',
    receiver_phone_last4: '9998',
    customer_id: mockCustomers[8]?.customer_id ?? null,
    customer_name: mockCustomers[8]?.name ?? null,
    updated_at: dayjs().subtract(12, 'minute').toISOString(),
    created_at: dayjs().subtract(10, 'hour').toISOString(),
    exception_flags: ['REVIEW_FLAGGED'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-MERGED',
    platform: Platform.Taobao,
    shop_id: 'SHOP-taobao-4',
    platform_order_id: 'PLAT-TEST-MERGED',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'PENDING_SHIPMENT',
    review_status: 'PASSED',
    total_amount_cent: 7890,
    receiver_name: '合并订单',
    receiver_phone: '13800009999',
    receiver_phone_last4: '9999',
    customer_id: mockCustomers[9]?.customer_id ?? null,
    customer_name: mockCustomers[9]?.name ?? null,
    updated_at: dayjs().subtract(10, 'minute').toISOString(),
    created_at: dayjs().subtract(11, 'hour').toISOString(),
    exception_flags: ['ORDER_MERGED'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: 'SF998880'
  },
  {
    id: 'TEST-ORDER-SPLIT',
    platform: Platform.Jd,
    shop_id: 'SHOP-jd-3',
    platform_order_id: 'PLAT-TEST-SPLIT',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'PENDING_SHIPMENT',
    review_status: 'PASSED',
    total_amount_cent: 5120,
    receiver_name: '拆分订单',
    receiver_phone: '13800009980',
    receiver_phone_last4: '9980',
    customer_id: mockCustomers[10]?.customer_id ?? null,
    customer_name: mockCustomers[10]?.name ?? null,
    updated_at: dayjs().subtract(8, 'minute').toISOString(),
    created_at: dayjs().subtract(12, 'hour').toISOString(),
    exception_flags: ['ORDER_SPLIT'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: 'SF998881'
  },
  {
    id: 'TEST-ORDER-MANY-ITEMS',
    platform: Platform.Pdd,
    shop_id: 'SHOP-pdd-3',
    platform_order_id: 'PLAT-TEST-MANY-ITEMS',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 8990,
    receiver_name: '多品类订单',
    receiver_phone: '13800009981',
    receiver_phone_last4: '9981',
    customer_id: mockCustomers[11]?.customer_id ?? null,
    customer_name: mockCustomers[11]?.name ?? null,
    updated_at: dayjs().subtract(6, 'minute').toISOString(),
    created_at: dayjs().subtract(13, 'hour').toISOString(),
    exception_flags: [],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-FULFILLMENT-READY',
    platform: Platform.Taobao,
    shop_id: 'SHOP-taobao-5',
    platform_order_id: 'PLAT-TEST-FULFILLMENT-READY',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'PENDING_SHIPMENT',
    review_status: 'PASSED',
    total_amount_cent: 4680,
    receiver_name: '待提交发货样本',
    receiver_phone: '13800009982',
    receiver_phone_last4: '9982',
    customer_id: mockCustomers[12]?.customer_id ?? null,
    customer_name: mockCustomers[12]?.name ?? null,
    updated_at: dayjs().subtract(5, 'minute').toISOString(),
    created_at: dayjs().subtract(14, 'hour').toISOString(),
    exception_flags: [],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-RESERVE-FAILED',
    platform: Platform.Jd,
    shop_id: 'SHOP-jd-4',
    platform_order_id: 'PLAT-TEST-RESERVE-FAILED',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'PENDING_SHIPMENT',
    review_status: 'PASSED',
    total_amount_cent: 5290,
    receiver_name: '预占失败样本',
    receiver_phone: '13800009983',
    receiver_phone_last4: '9983',
    customer_id: mockCustomers[13]?.customer_id ?? null,
    customer_name: mockCustomers[13]?.name ?? null,
    updated_at: dayjs().subtract(4, 'minute').toISOString(),
    created_at: dayjs().subtract(15, 'hour').toISOString(),
    exception_flags: [],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  },
  {
    id: 'TEST-ORDER-MANUAL-HOLD',
    platform: Platform.Douyin,
    shop_id: 'SHOP-douyin-3',
    platform_order_id: 'PLAT-TEST-MANUAL-HOLD',
    platform_status: 'PENDING_SHIPMENT',
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    total_amount_cent: 3860,
    receiver_name: '人工挂起样本',
    receiver_phone: '13800009984',
    receiver_phone_last4: '9984',
    customer_id: mockCustomers[14]?.customer_id ?? null,
    customer_name: mockCustomers[14]?.name ?? null,
    updated_at: dayjs().subtract(3, 'minute').toISOString(),
    created_at: dayjs().subtract(16, 'hour').toISOString(),
    exception_flags: ['REVIEW_FLAGGED'],
    mapping_missing: false,
    inventory_shortage: false,
    shipping_failed: false,
    tracking_no: null
  }
];

const buildAfterSales = (index: number): AfterSaleCaseSummary => {
  const platform = pickPlatform(index);
  const caseId = `CASE-${index + 1}`;
  const refundAmountCent = 1999 + index * 120;
  return {
    case_id: caseId,
    platform,
    shop_id: `SHOP-${platform}-${(index % 4) + 1}`,
    case_status: index % 2 === 0 ? 'CLOSED' : 'OPEN',
    case_type: index % 2 === 0 ? 'refund' : 'return',
    platform_order_id: `ORDER-${platform}-${1000 + index}`,
    platform_case_id: `PLAT-CASE-${1000 + index}`,
    after_sale_id: `AFTER-${1000 + index}`,
    after_sale_type: index % 2 === 0 ? 'refund' : 'return',
    refund_state: index % 2 === 0 ? 'SUCCESS' : 'PROCESSING',
    after_sale_state: index % 2 === 0 ? 'CLOSED' : 'OPEN',
    refund_amount_cent: refundAmountCent,
    item_count: (index % 3) + 1,
    order_count: (index % 2) + 1,
    total_qty: (index % 4) + 1,
    created_at: dayjs().subtract(index, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString(),
    id: caseId
  };
};

const buildAfterSaleItem = (
  summary: AfterSaleCaseSummary,
  itemIndex: number
): AfterSaleCaseItem => ({
  tenant_id: mockTenantId,
  case_id: summary.case_id,
  order_id: summary.platform_order_id ?? `ORDER-${summary.platform}-${1100 + itemIndex}`,
  order_month: dayjs().subtract(itemIndex, 'month').format('YYYY-MM-01'),
  line_id: `LINE-${summary.case_id}-${itemIndex + 1}`,
  sku_id: `SKU-${1000 + itemIndex}`,
  title: `示例商品-${itemIndex + 1}`,
  return_stocked: itemIndex % 2 === 0,
  return_warehouse_id: itemIndex % 2 === 0 ? `WH-${(itemIndex % 3) + 1}` : null,
  return_at: itemIndex % 2 === 0 ? dayjs().subtract(itemIndex, 'day').toISOString() : null,
  qty: (itemIndex % 3) + 1,
  refund_state: summary.refund_state,
  after_sale_state: summary.after_sale_state,
  refund_amount_cent:
    summary.refund_amount_cent && summary.item_count
      ? Math.round(summary.refund_amount_cent / summary.item_count)
      : summary.refund_amount_cent ?? null,
  after_sale_type: summary.after_sale_type ?? summary.case_type ?? null
});

const buildAfterSaleDetail = (summary: AfterSaleCaseSummary): AfterSaleCaseDetail => {
  const items = Array.from({ length: summary.item_count ?? 1 }, (_, index) =>
    buildAfterSaleItem(summary, index)
  );
  return {
    case: {
      case_id: summary.case_id,
      tenant_id: mockTenantId,
      platform: summary.platform,
      shop_id: summary.shop_id,
      case_status: summary.case_status,
      case_type: summary.case_type ?? null,
      platform_order_id: summary.platform_order_id ?? null,
      platform_case_id: summary.platform_case_id ?? null,
      after_sale_id: summary.after_sale_id ?? null,
      after_sale_type: summary.after_sale_type ?? null,
      refund_state: summary.refund_state,
      after_sale_state: summary.after_sale_state,
      refund_amount_cent: summary.refund_amount_cent ?? null,
      item_count: summary.item_count,
      total_qty: summary.total_qty,
      order_count: summary.order_count,
      created_at: summary.created_at,
      updated_at: summary.updated_at
    },
    items
  };
};

const buildInventory = (index: number): InventorySnapshot => {
  const platform = pickPlatform(index);
  const warehouse = mockWarehouses[index % mockWarehouses.length];
  const reserved = 10 + (index % 5);
  const available = 120 - index;
  return {
    id: `INV-${index + 1}`,
    platform,
    sku_id: `SKU-${index + 1}`,
    sku_code: `SKU-ERP-${1000 + index}`,
    barcode: `BAR-${1000 + index}`,
    warehouse_id: warehouse?.warehouse_id ?? 'WH-DEFAULT',
    on_hand: available + reserved,
    reserved,
    frozen: 0,
    in_transit: 0,
    available,
    updated_at: dayjs().subtract(index, 'minute').toISOString()
  };
};

const buildProduct = (index: number): Product => ({
  product_id: `PROD-${index + 1}`,
  tenant_id: mockTenantId,
  product_code: `CODE-${1000 + index}`,
  name: `示例商品-${index + 1}`,
  alias: index % 2 === 0 ? `别名-${index + 1}` : null,
  brand: index % 2 === 0 ? 'Default Brand' : null,
  category: index % 3 === 0 ? 'Category-A' : 'Category-B',
  base_unit: '件',
  second_unit: index % 2 === 0 ? '箱' : null,
  default_supplier_id: null,
  default_warehouse_id: null,
  primary_barcode: `PB-${1000 + index}`,
  product_tags: [`tag-${index % 3}`],
  remark: index % 3 === 0 ? '默认备注' : null,
  warning_stock: {
    upper: { mode: 'FIXED', value: 200 + index * 5 },
    lower: { mode: 'FIXED', value: 20 + index }
  },
  batch_enabled: index % 3 === 0,
  shelf_life_days: index % 3 === 0 ? 365 : null,
  forbid_sale_days: index % 3 === 0 ? 30 : null,
  serial_enabled: index % 4 === 0,
  serial_push_platform: index % 5 === 0,
  attrs: { color: index % 2 === 0 ? 'Red' : 'Blue' },
  allow_negative_stock: index % 2 === 0,
  status: 'ACTIVE',
  created_at: dayjs().subtract(index, 'day').toISOString(),
  updated_at: dayjs().subtract(index, 'hour').toISOString()
});

const mockProductSeeds = Array.from({ length: 12 }, (_, index) => buildProduct(index));
const specialRuntimeGovernanceProducts: Product[] = [
  {
    product_id: 'PROD-FOOD-GOV-001',
    tenant_id: mockTenantId,
    product_code: 'FOOD-GOV-001',
    alias: '食品运行治理样本',
    name: '食品批次治理样品',
    brand: '治理样本品牌',
    category: '食品治理',
    base_unit: '件',
    second_unit: null,
    default_supplier_id: null,
    default_warehouse_id: null,
    primary_barcode: 'PB-FOOD-GOV-001',
    product_tags: ['runtime-governance', 'food'],
    remark: '用于食品批次运行期治理验证',
    warning_stock: {
      upper: { mode: 'FIXED', value: 180 },
      lower: { mode: 'FIXED', value: 12 }
    },
    batch_enabled: true,
    shelf_life_days: 180,
    forbid_sale_days: 7,
    serial_enabled: false,
    serial_push_platform: false,
    attrs: {
      industry_profile: 'FOOD',
      production_date_enabled: true,
      shelf_life_unit: 'DAY',
      near_expiry_warning_days: 15
    },
    allow_negative_stock: false,
    status: 'ACTIVE',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'hour').toISOString()
  },
  {
    product_id: 'PROD-FOOD-GOV-002',
    tenant_id: mockTenantId,
    product_code: 'FOOD-GOV-002',
    alias: '食品禁批次样本',
    name: '食品禁建批次样品',
    brand: '治理样本品牌',
    category: '食品治理',
    base_unit: '件',
    second_unit: null,
    default_supplier_id: null,
    default_warehouse_id: null,
    primary_barcode: 'PB-FOOD-GOV-002',
    product_tags: ['runtime-governance', 'food'],
    remark: '用于验证未启用批次管理的食品 SKU',
    warning_stock: {
      upper: { mode: 'FIXED', value: 120 },
      lower: { mode: 'FIXED', value: 8 }
    },
    batch_enabled: false,
    shelf_life_days: 90,
    forbid_sale_days: 3,
    serial_enabled: false,
    serial_push_platform: false,
    attrs: {
      industry_profile: 'FOOD',
      production_date_enabled: true,
      shelf_life_unit: 'DAY',
      near_expiry_warning_days: 10
    },
    allow_negative_stock: false,
    status: 'ACTIVE',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'hour').toISOString()
  },
  {
    product_id: 'PROD-DIGITAL-GOV-001',
    tenant_id: mockTenantId,
    product_code: 'DIGITAL-GOV-001',
    alias: '数码运行治理样本',
    name: '数码序列治理样品',
    brand: '治理样本品牌',
    category: '数码治理',
    base_unit: '件',
    second_unit: null,
    default_supplier_id: null,
    default_warehouse_id: null,
    primary_barcode: 'PB-DIGITAL-GOV-001',
    product_tags: ['runtime-governance', 'digital'],
    remark: '用于数码序列号运行期治理验证',
    warning_stock: {
      upper: { mode: 'FIXED', value: 80 },
      lower: { mode: 'FIXED', value: 5 }
    },
    batch_enabled: false,
    shelf_life_days: null,
    forbid_sale_days: null,
    serial_enabled: true,
    serial_push_platform: true,
    attrs: {
      industry_profile: 'DIGITAL',
      serial_tracking_mode: 'DEVICE',
      serial_outbound_required: true,
      serial_inbound_unique_required: true
    },
    allow_negative_stock: false,
    status: 'ACTIVE',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(50, 'minute').toISOString()
  },
  {
    product_id: 'PROD-DIGITAL-GOV-002',
    tenant_id: mockTenantId,
    product_code: 'DIGITAL-GOV-002',
    alias: '数码禁序列号样本',
    name: '数码禁生成序列样品',
    brand: '治理样本品牌',
    category: '数码治理',
    base_unit: '件',
    second_unit: null,
    default_supplier_id: null,
    default_warehouse_id: null,
    primary_barcode: 'PB-DIGITAL-GOV-002',
    product_tags: ['runtime-governance', 'digital'],
    remark: '用于验证未启用序列号管理的数码 SKU',
    warning_stock: {
      upper: { mode: 'FIXED', value: 60 },
      lower: { mode: 'FIXED', value: 4 }
    },
    batch_enabled: false,
    shelf_life_days: null,
    forbid_sale_days: null,
    serial_enabled: false,
    serial_push_platform: false,
    attrs: {
      industry_profile: 'DIGITAL',
      serial_tracking_mode: 'DEVICE',
      serial_outbound_required: true,
      serial_inbound_unique_required: true
    },
    allow_negative_stock: false,
    status: 'ACTIVE',
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(90, 'minute').toISOString()
  }
];
const specialMockProducts: Product[] = [
  {
    product_id: 'PROD-NEG-001',
    tenant_id: mockTenantId,
    product_code: 'NEG-001',
    alias: '允许负库存',
    name: '允许负库存样品',
    brand: '测试品牌',
    category: '负库存测试',
    base_unit: '件',
    second_unit: null,
    default_supplier_id: null,
    default_warehouse_id: null,
    primary_barcode: 'PB-NEG-001',
    product_tags: ['tag-1'],
    remark: '允许负库存出库',
    warning_stock: {
      upper: { mode: 'FIXED', value: 300 },
      lower: { mode: 'FIXED', value: 30 }
    },
    batch_enabled: false,
    shelf_life_days: null,
    forbid_sale_days: null,
    serial_enabled: false,
    serial_push_platform: false,
    attrs: { color: 'Black', note: '允许负库存出库' },
    allow_negative_stock: true,
    status: 'ACTIVE',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'hour').toISOString()
  },
  {
    product_id: 'PROD-NEG-002',
    tenant_id: mockTenantId,
    product_code: 'NEG-002',
    alias: '禁止负库存',
    name: '禁止负库存样品',
    brand: '测试品牌',
    category: '负库存测试',
    base_unit: '件',
    second_unit: null,
    default_supplier_id: null,
    default_warehouse_id: null,
    primary_barcode: 'PB-NEG-002',
    product_tags: ['tag-2'],
    remark: '禁止负库存出库',
    warning_stock: {
      upper: { mode: 'FIXED', value: 160 },
      lower: { mode: 'FIXED', value: 20 }
    },
    batch_enabled: true,
    shelf_life_days: 180,
    forbid_sale_days: 20,
    serial_enabled: false,
    serial_push_platform: false,
    attrs: { color: 'White', note: '禁止负库存出库' },
    allow_negative_stock: false,
    status: 'ACTIVE',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'hour').toISOString()
  },
  {
    product_id: 'PROD-NEG-003',
    tenant_id: mockTenantId,
    product_code: 'NEG-003',
    alias: '停用商品',
    name: '停用负库存样品',
    brand: '测试品牌',
    category: '负库存测试',
    base_unit: '件',
    second_unit: null,
    default_supplier_id: null,
    default_warehouse_id: null,
    primary_barcode: 'PB-NEG-003',
    product_tags: ['tag-3'],
    remark: '停用商品',
    warning_stock: {
      upper: { mode: 'FIXED', value: 120 },
      lower: { mode: 'FIXED', value: 10 }
    },
    batch_enabled: false,
    shelf_life_days: null,
    forbid_sale_days: null,
    serial_enabled: true,
    serial_push_platform: true,
    attrs: { color: 'Gray', note: '停用商品' },
    allow_negative_stock: true,
    status: 'DISABLED',
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(4, 'hour').toISOString()
  }
];
const mockProducts = [
  ...specialRuntimeGovernanceProducts,
  ...specialMockProducts,
  ...mockProductSeeds
];

const mockProductCategories = [
  {
    category_id: 'PCAT-001',
    tenant_id: mockTenantId,
    name: 'Category-A',
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    category_id: 'PCAT-002',
    tenant_id: mockTenantId,
    name: 'Category-B',
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  },
  {
    category_id: 'PCAT-003',
    tenant_id: mockTenantId,
    name: 'Category-C',
    created_at: dayjs().subtract(6, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockProductBrands = [
  {
    brand_id: 'PBR-001',
    tenant_id: mockTenantId,
    name: 'Default Brand',
    created_at: dayjs().subtract(12, 'day').toISOString(),
    updated_at: dayjs().subtract(4, 'day').toISOString()
  },
  {
    brand_id: 'PBR-002',
    tenant_id: mockTenantId,
    name: '测试品牌',
    created_at: dayjs().subtract(9, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  }
];

const mockProductUnits = [
  {
    unit_id: 'PUNIT-001',
    tenant_id: mockTenantId,
    name: '件',
    created_at: dayjs().subtract(20, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'day').toISOString()
  },
  {
    unit_id: 'PUNIT-002',
    tenant_id: mockTenantId,
    name: '箱',
    created_at: dayjs().subtract(18, 'day').toISOString(),
    updated_at: dayjs().subtract(4, 'day').toISOString()
  },
  {
    unit_id: 'PUNIT-003',
    tenant_id: mockTenantId,
    name: '包',
    created_at: dayjs().subtract(16, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  }
];

const mockProductWarehouses = mockProducts.slice(0, 6).flatMap((product, index) => {
  const primary = mockWarehouses[index % mockWarehouses.length];
  const secondary = mockWarehouses[(index + 1) % mockWarehouses.length];
  const createdAt = dayjs().subtract(index, 'day').toISOString();
  const updatedAt = dayjs().subtract(index, 'hour').toISOString();
  return [
    {
      tenant_id: mockTenantId,
      product_id: product.product_id,
      warehouse_id: primary?.warehouse_id ?? `WH-${index + 1}`,
      warehouse_code: primary?.code ?? `WH-${index + 1}`,
      warehouse_name: primary?.name ?? `仓库-${index + 1}`,
      is_default: true,
      allow_negative_stock: product.allow_negative_stock,
      created_at: createdAt,
      updated_at: updatedAt
    },
    {
      tenant_id: mockTenantId,
      product_id: product.product_id,
      warehouse_id: secondary?.warehouse_id ?? `WH-${index + 2}`,
      warehouse_code: secondary?.code ?? `WH-${index + 2}`,
      warehouse_name: secondary?.name ?? `仓库-${index + 2}`,
      is_default: false,
      allow_negative_stock: false,
      created_at: createdAt,
      updated_at: updatedAt
    }
  ];
});

const buildSku = (index: number): Sku => ({
  sku_id: `SKU-${index + 1}`,
  tenant_id: mockTenantId,
  product_id: mockProducts[index % mockProducts.length]?.product_id ?? 'PROD-1',
  sku_code: `SKU-ERP-${1000 + index}`,
  barcode: `BAR-${1000 + index}`,
  note: index % 4 === 0 ? `sku-note-${index + 1}` : null,
  name: `示例 SKU-${index + 1}`,
  spec: { spec: `规格-${(index % 5) + 1}` },
  tags: [`tag-${index % 4}`],
  image_urls:
    index % 3 === 0
      ? [`https://picsum.photos/seed/sku-${index}/120/120`]
      : [],
  weight_gram: 300 + index * 15,
  length_mm: 200 + (index % 6) * 10,
  width_mm: 300 + (index % 4) * 10,
  height_mm: 100 + (index % 3) * 10,
  volume_cm3: 1200 + index * 3,
  cost_cent: 1200 + index * 35,
  sku_kind: index % 6 === 0 ? 'KIT' : 'NORMAL',
  is_active: index % 2 === 0,
  status: index % 2 === 0 ? 'ACTIVE' : 'INACTIVE',
  batch_enabled: index % 3 === 0,
  serial_enabled: index % 4 === 0,
  created_at: dayjs().subtract(index, 'day').toISOString(),
  updated_at: dayjs().subtract(index, 'hour').toISOString()
});

const shipmentCarrierCodes = ['SF', 'YTO', 'ZTO', 'JD'];
const shipmentWarehouses = ['WH-east', 'WH-south', 'WH-north'];

const buildShipment = (index: number): ShipmentSummary => {
  const platform = pickPlatform(index);
  const status: ShipmentSummary['status'] =
    index % 4 === 0 ? 'FAILED' : index % 4 === 1 ? 'SUCCESS' : 'REQUESTED';
  return {
    shipment_id: `SHIP-${1000 + index}`,
    order_id: `ORDER-${index + 1}`,
    platform,
    platform_order_id: `PLAT-${platform.toUpperCase()}-${1000 + index}`,
    shop_id: `SHOP-${platform}-${(index % 4) + 1}`,
    warehouse_id: shipmentWarehouses[index % shipmentWarehouses.length] ?? 'WH-east',
    carrier_code: shipmentCarrierCodes[index % shipmentCarrierCodes.length] ?? 'SF',
    tracking_no: `TRACK-${1000 + index}`,
    status,
    requested_at: dayjs().subtract(index, 'hour').toISOString(),
    shipped_at: status === 'SUCCESS' ? dayjs().subtract(index - 1, 'hour').toISOString() : null,
    updated_at: dayjs().subtract(index, 'minute').toISOString(),
    last_error: status === 'FAILED' ? '平台接口超时' : null,
    weight_gram: 500 + index * 10
  };
};

const baseMockOrders = Array.from({ length: 72 }, (_, index) => buildOrder(index));
const reviewPendingMockOrders = Array.from({ length: 120 }, (_, index) => {
  const order = buildOrder(index + baseMockOrders.length);
  const exceptionFlags = (order.exception_flags ?? []).filter(
    (flag) => flag !== 'REVIEW_REJECTED'
  );
  return {
    ...order,
    oms_status: 'REVIEW_PENDING',
    review_status: 'PENDING',
    platform_status: 'PENDING_SHIPMENT',
    exception_flags: exceptionFlags
  };
});
const prioritizedOrderIndexes = new Set([0, 1, 3]);
const mockOrders = [
  baseMockOrders[0],
  baseMockOrders[1],
  baseMockOrders[3],
  ...baseMockOrders.filter((_, index) => !prioritizedOrderIndexes.has(index)),
  ...reviewPendingMockOrders,
  ...specialMockOrders
];
const normalizedExceptionSeedOrderIds = mockOrders
  .filter(
    (order) =>
      !order.id.startsWith('TEST-ORDER-') &&
      !String(order.platform_order_id ?? '').includes('TEST')
  )
  .map((order) => order.id);
const fallbackExceptionSeedOrderId = normalizedExceptionSeedOrderIds[0] ?? mockOrders[0]?.id ?? 'DT-EXCEPTION-FALLBACK';
const getExceptionSeedOrderId = (index: number) =>
  normalizedExceptionSeedOrderIds[index] ?? fallbackExceptionSeedOrderId;
const mockOmsMainChainSeeds: MockOmsMainChainSeed[] = [
  {
    order_id: getExceptionSeedOrderId(0),
    audit_status: 'REVIEW_REQUIRED',
    exception_status: 'OPEN',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    reserve_precheck_passed: false,
    reserve_precheck_reasons: ['商品映射缺失，不能提交发货'],
    target_warehouse_id: 'WH-DEFAULT',
    blocking_reason: '商品映射缺失',
    action_logs: [
      {
        action: 'FLAG',
        message: '系统命中商品映射缺失规则，进入异常池',
        operator_id: null,
        operator_name: '系统',
        created_at: dayjs().subtract(28, 'minute').toISOString()
      }
    ]
  },
  {
    order_id: getExceptionSeedOrderId(1),
    audit_status: 'REVIEW_REQUIRED',
    exception_status: 'OPEN',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    reserve_precheck_passed: false,
    reserve_precheck_reasons: ['库存不足，不能完成预占'],
    target_warehouse_id: 'WH-EAST',
    blocking_reason: '库存不足',
    action_logs: [
      {
        action: 'FLAG',
        message: '系统命中库存不足规则，进入异常池',
        operator_id: null,
        operator_name: '系统',
        created_at: dayjs().subtract(26, 'minute').toISOString()
      }
    ]
  },
  {
    order_id: getExceptionSeedOrderId(2),
    audit_status: 'REVIEW_REQUIRED',
    exception_status: 'OPEN',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    reserve_precheck_passed: false,
    reserve_precheck_reasons: ['收件地址不完整，不能提交发货'],
    target_warehouse_id: 'WH-DEFAULT',
    blocking_reason: '地址信息不完整',
    action_logs: [
      {
        action: 'FLAG',
        message: '系统命中地址校验规则，进入异常池',
        operator_id: null,
        operator_name: '系统',
        created_at: dayjs().subtract(24, 'minute').toISOString()
      }
    ]
  },
  {
    order_id: getExceptionSeedOrderId(3),
    audit_status: 'UNDER_REVIEW',
    exception_status: 'PROCESSING',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    reserve_precheck_passed: false,
    reserve_precheck_reasons: ['风控拦截待人工复核'],
    target_warehouse_id: 'WH-DEFAULT',
    blocking_reason: '风控拦截待人工复核',
    assignee_id: 'USER-RISK-001',
    assignee_name: '风控专员',
    action_logs: [
      {
        action: 'CLAIM',
        message: '风控专员已领取异常单',
        operator_id: 'USER-RISK-001',
        operator_name: '风控专员',
        created_at: dayjs().subtract(20, 'minute').toISOString()
      }
    ]
  },
  {
    order_id: getExceptionSeedOrderId(4),
    audit_status: 'UNDER_REVIEW',
    exception_status: 'ASSIGNED',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    reserve_precheck_passed: false,
    reserve_precheck_reasons: ['客户资料待补充，当前挂起'],
    target_warehouse_id: 'WH-EAST',
    blocking_reason: '客户资料待补充',
    assignee_id: 'USER-OMS-002',
    assignee_name: '订单复核员',
    action_logs: [
      {
        action: 'HOLD',
        message: '订单已挂起，等待客户补充资料',
        operator_id: 'USER-OMS-002',
        operator_name: '订单复核员',
        created_at: dayjs().subtract(18, 'minute').toISOString()
      }
    ]
  },
  {
    order_id: getExceptionSeedOrderId(5),
    audit_status: 'REVIEW_PASSED',
    exception_status: 'NONE',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    reserve_precheck_passed: true,
    reserve_precheck_reasons: [],
    target_warehouse_id: 'WH-DEFAULT',
    blocking_reason: '可直接提交发货',
    action_logs: [
      {
        action: 'APPROVE',
        message: '订单复核通过，可提交发货',
        operator_id: 'USER-OMS-001',
        operator_name: '订单审核员',
        created_at: dayjs().subtract(14, 'minute').toISOString()
      }
    ]
  },
  {
    order_id: getExceptionSeedOrderId(6),
    audit_status: 'REVIEW_PASSED',
    exception_status: 'NONE',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    reserve_precheck_passed: false,
    reserve_precheck_reasons: ['库存预占失败，存在并发锁定'],
    target_warehouse_id: 'WH-EAST',
    blocking_reason: '库存预占失败',
    action_logs: [
      {
        action: 'APPROVE',
        message: '订单复核通过，待库存预占',
        operator_id: 'USER-OMS-001',
        operator_name: '订单审核员',
        created_at: dayjs().subtract(12, 'minute').toISOString()
      }
    ]
  }
];
const mockOrderReceiverAddresses = new Map<string, string>(
  mockOrders.map((order, index) => [order.id, `上海市浦东新区示例路 ${100 + index} 号`])
);
const mockAfterSales = Array.from({ length: 24 }, (_, index) => buildAfterSales(index));
const mockAfterSaleDetails = new Map(
  mockAfterSales.map((summary) => [summary.case_id, buildAfterSaleDetail(summary)])
);
const mockInventory = Array.from({ length: 36 }, (_, index) => buildInventory(index));
const mockInventoryLedgers: InventoryLedger[] = [];
const mockInventoryActionKeys = new Set<string>();
const mockStockSyncJobs: StockSyncJob[] = Array.from({ length: 8 }, (_, index) => ({
  job_id: `SYNC-${1000 + index}`,
  shop_id: `SHOP-${pickPlatform(index)}-${(index % 4) + 1}`,
  sku_id: `SKU-${1000 + index}`,
  warehouse_id: mockWarehouses[index % mockWarehouses.length]?.warehouse_id ?? 'WH-DEFAULT',
  target_qty: 80 + index * 2,
  status:
    index % 4 === 0 ? 'FAILED' : index % 4 === 1 ? 'PENDING' : index % 4 === 2 ? 'SENDING' : 'SENT',
  attempts: index % 4,
  last_error: index % 4 === 0 ? '平台限流' : null,
  created_at: dayjs().subtract(index + 1, 'hour').toISOString(),
  updated_at: dayjs().subtract(index, 'hour').toISOString()
}));
const specialRuntimeGovernanceSkus: Sku[] = [
  {
    sku_id: 'SKU-FOOD-GOV-001',
    tenant_id: mockTenantId,
    product_id: 'PROD-FOOD-GOV-001',
    sku_code: 'SKU-FOOD-GOV-001',
    barcode: 'BAR-FOOD-GOV-001',
    note: '食品批次治理启用样本',
    name: '食品批次治理SKU',
    spec: { spec: '食品治理标准装' },
    tags: ['runtime-governance', 'food'],
    image_urls: [],
    weight_gram: 520,
    length_mm: 180,
    width_mm: 120,
    height_mm: 90,
    volume_cm3: 1944,
    cost_cent: 2580,
    sku_kind: 'NORMAL',
    is_active: true,
    status: 'ACTIVE',
    batch_enabled: true,
    serial_enabled: false,
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'hour').toISOString()
  },
  {
    sku_id: 'SKU-FOOD-NOBATCH-001',
    tenant_id: mockTenantId,
    product_id: 'PROD-FOOD-GOV-002',
    sku_code: 'SKU-FOOD-NOBATCH-001',
    barcode: 'BAR-FOOD-NOBATCH-001',
    note: '食品批次治理禁用样本',
    name: '食品禁批次SKU',
    spec: { spec: '食品治理禁建批次装' },
    tags: ['runtime-governance', 'food'],
    image_urls: [],
    weight_gram: 480,
    length_mm: 170,
    width_mm: 110,
    height_mm: 88,
    volume_cm3: 1645,
    cost_cent: 2180,
    sku_kind: 'NORMAL',
    is_active: true,
    status: 'ACTIVE',
    batch_enabled: false,
    serial_enabled: false,
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'hour').toISOString()
  },
  {
    sku_id: 'SKU-DIGITAL-GOV-001',
    tenant_id: mockTenantId,
    product_id: 'PROD-DIGITAL-GOV-001',
    sku_code: 'SKU-DIGITAL-GOV-001',
    barcode: 'BAR-DIGITAL-GOV-001',
    note: '数码序列治理启用样本',
    name: '数码序列治理SKU',
    spec: { spec: '设备追踪版' },
    tags: ['runtime-governance', 'digital'],
    image_urls: [],
    weight_gram: 960,
    length_mm: 220,
    width_mm: 140,
    height_mm: 60,
    volume_cm3: 1848,
    cost_cent: 56800,
    sku_kind: 'NORMAL',
    is_active: true,
    status: 'ACTIVE',
    batch_enabled: false,
    serial_enabled: true,
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(40, 'minute').toISOString()
  },
  {
    sku_id: 'SKU-DIGITAL-NOSERIAL-001',
    tenant_id: mockTenantId,
    product_id: 'PROD-DIGITAL-GOV-002',
    sku_code: 'SKU-DIGITAL-NOSERIAL-001',
    barcode: 'BAR-DIGITAL-NOSERIAL-001',
    note: '数码序列治理禁用样本',
    name: '数码禁序列号SKU',
    spec: { spec: '设备追踪禁用版' },
    tags: ['runtime-governance', 'digital'],
    image_urls: [],
    weight_gram: 910,
    length_mm: 210,
    width_mm: 135,
    height_mm: 58,
    volume_cm3: 1644,
    cost_cent: 49800,
    sku_kind: 'NORMAL',
    is_active: true,
    status: 'ACTIVE',
    batch_enabled: false,
    serial_enabled: false,
    created_at: dayjs().subtract(3, 'day').toISOString(),
    updated_at: dayjs().subtract(70, 'minute').toISOString()
  }
];
const mockSkus = [...specialRuntimeGovernanceSkus, ...Array.from({ length: 42 }, (_, index) => buildSku(index))];
const buildSkuBatch = (index: number): SkuBatch => {
  const sku = mockSkus[index % mockSkus.length];
  return {
    batch_id: `BATCH-${index + 1}`,
    tenant_id: mockTenantId,
    sku_id: sku?.sku_id ?? 'SKU-1',
    batch_no: `BATCH-NO-${1000 + index}`,
    expire_at: dayjs().add(index + 10, 'day').toISOString(),
    created_at: dayjs().subtract(index, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString()
  };
};
const mockSkuBatches = Array.from({ length: 16 }, (_, index) => buildSkuBatch(index));
const buildSkuSerial = (index: number): SkuSerial => {
  const sku = mockSkus[index % mockSkus.length];
  return {
    serial_id: `SERIAL-${index + 1}`,
    tenant_id: mockTenantId,
    sku_id: sku?.sku_id ?? 'SKU-1',
    serial_no: `SN-${1000 + index}`,
    status: index % 3 === 0 ? 'ALLOCATED' : index % 3 === 1 ? 'SHIPPED' : 'IN_STOCK',
    ref_doc_type: index % 3 === 0 ? 'SHIPMENT' : null,
    ref_doc_id: index % 3 === 0 ? `SHIP-${1000 + index}` : null,
    created_at: dayjs().subtract(index, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString()
  };
};
const mockSkuSerials = Array.from({ length: 24 }, (_, index) => buildSkuSerial(index));
const buildMapping = (index: number): MappingRecord => {
  const platform = pickPlatform(index);
  const sku = mockSkus[index % mockSkus.length] ?? mockSkus[0];
  return {
    mapping_id: `MAP-${1000 + index}`,
    tenant_id: mockTenantId,
    platform,
    shop_id: `SHOP-${platform}-${(index % 4) + 1}`,
    platform_sku_id: `PLAT-SKU-${2000 + index}`,
    outer_sku_id: index % 3 === 0 ? `OUTER-${3000 + index}` : null,
    sku_id: sku.sku_id,
    sku_code: sku.sku_code,
    barcode: sku.barcode ?? null,
    created_at: dayjs().subtract(index, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString()
  };
};
const mockMappings: MappingRecord[] = Array.from({ length: 24 }, (_, index) =>
  buildMapping(index)
);
const mockSkuSearchIndex: SkuSearchItem[] = mockSkus.map((sku, index) => ({
  sku_id: sku.sku_id,
  sku_code: sku.sku_code,
  name: sku.name ?? null,
  barcode: sku.barcode ?? null,
  sku_kind: sku.sku_kind ?? (index % 6 === 0 ? 'KIT' : 'NORMAL')
}));
const mockKitSkus = mockSkuSearchIndex.filter((sku) => sku.sku_kind === 'KIT');
const mockStandardSkus = mockSkuSearchIndex.filter((sku) => sku.sku_kind !== 'KIT');
const mockKitDetails = new Map<string, KitDetail>(
  mockKitSkus.map((kit, index) => {
    const base = index * 3;
    const components = Array.from({ length: 3 }, (_, componentIndex) => {
      const sku = mockStandardSkus[(base + componentIndex) % mockStandardSkus.length];
      return {
        component_sku_id: sku.sku_id,
        component_sku_code: sku.sku_code,
        component_name: sku.name ?? null,
        component_barcode: sku.barcode ?? null,
        qty: componentIndex + 1
      };
    });
    return [
      kit.sku_id,
      {
        kit_sku: kit,
        components,
        updated_at: dayjs().subtract(index, 'day').toISOString()
      }
    ];
  })
);

const resolveMockSkuCost = (sku: Sku): number | null => {
  if (sku.sku_kind !== 'KIT') {
    return sku.cost_cent ?? null;
  }
  const detail = mockKitDetails.get(sku.sku_id);
  if (!detail) {
    return 0;
  }
  return detail.components.reduce((sum, component) => {
    const componentSku = mockSkus.find((item) => item.sku_id === component.component_sku_id);
    const componentCost = componentSku?.cost_cent ?? 0;
    return sum + componentCost * component.qty;
  }, 0);
};
const mockPrintTemplates: PrintTemplate[] = [
  {
    template_id: 'TPL-TB-1',
    platform: Platform.Taobao,
    shop_id: 'SHOP-taobao-1',
    template_type: 'SHIPPING_LABEL',
    version: 1,
    content: '<div>淘宝面单</div>',
    enabled: true,
    created_at: dayjs().subtract(15, 'day').toISOString(),
    updated_at: dayjs().subtract(6, 'day').toISOString()
  },
  {
    template_id: 'TPL-PDD-1',
    platform: Platform.Pdd,
    shop_id: 'SHOP-pdd-1',
    template_type: 'SHIPPING_LABEL',
    version: 1,
    content: '<div>拼多多默认面单</div>',
    enabled: true,
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    template_id: 'TPL-PDD-2',
    platform: Platform.Pdd,
    shop_id: 'SHOP-pdd-1',
    template_type: 'SHIPPING_LABEL',
    version: 2,
    content: '<div>拼多多新版面单</div>',
    enabled: false,
    created_at: dayjs().subtract(5, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    template_id: 'TPL-JD-1',
    platform: Platform.Jd,
    shop_id: 'SHOP-jd-1',
    template_type: 'SHIPPING_LABEL',
    version: 1,
    content: '<div>京东面单</div>',
    enabled: true,
    created_at: dayjs().subtract(12, 'day').toISOString(),
    updated_at: dayjs().subtract(4, 'day').toISOString()
  },
  {
    template_id: 'TPL-DY-1',
    platform: Platform.Douyin,
    shop_id: 'SHOP-douyin-1',
    template_type: 'SHIPPING_LABEL',
    version: 1,
    content: '<div>抖音面单</div>',
    enabled: true,
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  }
];
const mockPrintJobs: PrintJobDetail[] = [];
const mockShipments: ShipmentSummary[] = Array.from({ length: 28 }, (_, index) =>
  buildShipment(index)
);

const applyPagination = <T>(items: T[], offset = 0, limit = 20): T[] => {
  const start = Math.max(0, offset);
  return items.slice(start, start + limit);
};





















































































const buildSalesShopConfigView = (shop: Shop): SalesShopConfig => {
  const stored = mockSalesShopConfigs.get(shop.shop_id);
  const base = stored ?? buildSalesShopConfig(shop);
  const merged: SalesShopConfig = {
    ...base,
    shop_id: shop.shop_id,
    platform: shop.platform,
    platform_shop_id: shop.platform_shop_id,
    name: shop.name,
    status: shop.status
  };
  mockSalesShopConfigs.set(shop.shop_id, merged);
  return merged;
};















const buildOrderItem = (order: OrderSummary, index: number): OrderItem => {
  const mappingMissing = Boolean(order.mapping_missing) && index === 0;
  return {
    id: `${order.id}-ITEM-${index + 1}`,
    platform_sku_id: `PLAT-SKU-${100 + index}`,
    outer_sku_id: index % 2 === 0 ? `OUTER-${200 + index}` : null,
    title: `示例商品-${index + 1}`,
    quantity: (index % 3) + 1,
    price_cent: 990 + index * 50,
    mapping_missing: mappingMissing
  };
};











type AfterSalesOpsListQuery = {
  status?: string;
  after_sales_type?: string;
  order_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

type AfterSalesInterceptListQuery = {
  status?: AfterSalesInterceptStatus | string;
  order_id?: string;
  tracking_no?: string;
  requested_from?: string;
  requested_to?: string;
  offset?: number;
  limit?: number;
};

type AfterSalesInterceptCreatePayload = {
  order_id: string;
  after_sales_id?: string | null;
  carrier_name?: string | null;
  tracking_no?: string | null;
  status?: AfterSalesInterceptStatus;
  reason?: string | null;
  requested_at?: string | null;
};

type AfterSalesInspectionListQuery = {
  after_sales_id?: string;
  sku_id?: string;
  warehouse_id?: string;
  result_code?: AfterSalesInspectionResult | string;
  inspected_from?: string;
  inspected_to?: string;
  offset?: number;
  limit?: number;
};

type AfterSalesInspectionCreatePayload = {
  after_sales_id: string;
  sku_id: string;
  warehouse_id?: string | null;
  qty: number;
  result_code?: AfterSalesInspectionResult | string;
  inspector_name?: string | null;
  inspected_at?: string | null;
};

type AfterSalesInspectionUpdatePayload = {
  warehouse_id?: string | null;
  qty?: number | null;
  result_code?: AfterSalesInspectionResult | string;
  inspector_name?: string | null;
  inspected_at?: string | null;
};

type AfterSalesSlaRuleListQuery = {
  after_sales_type?: string;
  status?: AfterSalesSlaStatus | string;
  offset?: number;
  limit?: number;
};

type AfterSalesSlaRuleCreatePayload = {
  after_sales_type: string;
  response_hours: number;
  resolution_hours: number;
  escalation_role?: string | null;
  status?: AfterSalesSlaStatus | string;
};

type AfterSalesSlaRuleUpdatePayload = {
  after_sales_type?: string;
  response_hours?: number | null;
  resolution_hours?: number | null;
  escalation_role?: string | null;
  status?: AfterSalesSlaStatus | string;
};

type RefundDocListQuery = {
  status?: string;
  after_sales_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

type AfterSalesImportJobListQuery = {
  status?: string;
  offset?: number;
  limit?: number;
};

type AfterSalesImportJob = {
  job_id: string;
  file_name?: string | null;
  file_type: string;
  status: string;
  progress: number;
  total_rows: number;
  success_rows: number;
  failed_rows: number;
  attempts: number;
  last_error: string;
  created_at: string;
  updated_at: string;
};

type AfterSalesOpsCreatePayload = {
  order_id: string;
  type: string;
  reason_code?: string;
  amount_cent?: number;
  items: Array<{ sku_id: string; qty: number; refund_cent: number }>;
};

type AfterSalesOpsRecord = {
  id: string;
  tenant_id: string;
  order_id: string;
  type: string;
  status: string;
  reason_code?: string | null;
  amount_cent: number;
  created_at: string;
  updated_at: string;
};

const buildAfterSalesOpsRecord = (index: number): AfterSalesOpsRecord => {
  const now = new Date(Date.now() - index * 3600 * 1000).toISOString();
  const types = ['RETURN_REFUND', 'REFUND_ONLY', 'EXCHANGE', 'RESEND'];
  return {
    id: `AS-${index + 1}`,
    tenant_id: mockTenantId,
    order_id: `ORDER-${index + 1}`,
    type: types[index % types.length] ?? 'RETURN_REFUND',
    status: index % 3 === 0 ? 'RECEIVED' : index % 3 === 1 ? 'APPROVED' : 'OPEN',
    reason_code: index % 2 === 0 ? 'DEFECT' : 'SIZE',
    amount_cent: 1200 + index * 100,
    created_at: now,
    updated_at: now
  };
};

const mockAfterSalesOps = Array.from({ length: 6 }, (_, index) =>
  buildAfterSalesOpsRecord(index)
);

const mockAfterSalesIntercepts: AfterSalesIntercept[] = [
  {
    intercept_id: 'INT-2026-001',
    order_id: 'ORDER-1',
    after_sales_id: 'AS-1',
    carrier_name: '顺丰速运',
    tracking_no: 'SF1023001',
    status: 'PROCESSING',
    reason: '客户申请退款',
    requested_at: dayjs().subtract(3, 'hour').toISOString(),
    completed_at: null,
    created_at: dayjs().subtract(3, 'hour').toISOString(),
    updated_at: dayjs().subtract(1, 'hour').toISOString()
  },
  {
    intercept_id: 'INT-2026-002',
    order_id: 'ORDER-2',
    after_sales_id: 'AS-2',
    carrier_name: '中通快递',
    tracking_no: 'ZT882109',
    status: 'SUCCESS',
    reason: '重复发货',
    requested_at: dayjs().subtract(1, 'day').toISOString(),
    completed_at: dayjs().subtract(22, 'hour').toISOString(),
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(22, 'hour').toISOString()
  },
  {
    intercept_id: 'INT-2026-003',
    order_id: 'ORDER-3',
    after_sales_id: 'AS-3',
    carrier_name: '京东物流',
    tracking_no: 'JD772901',
    status: 'FAILED',
    reason: '仓库发错货',
    requested_at: dayjs().subtract(2, 'day').toISOString(),
    completed_at: dayjs().subtract(47, 'hour').toISOString(),
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(47, 'hour').toISOString()
  },
  {
    intercept_id: 'INT-2026-004',
    order_id: 'ORDER-4',
    after_sales_id: 'AS-4',
    carrier_name: '圆通速递',
    tracking_no: 'YT554301',
    status: 'ESCALATED',
    reason: '客户紧急召回',
    requested_at: dayjs().subtract(5, 'hour').toISOString(),
    completed_at: null,
    created_at: dayjs().subtract(5, 'hour').toISOString(),
    updated_at: dayjs().subtract(2, 'hour').toISOString()
  }
];

const mockAfterSalesInspections: AfterSalesInspection[] = [
  {
    inspection_id: 'AS-QC-001',
    after_sales_id: 'AS-1',
    sku_id: 'SKU-001',
    warehouse_id: 'WH-001',
    qty: 1,
    result_code: 'RESALE',
    inspector_name: '张琪',
    inspected_at: dayjs().subtract(6, 'hour').toISOString(),
    created_at: dayjs().subtract(6, 'hour').toISOString(),
    updated_at: dayjs().subtract(6, 'hour').toISOString()
  },
  {
    inspection_id: 'AS-QC-002',
    after_sales_id: 'AS-2',
    sku_id: 'SKU-018',
    warehouse_id: 'WH-002',
    qty: 2,
    result_code: 'SCRAP',
    inspector_name: '李杰',
    inspected_at: dayjs().subtract(1, 'day').toISOString(),
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    inspection_id: 'AS-QC-003',
    after_sales_id: 'AS-3',
    sku_id: 'SKU-032',
    warehouse_id: 'WH-003',
    qty: 1,
    result_code: 'PENDING',
    inspector_name: null,
    inspected_at: null,
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  }
];

const mockAfterSalesSlaRules: AfterSalesSlaRule[] = [
  {
    rule_id: 'SLA-REF-01',
    after_sales_type: 'REFUND_ONLY',
    response_hours: 4,
    resolution_hours: 24,
    escalation_role: '售后主管',
    status: 'ACTIVE',
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    rule_id: 'SLA-EX-02',
    after_sales_type: 'EXCHANGE',
    response_hours: 6,
    resolution_hours: 48,
    escalation_role: '仓储经理',
    status: 'ACTIVE',
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    rule_id: 'SLA-QC-03',
    after_sales_type: 'RETURN_REFUND',
    response_hours: 8,
    resolution_hours: 36,
    escalation_role: '质检组长',
    status: 'DISABLED',
    created_at: dayjs().subtract(15, 'day').toISOString(),
    updated_at: dayjs().subtract(7, 'day').toISOString()
  }
];











const mockAfterSalesImportJobs: AfterSalesImportJob[] = Array.from({ length: 3 }, (_, index) => {
  const now = new Date(Date.now() - index * 3600 * 1000).toISOString();
  return {
    job_id: `AS-IMPORT-${index + 1}`,
    file_name: `after_sales_${index + 1}.csv`,
    file_type: 'CSV',
    status: index === 0 ? 'DONE' : index === 1 ? 'RUNNING' : 'PENDING',
    progress: index === 0 ? 100 : index === 1 ? 55 : 0,
    total_rows: index === 0 ? 20 : index === 1 ? 40 : 0,
    success_rows: index === 0 ? 20 : index === 1 ? 22 : 0,
    failed_rows: index === 0 ? 0 : index === 1 ? 1 : 0,
    attempts: index === 2 ? 0 : 1,
    last_error: '',
    created_at: now,
    updated_at: now
  };
});

























const buildSkuSearchItem = (sku: Sku): SkuSearchItem => ({
  sku_id: sku.sku_id,
  sku_code: sku.sku_code,
  name: sku.name ?? null,
  barcode: sku.barcode ?? null,
  sku_kind: sku.sku_kind ?? 'NORMAL'
});

















const resolveMappingSkuInfo = (payload: MappingPayload) => {
  const matched = mockSkus.find((sku) => sku.sku_id === payload.sku_id);
  return {
    sku_code: payload.sku_code ?? matched?.sku_code ?? payload.sku_id,
    barcode: payload.barcode ?? matched?.barcode ?? null
  };
};









type LogisticsShipmentListQuery = {
  tracking_no?: string;
  offset?: number;
  limit?: number;
};











type KitListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};





type SaveKitPayload = {
  kit_sku_id: string;
  components: Array<{ component_sku_id: string; qty: number }>;
};



type PrintTemplateListQuery = {
  platform?: PlatformCode;
  shop_id?: string;
  template_type?: PrintTemplateType;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};



type PrintJobListQuery = {
  status?: PrintJobStatus;
  job_type?: string;
  ref_type?: string;
  ref_id?: string;
  offset?: number;
  limit?: number;
};

type CreatePrintJobPayload = {
  job_type: string;
  ref_type: string;
  ref_id: string;
  template_id?: string;
  payload?: PrintShippingLabelPayload | Record<string, unknown>;
};

type UpdatePrintJobPayload = {
  status?: PrintJobStatus;
  last_error?: string | null;
  output_url?: string | null;
  payload?: Record<string, unknown> | null;
};

const buildPrintJobSummary = (job: PrintJobDetail): PrintJobSummary => ({
  job_id: job.job_id,
  job_type: job.job_type ?? null,
  ref_type: job.ref_type ?? null,
  ref_id: job.ref_id ?? null,
  status: job.status,
  dispatch_status: job.dispatch_status,
  output_url: job.output_url ?? null,
  attempts: job.attempts ?? 0,
  last_error: job.last_error ?? null,
  device_fingerprint: job.device_fingerprint ?? null,
  device_name: job.device_name ?? null,
  created_at: job.created_at,
  updated_at: job.updated_at
});

const appendPrintJobEvent = (
  job: PrintJobDetail,
  status: PrintJobStatus,
  error?: string | null
) => {
  job.events.push({
    status,
    error: error ?? null,
    created_at: dayjs().toISOString()
  });
};









const mockPurchaseOrders: PurchaseOrderSummary[] = [
  {
    po_id: 'PO-1001',
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
    warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
    expected_at: dayjs().add(5, 'day').toISOString(),
    remark: '首批到货',
    status: 'DRAFT',
    total_cent: 2400,
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'hour').toISOString()
  }
];

const mockPurchaseOrderItems = new Map<string, PurchaseOrderDetail['items']>([
  [
    'PO-1001',
    [
      {
        sku_id: 'SKU-1001',
        sku_code: 'SKU-1001',
        qty: 10,
        price_cent: 120,
        received_qty: 0
      }
    ]
  ]
]);

const mockPurchaseOrderReceipts = new Map<string, PurchaseOrderDetail['receipts']>();
const mockPurchaseReceiptItems = new Map<string, PurchaseReceiptDetail['items']>();

const buildPurchaseOrderDetail = (order: PurchaseOrderSummary): PurchaseOrderDetail => ({
  order,
  items: mockPurchaseOrderItems.get(order.po_id) ?? [],
  receipts: mockPurchaseOrderReceipts.get(order.po_id) ?? []
});

const buildPurchaseOrderTotal = (items: PurchaseOrderDetail['items']) =>
  items.reduce((sum, item) => sum + item.qty * item.price_cent, 0);

















const collectPurchaseReceipts = (): PurchaseReceiptSummary[] =>
  Array.from(mockPurchaseOrderReceipts.values()).flat();





const mockPurchaseSuggestions: PurchaseSuggestion[] = [
  {
    sku_id: 'SKU-1001',
    sku_code: 'SKU-1001',
    suggested_qty: 50,
    reason: 'sales_velocity',
    computed_at: dayjs().toISOString()
  },
  {
    sku_id: 'SKU-1002',
    sku_code: 'SKU-1002',
    suggested_qty: 30,
    reason: 'safety_stock',
    computed_at: dayjs().toISOString()
  }
];



const mockPurchaseReturns: PurchaseReturnSummary[] = [];
const mockPurchaseReturnItems = new Map<string, PurchaseReturnDetail['items']>();

export {
  platforms,
  mockTenantId,
  mockUserId,
  mockWarehouseSeeds,
  buildWarehouse,
  mockWarehouses,
  mockCourierSeeds,
  buildCourier,
  mockCouriers,
  mockChannels,
  mockSupplierSeeds,
  mockSuppliers,
  mockCustomerCategorySeeds,
  mockCustomerCategories,
  mockMemberLevels,
  mockCustomerTags,
  buildCustomer,
  mockCustomers,
  mockCustomerTagLinks,
  pickPlatform,
  buildShopTokenStatus,
  buildShop,
  mockShops,
  mockSyncJobs,
  buildSalesShopConfig,
  mockSalesShopConfigs,
  mockSyncJobRuns,
  buildOrder,
  buildAfterSales,
  buildAfterSaleItem,
  buildAfterSaleDetail,
  buildInventory,
  buildProduct,
  mockProducts,
  mockProductCategories,
  mockProductBrands,
  mockProductUnits,
  mockProductWarehouses,
  buildSku,
  shipmentCarrierCodes,
  shipmentWarehouses,
  buildShipment,
  mockOrders,
  mockOmsMainChainSeeds,
  mockOrderReceiverAddresses,
  mockAfterSales,
  mockAfterSaleDetails,
  mockInventory,
  mockInventoryLedgers,
  mockInventoryActionKeys,
  mockStockSyncJobs,
  mockSkus,
  buildSkuBatch,
  mockSkuBatches,
  buildSkuSerial,
  mockSkuSerials,
  buildMapping,
  mockMappings,
  mockSkuSearchIndex,
  mockKitSkus,
  mockStandardSkus,
  mockKitDetails,
  resolveMockSkuCost,
  mockPrintTemplates,
  mockPrintJobs,
  mockShipments,
  applyPagination,
  buildSalesShopConfigView,
  buildOrderItem,
  buildAfterSalesOpsRecord,
  mockAfterSalesOps,
  mockAfterSalesIntercepts,
  mockAfterSalesInspections,
  mockAfterSalesSlaRules,
  mockAfterSalesImportJobs,
  buildSkuSearchItem,
  resolveMappingSkuInfo,
  buildPrintJobSummary,
  appendPrintJobEvent,
  mockPurchaseOrders,
  mockPurchaseOrderItems,
  mockPurchaseOrderReceipts,
  mockPurchaseReceiptItems,
  buildPurchaseOrderDetail,
  buildPurchaseOrderTotal,
  collectPurchaseReceipts,
  mockPurchaseSuggestions,
  mockPurchaseReturns,
  mockPurchaseReturnItems
};

export type {
  OrderListQuery,
  AfterSalesListQuery,
  InventoryListQuery,
  SkuListQuery,
  OrderSummary,
  OrderDetail,
  OrderItem,
  OrderReceiverUpdatePayload,
  MockOmsMainChainActionSeed,
  MockOmsMainChainSeed,
  AfterSaleCaseSummary,
  AfterSaleCaseDetail,
  AfterSaleCaseItem,
  AfterSalesIntercept,
  AfterSalesInspection,
  AfterSalesSlaRule,
  InventorySnapshot,
  Product,
  ProductListResponse,
  ProductListQuery,
  ProductCreateRequest,
  ProductUpdateRequest,
  Sku,
  SkuListResponse,
  SkuKind,
  SkuSearchItem,
  SkuCreatePayload,
  SkuUpdatePayload,
  SkuBatch,
  SkuBatchListResponse,
  SkuBatchListQuery,
  SkuBatchCreateRequest,
  SkuBatchUpdateRequest,
  SkuSerial,
  SkuSerialListResponse,
  SkuSerialListQuery,
  SkuSerialGenerateRequest,
  SkuSerialPrintRequest,
  PrintJob,
  MappingListQuery,
  MappingPayload,
  MappingRecord,
  KitComponent,
  KitDetail,
  KitSummary,
  PrintJobStatus,
  PrintJobDispatchStatus,
  PrintJobEvent,
  PrintShippingLabelPayload,
  PrintJobDetail,
  PrintJobSummary,
  PrintTemplateType,
  PrintTemplate,
  AfterSalesOpsListQuery,
  AfterSalesInterceptListQuery,
  AfterSalesInterceptCreatePayload,
  AfterSalesInspectionListQuery,
  AfterSalesInspectionCreatePayload,
  AfterSalesInspectionUpdatePayload,
  AfterSalesSlaRuleListQuery,
  AfterSalesSlaRuleCreatePayload,
  AfterSalesSlaRuleUpdatePayload,
  RefundDocListQuery,
  AfterSalesImportJobListQuery,
  AfterSalesImportJob,
  AfterSalesOpsCreatePayload,
  AfterSalesOpsRecord,
  LogisticsShipmentListQuery,
  KitListQuery,
  SaveKitPayload,
  PrintTemplateListQuery,
  PrintJobListQuery,
  CreatePrintJobPayload,
  UpdatePrintJobPayload
};
