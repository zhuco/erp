import { describe, expect, it, vi } from 'vitest';

vi.mock('@/utils/env', () => ({
  apiBaseUrl: 'http://localhost:8080',
  useMocks: true
}));

import { deleteMarketingCoupon, updateMarketingCoupon } from './marketingCoupons';
import { deleteMarketingCost, updateMarketingCost } from './marketingCosts';
import { deleteMarketingGiftPolicy, updateMarketingGiftPolicy } from './marketingGifts';
import { deleteMarketingPostmortem, updateMarketingPostmortem } from './marketingPostmortems';
import { deleteMarketingPriceList, updateMarketingPriceList } from './marketingPriceLists';
import { deleteMarketingPromotion, updateMarketingPromotion } from './marketingPromotions';

const tenantId = '11111111-1111-1111-1111-111111111111';

describe('marketing mock API error branches', () => {
  it('throws on invalid price list id', async () => {
    await expect(
      updateMarketingPriceList(tenantId, 'MPL-NOT-EXISTS', {
        name: 'invalid-price-list'
      })
    ).rejects.toThrow('价表不存在');

    await expect(deleteMarketingPriceList(tenantId, 'MPL-NOT-EXISTS')).rejects.toThrow('价表不存在');
  });

  it('throws on invalid promotion id', async () => {
    await expect(
      updateMarketingPromotion(tenantId, 'MP-NOT-EXISTS', {
        name: 'invalid-promotion'
      })
    ).rejects.toThrow('促销活动不存在');

    await expect(deleteMarketingPromotion(tenantId, 'MP-NOT-EXISTS')).rejects.toThrow('促销活动不存在');
  });

  it('throws on invalid gift policy id', async () => {
    await expect(
      updateMarketingGiftPolicy(tenantId, 'MGP-NOT-EXISTS', {
        name: 'invalid-gift-policy'
      })
    ).rejects.toThrow('赠品策略不存在');

    await expect(deleteMarketingGiftPolicy(tenantId, 'MGP-NOT-EXISTS')).rejects.toThrow('赠品策略不存在');
  });

  it('throws on invalid coupon id', async () => {
    await expect(
      updateMarketingCoupon(tenantId, 'MC-NOT-EXISTS', {
        name: 'invalid-coupon'
      })
    ).rejects.toThrow('优惠券不存在');

    await expect(deleteMarketingCoupon(tenantId, 'MC-NOT-EXISTS')).rejects.toThrow('优惠券不存在');
  });

  it('throws on invalid cost id', async () => {
    await expect(
      updateMarketingCost(tenantId, 'MCO-NOT-EXISTS', {
        name: 'invalid-cost'
      })
    ).rejects.toThrow('促销费用不存在');

    await expect(deleteMarketingCost(tenantId, 'MCO-NOT-EXISTS')).rejects.toThrow('促销费用不存在');
  });

  it('throws on invalid postmortem id', async () => {
    await expect(
      updateMarketingPostmortem(tenantId, 'MPM-NOT-EXISTS', {
        name: 'invalid-postmortem'
      })
    ).rejects.toThrow('活动复盘不存在');

    await expect(deleteMarketingPostmortem(tenantId, 'MPM-NOT-EXISTS')).rejects.toThrow('活动复盘不存在');
  });
});

