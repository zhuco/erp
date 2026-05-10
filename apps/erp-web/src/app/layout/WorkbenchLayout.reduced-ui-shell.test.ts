import { describe, expect, it } from 'vitest';

import { isReducedUiPathAllowed } from './WorkbenchLayout';

describe('WorkbenchLayout reduced UI shell', () => {
  it('allows the upgraded directory and sales shell routes only', () => {
    expect(isReducedUiPathAllowed('/')).toBe(true);
    expect(isReducedUiPathAllowed('/dashboard')).toBe(true);
    expect(isReducedUiPathAllowed('/channels')).toBe(true);
    expect(isReducedUiPathAllowed('/channels/service-purchase')).toBe(true);
    expect(isReducedUiPathAllowed('/channels/authorize')).toBe(true);
    expect(isReducedUiPathAllowed('/couriers')).toBe(true);
    expect(isReducedUiPathAllowed('/tags')).toBe(true);
    expect(isReducedUiPathAllowed('/warehouses')).toBe(true);
    expect(isReducedUiPathAllowed('/logistics-fees')).toBe(true);
    expect(isReducedUiPathAllowed('/suppliers')).toBe(true);
    expect(isReducedUiPathAllowed('/customers/categories')).toBe(true);
    expect(isReducedUiPathAllowed('/customers/member-levels')).toBe(true);
    expect(isReducedUiPathAllowed('/customers/tags')).toBe(true);
    expect(isReducedUiPathAllowed('/wms/containers')).toBe(true);
    expect(isReducedUiPathAllowed('/wms/sender-addresses')).toBe(true);
    expect(isReducedUiPathAllowed('/wms/warehouse-areas')).toBe(true);
    expect(isReducedUiPathAllowed('/wms/warehouse-locations')).toBe(true);
    expect(isReducedUiPathAllowed('/system/permissions')).toBe(true);
    expect(isReducedUiPathAllowed('/system/roles')).toBe(true);
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
