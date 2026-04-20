import { describe, expect, it } from 'vitest';

import { isReducedUiPathAllowed } from './WorkbenchLayout';

describe('WorkbenchLayout reduced UI shell', () => {
  it('only allows the approved five-page shell routes', () => {
    expect(isReducedUiPathAllowed('/')).toBe(true);
    expect(isReducedUiPathAllowed('/dashboard')).toBe(true);
    expect(isReducedUiPathAllowed('/channels')).toBe(true);
    expect(isReducedUiPathAllowed('/sales/order-query')).toBe(true);
    expect(isReducedUiPathAllowed('/orders/review')).toBe(true);
    expect(isReducedUiPathAllowed('/orders/recheck')).toBe(true);
    expect(isReducedUiPathAllowed('/sales/settings')).toBe(true);

    expect(isReducedUiPathAllowed('/orders')).toBe(false);
    expect(isReducedUiPathAllowed('/orders/exceptions')).toBe(false);
    expect(isReducedUiPathAllowed('/channels/shops')).toBe(false);
    expect(isReducedUiPathAllowed('/products')).toBe(false);
  });
});
