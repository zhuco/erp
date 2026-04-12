import dayjs from 'dayjs';

import type {
  MarketingCoupon,
  MarketingCouponCreatePayload,
  MarketingCouponListQuery,
  MarketingCouponListResponse,
  MarketingCouponUpdatePayload
} from '@/api/marketingCoupons';
import type {
  MarketingCost,
  MarketingCostCreatePayload,
  MarketingCostListQuery,
  MarketingCostListResponse,
  MarketingCostUpdatePayload
} from '@/api/marketingCosts';
import type {
  MarketingGiftPolicy,
  MarketingGiftPolicyCreatePayload,
  MarketingGiftPolicyListQuery,
  MarketingGiftPolicyListResponse,
  MarketingGiftPolicyUpdatePayload
} from '@/api/marketingGifts';
import type {
  MarketingPostmortem,
  MarketingPostmortemCreatePayload,
  MarketingPostmortemListQuery,
  MarketingPostmortemListResponse,
  MarketingPostmortemUpdatePayload
} from '@/api/marketingPostmortems';
import type {
  MarketingPriceItem,
  MarketingPriceItemCreatePayload,
  MarketingPriceItemUpdatePayload,
  MarketingPriceList,
  MarketingPriceListCreatePayload,
  MarketingPriceListDetail,
  MarketingPriceListListQuery,
  MarketingPriceListListResponse,
  MarketingPriceListUpdatePayload
} from '@/api/marketingPriceLists';
import type {
  MarketingPromotion,
  MarketingPromotionCreatePayload,
  MarketingPromotionListQuery,
  MarketingPromotionListResponse,
  MarketingPromotionUpdatePayload
} from '@/api/marketingPromotions';

import { applyPagination, mockTenantId } from './data';

const nowIso = () => dayjs().toISOString();

let sequence = 1;
const nextId = (prefix: string) => `${prefix}-${Date.now()}-${sequence++}`;

const paginate = <T>(items: T[], offset?: number, limit?: number): { items: T[]; total: number } => {
  const safeOffset = Math.max(0, offset ?? 0);
  const safeLimit = Math.max(1, limit ?? 20);
  return {
    items: applyPagination(items, safeOffset, safeLimit),
    total: items.length
  };
};

const includesKeyword = (value: string | null | undefined, keyword: string | undefined): boolean => {
  if (!keyword) {
    return true;
  }
  return (value ?? '').toLowerCase().includes(keyword.toLowerCase());
};

