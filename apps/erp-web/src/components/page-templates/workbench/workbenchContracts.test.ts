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

  it('keeps WorkbenchPageTemplate as the recommended page entry', () => {
    const wrapperSource = readFileSync(resolve(__dirname, 'WorkbenchPageTemplate.tsx'), 'utf8');
    const templateSource = readFileSync(
      resolve(__dirname, '../../workbench/templates/OrderReviewWorkbenchTemplate.tsx'),
      'utf8',
    );

    expect(wrapperSource).toContain('export const WorkbenchPageTemplate');
    expect(wrapperSource).toContain("Omit<OrderReviewWorkbenchTemplateProps, 'toolbar'>");
    expect(wrapperSource).toContain('type WorkbenchToolbarElement = ReactElement<WorkbenchToolbarProps, typeof WorkbenchToolbar>;');
    expect(wrapperSource).toContain('toolbar?: WorkbenchToolbarElement | null;');
    expect(wrapperSource).toContain('isWorkbenchToolbarElement');
    expect(wrapperSource).toContain('toolbar.type === WorkbenchToolbar');
    expect(wrapperSource).toContain('toolbar ???? WorkbenchToolbar');
    expect(templateSource).not.toContain('toolbarLeft?: ReactNode;');
    expect(templateSource).not.toContain('toolbarCenter?: ReactNode;');
    expect(templateSource).not.toContain('toolbarRight?: ReactNode;');
  });


  it('keeps WorkbenchToolbar as the recommended toolbar entry', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchToolbar.tsx'), 'utf8');

    expect(source).toContain('export const WorkbenchToolbar');
    expect(source).toContain('<OrderReviewWorkbenchToolbarTemplate');
    expect(source).toContain("WorkbenchToolbar.displayName = 'WorkbenchToolbar';");
  });

  it('keeps main-table section toolbar aligned to WorkbenchToolbar', () => {
    const source = readFileSync(resolve(__dirname, '../../ui-modules/UiModule03MainTableSection.tsx'), 'utf8');

    expect(source).toContain('type UiModule03ToolbarElement = ReactElement<WorkbenchToolbarProps, typeof WorkbenchToolbar>;');
    expect(source).toContain('toolbar?: UiModule03ToolbarElement | null;');
    expect(source).toContain('<WorkbenchToolbar');
  });

});
