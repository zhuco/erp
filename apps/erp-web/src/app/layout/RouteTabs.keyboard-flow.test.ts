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

  it('supports close undo restoration', () => {
    const source = readFileSync(resolve(__dirname, 'RouteTabs.tsx'), 'utf8');

    expect(source).toContain('ROUTE_TABS_UNDO_MESSAGE_KEY');
    expect(source).toContain('restoreClosedTab');
    expect(source).toContain('已关闭“{tab.title}”');
    expect(source).toContain('已撤销关闭页签');
  });
});
