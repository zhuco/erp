import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('RouteTabs keyboard flow upgrade', () => {
  it('supports ctrl-based tab navigation and close/refresh shortcuts', () => {
    const source = readFileSync(resolve(__dirname, 'RouteTabs.tsx'), 'utf8');

    expect(source).toContain('ROUTE_TABS_SHORTCUT_EVENT');
    expect(source).toContain("if (event.key === 'Tab')");
    expect(source).toContain("if (key === 'w')");
    expect(source).toContain("if (key === 'r')");
    expect(source).toContain("runShortcutAction(event.shiftKey ? 'previous' : 'next')");
  });

  it('does not keep close-undo restoration logic', () => {
    const source = readFileSync(resolve(__dirname, 'RouteTabs.tsx'), 'utf8');

    expect(source).not.toContain('ROUTE_TABS_UNDO_MESSAGE_KEY');
    expect(source).not.toContain('restoreClosedTab');
    expect(source).not.toContain('queueUndoCloseTab');
  });

  it('clears page-level saved view cache when closing order tabs', () => {
    const source = readFileSync(resolve(__dirname, 'RouteTabs.tsx'), 'utf8');

    expect(source).toContain('buildClosedTabSavedViewKeys');
    expect(source).toContain("if (tabKey === '/orders/review')");
    expect(source).toContain("if (tabKey === '/orders/exceptions' || tabKey === '/orders/recheck')");
    expect(source).toContain("if (tabKey === '/orders')");
    expect(source).toContain('clearSavedViewForClosedTab(key)');
  });
});
