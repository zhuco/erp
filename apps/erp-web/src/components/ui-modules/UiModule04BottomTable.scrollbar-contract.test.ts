import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('UiModule04BottomTable scrollbar contract', () => {
  it('only exposes the child-table scrollbar shell when horizontal overflow exists', () => {
    const source = readFileSync(resolve(__dirname, 'UiModule04BottomTable.tsx'), 'utf8');
    const css = readFileSync(resolve(__dirname, 'UiModule04BottomTable.css'), 'utf8');
    const hook = readFileSync(resolve(__dirname, 'hooks/useHorizontalScrollSync.ts'), 'utf8');

    expect(hook).toContain('hasHorizontalOverflow');
    expect(hook).toContain('setHasHorizontalOverflow');
    expect(hook).toContain('attributes: true');
    expect(hook).toContain('subtree: true');
    expect(source).toContain("data-horizontal-overflow={hasHorizontalOverflow ? 'true' : 'false'}");
    expect(source).toContain("!hasHorizontalOverflow ? 'order-workbench-bottom-scrollbar-row-hidden' : null");
    expect(css).toContain('.order-workbench-bottom-scrollbar-row-hidden');
    expect(hook).toContain('HORIZONTAL_OVERFLOW_TOLERANCE_PX');
    expect(hook).toContain('scrollWidth - clientWidth > HORIZONTAL_OVERFLOW_TOLERANCE_PX');
    expect(hook).toContain("scrollInner.style.width = '0px'");
    expect(hook).toContain('setHasHorizontalOverflow(false);');
    expect(hook).toContain('element.getClientRects().length > 0');
  });

  it('prefers the live AG Grid content viewport over the hidden native horizontal scrollbar viewport', () => {
    const hook = readFileSync(resolve(__dirname, 'hooks/useHorizontalScrollSync.ts'), 'utf8');
    const centerViewportIndex = hook.indexOf("'.ag-center-cols-viewport'");
    const hiddenScrollbarViewportIndex = hook.indexOf("'.ag-body-horizontal-scroll-viewport'");

    expect(centerViewportIndex).toBeGreaterThan(-1);
    expect(hiddenScrollbarViewportIndex).toBeGreaterThan(-1);
    expect(centerViewportIndex).toBeLessThan(hiddenScrollbarViewportIndex);
  });
});
