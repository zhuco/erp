import dayjs from 'dayjs';

import type {
  OrderDetail,
  OrderListQuery,
  OrderReceiverUpdatePayload,
  OrderSummary
} from './data';
import {
  applyPagination,
  buildOrder,
  buildOrderItem,
  mockCustomers,
  mockOmsMainChainSeeds,
  mockOrderReceiverAddresses,
  mockOrders,
  specialMockOrderIds,
  mockTenantId,
  mockWarehouses
} from './data';
import {
  calculateMockOrderGoodsAmountCent,
  calculateMockOrderGoodsQty,
  createMockOrderNoRegistry,
  ensureMockOrderCanReview,
  hasMockOrderGoods,
} from './orderMockGovernance';

type OrderReviewState = {
  oms_status: OrderSummary['oms_status'];
  review_status: OrderSummary['review_status'];
  exception_flags: string[];
  version: number;
};

type OrderEditRequest = {
  receiver_name?: string | null;
  receiver_phone?: string | null;
  receiver_tel?: string | null;
  receiver_address?: string | null;
  logistics_company?: string | null;
  tracking_no?: string | null;
  order_type?: string | null;
  sales_channel?: string | null;
  order_source?: string | null;
  salesperson?: string | null;
  ship_warehouse_id?: string | null;
  shipment_no?: string | null;
  estimated_weight_gram?: number | null;
  package_weight_gram?: number | null;
  customer_account?: string | null;
  invoice_type?: string | null;
  invoice_title?: string | null;
  payment_status?: string | null;
  buyer_message?: string | null;
  seller_remark?: string | null;
  freeze_reason?: string | null;
  cancel_reason?: string | null;
  exception_reason?: string | null;
  extra_remark?: string | null;
  items?: Array<{
    line_id?: string | null;
    sku_id?: string | null;
    title?: string | null;
    qty: number;
    price_cent: number;
  }>;
};

type OrderCancelRequest = {
  reason?: string | null;
};

type MockOrderExceptionActionRecord = {
  action: string;
  message: string;
  operator_id?: string | null;
  operator_name?: string | null;
  created_at: string;
};

type MockOrderExceptionSummary = {
  order_id: string;
  order_no: string;
  platform: string;
  shop_id: string;
  receiver_name?: string | null;
  total_amount_cent: number;
  audit_status: string;
  exception_status: string;
  fulfillment_status: string;
  reserve_status: string | null;
  target_warehouse_id: string | null;
  blocking_reason: string;
  assignee_id?: string | null;
  assignee_name?: string | null;
  latest_action?: string | null;
  latest_action_at?: string | null;
  tags?: string[];
  exception_flags?: string[];
  freeze_reason?: string | null;
  presale?: boolean;
  refund_state?: string | null;
  merge_group_id?: string | null;
  split_source_order_id?: string | null;
  platform_ship_deadline_at?: string | null;
};

type MockFulfillmentPrecheckResult = {
  order_id: string;
  order_no: string;
  passed: boolean;
  reasons: string[];
  reserve_status: string | null;
  fulfillment_status: string;
  target_warehouse_id: string | null;
  outbound_doc_count: number;
};

type MockOmsMainChainState = {
  order_id: string;
  audit_status: string;
  exception_status: string;
  fulfillment_status: string;
  reserve_status: string | null;
  reserve_precheck_passed: boolean;
  reserve_precheck_reasons: string[];
  target_warehouse_id: string | null;
  blocking_reason: string;
  assignee_id?: string | null;
  assignee_name?: string | null;
  action_logs: MockOrderExceptionActionRecord[];
};

type MockOrderSettlementStackMode = 'STACKED' | 'EXCLUSIVE' | 'PRIORITY';

type MockOrderSettlementRule = {
  stack_mode: MockOrderSettlementStackMode;
  member_discount_rate: number;
  promo_discount_rate: number;
  max_discount_rate: number;
  version: number;
  updated_at?: string | null;
};

type MockOrderSettlementRuleUpdatePayload = {
  stack_mode: MockOrderSettlementStackMode;
  member_discount_rate: number;
  promo_discount_rate: number;
  max_discount_rate: number;
  expected_version?: number;
};

type MockOrderSettlementActionPayload = {
  operator_id?: string | null;
  operator_name?: string | null;
  export_job_id?: string | null;
};

type MockOrderSettlementRecord = {
  status: 'OPEN' | 'LOCKED' | 'EXPORTED';
  locked_at?: string | null;
  export_job_id?: string | null;
  action_logs: MockOrderExceptionActionRecord[];
};

type MockOrderReconciliationActionPayload = {
  operator_id?: string | null;
  operator_name?: string | null;
  note?: string | null;
};

type MockOrderReconciliationRecord = {
  status: 'PENDING' | 'DONE';
  note?: string | null;
  settled_at?: string | null;
};

const mockOrderItems = new Map<string, OrderDetail['items']>();
const mockOrderLogisticsCompany = new Map<string, string>();
const mockOrderWarehouseId = new Map<string, string | null>();
const mockOrderNotes = new Map<string, OrderDetail['notes']>();
const mockOrderTags = new Map<string, OrderDetail['tags']>();
const mockOrderDetailOverrides = new Map<string, Partial<OrderDetail['order']>>();
const mockOmsMainChainStates = new Map<string, MockOmsMainChainState>(
  mockOmsMainChainSeeds.map((seed) => [
    seed.order_id,
    {
      order_id: seed.order_id,
      audit_status: seed.audit_status,
      exception_status: seed.exception_status,
      fulfillment_status: seed.fulfillment_status,
      reserve_status: seed.reserve_status ?? null,
      reserve_precheck_passed: seed.reserve_precheck_passed,
      reserve_precheck_reasons: [...seed.reserve_precheck_reasons],
      target_warehouse_id: seed.target_warehouse_id ?? null,
      blocking_reason: seed.blocking_reason,
      assignee_id: seed.assignee_id ?? null,
      assignee_name: seed.assignee_name ?? null,
      action_logs: seed.action_logs.map((record) => ({ ...record }))
    }
  ])
);
const defaultMockOrderSettlementRule: MockOrderSettlementRule = {
  stack_mode: 'STACKED',
  member_discount_rate: 3,
  promo_discount_rate: 5,
  max_discount_rate: 12,
  version: 0,
  updated_at: null
};
let mockOrderSettlementRule: MockOrderSettlementRule = { ...defaultMockOrderSettlementRule };
const mockOrderSettlementRecords = new Map<string, MockOrderSettlementRecord>();
const mockOrderReconciliationRecords = new Map<string, MockOrderReconciliationRecord>();
const specialOrderIds = specialMockOrderIds;
const mockOrderNoRegistry = createMockOrderNoRegistry(mockOrders.map((order) => order.id));

export const reserveMockOrderId = (dateValue?: string | dayjs.Dayjs) => mockOrderNoRegistry.reserve(dateValue);

const buildDefaultNotes = (): OrderDetail['notes'] => [
  {
    note_id: `NOTE-${Date.now()}`,
    note: '客户备注：优先发货',
    created_by: null,
    created_at: dayjs().toISOString()
  }
];

const normalizeTagValues = (tags: string[]) =>
  Array.from(new Set(tags.map((tag) => tag.trim()).filter(Boolean)));

const buildRuntimeOrderTagNames = (order: OrderSummary, orderIndex: number) => {
  const tags: string[] = [];
  if (orderIndex % 9 === 0) {
    tags.push('加急');
  }
  if (orderIndex % 11 === 0) {
    tags.push('人工复核');
  }
  if (orderIndex % 17 === 0) {
    tags.push('直播赠品');
  }
  if (String(order.shop_id ?? '').includes('douyin') && orderIndex % 5 === 0) {
    tags.push('直播间订单');
  }
  return normalizeTagValues(tags);
};

const buildRuntimeOrderSummarySignals = (order: OrderSummary, orderIndex: number) => ({
  tags: buildRuntimeOrderTagNames(order, orderIndex),
  presale: orderIndex % 10 === 0,
  platform_ship_deadline_at:
    orderIndex % 12 === 0 ? dayjs().subtract((orderIndex % 3) + 1, 'day').toISOString() : undefined,
  refund_state: orderIndex % 14 === 0 ? 'PARTIAL_REFUND' : undefined,
  merge_group_id: orderIndex % 16 === 0 ? `MERGE-GROUP-${orderIndex + 1}` : undefined,
  split_source_order_id: orderIndex % 18 === 0 ? `PARENT-${order.id}` : undefined,
  freeze_reason:
    (order.exception_flags ?? []).includes('ORDER_FROZEN') || orderIndex % 20 === 0
      ? '风控自动冻结'
      : undefined,
});

const buildDefaultTags = (order: OrderSummary, orderIndex: number): OrderDetail['tags'] => {
  const tagNames = buildRuntimeOrderTagNames(order, orderIndex);
  const createdAt = dayjs().toISOString();
  return tagNames.map((tag) => ({
    tag,
    created_by: null,
    created_at: createdAt,
  }));
};

const resolveMockOrderItems = (order: OrderSummary) =>
  mockOrderItems.get(order.id) ??
  Array.from({ length: 3 }, (_, index) => buildOrderItem(order, index));

