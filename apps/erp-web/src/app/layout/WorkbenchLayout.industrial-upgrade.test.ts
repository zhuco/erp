import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout industrial upgrade', () => {
  it('supports URL-shareable panel state with workbench font scale observability', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

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
    expect(source).toContain('data-workbench-font-scale={workbenchFontScale}');
    expect(source).toContain('fontScaleStatusText');
    expect(source).toContain('resolveWorkbenchFontScaleLabel');
    expect(source).not.toContain("LAYOUT_PRESET_QUERY_KEY = 'layoutPreset'");
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

  it('uses the 衡策云 brand in the workbench header', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('logo="衡策云"');
    expect(source).not.toContain('logo="ERP"');
  });

  it('keeps the shared panel collapse toggles visually compact', () => {
    const css = readFileSync(resolve(__dirname, 'workbenchLayout.css'), 'utf8');

    expect(css).toContain('.workbench-resizer-toggle {');
    expect(css).toContain('.workbench-resizer-horizontal-toggle {');
    expect(css).toContain('.workbench-resizer::before');
    expect(css).toContain('.workbench-resizer-horizontal::before');
    expect(css).toContain('width: 6px;');
    expect(css).toContain('height: 6px;');
    expect(css).toContain('width: 1px;');
    expect(css).toContain('height: 1px;');
    expect(css).toContain('width: 7px;');
    expect(css).toContain('height: 13px;');
    expect(css).toContain('width: 13px;');
    expect(css).toContain('height: 7px;');
    expect(css).toContain('font-size: 9px;');
    expect(css).toContain('line-height: 1;');
    expect(css).toContain('.workbench-panel-left {');
    expect(css).toContain('border-right: none;');
    expect(css).toContain('.workbench-panel-right {');
    expect(css).toContain('border-left: none;');
    expect(css).toContain('.workbench-main-bottom {');
    expect(css).toContain('border-top: none;');
  });

  it('uses Chinese tooltip and aria labels for shared panel collapse toggles', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('workbenchI18n.resizer.expandLeft');
    expect(source).toContain('workbenchI18n.resizer.collapseLeft');
    expect(source).toContain('workbenchI18n.resizer.expandRight');
    expect(source).toContain('workbenchI18n.resizer.collapseRight');
    expect(source).toContain('workbenchI18n.resizer.expandBottom');
    expect(source).toContain('workbenchI18n.resizer.collapseBottom');
    expect(source).not.toContain("'Expand left panel'");
    expect(source).not.toContain("'Collapse left panel'");
    expect(source).not.toContain("'Expand right panel'");
    expect(source).not.toContain("'Collapse right panel'");
    expect(source).not.toContain("'Expand bottom panel'");
    expect(source).not.toContain("'Collapse bottom panel'");
  });

  it('uses tighter industrial panel spacing between filter, main grid, detail, and bottom panes', () => {
    const css = readFileSync(resolve(__dirname, 'workbenchLayout.css'), 'utf8');

    expect(css).toContain('.workbench-panel-body {');
    expect(css).toContain('.workbench-main-content {');
    expect(css).toContain('.workbench-main-top {');
    expect(css).toContain('.workbench-main-bottom {');
    expect(css).toContain('padding: 2px;');
  });
});
