import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout quick presets', () => {
  it('defines and applies compact/default/analysis layout presets', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('type WorkbenchLayoutPreset =');
    expect(source).toContain("'compact'");
    expect(source).toContain("'default'");
    expect(source).toContain("'analysis'");
    expect(source).toContain('applyLayoutPreset');
    expect(source).toContain('workbench-layout-preset-compact');
    expect(source).toContain('workbench-layout-preset-default');
    expect(source).toContain('workbench-layout-preset-analysis');
  });

  it('supports Alt+Shift+1/2/3 keyboard shortcuts for layout presets', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('event.altKey');
    expect(source).toContain('event.shiftKey');
    expect(source).toContain("event.key === '1'");
    expect(source).toContain("event.key === '2'");
    expect(source).toContain("event.key === '3'");
  });
});