const resolveAuditStatusFromReviewStatus = (reviewStatus?: string | null) => {
  if (reviewStatus === 'PASSED') {
    return 'REVIEW_PASSED';
  }
  if (reviewStatus === 'REJECTED') {
    return 'REVIEW_REJECTED';
  }
  return 'REVIEW_REQUIRED';
};

const resolveExceptionStatusFromOrder = (order: OrderSummary) =>
  (order.exception_flags?.length ?? 0) > 0 ? 'OPEN' : 'NONE';

const resolveFulfillmentStatusFromOmsStatus = (omsStatus?: string | null) => {
  if (omsStatus === 'SHIPPED') {
    return 'SHIPPED';
  }
  if (omsStatus === 'ALLOCATED') {
    return 'WAVED';
  }
  return 'NOT_SUBMITTED';
};

const resolveOrderMainChainState = (orderId: string): MockOmsMainChainState => {
  const existing = mockOmsMainChainStates.get(orderId);
  if (existing) {
    return existing;
  }

  const order = mockOrders.find((item) => item.id === orderId) ?? buildOrder(0);
  const nextState: MockOmsMainChainState = {
    order_id: orderId,
    audit_status: resolveAuditStatusFromReviewStatus(order.review_status),
    exception_status: resolveExceptionStatusFromOrder(order),
    fulfillment_status: resolveFulfillmentStatusFromOmsStatus(order.oms_status),
    reserve_status: null,
    reserve_precheck_passed: (order.exception_flags?.length ?? 0) === 0,
    reserve_precheck_reasons:
      (order.exception_flags?.length ?? 0) === 0 ? [] : ['瀛樺湪鏈理异常，不能提交发货'],
    target_warehouse_id: mockWarehouses[0]?.warehouse_id ?? null,
    blocking_reason:
      (order.exception_flags?.length ?? 0) === 0 ? '可直接提交发货' : '存在未处理异常，需先复核',
    assignee_id: null,
    assignee_name: null,
    action_logs: []
  };

  mockOmsMainChainStates.set(orderId, nextState);
  return nextState;
};

const closedExceptionStatuses = new Set(['NONE', 'RESOLVED', 'CLOSED']);

const syncOrderSummaryWithMainChain = (orderId: string, state: MockOmsMainChainState) => {
  const index = mockOrders.findIndex((item) => item.id === orderId);
  if (index < 0) {
    return;
  }

  const current = mockOrders[index];
  const nextExceptionFlags =
    closedExceptionStatuses.has(state.exception_status)
      ? []
      : Array.from(
          new Set([
            ...(current.exception_flags ?? []),
            state.blocking_reason === '鍟嗗搧鏄犲皠缂哄け'
              ? 'MAPPING_MISSING'
              : state.blocking_reason === '库存不足'
                ? 'INVENTORY_SHORTAGE'
                : 'REVIEW_FLAGGED'
          ])
        );

  mockOrders[index] = {
    ...current,
    review_status:
      state.audit_status === 'REVIEW_PASSED'
        ? 'PASSED'
        : state.audit_status === 'REVIEW_REJECTED'
          ? 'REJECTED'
          : 'PENDING',
    oms_status:
      state.fulfillment_status === 'SHIPPED'
        ? 'SHIPPED'
        : state.audit_status === 'REVIEW_PASSED'
          ? 'PENDING_SHIPMENT'
          : 'REVIEW_PENDING',
    exception_flags: nextExceptionFlags,
    updated_at: dayjs().toISOString()
  };
};

const buildMockOrderExceptionSummary = (state: MockOmsMainChainState): MockOrderExceptionSummary => {
  const order = mockOrders.find((item) => item.id === state.order_id) ?? buildOrder(0);
  const orderIndex = Math.max(0, mockOrders.findIndex((item) => item.id === order.id));
  const runtimeSignals = buildRuntimeOrderSummarySignals(order, orderIndex);
  const latestAction = state.action_logs.at(-1) ?? null;
  return {
    order_id: state.order_id,
    order_no: order.id,
    platform: order.platform,
    shop_id: order.shop_id,
    receiver_name: order.receiver_name ?? null,
    total_amount_cent: order.total_amount_cent,
    audit_status: state.audit_status,
    exception_status: state.exception_status,
    fulfillment_status: state.fulfillment_status,
    reserve_status: state.reserve_status,
    target_warehouse_id: state.target_warehouse_id,
    blocking_reason: state.blocking_reason,
    assignee_id: state.assignee_id ?? null,
    assignee_name: state.assignee_name ?? null,
    latest_action: latestAction?.message ?? null,
    latest_action_at: latestAction?.created_at ?? null,
    tags: runtimeSignals.tags,
    exception_flags: order.exception_flags ?? [],
    freeze_reason: runtimeSignals.freeze_reason ?? null,
    presale: runtimeSignals.presale,
    refund_state: runtimeSignals.refund_state ?? null,
    merge_group_id: runtimeSignals.merge_group_id ?? null,
    split_source_order_id: runtimeSignals.split_source_order_id ?? null,
    platform_ship_deadline_at: runtimeSignals.platform_ship_deadline_at ?? null,
  };
};

mockOrderItems.set(specialOrderIds.noItems, []);
mockOrderItems.set(specialOrderIds.missingSku, [
  {
    id: `${specialOrderIds.missingSku}-ITEM-1`,
    platform_sku_id: null,
    outer_sku_id: null,
    title: '未匹配商品',
    spec_text: '奶白 / M',
    quantity: 1,
    price_cent: 1299,
    mapping_missing: true
  }
]);
mockOrderItems.set(specialOrderIds.shortage, [
  {
    id: `${specialOrderIds.shortage}-ITEM-1`,
    platform_sku_id: 'PLAT-SKU-NEG-1',
    outer_sku_id: 'OUTER-NEG-1',
    title: '缂鸿揣鍟嗗搧A',
    spec_text: '黑色 / L',
    quantity: 2,
    price_cent: 1599,
    mapping_missing: false
  },
  {
    id: `${specialOrderIds.shortage}-ITEM-2`,
    platform_sku_id: 'PLAT-SKU-NEG-2',
    outer_sku_id: null,
    title: '缂鸿揣鍟嗗搧B',
    spec_text: '白色 / XL',
    quantity: 1,
    price_cent: 899,
    mapping_missing: false
  }
]);
mockOrderItems.set(specialOrderIds.frozen, [
  {
    id: `${specialOrderIds.frozen}-ITEM-1`,
    platform_sku_id: 'PLAT-SKU-FROZEN-1',
    outer_sku_id: null,
    title: '鍐荤粨订单鍟嗗搧',
    quantity: 1,
    price_cent: 1399,
    mapping_missing: false
  }
]);
mockOrderItems.set(
  specialOrderIds.manyItems,
  Array.from({ length: 8 }, (_, index) => ({
    id: `${specialOrderIds.manyItems}-ITEM-${index + 1}`,
    platform_sku_id: `PLAT-SKU-BULK-${index + 1}`,
    outer_sku_id: index % 2 === 0 ? `OUTER-BULK-${index + 1}` : null,
    title: `鎵归噺娴嬭瘯鍟嗗搧-${index + 1}`,
    quantity: (index % 3) + 1,
    price_cent: 1299 + index * 80,
    mapping_missing: index === 5
  }))
);
mockOrderItems.set(specialOrderIds.merged, [
  {
    id: `${specialOrderIds.merged}-ITEM-1`,
    platform_sku_id: 'PLAT-SKU-MERGE-1',
    outer_sku_id: 'OUTER-MERGE-1',
    title: '鍚堝苟订单鍟嗗搧A',
    spec_text: '卡其 / S',
    quantity: 1,
    price_cent: 1899,
    mapping_missing: false
  },
  {
    id: `${specialOrderIds.merged}-ITEM-2`,
    platform_sku_id: 'PLAT-SKU-MERGE-2',
    outer_sku_id: 'OUTER-MERGE-2',
    title: '鍚堝苟订单鍟嗗搧B',
    spec_text: '灰色 / M',
    quantity: 2,
    price_cent: 999,
    mapping_missing: false
  }
]);
mockOrderItems.set(specialOrderIds.split, [
  {
    id: `${specialOrderIds.split}-ITEM-1`,
    platform_sku_id: 'PLAT-SKU-SPLIT-1',
    outer_sku_id: 'OUTER-SPLIT-1',
    title: '鎷嗗垎订单鍟嗗搧A',
    spec_text: '深蓝 / M',
    quantity: 1,
    price_cent: 1599,
    mapping_missing: false
  },
  {
    id: `${specialOrderIds.split}-ITEM-2`,
    platform_sku_id: 'PLAT-SKU-SPLIT-2',
    outer_sku_id: null,
    title: '鎷嗗垎订单鍟嗗搧B',
    spec_text: '咖色 / L',
    quantity: 1,
    price_cent: 1299,
    mapping_missing: false
  },
  {
    id: `${specialOrderIds.split}-ITEM-3`,
    platform_sku_id: 'PLAT-SKU-SPLIT-3',
    outer_sku_id: 'OUTER-SPLIT-3',
    title: '鎷嗗垎订单鍟嗗搧C',
    spec_text: '军绿 / XL',
    quantity: 2,
    price_cent: 899,
    mapping_missing: false
  }
]);
mockOrderReceiverAddresses.set(
  specialOrderIds.addressInvalid,
  '鍦板潃异常锛氱己灏戣閬撲笌妤煎眰淇℃伅'
);
mockOrderLogisticsCompany.set(specialOrderIds.noLogistics, '');
mockOrderLogisticsCompany.set(specialOrderIds.merged, '京东物流');
mockOrderLogisticsCompany.set(specialOrderIds.split, '圆通速递');
mockOrderWarehouseId.set(specialOrderIds.merged, 'WH-EAST');
mockOrderWarehouseId.set(specialOrderIds.split, 'WH-DEFAULT');
mockOrderNotes.set(specialOrderIds.reviewFlagged, [
  ...buildDefaultNotes(),
  {
    note_id: `NOTE-${Date.now() + 1}`,
    note: '椋庢帶鎷︽埅锛氶渶浜哄伐澶嶆牳',
    created_by: null,
    created_at: dayjs().subtract(25, 'minute').toISOString()
  },
  {
    note_id: `NOTE-${Date.now() + 2}`,
    note: '寤鸿鑱旂郴客户确认鏀惰揣淇℃伅',
    created_by: null,
    created_at: dayjs().subtract(15, 'minute').toISOString()
  }
]);
mockOrderTags.set(specialOrderIds.blacklist, [
  {
    tag: '黑名单',
    created_by: null,
    created_at: dayjs().toISOString()
  },
  {
    tag: '高风险',
    created_by: null,
    created_at: dayjs().subtract(10, 'minute').toISOString()
  }
]);
mockOrderTags.set(specialOrderIds.duplicate, [
  {
    tag: '閲嶅订单',
    created_by: null,
    created_at: dayjs().toISOString()
  },
  {
    tag: '闇€鏍稿',
    created_by: null,
    created_at: dayjs().subtract(5, 'minute').toISOString()
  }
]);
mockOrderDetailOverrides.set(specialOrderIds.frozen, {
  freeze_reason: '高风险订单自动冻结',
});
mockOrderDetailOverrides.set(specialOrderIds.addressInvalid, {
  exception_reason: '鍦板潃瀛楁涓嶅畬鏁达紝闇€客户琛ュ厖'
});
mockOrderDetailOverrides.set(specialOrderIds.duplicate, {
  exception_reason: '平台閲嶅鎷夊崟锛屽緟确认鏄惁鍚堝苟'
});
mockOrderDetailOverrides.set(specialOrderIds.reviewFlagged, {
  exception_reason: '楂樺单需复核'
});
mockOrderDetailOverrides.set(specialOrderIds.merged, {
  extra_remark: '已合并同收件人订单',
});
mockOrderDetailOverrides.set(specialOrderIds.split, {
  extra_remark: '已拆分按仓发货',
});

