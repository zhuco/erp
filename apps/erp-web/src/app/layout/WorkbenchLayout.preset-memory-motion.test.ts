import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout preset memory and motion polish', () => {
  it('persists layout presets by route key in localStorage', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('layoutPresetStorageKey');
    expect(source).toContain('erp.ui.workbench.layout.preset.');
    expect(source).toContain('layoutPresetHydratedRef');
    expect(source).toContain('parsed[currentRouteKey]');
    expect(source).toContain('localStorage.setItem(layoutPresetStorageKey');
  });

  it('exposes preset status and shell data attribute for UI/e2e observability', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('data-workbench-layout-preset={layoutPreset}');
    expect(source).toContain('workbench-layout-preset-status');
    expect(source).toContain('aria-live="polite"');
    expect(source).toContain('layoutPresetStatusText');
  });

  it('adds transition polish with reduced-motion fallback', () => {
    const cssSource = readFileSync(resolve(__dirname, 'workbenchLayout.css'), 'utf8');

    expect(cssSource).toContain('.workbench-panel {');
    expect(cssSource).toContain('transition: width');
    expect(cssSource).toContain('.workbench-main-bottom {');
    expect(cssSource).toContain('transition: height');
    expect(cssSource).toContain('.workbench-layout-preset-status {');
    expect(cssSource).toContain('@media (prefers-reduced-motion: reduce)');
  });
});
