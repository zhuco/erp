import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout interaction polish', () => {
  it('supports splitter reset, snap points and resize indicator feedback', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('WORKBENCH_RESIZE_SNAP_THRESHOLD');
    expect(source).toContain('resolveSnappedPanelSize');
    expect(source).toContain("onDoubleClick={handleResizeReset('left')}");
    expect(source).toContain("onDoubleClick={handleResizeReset('right')}");
    expect(source).toContain("onDoubleClick={handleResizeReset('bottom')}");
    expect(source).toContain("if (event.key === 'Enter')");
    expect(source).toContain('workbench-resize-indicator');
    expect(source).toContain('resizeIndicatorLabel');
  });

  it('returns focus to main work area after function-search navigation', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('pendingFunctionSearchFocusRef');
    expect(source).toContain('mainRouteRegionRef');
    expect(source).toContain('mainRouteRegionRef.current?.focus');
    expect(source).toContain('tabIndex={-1}');
  });
});