const mockMarketingPriceLists: MarketingPriceList[] = [
  {
    price_list_id: 'MPL-DEFAULT',
    tenant_id: mockTenantId,
    name: '默认渠道价表',
    channel_id: null,
    currency: 'CNY',
    includes_tax: true,
    status: 'ACTIVE',
    version: 1,
    effective_from: dayjs().subtract(30, 'day').toISOString(),
    effective_to: null,
    notes: '系统默认价表',
    created_by: null,
    created_at: dayjs().subtract(30, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockMarketingPriceItems = new Map<string, MarketingPriceItem[]>([
  [
    'MPL-DEFAULT',
    [
      {
        item_id: 'MPI-DEFAULT-1',
        tenant_id: mockTenantId,
        price_list_id: 'MPL-DEFAULT',
        sku_id: 'SKU-001',
        base_price_cent: 1990,
        channel_price_cent: 1880,
        member_price_cent: 1790,
        created_at: dayjs().subtract(20, 'day').toISOString(),
        updated_at: dayjs().subtract(2, 'day').toISOString()
      }
    ]
  ]
]);

const mockMarketingPromotions: MarketingPromotion[] = [
  {
    promotion_id: 'MP-DEFAULT',
    tenant_id: mockTenantId,
    name: '默认促销活动',
    promo_type: 'FULL_REDUCTION',
    status: 'ACTIVE',
    priority: 10,
    exclusive: false,
    scope_type: 'ALL',
    scope_json: {},
    rule_json: { threshold: 2, discount: 0.9 },
    start_at: dayjs().subtract(7, 'day').toISOString(),
    end_at: dayjs().add(7, 'day').toISOString(),
    created_by: null,
    created_at: dayjs().subtract(7, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockMarketingGiftPolicies: MarketingGiftPolicy[] = [
  {
    policy_id: 'MGP-DEFAULT',
    tenant_id: mockTenantId,
    name: '默认赠品策略',
    status: 'ACTIVE',
    strategy: 'RANDOM',
    probability: 0.5,
    scope_json: {},
    gift_pool: [{ sku_id: 'SKU-GIFT-1', qty: 1 }],
    start_at: dayjs().subtract(15, 'day').toISOString(),
    end_at: null,
    created_by: null,
    created_at: dayjs().subtract(15, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockMarketingCoupons: MarketingCoupon[] = [
  {
    coupon_id: 'MC-DEFAULT',
    tenant_id: mockTenantId,
    name: '默认优惠券',
    code_prefix: 'ERP',
    discount_type: 'PERCENTAGE',
    discount_value: 9,
    max_usage: 1000,
    per_user_limit: 1,
    status: 'ACTIVE',
    rules_json: {},
    start_at: dayjs().subtract(10, 'day').toISOString(),
    end_at: dayjs().add(20, 'day').toISOString(),
    created_by: null,
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  }
];

const mockMarketingCosts: MarketingCost[] = [
  {
    cost_id: 'MCO-DEFAULT',
    tenant_id: mockTenantId,
    name: '默认平台广告费',
    channel_id: null,
    amount_cent: 500000,
    currency: 'CNY',
    allocation_rule: { mode: 'revenue_ratio' },
    period_start: dayjs().subtract(1, 'month').format('YYYY-MM-DD'),
    period_end: dayjs().format('YYYY-MM-DD'),
    status: 'ACTIVE',
    created_by: null,
    created_at: dayjs().subtract(20, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockMarketingPostmortems: MarketingPostmortem[] = [
  {
    postmortem_id: 'MPM-DEFAULT',
    tenant_id: mockTenantId,
    name: '默认活动复盘',
    promotion_id: 'MP-DEFAULT',
    channel_id: null,
    summary: '整体 ROI 稳定，建议复用策略。',
    metrics_json: { roi: 1.8, cvr: 0.12 },
    created_by: null,
    created_at: dayjs().subtract(5, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

export const listMockMarketingPriceLists = (
  query: MarketingPriceListListQuery
): MarketingPriceListListResponse => {
  const keyword = query.keyword?.trim();
  const filtered = mockMarketingPriceLists.filter((item) => {
    if (!includesKeyword(item.name, keyword)) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.channel_id && item.channel_id !== query.channel_id) {
      return false;
    }
    if (query.currency && item.currency !== query.currency) {
      return false;
    }
    return true;
  });
  return paginate(filtered, query.offset, query.limit);
};

export const getMockMarketingPriceList = (priceListId: string): MarketingPriceListDetail => {
  const priceList = mockMarketingPriceLists.find((item) => item.price_list_id === priceListId);
  if (!priceList) {
    throw new Error('价表不存在');
  }
  return {
    price_list: { ...priceList },
    items: [...(mockMarketingPriceItems.get(priceListId) ?? [])]
  };
};

export const createMockMarketingPriceList = (
  payload: MarketingPriceListCreatePayload
): MarketingPriceList => {
  const now = nowIso();
  const record: MarketingPriceList = {
    price_list_id: nextId('MPL'),
    tenant_id: mockTenantId,
    name: payload.name,
    channel_id: payload.channel_id ?? null,
    currency: payload.currency ?? 'CNY',
    includes_tax: payload.includes_tax ?? true,
    status: payload.status ?? 'DRAFT',
    version: payload.version ?? 1,
    effective_from: payload.effective_from ?? null,
    effective_to: payload.effective_to ?? null,
    notes: payload.notes ?? null,
    created_by: null,
    created_at: now,
    updated_at: now
  };
  mockMarketingPriceLists.unshift(record);
  mockMarketingPriceItems.set(record.price_list_id, []);
  return record;
};

export const updateMockMarketingPriceList = (
  priceListId: string,
  payload: MarketingPriceListUpdatePayload
): MarketingPriceList => {
  const index = mockMarketingPriceLists.findIndex((item) => item.price_list_id === priceListId);
  if (index < 0) {
    throw new Error('价表不存在');
  }
  const current = mockMarketingPriceLists[index];
  const next: MarketingPriceList = {
    ...current,
    name: payload.name ?? current.name,
    channel_id: payload.channel_id ?? current.channel_id,
    currency: payload.currency ?? current.currency,
    includes_tax: payload.includes_tax ?? current.includes_tax,
    status: payload.status ?? current.status,
    version: payload.version ?? current.version,
    effective_from: payload.effective_from ?? current.effective_from,
    effective_to: payload.effective_to ?? current.effective_to,
    notes: payload.notes ?? current.notes,
    updated_at: nowIso()
  };
  mockMarketingPriceLists[index] = next;
  return next;
};

export const deleteMockMarketingPriceList = (priceListId: string): MarketingPriceList => {
  const index = mockMarketingPriceLists.findIndex((item) => item.price_list_id === priceListId);
  if (index < 0) {
    throw new Error('价表不存在');
  }
  const [removed] = mockMarketingPriceLists.splice(index, 1);
  mockMarketingPriceItems.delete(priceListId);
  return removed;
};

export const createMockMarketingPriceItem = (
  priceListId: string,
  payload: MarketingPriceItemCreatePayload
): MarketingPriceItem => {
  const parentExists = mockMarketingPriceLists.some((item) => item.price_list_id === priceListId);
  if (!parentExists) {
    throw new Error('价表不存在');
  }
  const now = nowIso();
  const record: MarketingPriceItem = {
    item_id: nextId('MPI'),
    tenant_id: mockTenantId,
    price_list_id: priceListId,
    sku_id: payload.sku_id,
    base_price_cent: payload.base_price_cent ?? null,
    channel_price_cent: payload.channel_price_cent ?? null,
    member_price_cent: payload.member_price_cent ?? null,
    created_at: now,
    updated_at: now
  };
  const items = mockMarketingPriceItems.get(priceListId) ?? [];
  items.unshift(record);
  mockMarketingPriceItems.set(priceListId, items);
  return record;
};

export const updateMockMarketingPriceItem = (
  itemId: string,
  payload: MarketingPriceItemUpdatePayload
): MarketingPriceItem => {
  for (const [priceListId, items] of mockMarketingPriceItems.entries()) {
    const index = items.findIndex((item) => item.item_id === itemId);
    if (index >= 0) {
      const current = items[index];
      const next: MarketingPriceItem = {
        ...current,
        base_price_cent: payload.base_price_cent ?? current.base_price_cent,
        channel_price_cent: payload.channel_price_cent ?? current.channel_price_cent,
        member_price_cent: payload.member_price_cent ?? current.member_price_cent,
        updated_at: nowIso()
      };
      items[index] = next;
      mockMarketingPriceItems.set(priceListId, items);
      return next;
    }
  }
  throw new Error('价表明细不存在');
};

export const deleteMockMarketingPriceItem = (itemId: string): MarketingPriceItem => {
  for (const [priceListId, items] of mockMarketingPriceItems.entries()) {
    const index = items.findIndex((item) => item.item_id === itemId);
    if (index >= 0) {
      const [removed] = items.splice(index, 1);
      mockMarketingPriceItems.set(priceListId, items);
      return removed;
    }
  }
  throw new Error('价表明细不存在');
};

export const listMockMarketingPromotions = (
  query: MarketingPromotionListQuery
): MarketingPromotionListResponse => {
  const keyword = query.keyword?.trim();
  const filtered = mockMarketingPromotions.filter((item) => {
    if (!includesKeyword(item.name, keyword)) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.promo_type && item.promo_type !== query.promo_type) {
      return false;
    }
    return true;
  });
  return paginate(filtered, query.offset, query.limit);
};

export const createMockMarketingPromotion = (
  payload: MarketingPromotionCreatePayload
): MarketingPromotion => {
  const now = nowIso();
  const record: MarketingPromotion = {
    promotion_id: nextId('MP'),
    tenant_id: mockTenantId,
    name: payload.name,
    promo_type: payload.promo_type,
    status: payload.status ?? 'DRAFT',
    priority: payload.priority ?? 0,
    exclusive: payload.exclusive ?? false,
    scope_type: payload.scope_type ?? 'ALL',
    scope_json: payload.scope_json ?? {},
    rule_json: payload.rule_json ?? {},
    start_at: payload.start_at ?? null,
    end_at: payload.end_at ?? null,
    created_by: null,
    created_at: now,
    updated_at: now
  };
  mockMarketingPromotions.unshift(record);
  return record;
};

export const updateMockMarketingPromotion = (
  promotionId: string,
  payload: MarketingPromotionUpdatePayload
): MarketingPromotion => {
  const index = mockMarketingPromotions.findIndex((item) => item.promotion_id === promotionId);
  if (index < 0) {
    throw new Error('促销活动不存在');
  }
  const current = mockMarketingPromotions[index];
  const next: MarketingPromotion = {
    ...current,
    name: payload.name ?? current.name,
    promo_type: payload.promo_type ?? current.promo_type,
    status: payload.status ?? current.status,
    priority: payload.priority ?? current.priority,
    exclusive: payload.exclusive ?? current.exclusive,
    scope_type: payload.scope_type ?? current.scope_type,
    scope_json: payload.scope_json ?? current.scope_json,
    rule_json: payload.rule_json ?? current.rule_json,
    start_at: payload.start_at ?? current.start_at,
    end_at: payload.end_at ?? current.end_at,
    updated_at: nowIso()
  };
  mockMarketingPromotions[index] = next;
  return next;
};

export const deleteMockMarketingPromotion = (promotionId: string): MarketingPromotion => {
  const index = mockMarketingPromotions.findIndex((item) => item.promotion_id === promotionId);
  if (index < 0) {
    throw new Error('促销活动不存在');
  }
  const [removed] = mockMarketingPromotions.splice(index, 1);
  return removed;
};

export const listMockMarketingGiftPolicies = (
  query: MarketingGiftPolicyListQuery
): MarketingGiftPolicyListResponse => {
  const keyword = query.keyword?.trim();
  const filtered = mockMarketingGiftPolicies.filter((item) => {
    if (!includesKeyword(item.name, keyword)) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    return true;
  });
  return paginate(filtered, query.offset, query.limit);
};

export const createMockMarketingGiftPolicy = (
  payload: MarketingGiftPolicyCreatePayload
): MarketingGiftPolicy => {
  const now = nowIso();
  const record: MarketingGiftPolicy = {
    policy_id: nextId('MGP'),
    tenant_id: mockTenantId,
    name: payload.name,
    status: payload.status ?? 'DRAFT',
    strategy: payload.strategy ?? 'RANDOM',
    probability: payload.probability ?? null,
    scope_json: payload.scope_json ?? {},
    gift_pool: payload.gift_pool ?? [],
    start_at: payload.start_at ?? null,
    end_at: payload.end_at ?? null,
    created_by: null,
    created_at: now,
    updated_at: now
  };
  mockMarketingGiftPolicies.unshift(record);
  return record;
};

export const updateMockMarketingGiftPolicy = (
  policyId: string,
  payload: MarketingGiftPolicyUpdatePayload
): MarketingGiftPolicy => {
  const index = mockMarketingGiftPolicies.findIndex((item) => item.policy_id === policyId);
  if (index < 0) {
    throw new Error('赠品策略不存在');
  }
  const current = mockMarketingGiftPolicies[index];
  const next: MarketingGiftPolicy = {
    ...current,
    name: payload.name ?? current.name,
    status: payload.status ?? current.status,
    strategy: payload.strategy ?? current.strategy,
    probability: payload.probability ?? current.probability,
    scope_json: payload.scope_json ?? current.scope_json,
    gift_pool: payload.gift_pool ?? current.gift_pool,
    start_at: payload.start_at ?? current.start_at,
    end_at: payload.end_at ?? current.end_at,
    updated_at: nowIso()
  };
  mockMarketingGiftPolicies[index] = next;
  return next;
};

export const deleteMockMarketingGiftPolicy = (policyId: string): MarketingGiftPolicy => {
  const index = mockMarketingGiftPolicies.findIndex((item) => item.policy_id === policyId);
  if (index < 0) {
    throw new Error('赠品策略不存在');
  }
  const [removed] = mockMarketingGiftPolicies.splice(index, 1);
  return removed;
};

export const listMockMarketingCoupons = (
  query: MarketingCouponListQuery
): MarketingCouponListResponse => {
  const keyword = query.keyword?.trim();
  const filtered = mockMarketingCoupons.filter((item) => {
    if (!includesKeyword(item.name, keyword)) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.discount_type && item.discount_type !== query.discount_type) {
      return false;
    }
    return true;
  });
  return paginate(filtered, query.offset, query.limit);
};

export const createMockMarketingCoupon = (
  payload: MarketingCouponCreatePayload
): MarketingCoupon => {
  const now = nowIso();
  const record: MarketingCoupon = {
    coupon_id: nextId('MC'),
    tenant_id: mockTenantId,
    name: payload.name,
    code_prefix: payload.code_prefix ?? null,
    discount_type: payload.discount_type,
    discount_value: payload.discount_value,
    max_usage: payload.max_usage ?? 0,
    per_user_limit: payload.per_user_limit ?? 1,
    status: payload.status ?? 'DRAFT',
    rules_json: payload.rules_json ?? {},
    start_at: payload.start_at ?? null,
    end_at: payload.end_at ?? null,
    created_by: null,
    created_at: now,
    updated_at: now
  };
  mockMarketingCoupons.unshift(record);
  return record;
};

export const updateMockMarketingCoupon = (
  couponId: string,
  payload: MarketingCouponUpdatePayload
): MarketingCoupon => {
  const index = mockMarketingCoupons.findIndex((item) => item.coupon_id === couponId);
  if (index < 0) {
    throw new Error('优惠券不存在');
  }
  const current = mockMarketingCoupons[index];
  const next: MarketingCoupon = {
    ...current,
    name: payload.name ?? current.name,
    code_prefix: payload.code_prefix ?? current.code_prefix,
    discount_type: payload.discount_type ?? current.discount_type,
    discount_value: payload.discount_value ?? current.discount_value,
    max_usage: payload.max_usage ?? current.max_usage,
    per_user_limit: payload.per_user_limit ?? current.per_user_limit,
    status: payload.status ?? current.status,
    rules_json: payload.rules_json ?? current.rules_json,
    start_at: payload.start_at ?? current.start_at,
    end_at: payload.end_at ?? current.end_at,
    updated_at: nowIso()
  };
  mockMarketingCoupons[index] = next;
  return next;
};

export const deleteMockMarketingCoupon = (couponId: string): MarketingCoupon => {
  const index = mockMarketingCoupons.findIndex((item) => item.coupon_id === couponId);
  if (index < 0) {
    throw new Error('优惠券不存在');
  }
  const [removed] = mockMarketingCoupons.splice(index, 1);
  return removed;
};

export const listMockMarketingCosts = (
  query: MarketingCostListQuery
): MarketingCostListResponse => {
  const keyword = query.keyword?.trim();
  const filtered = mockMarketingCosts.filter((item) => {
    if (!includesKeyword(item.name, keyword)) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.channel_id && item.channel_id !== query.channel_id) {
      return false;
    }
    return true;
  });
  return paginate(filtered, query.offset, query.limit);
};

export const createMockMarketingCost = (payload: MarketingCostCreatePayload): MarketingCost => {
  const now = nowIso();
  const record: MarketingCost = {
    cost_id: nextId('MCO'),
    tenant_id: mockTenantId,
    name: payload.name,
    channel_id: payload.channel_id ?? null,
    amount_cent: payload.amount_cent,
    currency: payload.currency ?? 'CNY',
    allocation_rule: payload.allocation_rule ?? {},
    period_start: payload.period_start ?? null,
    period_end: payload.period_end ?? null,
    status: payload.status ?? 'DRAFT',
    created_by: null,
    created_at: now,
    updated_at: now
  };
  mockMarketingCosts.unshift(record);
  return record;
};

export const updateMockMarketingCost = (
  costId: string,
  payload: MarketingCostUpdatePayload
): MarketingCost => {
  const index = mockMarketingCosts.findIndex((item) => item.cost_id === costId);
  if (index < 0) {
    throw new Error('促销费用不存在');
  }
  const current = mockMarketingCosts[index];
  const next: MarketingCost = {
    ...current,
    name: payload.name ?? current.name,
    channel_id: payload.channel_id ?? current.channel_id,
    amount_cent: payload.amount_cent ?? current.amount_cent,
    currency: payload.currency ?? current.currency,
    allocation_rule: payload.allocation_rule ?? current.allocation_rule,
    period_start: payload.period_start ?? current.period_start,
    period_end: payload.period_end ?? current.period_end,
    status: payload.status ?? current.status,
    updated_at: nowIso()
  };
  mockMarketingCosts[index] = next;
  return next;
};

export const deleteMockMarketingCost = (costId: string): MarketingCost => {
  const index = mockMarketingCosts.findIndex((item) => item.cost_id === costId);
  if (index < 0) {
    throw new Error('促销费用不存在');
  }
  const [removed] = mockMarketingCosts.splice(index, 1);
  return removed;
};

export const listMockMarketingPostmortems = (
  query: MarketingPostmortemListQuery
): MarketingPostmortemListResponse => {
  const keyword = query.keyword?.trim();
  const filtered = mockMarketingPostmortems.filter((item) => {
    if (!includesKeyword(item.name, keyword)) {
      return false;
    }
    if (query.promotion_id && item.promotion_id !== query.promotion_id) {
      return false;
    }
    if (query.channel_id && item.channel_id !== query.channel_id) {
      return false;
    }
    return true;
  });
  return paginate(filtered, query.offset, query.limit);
};

export const createMockMarketingPostmortem = (
  payload: MarketingPostmortemCreatePayload
): MarketingPostmortem => {
  const now = nowIso();
  const record: MarketingPostmortem = {
    postmortem_id: nextId('MPM'),
    tenant_id: mockTenantId,
    name: payload.name,
    promotion_id: payload.promotion_id ?? null,
    channel_id: payload.channel_id ?? null,
    summary: payload.summary ?? null,
    metrics_json: payload.metrics_json ?? {},
    created_by: null,
    created_at: now,
    updated_at: now
  };
  mockMarketingPostmortems.unshift(record);
  return record;
};

export const updateMockMarketingPostmortem = (
  postmortemId: string,
  payload: MarketingPostmortemUpdatePayload
): MarketingPostmortem => {
  const index = mockMarketingPostmortems.findIndex((item) => item.postmortem_id === postmortemId);
  if (index < 0) {
    throw new Error('活动复盘不存在');
  }
  const current = mockMarketingPostmortems[index];
  const next: MarketingPostmortem = {
    ...current,
    name: payload.name ?? current.name,
    promotion_id: payload.promotion_id ?? current.promotion_id,
    channel_id: payload.channel_id ?? current.channel_id,
    summary: payload.summary ?? current.summary,
    metrics_json: payload.metrics_json ?? current.metrics_json,
    updated_at: nowIso()
  };
  mockMarketingPostmortems[index] = next;
  return next;
};

export const deleteMockMarketingPostmortem = (postmortemId: string): MarketingPostmortem => {
  const index = mockMarketingPostmortems.findIndex((item) => item.postmortem_id === postmortemId);
  if (index < 0) {
    throw new Error('活动复盘不存在');
  }
  const [removed] = mockMarketingPostmortems.splice(index, 1);
  return removed;
};
