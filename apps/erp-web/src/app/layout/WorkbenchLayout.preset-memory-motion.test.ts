import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout font scale memory and motion polish', () => {
  it('persists workbench font scale by local user key in localStorage', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('workbenchFontScaleStorageKey');
    expect(source).toContain('erp.ui.workbench.font-scale.');
    expect(source).toContain('localStorage.getItem(workbenchFontScaleStorageKey)');
    expect(source).toContain('localStorage.setItem(workbenchFontScaleStorageKey');
  });

  it('exposes font scale status and shell data attribute for UI/e2e observability', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('data-workbench-font-scale={workbenchFontScale}');
    expect(source).toContain('workbench-font-scale-status');
    expect(source).toContain('aria-live="polite"');
    expect(source).toContain('fontScaleStatusText');
  });

  it('adds transition polish with reduced-motion fallback', () => {
    const cssSource = readFileSync(resolve(__dirname, 'workbenchLayout.css'), 'utf8');

    expect(cssSource).toContain('.workbench-panel {');
    expect(cssSource).toContain('transition: width');
    expect(cssSource).toContain('.workbench-main-bottom {');
    expect(cssSource).toContain('transition: height');
    expect(cssSource).toContain('.workbench-font-scale-status {');
    expect(cssSource).toContain('@media (prefers-reduced-motion: reduce)');
  });
});
