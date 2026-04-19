import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout splitter accessibility', () => {
  it('exposes keyboard-accessible separators with value semantics', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('data-testid="workbench-left-resizer"');
    expect(source).toContain('data-testid="workbench-right-resizer"');
    expect(source).toContain('data-testid="workbench-bottom-resizer"');

    expect(source).toContain('role="separator"');
    expect(source).toContain('tabIndex={0}');
    expect(source).toContain('aria-orientation="vertical"');
    expect(source).toContain('aria-orientation="horizontal"');
    expect(source).toContain('aria-valuemin=');
    expect(source).toContain('aria-valuemax=');
    expect(source).toContain('aria-valuenow=');
    expect(source).toContain("onKeyDown={handleResizeKeyDown('left')}");
    expect(source).toContain("onKeyDown={handleResizeKeyDown('right')}");
    expect(source).toContain("onKeyDown={handleResizeKeyDown('bottom')}");
  });
});
