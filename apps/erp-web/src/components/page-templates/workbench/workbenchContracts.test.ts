import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('workbench unified contracts', () => {
  const readPageFiles = (directory: string): Array<{ file: string; source: string }> =>
    readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
      const file = resolve(directory, entry.name);
      if (entry.isDirectory()) {
        return readPageFiles(file);
      }
      if (!entry.isFile() || !file.endsWith('.tsx')) {
        return [];
      }
      return [{ file, source: readFileSync(file, 'utf8') }];
    });

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

  it('re-exports industrial workbench selectors for page and E2E consistency', () => {
    const source = readFileSync(resolve(__dirname, 'index.ts'), 'utf8');

    expect(source).toContain('INDUSTRIAL_WORKBENCH_TEST_IDS');
    expect(source).toContain('INDUSTRIAL_WORKBENCH_REGIONS');
    expect(source).toContain('DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME');
  });

  it('keeps default table module classes in the template instead of repeated pages', () => {
    const repeatedDefaultTableClass =
      'tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"';
    const repeatedPages = readPageFiles(resolve(__dirname, '../../../pages'))
      .filter(({ source }) => source.includes(repeatedDefaultTableClass))
      .map(({ file }) => file);

    expect(repeatedPages).toEqual([]);
  });

  it('extends the unified workbench template props with left-panel support', () => {
    const source = readFileSync(
      resolve(__dirname, 'IndustrialWorkbenchTemplate.tsx'),
      'utf8',
    );

    expect(source).toContain('leftPanel?: ReactNode;');
    expect(source).toContain('leftPanelVisible?: boolean;');
    expect(source).toContain('leftPanelSyncKey?: string | number;');
  });


  it('keeps industrial template slots wired to the unified workbench contract', () => {
    const source = readFileSync(
      resolve(__dirname, 'IndustrialWorkbenchTemplate.tsx'),
      'utf8',
    );

    [
      'pageTestId?: string;',
      'toolbar?: ReactNode;',
      'header?: ReactNode;',
      'actionBar?: ReactNode;',
      'tableContent: ReactNode;',
      'tableFooter?: ReactNode;',
      'footerSummary?: ReactNode;',
      'leftPanel?: ReactNode;',
      'leftPanelVisible?: boolean;',
      'rightPanel?: ReactNode;',
      'rightPanelVisible?: boolean;',
      'bottomPanel?: ReactNode;',
      'bottomPanelVisible?: boolean;',
    ].forEach((contract) => {
      expect(source).toContain(contract);
    });

    expect(source).toContain('data-testid={pageTestId}');
    expect(source).toContain('data-workbench-layout="industrial"');
    expect(source).toContain('data-left-panel-visible={layoutState.left.visible ? \'true\' : \'false\'}');
    expect(source).toContain('data-right-panel-visible={layoutState.right.visible ? \'true\' : \'false\'}');
    expect(source).toContain('data-bottom-panel-visible={layoutState.bottom.visible ? \'true\' : \'false\'}');
    expect(source).toContain('{toolbar ?? null}');
    expect(source).toContain('data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.body}');
    expect(source).toContain('data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.main}');
    expect(source).toContain('data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.table}');
    expect(source).toContain('data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.tableContent}');
    expect(source).toContain('data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.action}');
    expect(source).toContain('data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.bottomPanel}');
    expect(source).toContain('data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.tableContent}');
    expect(source).toContain('summary={footerSummary}');
  });

  it('keeps left, right, and bottom panels routed through the layout controller', () => {
    const source = readFileSync(
      resolve(__dirname, 'IndustrialWorkbenchTemplate.tsx'),
      'utf8',
    );

    expect(source).toContain('const layoutState = useWorkbenchLayoutController({');
    expect(source).toContain('leftPanel,');
    expect(source).toContain('leftPanelVisible,');
    expect(source).toContain('leftPanelSyncKey,');
    expect(source).toContain('leftPanelWidth,');
    expect(source).toContain('rightPanel,');
    expect(source).toContain('rightPanelVisible,');
    expect(source).toContain('rightPanelSyncKey,');
    expect(source).toContain('rightPanelWidth,');
    expect(source).toContain('bottomPanel: wrappedBottomPanel,');
    expect(source).toContain('bottomPanelVisible: shouldShowBottomPanel,');
    expect(source).toContain('bottomPanelSyncKey,');
    expect(source).toContain('bottomPanelHeight,');
  });

  it('keeps WorkbenchPageTemplate as the recommended page entry', () => {
    const wrapperSource = readFileSync(resolve(__dirname, 'WorkbenchPageTemplate.tsx'), 'utf8');
    const templateSource = readFileSync(
      resolve(__dirname, 'IndustrialWorkbenchTemplate.tsx'),
      'utf8',
    );

    expect(wrapperSource).toContain('export const WorkbenchPageTemplate');
    expect(wrapperSource).toContain("Omit<IndustrialWorkbenchTemplateProps, 'toolbar'>");
    expect(wrapperSource).toContain('type WorkbenchToolbarElement = ReactElement<WorkbenchToolbarProps, typeof WorkbenchToolbar>;');
    expect(wrapperSource).toContain('toolbar?: WorkbenchToolbarElement | null;');
    expect(wrapperSource).toContain('isWorkbenchToolbarElement');
    expect(wrapperSource).toContain('toolbar.type === WorkbenchToolbar');
    expect(wrapperSource).toContain('toolbar 必须使用 WorkbenchToolbar');
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
