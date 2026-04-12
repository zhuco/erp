import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/utils/env', () => ({
  apiBaseUrl: 'http://localhost:8080',
  useMocks: true
}));

import {
  createMarketingPromotion,
  deleteMarketingPromotion,
  listMarketingPromotions,
  updateMarketingPromotion
} from './marketingPromotions';

const tenantId = '11111111-1111-1111-1111-111111111111';

describe('marketingPromotions mock API', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('creates and updates promotion', async () => {
    const created = await createMarketingPromotion(tenantId, {
      name: 'UT-促销-001',
      promo_type: 'DISCOUNT',
      scope_json: { channel_ids: ['CH-UT'] },
      rule_json: { discount: 0.85 }
    });
    expect(created.name).toBe('UT-促销-001');

    const updated = await updateMarketingPromotion(tenantId, created.promotion_id, {
      name: 'UT-促销-001-已更新',
      status: 'ACTIVE'
    });
    expect(updated.name).toBe('UT-促销-001-已更新');
    expect(updated.status).toBe('ACTIVE');

    const list = await listMarketingPromotions(tenantId, { keyword: 'UT-促销-001-已更新' });
    expect(list.items.some((item) => item.promotion_id === created.promotion_id)).toBe(true);
  });

  it('deletes created promotion', async () => {
    const created = await createMarketingPromotion(tenantId, {
      name: 'UT-促销-DELETE',
      promo_type: 'FULL_REDUCTION'
    });

    const deleted = await deleteMarketingPromotion(tenantId, created.promotion_id);
    expect(deleted.promotion_id).toBe(created.promotion_id);

    const list = await listMarketingPromotions(tenantId, { keyword: 'UT-促销-DELETE' });
    expect(list.items.some((item) => item.promotion_id === created.promotion_id)).toBe(false);
  });
});

