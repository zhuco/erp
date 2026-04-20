import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('workbench unified contracts', () => {
  it('defines shared panel, shortcut, context-menu, and selection contracts', () => {
    const source = readFileSync(resolve(__dirname, 'contracts.ts'), 'utf8');

    expect(source).toContain('export type WorkbenchPanelVisibilityConfig');
    expect(source).toContain('export type WorkbenchKeyboardShortcut');
    expect(source).toContain('export type WorkbenchContextMenuState');
    expect(source).toContain('export type WorkbenchSelectionState');
  });

  it('re-exports unified contracts through the page-template workbench index', () => {
    const source = readFileSync(resolve(__dirname, 'index.ts'), 'utf8');

    expect(source).toContain("from './contracts'");
    expect(source).toContain('WorkbenchKeyboardShortcut');
    expect(source).toContain('WorkbenchContextMenuState');
    expect(source).toContain('WorkbenchSelectionState');
  });

  it('extends the unified workbench template props with left-panel support', () => {
    const source = readFileSync(
      resolve(__dirname, '../../workbench/templates/OrderReviewWorkbenchTemplate.tsx'),
      'utf8',
    );

    expect(source).toContain('leftPanel?: ReactNode;');
    expect(source).toContain('leftPanelVisible?: boolean;');
    expect(source).toContain('leftPanelSyncKey?: string | number;');
  });
});
