import { describe, expect, it } from 'vitest';

import {
  createMockShopProductMatchTask,
  getMockShopProductSkuList,
  manualMockShopProductMatch
} from './shopProducts';

describe('shop product match governance mocks', () => {
  it('supports mapping_status, match_source and conflict_reason filters', () => {
    const mapped = getMockShopProductSkuList({ mapping_status: 'ACTIVE', limit: 50 });
    expect(mapped.items.length).toBeGreaterThan(0);
    expect(mapped.items.every((item) => item.mapping_status === 'ACTIVE')).toBe(true);

    const manual = getMockShopProductSkuList({ match_source: 'MANUAL', limit: 50 });
    expect(manual.items.length).toBeGreaterThan(0);
    expect(manual.items.every((item) => item.match_source === 'MANUAL')).toBe(true);

    const conflict = getMockShopProductSkuList({ conflict_reason: 'NO_CANDIDATE', limit: 50 });
    expect(conflict.items.length).toBeGreaterThan(0);
    expect(conflict.items.every((item) => item.conflict_reason === 'NO_CANDIDATE')).toBe(true);
  });

  it('keeps failed rows unmatched in auto-match and allows manual remediation', () => {
    const before = getMockShopProductSkuList({ conflict_reason: 'NO_CANDIDATE', limit: 50 });
    expect(before.items.length).toBeGreaterThan(0);
    const target = before.items[0];
    expect(target.mapping_id ?? null).toBeNull();

    const autoResult = createMockShopProductMatchTask({ sku_record_ids: [target.sku_record_id] });
    expect(autoResult.failures.length).toBe(1);
    expect(autoResult.failures[0]?.sku_record_id).toBe(target.sku_record_id);

    const afterAuto = getMockShopProductSkuList({ conflict_reason: 'NO_CANDIDATE', limit: 50 });
    const autoTarget = afterAuto.items.find((item) => item.sku_record_id === target.sku_record_id);
    expect(autoTarget?.mapping_id ?? null).toBeNull();

    manualMockShopProductMatch({ sku_record_ids: [target.sku_record_id], sku_id: 'sku-remediate' });
    const afterManual = getMockShopProductSkuList({ limit: 50 });
    const remediated = afterManual.items.find((item) => item.sku_record_id === target.sku_record_id);
    expect(remediated?.mapping_id).toBeTruthy();
    expect(remediated?.match_source).toBe('MANUAL');
    expect(remediated?.conflict_reason ?? null).toBeNull();
  });
});
