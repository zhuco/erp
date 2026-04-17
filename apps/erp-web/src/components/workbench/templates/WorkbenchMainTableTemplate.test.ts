import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createElement, createRef } from 'react';
import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';

import { WorkbenchMainTableTemplate } from './WorkbenchMainTableTemplate';

describe('WorkbenchMainTableTemplate', () => {
  it('renders the unified main table shell with auto footer summary and pagination area', () => {
    const tableRef = createRef<HTMLDivElement>();
    const markup = renderToStaticMarkup(
      createElement(WorkbenchMainTableTemplate, {
        className: 'order-review-page',
        toolbar: createElement('div', { 'data-testid': 'toolbar-slot' }, 'toolbar'),
        header: createElement('div', { 'data-testid': 'header-slot' }, 'header'),
        tableContainerRef: tableRef,
        tableClassName: 'order-review-table',
        tableContent: createElement('div', { 'data-testid': 'table-slot' }, 'table'),
        total: 123,
        pagination: {
          current: 2,
          pageSize: 100,
          pageSizeOptions: ['50', '100', '200'],
        },
        footerSummary: createElement(
          'span',
          { 'data-testid': 'summary-slot' },
          '总数 123 条，本页 100 条',
        ),
        onPaginationChange: () => undefined,
      }),
    );

    expect(markup).toContain('order-workbench-page order-review-page');
    expect(markup).toContain('data-testid="toolbar-slot"');
    expect(markup).toContain('data-testid="header-slot"');
    expect(markup).toContain('data-testid="table-slot"');
    expect(markup).toContain(
      'order-workbench-table order-review-table order-workbench-table-with-footer',
    );
    expect(markup).toContain('order-review-pagination');
    expect(markup).toContain('data-testid="summary-slot"');
  });

  it('does not manage shared bottom panels unless the template explicitly receives bottom panel props', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchMainTableTemplate.tsx'), 'utf8');

    expect(source).toContain("hasOwn.call(props, 'bottomPanel')");
    expect(source).toContain('if (!workbenchPanels || !shouldSyncBottomPanel)');
  });

  it('keeps horizontal scrolling available in footer mode table content', () => {
    const cssSource = readFileSync(resolve(__dirname, 'WorkbenchMainTableTemplate.css'), 'utf8');

    expect(cssSource).toContain('.order-workbench-table-with-footer .ant-table-content');
    expect(cssSource).toContain('overflow-x: auto');
    expect(cssSource).not.toContain('overflow-x: hidden !important');
  });
});