export const getMockOrderList = (query: OrderListQuery) => {
  const resolvedOrders = mockOrders.map((order, orderIndex) => {
    const resolvedItems = resolveMockOrderItems(order);
    const runtimeSignals = buildRuntimeOrderSummarySignals(order, orderIndex);
    return {
      ...order,
      ship_warehouse_id:
        mockOrderWarehouseId.get(order.id) ??
        mockWarehouses[orderIndex % mockWarehouses.length]?.warehouse_id ??
        null,
      logistics_company: mockOrderLogisticsCompany.get(order.id) ?? '',
      goods_qty: calculateMockOrderGoodsQty(resolvedItems),
      ...runtimeSignals,
    } as OrderSummary;
  });

  const filtered = resolvedOrders.filter((order, orderIndex) => {
    const resolvedWarehouseId = order.ship_warehouse_id ?? null;
    const resolvedReceiverAddress =
      mockOrderReceiverAddresses.get(order.id) ?? '';
    const resolvedLogisticsCompany = order.logistics_company ?? '';
    const resolvedItems = resolveMockOrderItems(order);
    const resolvedGoodsQty = calculateMockOrderGoodsQty(resolvedItems);
    const estimatedPostageCent = 300 + orderIndex * 5;
    const estimatedProfitCent = order.total_amount_cent - estimatedPostageCent;
    const estimatedProfitRate =
      order.total_amount_cent > 0
        ? (estimatedProfitCent / order.total_amount_cent) * 100
        : 0;
    const parseList = (raw?: string) =>
      raw
        ?.split(',')
        .map((value) => value.trim())
        .filter((value) => value.length);
    if (query.platform) {
      const platforms = parseList(query.platform) ?? [];
      if (!platforms.includes(order.platform)) {
        return false;
      }
    }
    if (query.shop_id) {
      const shopIds = parseList(query.shop_id) ?? [];
      if (!shopIds.includes(order.shop_id ?? '')) {
        return false;
      }
    }
    if (query.ship_warehouse_id) {
      const warehouseIds = parseList(query.ship_warehouse_id) ?? [];
      if (!warehouseIds.includes(resolvedWarehouseId ?? '')) {
        return false;
      }
    }
    if (query.oms_status) {
      const omsStatuses = parseList(query.oms_status) ?? [];
      if (!omsStatuses.includes(order.oms_status ?? '')) {
        return false;
      }
    }
    if (query.review_status && order.review_status !== query.review_status) {
      return false;
    }
    if (
      query.platform_order_id &&
      !order.platform_order_id?.toLowerCase().includes(query.platform_order_id.trim().toLowerCase())
    ) {
      return false;
    }
    if (query.item_keyword) {
      const keyword = query.item_keyword.trim().toLowerCase();
      const matches = resolvedItems.some((item) => {
        return (
          item.title?.toLowerCase().includes(keyword) ||
          item.platform_sku_id?.toLowerCase().includes(keyword) ||
          item.outer_sku_id?.toLowerCase().includes(keyword) ||
          item.id?.toLowerCase().includes(keyword)
        );
      });
      if (!matches) {
        return false;
      }
    }
    if (query.total_amount_cent != null && order.total_amount_cent !== query.total_amount_cent) {
      return false;
    }
    if (query.goods_qty != null && resolvedGoodsQty !== query.goods_qty) {
      return false;
    }
    if (query.customer_account) {
      const keyword = query.customer_account.trim().toLowerCase();
      const matches =
        order.customer_name?.toLowerCase().includes(keyword) ||
        order.customer_id?.toLowerCase().includes(keyword) ||
        order.id?.toLowerCase().includes(keyword);
      if (!matches) {
        return false;
      }
    }
    if (
      query.receiver_name &&
      !order.receiver_name?.toLowerCase().includes(query.receiver_name.trim().toLowerCase())
    ) {
      return false;
    }
    if (
      query.receiver_phone &&
      !order.receiver_phone?.toLowerCase().includes(query.receiver_phone.trim().toLowerCase())
    ) {
      return false;
    }
    if (
      query.receiver_address &&
      !resolvedReceiverAddress.toLowerCase().includes(query.receiver_address.trim().toLowerCase())
    ) {
      return false;
    }
    if (query.remark) {
      const keyword = query.remark.trim().toLowerCase();
      const matches =
        order.platform_order_id?.toLowerCase().includes(keyword) ||
        order.id?.toLowerCase().includes(keyword);
      if (!matches) {
        return false;
      }
    }
    if (
      query.order_source &&
      !order.platform?.toLowerCase().includes(query.order_source.trim().toLowerCase())
    ) {
      return false;
    }
    if (
      query.logistics_company &&
      !resolvedLogisticsCompany.toLowerCase().includes(query.logistics_company.trim().toLowerCase())
    ) {
      return false;
    }
    if (
      query.estimated_postage_cent != null &&
      estimatedPostageCent !== query.estimated_postage_cent
    ) {
      return false;
    }
    if (
      query.estimated_profit_cent != null &&
      estimatedProfitCent !== query.estimated_profit_cent
    ) {
      return false;
    }
    if (query.estimated_profit_rate != null) {
      const roundedRate = Number(estimatedProfitRate.toFixed(2));
      if (roundedRate !== query.estimated_profit_rate) {
        return false;
      }
    }
    if (query.exception_flags) {
      const flags = query.exception_flags.split(',').map((flag) => flag.trim());
      const matches = flags.some((flag) => order.exception_flags?.includes(flag));
      if (!matches) {
        return false;
      }
    }
    if (query.keyword) {
      const keyword = query.keyword.trim().toLowerCase();
      const matches =
        order.id?.toLowerCase().includes(keyword) ||
        order.platform_order_id?.toLowerCase().includes(keyword) ||
        order.receiver_name?.toLowerCase().includes(keyword) ||
        order.receiver_phone_last4?.includes(keyword) ||
        order.tracking_no?.toLowerCase().includes(keyword);
      if (!matches) {
        return false;
      }
    }
    if (query.updated_from && order.updated_at) {
      if (dayjs(order.updated_at).isBefore(query.updated_from)) {
        return false;
      }
    }
    if (query.updated_to && order.updated_at) {
      if (dayjs(order.updated_at).isAfter(query.updated_to)) {
        return false;
      }
    }
    if (query.created_from && order.created_at) {
      if (dayjs(order.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && order.created_at) {
      if (dayjs(order.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length,
    next_cursor: null
  };
};

export const getMockOrderDetail = (orderId: string): OrderDetail => {
  const order = mockOrders.find((item) => item.id === orderId) ?? buildOrder(0);
  const orderIndex = Math.max(mockOrders.findIndex((item) => item.id === order.id), 0);
  const customer = mockCustomers.find((item) => item.customer_id === order.customer_id);
  const items = resolveMockOrderItems(order);
  const receiverAddress =
    mockOrderReceiverAddresses.get(order.id) ?? '上海市浦东新区示例路 100 号';
  const logisticsCompany = mockOrderLogisticsCompany.get(order.id) ?? '';
  const shipWarehouseId =
    mockOrderWarehouseId.get(order.id) ??
    mockWarehouses[orderIndex % mockWarehouses.length]?.warehouse_id ??
    null;
  const runtimeSignals = buildRuntimeOrderSummarySignals(order, orderIndex);
  const notes = mockOrderNotes.get(order.id) ?? buildDefaultNotes();
  if (!mockOrderNotes.has(order.id)) {
    mockOrderNotes.set(order.id, notes);
  }
  const overrides = mockOrderDetailOverrides.get(order.id) ?? {};
  const orderDetail = {
    id: order.id,
    platform: order.platform,
    shop_id: order.shop_id,
    platform_order_id: order.platform_order_id,
    platform_status: order.platform_status,
    oms_status: order.oms_status,
    review_status: order.review_status,
    exception_flags: [...(order.exception_flags ?? [])],
    total_amount_cent: order.total_amount_cent,
    goods_amount_cent: calculateMockOrderGoodsAmountCent(items),
    postage_amount_cent: 500,
    discount_amount_cent: 300,
    receiver_name: order.receiver_name ?? null,
    receiver_phone: order.receiver_phone ?? null,
    receiver_tel: overrides.receiver_tel ?? null,
    receiver_address: receiverAddress,
    ship_warehouse_id: shipWarehouseId,
    logistics_company: logisticsCompany,
    tracking_no: order.tracking_no ?? null,
    shipment_no: overrides.shipment_no ?? null,
    package_weight_gram: overrides.package_weight_gram ?? null,
    invoice_type: overrides.invoice_type ?? null,
    invoice_title: overrides.invoice_title ?? null,
    payment_status: overrides.payment_status ?? null,
    buyer_message: overrides.buyer_message ?? null,
    seller_remark: overrides.seller_remark ?? null,
    cancel_reason: overrides.cancel_reason ?? null,
    exception_reason: overrides.exception_reason ?? null,
    extra_remark: overrides.extra_remark ?? null,
    customer_id: order.customer_id ?? null,
    customer_account: overrides.customer_account ?? null,
    customer_name: order.customer_name ?? null,
    customer_status: customer?.status ?? null,
    order_created_at: order.created_at ?? dayjs().toISOString(),
    updated_at: order.updated_at ?? null,
    version: 1,
    order_type: order.order_type ?? null,
    sales_channel: order.sales_channel ?? null,
    order_source: order.order_source ?? null,
    salesperson: order.salesperson ?? null,
    presale: runtimeSignals.presale ?? false,
    refund_state: runtimeSignals.refund_state ?? null,
    freeze_reason: overrides.freeze_reason ?? runtimeSignals.freeze_reason ?? null,
    merge_group_id: runtimeSignals.merge_group_id ?? null,
    split_source_order_id: runtimeSignals.split_source_order_id ?? null,
    platform_ship_deadline_at: runtimeSignals.platform_ship_deadline_at ?? null,
  } as OrderDetail['order'] & {
    order_type?: string | null;
    sales_channel?: string | null;
    order_source?: string | null;
    salesperson?: string | null;
    presale?: boolean;
    refund_state?: string | null;
    merge_group_id?: string | null;
    split_source_order_id?: string | null;
    platform_ship_deadline_at?: string | null;
  };
  const resolvedTags = mockOrderTags.get(order.id) ?? buildDefaultTags(order, orderIndex);
  return {
    order: orderDetail,
    items: items.map((item) => ({ ...item })),
    review_hits: [
      {
        rule_id: 'RULE-1',
        rule_name: '高客单拦截',
        action: 'FLAG',
        hit: true,
        message: 'total_cent > 50000',
        created_at: dayjs().toISOString()
      }
    ],
    tags: resolvedTags.map((tag) => ({ ...tag })),
    notes: notes.map((note) => ({ ...note })),
    audit_logs: [
      {
        audit_id: 'AUDIT-1',
        tenant_id: mockTenantId,
        user_id: null,
        action: 'OMS.REVIEW_AUTO_FLAG',
        entity_type: 'ORDER',
        entity_id: order.id,
        before_json: null,
        after_json: null,
        ip: null,
        ua: null,
        created_at: dayjs().toISOString()
      }
    ]
  };
};

export const addMockOrderNote = (
  orderId: string,
  note: string
): OrderDetail['notes'][number] => {
  const trimmed = note.trim();
  const nextNote = {
    note_id: `NOTE-${Date.now()}`,
    note: trimmed,
    created_by: null,
    created_at: dayjs().toISOString()
  };
  const existing = mockOrderNotes.get(orderId) ?? buildDefaultNotes();
  const nextNotes = [...existing, nextNote];
  mockOrderNotes.set(orderId, nextNotes);
  return nextNote;
};

export const addMockOrderTags = (orderId: string, tags: string[]): OrderDetail['tags'] => {
  const normalized = normalizeTagValues(tags);
  const now = dayjs().toISOString();
  const nextTags = normalized.map((tag) => ({
    tag,
    created_by: null,
    created_at: now
  }));
  mockOrderTags.set(orderId, nextTags);
  return nextTags;
};

export const updateMockOrder = (orderId: string, payload: OrderEditRequest): OrderDetail => {
  const now = dayjs().toISOString();
  const index = mockOrders.findIndex((item) => item.id === orderId);
  const base = index >= 0 ? mockOrders[index] : buildOrder(0);
  const nextItems =
    payload.items?.map((item, idx) => ({
      id: item.line_id ?? `${orderId}-ITEM-${idx + 1}`,
      platform_sku_id: item.sku_id ?? null,
      outer_sku_id: null,
      title: item.title ?? null,
      quantity: item.qty,
      price_cent: item.price_cent,
      mapping_missing: false
    })) ?? resolveMockOrderItems(base);
  const normalizedPhone = payload.receiver_phone?.trim() ?? base.receiver_phone ?? '';
  const updated: OrderSummary = {
    ...base,
    id: orderId,
    receiver_name: payload.receiver_name?.trim() ?? base.receiver_name ?? null,
    receiver_phone: normalizedPhone || base.receiver_phone || null,
    receiver_phone_last4: normalizedPhone ? normalizedPhone.slice(-4) : base.receiver_phone_last4,
    tracking_no: payload.tracking_no ?? base.tracking_no ?? null,
    order_type: payload.order_type ?? base.order_type ?? null,
    sales_channel: payload.sales_channel ?? base.sales_channel ?? null,
    order_source: payload.order_source ?? base.order_source ?? null,
    salesperson: payload.salesperson ?? base.salesperson ?? null,
    goods_qty: calculateMockOrderGoodsQty(nextItems),
    updated_at: now
  };
  if (index >= 0) {
    mockOrders[index] = updated;
  } else {
    mockOrders.unshift(updated);
  }
  if (payload.receiver_address) {
    mockOrderReceiverAddresses.set(orderId, payload.receiver_address.trim());
  }
  if (payload.ship_warehouse_id !== undefined) {
    mockOrderWarehouseId.set(orderId, payload.ship_warehouse_id ?? null);
  }
  if (payload.logistics_company !== undefined) {
    mockOrderLogisticsCompany.set(orderId, payload.logistics_company?.trim() ?? '');
  }
  const existingOverrides = mockOrderDetailOverrides.get(orderId) ?? {};
  const nextOverrides: Partial<OrderDetail['order']> = {
    receiver_tel:
      payload.receiver_tel !== undefined
        ? payload.receiver_tel?.trim() ?? null
        : existingOverrides.receiver_tel ?? null,
    shipment_no:
      payload.shipment_no !== undefined
        ? payload.shipment_no?.trim() ?? null
        : existingOverrides.shipment_no ?? null,
    package_weight_gram:
      payload.package_weight_gram !== undefined
        ? payload.package_weight_gram ?? null
        : existingOverrides.package_weight_gram ?? null,
    customer_account:
      payload.customer_account !== undefined
        ? payload.customer_account?.trim() ?? null
        : existingOverrides.customer_account ?? null,
    invoice_type:
      payload.invoice_type !== undefined
        ? payload.invoice_type?.trim() ?? null
        : existingOverrides.invoice_type ?? null,
    invoice_title:
      payload.invoice_title !== undefined
        ? payload.invoice_title?.trim() ?? null
        : existingOverrides.invoice_title ?? null,
    payment_status:
      payload.payment_status !== undefined
        ? payload.payment_status?.trim() ?? null
        : existingOverrides.payment_status ?? null,
    buyer_message:
      payload.buyer_message !== undefined
        ? payload.buyer_message?.trim() ?? null
        : existingOverrides.buyer_message ?? null,
    seller_remark:
      payload.seller_remark !== undefined
        ? payload.seller_remark?.trim() ?? null
        : existingOverrides.seller_remark ?? null,
    freeze_reason:
      payload.freeze_reason !== undefined
        ? payload.freeze_reason?.trim() ?? null
        : existingOverrides.freeze_reason ?? null,
    cancel_reason:
      payload.cancel_reason !== undefined
        ? payload.cancel_reason?.trim() ?? null
        : existingOverrides.cancel_reason ?? null,
    exception_reason:
      payload.exception_reason !== undefined
        ? payload.exception_reason?.trim() ?? null
        : existingOverrides.exception_reason ?? null,
    extra_remark:
      payload.extra_remark !== undefined
        ? payload.extra_remark?.trim() ?? null
        : existingOverrides.extra_remark ?? null
  };
  mockOrderDetailOverrides.set(orderId, { ...existingOverrides, ...nextOverrides });
  if (payload.items) {
    mockOrderItems.set(orderId, nextItems);
  }
  return getMockOrderDetail(orderId);
};

export const reviewMockOrder = (
  orderId: string,
  payload?: { action?: string | null; reason?: string | null }
): OrderReviewState => {
  const index = mockOrders.findIndex((item) => item.id === orderId);
  const currentOrder = index >= 0 ? mockOrders[index] : null;
  if (!currentOrder) {
    throw new Error('未找到要审核的订单');
  }

  const normalizedAction = String(payload?.action ?? 'PASS').trim().toUpperCase();
  if (normalizedAction === 'PASS') {
    ensureMockOrderCanReview(resolveMockOrderItems(currentOrder), currentOrder.id);
  }

  const now = dayjs().toISOString();
  const nextExceptionFlags =
    normalizedAction === 'PASS'
      ? (currentOrder.exception_flags ?? []).filter((flag) => flag !== 'REVIEW_REJECTED')
      : Array.from(new Set([...(currentOrder.exception_flags ?? []), 'REVIEW_REJECTED']));

  const nextReviewStatus = normalizedAction === 'PASS' ? 'PASSED' : 'REJECTED';
  const nextOmsStatus = normalizedAction === 'PASS' ? 'PENDING_SHIPMENT' : 'REVIEW_PENDING';

  mockOrders[index] = {
    ...currentOrder,
    review_status: nextReviewStatus,
    oms_status: nextOmsStatus,
    exception_flags: nextExceptionFlags,
    updated_at: now
  };

  updateMockOmsState(orderId, (state) => {
    state.audit_status = normalizedAction === 'PASS' ? 'REVIEW_PASSED' : 'REVIEW_REJECTED';
    state.exception_status =
      normalizedAction === 'PASS' && nextExceptionFlags.length === 0 ? 'NONE' : 'OPEN';
    state.blocking_reason =
      normalizedAction === 'PASS'
        ? nextExceptionFlags.length === 0
          ? '可直接提交发货'
          : '仍存在待处理异常'
        : payload?.reason?.trim() || '订单审核驳回';
    state.reserve_precheck_passed = normalizedAction === 'PASS' && hasMockOrderGoods(resolveMockOrderItems(mockOrders[index]!));
    state.reserve_precheck_reasons = state.reserve_precheck_passed ? [] : [state.blocking_reason];
  });

  return {
    oms_status: nextOmsStatus,
    review_status: nextReviewStatus,
    exception_flags: nextExceptionFlags,
    version: 1
  };
};

export const cancelMockOrder = (
  orderId: string,
  _payload: OrderCancelRequest
): OrderReviewState => {
  void _payload;
  const now = dayjs().toISOString();
  const index = mockOrders.findIndex((item) => item.id === orderId);
  if (index >= 0) {
    const current = mockOrders[index];
    mockOrders[index] = {
      ...current,
      oms_status: 'CANCELLED',
      review_status: 'REJECTED',
      updated_at: now
    };
  }
  return {
    oms_status: 'CANCELLED',
    review_status: 'REJECTED',
    exception_flags: [],
    version: 1
  };
};

export const updateMockOrderReceiver = (
  orderId: string,
  payload: OrderReceiverUpdatePayload
): OrderDetail => {
  const now = dayjs().toISOString();
  const index = mockOrders.findIndex((item) => item.id === orderId);
  const base = index >= 0 ? mockOrders[index] : buildOrder(0);
  const normalizedPhone = payload.receiver_phone.trim();
  const updated: OrderSummary = {
    ...base,
    id: orderId,
    receiver_name: payload.receiver_name.trim(),
    receiver_phone: normalizedPhone,
    receiver_phone_last4: normalizedPhone ? normalizedPhone.slice(-4) : base.receiver_phone_last4,
    updated_at: now
  };
  if (index >= 0) {
    mockOrders[index] = updated;
  } else {
    mockOrders.unshift(updated);
  }
  mockOrderReceiverAddresses.set(orderId, payload.receiver_address.trim());
  return getMockOrderDetail(orderId);
};

export const rematchMockOrderItems = (orderId: string): OrderDetail => {
  const now = dayjs().toISOString();
  const index = mockOrders.findIndex((item) => item.id === orderId);
  if (index >= 0) {
    const current = mockOrders[index];
    const nextFlags = (current.exception_flags ?? []).filter(
      (flag) => flag !== 'MAPPING_MISSING'
    );
    mockOrders[index] = {
      ...current,
      mapping_missing: false,
      exception_flags: nextFlags,
      updated_at: now
    };
  }
  return getMockOrderDetail(orderId);
};

export const returnMockOrderToDownload = (orderId: string): OrderReviewState => {
  const now = dayjs().toISOString();
  const index = mockOrders.findIndex((item) => item.id === orderId);
  if (index >= 0) {
    const current = mockOrders[index];
    const nextFlags = Array.from(
      new Set([...(current.exception_flags ?? []), 'REVIEW_REJECTED'])
    );
    mockOrders[index] = {
      ...current,
      review_status: 'REJECTED',
      oms_status: 'REVIEW_PENDING',
      exception_flags: nextFlags,
      updated_at: now
    };
  }
  const order = mockOrders.find((item) => item.id === orderId) ?? buildOrder(0);
  return {
    oms_status: order.oms_status,
    review_status: order.review_status,
    exception_flags: order.exception_flags ?? [],
    version: 1
  };
};

export const requeueMockOrderReview = (orderId: string): OrderReviewState => {
  const now = dayjs().toISOString();
  const index = mockOrders.findIndex((item) => item.id === orderId);
  if (index >= 0) {
    const current = mockOrders[index];
    const nextFlags = (current.exception_flags ?? []).filter(
      (flag) => flag !== 'REVIEW_REJECTED' && flag !== 'REVIEW_FLAGGED'
    );
    mockOrders[index] = {
      ...current,
      review_status: 'PENDING',
      oms_status: 'REVIEW_PENDING',
      exception_flags: nextFlags,
      updated_at: now
    };
  }
  const order = mockOrders.find((item) => item.id === orderId) ?? buildOrder(0);
  return {
    oms_status: order.oms_status,
    review_status: order.review_status,
    exception_flags: order.exception_flags ?? [],
    version: 1
  };
};

const appendMockOmsAction = (
  state: MockOmsMainChainState,
  action: string,
  message: string,
  operatorId?: string | null,
  operatorName?: string | null
) => {
  state.action_logs.push({
    action,
    message,
    operator_id: operatorId ?? null,
    operator_name: operatorName ?? null,
    created_at: dayjs().toISOString()
  });
};

const updateMockOmsState = (
  orderId: string,
  updater: (state: MockOmsMainChainState) => void
) => {
  const state = resolveOrderMainChainState(orderId);
  updater(state);
  mockOmsMainChainStates.set(orderId, state);
  syncOrderSummaryWithMainChain(orderId, state);
  return state;
};

export const listMockOrderExceptions = (query?: {
  status?: string;
  keyword?: string;
  assignee_id?: string | null;
  offset?: number;
  limit?: number;
}) => {
  const items = Array.from(mockOmsMainChainStates.values())
    .filter((state) => state.fulfillment_status === 'NOT_SUBMITTED')
    .filter((state) => state.exception_status !== 'NONE')
    .filter((state) => (query?.status ? state.exception_status === query.status : true))
    .filter((state) => (query?.assignee_id ? state.assignee_id === query.assignee_id : true))
    .filter((state) => {
      if (!query?.keyword) {
        return true;
      }
      const summary = buildMockOrderExceptionSummary(state);
      const keyword = query.keyword.trim().toLowerCase();
      return (
        summary.order_id.toLowerCase().includes(keyword) ||
        summary.order_no.toLowerCase().includes(keyword) ||
        summary.receiver_name?.toLowerCase().includes(keyword) ||
        summary.blocking_reason.toLowerCase().includes(keyword)
      );
    })
    .map((state) => buildMockOrderExceptionSummary(state));

  return {
    items: applyPagination(items, query?.offset, query?.limit),
    total: items.length
  };
};

export const getMockOrderExceptionDetail = (orderId: string) => {
  const state = resolveOrderMainChainState(orderId);
  return {
    ...buildMockOrderExceptionSummary(state),
    blocking_reasons: state.reserve_precheck_reasons.length
      ? [...state.reserve_precheck_reasons]
      : [state.blocking_reason],
    action_logs: state.action_logs.map((record) => ({ ...record })),
    order: getMockOrderDetail(orderId)
  };
};

export const submitMockOrderFulfillment = (
  orderId: string,
  options?: { dryRun?: boolean }
): MockFulfillmentPrecheckResult => {
  const state = resolveOrderMainChainState(orderId);
  const orderDetail = getMockOrderDetail(orderId);
  const reasons: string[] = [];

  if (state.audit_status !== 'REVIEW_PASSED') {
    reasons.push('审核未通过，不能提交发货');
  }
  if (!['NONE', 'RESOLVED', 'CLOSED'].includes(state.exception_status)) {
    reasons.push('异常鏈棴鐜紝涓嶈兘提交鍙戣揣');
  }
  if (!hasMockOrderGoods(orderDetail.items)) {
    reasons.push('订单无货品，不能提交发货');
  }
  if (!state.reserve_precheck_passed) {
    reasons.push(...state.reserve_precheck_reasons);
  }

  if (reasons.length > 0) {
    if (!options?.dryRun) {
      updateMockOmsState(orderId, (current) => {
        current.reserve_status = 'RESERVE_FAILED';
        current.fulfillment_status = 'NOT_SUBMITTED';
        appendMockOmsAction(current, 'SUBMIT_FAILED', '提交发货失败', null, '绯荤粺');
      });
    }

    return {
      order_id: orderId,
      order_no: orderDetail.order.id ?? orderId,
      passed: false,
      reasons,
      reserve_status: 'RESERVE_FAILED',
      fulfillment_status: 'NOT_SUBMITTED',
      target_warehouse_id: state.target_warehouse_id,
      outbound_doc_count: 0
    };
  }

  if (!options?.dryRun) {
    updateMockOmsState(orderId, (current) => {
      current.reserve_status = 'RESERVED';
      current.fulfillment_status = 'WAVE_PENDING';
      current.reserve_precheck_reasons = [];
      appendMockOmsAction(current, 'SUBMIT', '提交鍙戣揣成功锛岃繘鍏ュ緟波次', null, '绯荤粺');
    });
  }

  return {
    order_id: orderId,
    order_no: orderDetail.order.id ?? orderId,
    passed: true,
    reasons: [],
    reserve_status: options?.dryRun ? state.reserve_status : 'RESERVED',
    fulfillment_status: options?.dryRun ? state.fulfillment_status : 'WAVE_PENDING',
    target_warehouse_id: state.target_warehouse_id,
    outbound_doc_count: 1
  };
};

export const claimMockOrderException = (
  orderId: string,
  payload?: { operator_id?: string | null; operator_name?: string | null }
) =>
  buildMockOrderExceptionSummary(
    updateMockOmsState(orderId, (state) => {
      state.audit_status = 'UNDER_REVIEW';
      state.exception_status = 'ASSIGNED';
      state.assignee_id = payload?.operator_id ?? state.assignee_id ?? null;
      state.assignee_name = payload?.operator_name ?? state.assignee_name ?? null;
      appendMockOmsAction(
        state,
        'CLAIM',
        '异常鍗曞凡棰嗗彇',
        payload?.operator_id ?? null,
        payload?.operator_name ?? null
      );
    })
  );

export const assignMockOrderException = (
  orderId: string,
  payload?: {
    operator_id?: string | null;
    operator_name?: string | null;
    assignee_id?: string | null;
    assignee_name?: string | null;
  }
) =>
  buildMockOrderExceptionSummary(
    updateMockOmsState(orderId, (state) => {
      state.audit_status = 'UNDER_REVIEW';
      state.exception_status = 'ASSIGNED';
      state.assignee_id = payload?.assignee_id ?? null;
      state.assignee_name = payload?.assignee_name ?? null;
      appendMockOmsAction(
        state,
        'ASSIGN',
        异常单已分派给 ,
        payload?.operator_id ?? null,
        payload?.operator_name ?? null
      );
    })
  );

export const escalateMockOrderException = (
  orderId: string,
  payload?: { operator_id?: string | null; operator_name?: string | null; reason?: string | null }
) =>
  buildMockOrderExceptionSummary(
    updateMockOmsState(orderId, (state) => {
      state.exception_status = 'ESCALATED';
      if (payload?.reason?.trim()) {
        state.blocking_reason = payload.reason.trim();
      }
      appendMockOmsAction(
        state,
        'ESCALATE',
        payload?.reason?.trim() || '异常单已升级处理',
        payload?.operator_id ?? null,
        payload?.operator_name ?? null
      );
    })
  );

export const resolveMockOrderException = (
  orderId: string,
  payload?: { operator_id?: string | null; operator_name?: string | null; reason?: string | null }
) =>
  buildMockOrderExceptionSummary(
    updateMockOmsState(orderId, (state) => {
      state.audit_status = 'REVIEW_PASSED';
      state.exception_status = 'RESOLVED';
      state.blocking_reason = payload?.reason?.trim() || '异常已复核通过';
      state.reserve_precheck_passed = true;
      state.reserve_precheck_reasons = [];
      appendMockOmsAction(
        state,
        'RESOLVE',
        '订单复核通过，待递交WMS发货',
        payload?.operator_id ?? null,
        payload?.operator_name ?? null
      );
    })
  );

export const holdMockOrderException = (
  orderId: string,
  payload?: { operator_id?: string | null; operator_name?: string | null; reason?: string | null }
) =>
  buildMockOrderExceptionSummary(
    updateMockOmsState(orderId, (state) => {
      state.audit_status = 'UNDER_REVIEW';
      state.exception_status = 'PROCESSING';
      state.blocking_reason = payload?.reason?.trim() || '异常挂起处理中';
      state.reserve_precheck_passed = false;
      state.reserve_precheck_reasons = [state.blocking_reason];
      appendMockOmsAction(
        state,
        'HOLD',
        state.blocking_reason,
        payload?.operator_id ?? null,
        payload?.operator_name ?? null
      );
    })
  );

export const rejectMockOrderException = (
  orderId: string,
  payload?: { operator_id?: string | null; operator_name?: string | null; reason?: string | null }
) =>
  buildMockOrderExceptionSummary(
    updateMockOmsState(orderId, (state) => {
      state.audit_status = 'REVIEW_REJECTED';
      state.exception_status = 'CLOSED';
      state.reserve_status = null;
      state.fulfillment_status = 'NOT_SUBMITTED';
      state.reserve_precheck_passed = false;
      state.blocking_reason = payload?.reason?.trim() || '订单已驳回';
      state.reserve_precheck_reasons = [state.blocking_reason];
      appendMockOmsAction(
        state,
        'REJECT',
        state.blocking_reason,
        payload?.operator_id ?? null,
        payload?.operator_name ?? null
      );
    })
  );

export const requeueMockOrderAudit = (
  orderId: string,
  payload?: { operator_id?: string | null; operator_name?: string | null; reason?: string | null }
) =>
  buildMockOrderExceptionSummary(
    updateMockOmsState(orderId, (state) => {
      state.audit_status = 'REVIEW_REQUIRED';
      state.exception_status = 'OPEN';
      state.fulfillment_status = 'NOT_SUBMITTED';
      state.reserve_status = null;
      state.reserve_precheck_passed = false;
      state.blocking_reason = payload?.reason?.trim() || '宸查€€鍥炲緟瀹℃牳';
      state.reserve_precheck_reasons = [state.blocking_reason];
      state.assignee_id = null;
      state.assignee_name = null;
      appendMockOmsAction(
        state,
        'REQUEUE_AUDIT',
        state.blocking_reason,
        payload?.operator_id ?? null,
        payload?.operator_name ?? null
      );
    })
  );

const normalizeMockOrderSettlementStatus = (
  raw?: string | null
): MockOrderSettlementRecord['status'] => {
  if (raw === 'LOCKED' || raw === 'EXPORTED') {
    return raw;
  }
  return 'OPEN';
};

const resolveMockOrderSettlementRecord = (orderId: string): MockOrderSettlementRecord => {
  const existing = mockOrderSettlementRecords.get(orderId);
  if (existing) {
    return existing;
  }

  const nextRecord: MockOrderSettlementRecord = {
    status: 'OPEN',
    locked_at: null,
    export_job_id: null,
    action_logs: []
  };
  mockOrderSettlementRecords.set(orderId, nextRecord);
  return nextRecord;
};

const appendMockOrderSettlementAction = (
  record: MockOrderSettlementRecord,
  action: string,
  message: string,
  operatorId?: string | null,
  operatorName?: string | null
) => {
  record.action_logs.push({
    action,
    message,
    operator_id: operatorId ?? null,
    operator_name: operatorName ?? null,
    created_at: dayjs().toISOString()
  });
};

const buildMockOrderSettlementPreview = (
  totalAmountCent: number,
  rule: MockOrderSettlementRule = mockOrderSettlementRule
) => {
  const memberDiscount = Math.round((totalAmountCent * rule.member_discount_rate) / 100);
  const promoDiscount = Math.round((totalAmountCent * rule.promo_discount_rate) / 100);
  const maxDiscount = Math.round((totalAmountCent * rule.max_discount_rate) / 100);
  const promotions = [
    {
      name: '浼氬憳鎶樻墸',
      promotion_type: 'MEMBER',
      rate: rule.member_discount_rate,
      amount_cent: memberDiscount
    },
    {
      name: '娓犻亾淇冮攢',
      promotion_type: 'PROMO',
      rate: rule.promo_discount_rate,
      amount_cent: promoDiscount
    }
  ];

  if (rule.stack_mode === 'EXCLUSIVE') {
    const selected = memberDiscount >= promoDiscount ? promotions[0]! : promotions[1]!;
    const discountAmountCent = Math.min(selected.amount_cent, maxDiscount);
    return {
      discount_amount_cent: discountAmountCent,
      receivable_amount_cent: totalAmountCent - discountAmountCent,
      promotions: [
        {
          ...selected,
          amount_cent: discountAmountCent
        }
      ]
    };
  }

  if (rule.stack_mode === 'PRIORITY') {
    const selected =
      rule.member_discount_rate >= rule.promo_discount_rate ? promotions[0]! : promotions[1]!;
    const discountAmountCent = Math.min(selected.amount_cent, maxDiscount);
    return {
      discount_amount_cent: discountAmountCent,
      receivable_amount_cent: totalAmountCent - discountAmountCent,
      promotions: [
        {
          ...selected,
          amount_cent: discountAmountCent
        }
      ]
    };
  }

  const discountAmountCent = Math.min(memberDiscount + promoDiscount, maxDiscount);
  return {
    discount_amount_cent: discountAmountCent,
    receivable_amount_cent: totalAmountCent - discountAmountCent,
    promotions
  };
};

const filterMockSettlementOrders = (query?: {
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: string;
  created_from?: string;
  created_to?: string;
}) =>
  [...mockOrders]
    .filter((order) => {
      const settlementRecord = resolveMockOrderSettlementRecord(order.id);
      const settlementStatus = normalizeMockOrderSettlementStatus(settlementRecord.status);
      const createdAt = dayjs(order.created_at);
      const keyword = query?.keyword?.trim().toLowerCase();

      if (query?.platform && order.platform !== query.platform) {
        return false;
      }
      if (query?.shop_id && order.shop_id !== query.shop_id) {
        return false;
      }
      if (query?.status && settlementStatus !== query.status) {
        return false;
      }
      if (query?.created_from && createdAt.isBefore(dayjs(query.created_from))) {
        return false;
      }
      if (query?.created_to && createdAt.isAfter(dayjs(query.created_to))) {
        return false;
      }
      if (!keyword) {
        return true;
      }

      return (
        order.id.toLowerCase().includes(keyword) ||
        (order.platform_order_id ?? '').toLowerCase().includes(keyword) ||
        (order.receiver_name ?? '').toLowerCase().includes(keyword)
      );
    })
    .sort((left, right) => {
      const byCreatedAt = dayjs(right.created_at).valueOf() - dayjs(left.created_at).valueOf();
      if (byCreatedAt !== 0) {
        return byCreatedAt;
      }
      return right.id.localeCompare(left.id);
    });

const buildMockOrderSettlementSummary = (order: OrderSummary) => {
  const settlementRecord = resolveMockOrderSettlementRecord(order.id);
  const latestAction = settlementRecord.action_logs.at(-1) ?? null;
  const preview = buildMockOrderSettlementPreview(order.total_amount_cent);

  return {
    order_id: order.id,
    order_no: order.platform_order_id ?? order.id,
    platform: order.platform,
    shop_id: order.shop_id,
    receiver_name: order.receiver_name ?? null,
    created_at: order.created_at ?? dayjs().toISOString(),
    total_amount_cent: order.total_amount_cent,
    discount_amount_cent: preview.discount_amount_cent,
    receivable_amount_cent: preview.receivable_amount_cent,
    settlement_status: normalizeMockOrderSettlementStatus(settlementRecord.status),
    locked_at: settlementRecord.locked_at ?? null,
    export_job_id: settlementRecord.export_job_id ?? null,
    latest_action: latestAction?.message ?? null,
    latest_action_at: latestAction?.created_at ?? null
  };
};

const normalizeMockReconciliationAuditStatus = (
  state: MockOmsMainChainState,
  order: OrderSummary
) => {
  if (
    state.audit_status === 'REVIEW_PASSED' ||
    state.audit_status === 'REVIEW_REJECTED' ||
    state.audit_status === 'REVIEW_REQUIRED'
  ) {
    return state.audit_status;
  }
  return resolveAuditStatusFromReviewStatus(order.review_status);
};

const normalizeMockReconciliationExceptionStatus = (state: MockOmsMainChainState) =>
  ['NONE', 'RESOLVED', 'CLOSED'].includes(state.exception_status) ? 'NONE' : 'OPEN';

const buildMockOrderReconciliationBatchId = (
  platform: string,
  shopId: string,
  periodStart: string,
  periodEnd: string
) => `${platform}:${shopId}:${periodStart}:${periodEnd}`;

const parseMockOrderReconciliationBatchId = (raw: string) => {
  const parts = raw.split(':');
  if (parts.length !== 4) {
    throw new Error('鎵规编号鏍煎紡鏃犳晥');
  }

  const [, , periodStart, periodEnd] = parts;
  if (!dayjs(periodStart, 'YYYY-MM-DD', true).isValid()) {
    throw new Error('批次开始日期无效');
  }
  if (!dayjs(periodEnd, 'YYYY-MM-DD', true).isValid()) {
    throw new Error('鎵规缁撴潫鏃ユ湡鏃犳晥');
  }
  if (dayjs(periodStart).isAfter(dayjs(periodEnd))) {
    throw new Error('鎵规鏃ユ湡鑼冨洿鏃犳晥');
  }

  return {
    platform: parts[0]!,
    shop_id: parts[1]!,
    period_start: periodStart,
    period_end: periodEnd,
    batch_id: raw
  };
};

const buildMockOrderReconciliationBatch = (
  orders: OrderSummary[],
  input: {
    platform: string;
    shop_id: string;
    period_start: string;
    period_end: string;
    batch_id: string;
  }
) => {
  const persisted = mockOrderReconciliationRecords.get(input.batch_id);
  let expectedCent = 0;
  let actualCent = 0;
  const reasons: string[] = [];

  const rows = orders.map((order) => {
    const settlementSummary = buildMockOrderSettlementSummary(order);
    const state = resolveOrderMainChainState(order.id);
    const auditStatus = normalizeMockReconciliationAuditStatus(state, order);
    const exceptionStatus = normalizeMockReconciliationExceptionStatus(state);
    const unresolved = exceptionStatus !== 'NONE' || auditStatus === 'REVIEW_REJECTED';

    expectedCent += settlementSummary.receivable_amount_cent;
    actualCent += unresolved ? 0 : settlementSummary.receivable_amount_cent;

    if (exceptionStatus !== 'NONE') {
      const flags = order.exception_flags?.length ? order.exception_flags : [state.blocking_reason];
      for (const reason of flags) {
        if (reason && !reasons.includes(reason)) {
          reasons.push(reason);
        }
      }
    }
    if (auditStatus === 'REVIEW_REJECTED' && !reasons.includes('REVIEW_REJECTED')) {
      reasons.push('REVIEW_REJECTED');
    }

    return {
      order_id: order.id,
      order_no: order.platform_order_id ?? order.id,
      receiver_name: order.receiver_name ?? null,
      total_amount_cent: order.total_amount_cent,
      receivable_amount_cent: settlementSummary.receivable_amount_cent,
      settlement_status: settlementSummary.settlement_status,
      audit_status: auditStatus,
      exception_status: exceptionStatus,
      blocking_reason:
        exceptionStatus !== 'NONE'
          ? state.blocking_reason
          : auditStatus === 'REVIEW_REJECTED'
            ? 'REVIEW_REJECTED'
            : '无差异'
    };
  });

  const diffCent = expectedCent - actualCent;
  const summary = {
    batch_id: input.batch_id,
    platform: input.platform,
    shop_id: input.shop_id,
    period_start: input.period_start,
    period_end: input.period_end,
    order_count: rows.length,
    expected_cent: expectedCent,
    actual_cent: actualCent,
    diff_cent: diffCent,
    status: persisted?.status ?? (diffCent === 0 ? 'DONE' : 'PENDING'),
    reasons,
    settled_at: persisted?.settled_at ?? null
  };

  return {
    summary,
    note: persisted?.note ?? null,
    orders: rows
  };
};

const listMockReconciliationBatches = (query?: {
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
}) => {
  const filteredOrders = filterMockSettlementOrders({
    platform: query?.platform,
    shop_id: query?.shop_id,
    keyword: query?.keyword,
    created_from: query?.created_from,
    created_to: query?.created_to
  });
  const groupedOrders = new Map<string, OrderSummary[]>();

  for (const order of filteredOrders) {
    const key = `${order.platform}::${order.shop_id}`;
    const existing = groupedOrders.get(key) ?? [];
    existing.push(order);
    groupedOrders.set(key, existing);
  }

  const batches = Array.from(groupedOrders.entries())
    .map(([key, orders]) => {
      const [platform, shopId] = key.split('::');
      const createdDates = orders.map((order) => dayjs(order.created_at ?? dayjs().toISOString()));
      const periodStart = query?.created_from
        ? dayjs(query.created_from).format('YYYY-MM-DD')
        : dayjs(Math.min(...createdDates.map((value) => value.valueOf()))).format('YYYY-MM-DD');
      const periodEnd = query?.created_to
        ? dayjs(query.created_to).format('YYYY-MM-DD')
        : dayjs(Math.max(...createdDates.map((value) => value.valueOf()))).format('YYYY-MM-DD');
      const input = {
        platform: platform ?? orders[0]!.platform,
        shop_id: shopId ?? orders[0]!.shop_id,
        period_start: periodStart,
        period_end: periodEnd,
        batch_id: buildMockOrderReconciliationBatchId(
          platform ?? orders[0]!.platform,
          shopId ?? orders[0]!.shop_id,
          periodStart,
          periodEnd
        )
      };

      return buildMockOrderReconciliationBatch(orders, input);
    })
    .filter((batch) => (query?.status ? batch.summary.status === query.status : true))
    .sort((left, right) => {
      if (right.summary.diff_cent !== left.summary.diff_cent) {
        return right.summary.diff_cent - left.summary.diff_cent;
      }
      if (right.summary.order_count !== left.summary.order_count) {
        return right.summary.order_count - left.summary.order_count;
      }
      return left.summary.batch_id.localeCompare(right.summary.batch_id);
    });

  return {
    summary: {
      total_batches: batches.length,
      pending_batches: batches.filter((item) => item.summary.status === 'PENDING').length,
      done_batches: batches.filter((item) => item.summary.status === 'DONE').length,
      expected_total_cent: batches.reduce((total, item) => total + item.summary.expected_cent, 0),
      diff_total_cent: batches.reduce((total, item) => total + item.summary.diff_cent, 0)
    },
    total: batches.length,
    items: applyPagination(
      batches.map((item) => item.summary),
      query?.offset,
      query?.limit
    )
  };
};

export const getMockOrderSettlementRule = (): MockOrderSettlementRule => ({
  ...mockOrderSettlementRule
});

export const updateMockOrderSettlementRule = (
  payload: MockOrderSettlementRuleUpdatePayload
): MockOrderSettlementRule => {
  if (
    payload.expected_version !== undefined &&
    payload.expected_version !== mockOrderSettlementRule.version
  ) {
    throw new Error('结算规则版本冲突，请刷新后重试');
  }

  mockOrderSettlementRule = {
    stack_mode: payload.stack_mode,
    member_discount_rate: Math.max(0, payload.member_discount_rate),
    promo_discount_rate: Math.max(0, payload.promo_discount_rate),
    max_discount_rate: Math.max(0, payload.max_discount_rate),
    version: mockOrderSettlementRule.version + 1,
    updated_at: dayjs().toISOString()
  };

  return getMockOrderSettlementRule();
};

export const listMockOrderSettlements = (query?: {
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
}) => {
  const filteredOrders = filterMockSettlementOrders(query);
  const summaries = filteredOrders.map((order) => buildMockOrderSettlementSummary(order));

  return {
    items: applyPagination(summaries, query?.offset, query?.limit),
    total: summaries.length,
    summary: {
      total_orders: summaries.length,
      open_orders: summaries.filter((item) => item.settlement_status === 'OPEN').length,
      locked_orders: summaries.filter((item) => item.settlement_status === 'LOCKED').length,
      exported_orders: summaries.filter((item) => item.settlement_status === 'EXPORTED').length,
      total_amount_cent: summaries.reduce((total, item) => total + item.total_amount_cent, 0),
      receivable_amount_cent: summaries.reduce(
        (total, item) => total + item.receivable_amount_cent,
        0
      )
    }
  };
};

export const getMockOrderSettlementDetail = (orderId: string) => {
  const order = mockOrders.find((item) => item.id === orderId);
  if (!order) {
    throw new Error('未找到订单结算记录');
  }

  const summary = buildMockOrderSettlementSummary(order);
  const settlementRecord = resolveMockOrderSettlementRecord(orderId);
  const preview = buildMockOrderSettlementPreview(order.total_amount_cent);

  return {
    ...summary,
    promotions: preview.promotions.map((item) => ({ ...item })),
    action_logs: settlementRecord.action_logs.map((item) => ({ ...item }))
  };
};

const executeMockOrderSettlementAction = (
  orderId: string,
  action: 'LOCK' | 'UNLOCK' | 'EXPORT',
  payload?: MockOrderSettlementActionPayload
) => {
  const order = mockOrders.find((item) => item.id === orderId);
  if (!order) {
    throw new Error('未找到订单结算记录');
  }

  const settlementRecord = resolveMockOrderSettlementRecord(orderId);
  const currentStatus = normalizeMockOrderSettlementStatus(settlementRecord.status);

  if (action === 'LOCK') {
    if (currentStatus === 'EXPORTED') {
      throw new Error('已导出的结算单不能再次锁定');
    }
    settlementRecord.status = 'LOCKED';
    settlementRecord.locked_at = dayjs().toISOString();
    appendMockOrderSettlementAction(
      settlementRecord,
      'LOCK',
      '订单结算已锁定',
      payload?.operator_id ?? null,
      payload?.operator_name ?? null
    );
  }

  if (action === 'UNLOCK') {
    if (currentStatus !== 'LOCKED') {
      throw new Error('浠呭凡閿佸畾缁撶畻鍗曞彲瑙ｉ攣');
    }
    settlementRecord.status = 'OPEN';
    settlementRecord.locked_at = null;
    appendMockOrderSettlementAction(
      settlementRecord,
      'UNLOCK',
      '订单结算已解锁',
      payload?.operator_id ?? null,
      payload?.operator_name ?? null
    );
  }

  if (action === 'EXPORT') {
    if (currentStatus !== 'LOCKED') {
      throw new Error('仅已锁定结算单可导出');
    }
    if (!payload?.export_job_id?.trim()) {
      throw new Error('瀵煎嚭任务编号不能为空');
    }
    settlementRecord.status = 'EXPORTED';
    settlementRecord.locked_at = settlementRecord.locked_at ?? dayjs().toISOString();
    settlementRecord.export_job_id = payload.export_job_id.trim();
    appendMockOrderSettlementAction(
      settlementRecord,
      'EXPORT',
      '订单结算已导出',
      payload?.operator_id ?? null,
      payload?.operator_name ?? null
    );
  }

  mockOrderSettlementRecords.set(orderId, settlementRecord);
  return buildMockOrderSettlementSummary(order);
};

export const lockMockOrderSettlement = (
  orderId: string,
  payload?: MockOrderSettlementActionPayload
) => executeMockOrderSettlementAction(orderId, 'LOCK', payload);

export const unlockMockOrderSettlement = (
  orderId: string,
  payload?: MockOrderSettlementActionPayload
) => executeMockOrderSettlementAction(orderId, 'UNLOCK', payload);

export const exportMockOrderSettlement = (
  orderId: string,
  payload?: MockOrderSettlementActionPayload
) => executeMockOrderSettlementAction(orderId, 'EXPORT', payload);

export const listMockOrderReconciliations = (query?: {
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
}) => listMockReconciliationBatches(query);

export const getMockOrderReconciliationDetail = (batchId: string) => {
  const parsed = parseMockOrderReconciliationBatchId(batchId);
  const orders = filterMockSettlementOrders({
    platform: parsed.platform,
    shop_id: parsed.shop_id,
    created_from: dayjs(parsed.period_start).startOf('day').toISOString(),
    created_to: dayjs(parsed.period_end).endOf('day').toISOString()
  });

  if (orders.length === 0) {
    throw new Error('未找到销售对账记录');
  }

  const batch = buildMockOrderReconciliationBatch(orders, parsed);
  return {
    ...batch.summary,
    note: batch.note,
    orders: batch.orders
  };
};

export const settleMockOrderReconciliation = (
  batchId: string,
  payload?: MockOrderReconciliationActionPayload
) => {
  const current = getMockOrderReconciliationDetail(batchId);
  mockOrderReconciliationRecords.set(batchId, {
    status: 'DONE',
    note: payload?.note?.trim() || null,
    settled_at: dayjs().toISOString()
  });

  const next = getMockOrderReconciliationDetail(batchId);
  return {
    ...next,
    status: 'DONE',
    settled_at: next.settled_at ?? current.settled_at ?? dayjs().toISOString()
  };
};
