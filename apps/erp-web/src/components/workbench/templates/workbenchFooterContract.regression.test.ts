import { readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

const repoRoot = path.resolve(__dirname, '../../../../../..');

const readRepoFile = (relativePath: string) =>
  readFileSync(path.resolve(repoRoot, relativePath), 'utf8');

describe('workbench footer contract regression', () => {
  it('wraps table content in a shared flex shell and keeps shared external-scrollbar rules in the template layer', () => {
    const templateSource = readRepoFile(
      'apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx',
    );
    const cssSource = readRepoFile(
      'apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.css',
    );
    const footerCssSource = readRepoFile(
      'apps/erp-web/src/components/ui-modules/UiModule03TableFooter.css',
    );

    expect(templateSource).toContain('order-workbench-table-content');
    expect(cssSource).toContain('.order-workbench-table-content');
    expect(cssSource).toContain('.order-workbench-table-external-scrollbar');
    expect(cssSource).toContain('.order-workbench-bottom-panel .ant-tabs-tabpane');
    expect(cssSource).toContain('overflow: hidden;');
    expect(cssSource).toContain('.order-review-main-grid-shell .ag-body-horizontal-scroll');
    expect(cssSource).toContain('min-height: 0;');
    expect(cssSource).toContain('height: 0 !important;');
    expect(cssSource).toContain('min-height: 0 !important;');
    expect(footerCssSource).toContain('flex: 0 0 auto;');
  });

  it('keeps the shared footer in normal document flow so the last row is not covered by the external scrollbar', () => {
    const footerCssSource = readRepoFile(
      'apps/erp-web/src/components/ui-modules/UiModule03TableFooter.css',
    );

    expect(footerCssSource).toContain('position: relative;');
    expect(footerCssSource).not.toContain('position: sticky;');
  });

  it('resets the current page when the shared page size changes', () => {
    const source = readRepoFile(
      'apps/erp-web/src/components/workbench/templates/useWorkbenchTablePagination.ts',
    );

    expect(source).toContain('const pageSizeChanged');
    expect(source).toContain('current: pageSizeChanged ? 1 :');
  });
});
