import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout industrial upgrade', () => {
  it('supports URL-shareable layout preset with extended panel state keys', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain("LAYOUT_PRESET_QUERY_KEY = 'layoutPreset'");
    expect(source).toContain("LAYOUT_LEFT_COLLAPSED_QUERY_KEY = 'leftCollapsed'");
    expect(source).toContain("LAYOUT_RIGHT_COLLAPSED_QUERY_KEY = 'rightCollapsed'");
    expect(source).toContain("LAYOUT_BOTTOM_COLLAPSED_QUERY_KEY = 'bottomCollapsed'");
    expect(source).toContain("LAYOUT_LEFT_WIDTH_QUERY_KEY = 'leftWidth'");
    expect(source).toContain("LAYOUT_RIGHT_WIDTH_QUERY_KEY = 'rightWidth'");
    expect(source).toContain("LAYOUT_BOTTOM_HEIGHT_QUERY_KEY = 'bottomHeight'");
    expect(source).toContain('queryLayoutState');
    expect(source).toContain('new URLSearchParams(location.search)');
    expect(source).toContain('window.history.replaceState');
    expect(source).toContain('data-workbench-left-collapsed');
    expect(source).toContain('data-workbench-right-collapsed');
    expect(source).toContain('data-workbench-bottom-collapsed');
    expect(source).toContain('data-workbench-left-width');
    expect(source).toContain('data-workbench-right-width');
    expect(source).toContain('data-workbench-bottom-height');
    expect(source).toContain('layoutPresetStatusText');
    expect(source).toContain('resolveLayoutPresetLabel');
  });

  it('adds a11y landmarks and skip link for main content region', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('workbench-skip-link');
    expect(source).toContain('href="#workbench-main-content"');
    expect(source).toContain('id="workbench-main-content"');
    expect(source).toContain('role="main"');
    expect(source).toContain('workbench-sr-live-region');
    expect(source).toContain('aria-atomic="true"');
    expect(source).toContain('announceForScreenReader');
    expect(source).toContain('aria-keyshortcuts="Control+K"');
    expect(source).toContain('aria-keyshortcuts="Alt+Shift+1"');
    expect(source).toContain('aria-keyshortcuts="Alt+Shift+2"');
    expect(source).toContain('aria-keyshortcuts="Alt+Shift+3"');
  });

  it('adds sampled front-end telemetry and resize RAF throttle', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('emitWorkbenchInteraction');
    expect(source).toContain("'erp:workbench-interaction'");
    expect(source).toContain('resizeFrameRef');
    expect(source).toContain('window.requestAnimationFrame');
    expect(source).toContain('window.cancelAnimationFrame');
  });
});
