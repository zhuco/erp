import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout font scale quick controls', () => {
  it('defines and applies small/medium/large workbench font scale controls', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('applyWorkbenchFontScale');
    expect(source).toContain('workbench-font-scale-small');
    expect(source).toContain('workbench-font-scale-medium');
    expect(source).toContain('workbench-font-scale-large');
    expect(source).toContain('data-workbench-font-scale={workbenchFontScale}');
    expect(source).not.toContain('workbench-layout-preset-compact');
    expect(source).not.toContain('workbench-layout-preset-default');
    expect(source).not.toContain('workbench-layout-preset-analysis');
  });

  it('supports Alt+Shift+1/2/3 keyboard shortcuts for workbench font scale', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('event.altKey');
    expect(source).toContain('event.shiftKey');
    expect(source).toContain("event.key === '1'");
    expect(source).toContain("event.key === '2'");
    expect(source).toContain("event.key === '3'");
    expect(source).toContain("applyWorkbenchFontScale('small')");
    expect(source).toContain("applyWorkbenchFontScale('medium')");
    expect(source).toContain("applyWorkbenchFontScale('large')");
  });
});
