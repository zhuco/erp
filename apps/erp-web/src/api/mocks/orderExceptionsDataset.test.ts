import { describe, expect, it } from 'vitest';

import { listMockOrderExceptions } from './orders';

describe('mock order exceptions dataset', () => {
  it('does not expose test-only order ids in exception pool', () => {
    const { items } = listMockOrderExceptions();
    expect(items.length).toBeGreaterThan(0);

    items.forEach((item) => {
      expect(item.order_id.startsWith('TEST-ORDER-')).toBe(false);
      expect(item.order_no.includes('PLAT-TEST-')).toBe(false);
    });
  });

  it('keeps manually handled exception records with assignee', () => {
    const { items } = listMockOrderExceptions();
    const manualItems = items.filter((item) => ['ASSIGNED', 'PROCESSING'].includes(String(item.exception_status)));

    expect(manualItems.length).toBeGreaterThan(0);
    manualItems.forEach((item) => {
      expect(String(item.assignee_name ?? '').trim()).not.toBe('');
    });
  });

  it('includes summary-level tags so the recheck main grid marks do not depend on detail hydration', () => {
    const { items } = listMockOrderExceptions();
    const taggedItems = items.filter((item) => Array.isArray((item as { tags?: unknown }).tags));

    expect(taggedItems.length).toBeGreaterThan(0);
    expect(
      taggedItems.some((item) => ((item as { tags?: string[] }).tags ?? []).some((tag) => String(tag).trim().length > 0)),
    ).toBe(true);
  });
});
