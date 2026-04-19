import { describe, expect, it } from 'vitest';

import { BundleComboPage } from './BundleComboPage';
import { HotSkuPage } from './HotSkuPage';
import { SlowSkuPage } from './SlowSkuPage';

describe('catalog insight pages', () => {
  it('exports the phase one wrapper pages with stable test ids', () => {
    expect(HotSkuPage().props['data-testid']).toBe('insights-catalog-hot-sku-page');
    expect(SlowSkuPage().props['data-testid']).toBe('insights-catalog-slow-sku-page');
    expect(BundleComboPage().props['data-testid']).toBe('insights-catalog-bundle-combo-page');
  });
});
