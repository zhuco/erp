import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

import { describe, expect, it } from 'vitest';

import {
  DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME,
  INDUSTRIAL_WORKBENCH_REGIONS,
  INDUSTRIAL_WORKBENCH_TEST_IDS,
  IndustrialWorkbenchTemplate,
} from './IndustrialWorkbenchTemplate';

describe('IndustrialWorkbenchTemplate', () => {
  it('exports stable selectors for shared E2E and page contracts', () => {
    expect(INDUSTRIAL_WORKBENCH_TEST_IDS).toMatchObject({
      shell: 'industrial-workbench-shell',
      body: 'industrial-workbench-body',
      main: 'industrial-workbench-main',
      header: 'industrial-workbench-header',
      table: 'industrial-workbench-table',
      tableContent: 'industrial-workbench-table-content',
      action: 'industrial-workbench-action',
      overlays: 'industrial-workbench-overlays',
      bottomPanel: 'industrial-workbench-bottom-panel',
    });
    expect(INDUSTRIAL_WORKBENCH_REGIONS).toMatchObject({
      shell: 'shell',
      body: 'body',
      main: 'main',
      header: 'header',
      table: 'table',
      tableContent: 'table-content',
      action: 'action',
      overlays: 'overlays',
      bottomPanel: 'bottom-panel',
    });
    expect(DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME).toContain(
      'industrial-workbench-table-module-03',
    );
    expect(DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME).toContain(
      'industrial-workbench-table-external-scrollbar',
    );
    expect(DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME).toContain('order-review-table');
    expect(DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME).toContain(
      'order-workbench-table-module-03',
    );
    expect(DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME).toContain(
      'order-review-table-external-scrollbar',
    );
  });

  it('does not expose order-review defaults in the generic template', () => {
    const source = readFileSync(resolve(__dirname, 'IndustrialWorkbenchTemplate.tsx'), 'utf8');

    expect(source).toContain('industrial-workbench-page');
    expect(source).not.toContain('order-review-page');
    expect(source).not.toContain('OrderReviewWorkbenchTemplate');
  });

  it('preserves generic order-workbench structure classes for legacy layout compatibility', () => {
    const source = readFileSync(resolve(__dirname, 'IndustrialWorkbenchTemplate.tsx'), 'utf8');

    expect(source).toContain('order-workbench-page');
    expect(source).toContain('order-workbench-body');
    expect(source).toContain('order-workbench-main');
    expect(source).toContain('order-workbench-table');
    expect(source).toContain('order-workbench-table-content');
    expect(source).toContain('order-workbench-bottom-panel');
    expect(source).toContain('INDUSTRIAL_WORKBENCH_TEST_IDS.bottomPanel');
    expect(source).toContain('INDUSTRIAL_WORKBENCH_REGIONS.bottomPanel');
    expect(source).toContain('order-workbench-table-with-footer');
  });


  it('keeps the overlay region from adding layout boxes', () => {
    const css = readFileSync(resolve(__dirname, 'industrialWorkbenchTemplate.css'), 'utf8');

    expect(css).toContain('.industrial-workbench-overlays');
    expect(css).toContain('display: contents;');
  });

  it('renders stable structure test ids, regions, and panel visibility state', () => {
    const html = renderToStaticMarkup(
      createElement(IndustrialWorkbenchTemplate, {
        className: 'custom-workbench-page',
        header: createElement('span', null, '页头'),
        actionBar: createElement('button', { type: 'button' }, '批量操作'),
        leftPanel: createElement('aside', null, '筛选'),
        rightPanel: createElement('aside', null, '详情'),
        rightPanelVisible: false,
        bottomPanel: createElement('section', null, '明细'),
        tableClassName: 'custom-table-class',
        tableContent: createElement('div', null, '主表'),
        tableFooter: createElement('footer', null, '分页'),
        overlays: createElement('div', { 'data-testid': 'custom-overlay' }, '??'),
      }),
    );

    expect(html).toContain('data-workbench-layout="industrial"');
    expect(html).toContain('data-left-panel-visible="true"');
    expect(html).toContain('data-right-panel-visible="false"');
    expect(html).toContain('data-bottom-panel-visible="true"');
    expect(html).toContain('data-testid="industrial-workbench-shell"');
    expect(html).toContain('data-testid="industrial-workbench-body"');
    expect(html).toContain('data-testid="industrial-workbench-main"');
    expect(html).toContain('data-testid="industrial-workbench-header"');
    expect(html).toContain('data-testid="industrial-workbench-table"');
    expect(html).toContain('data-testid="industrial-workbench-table-content"');
    expect(html).toContain('data-testid="industrial-workbench-action"');
    expect(html).toContain('data-testid="industrial-workbench-overlays"');
    expect(html).toContain('data-workbench-region="table"');
    expect(html).toContain('data-workbench-region="overlays"');
    expect(html).toContain('data-testid="custom-overlay"');
    expect(html).toContain('custom-workbench-page');
    expect(html).toContain('custom-table-class');
    expect(html).toContain('order-workbench-table-with-footer');
  });

  it('renders legacy-compatible table classes from the default template contract', () => {
    const html = renderToStaticMarkup(
      createElement(IndustrialWorkbenchTemplate, {
        tableContent: createElement('div', null, '??'),
      }),
    );

    expect(html).toContain('industrial-workbench-table-module-03');
    expect(html).toContain('industrial-workbench-table-external-scrollbar');
    expect(html).toContain('order-review-table');
    expect(html).toContain('order-workbench-table-module-03');
    expect(html).toContain('order-review-table-external-scrollbar');
  });


  it('renders the root, toolbar, main grid, action bar, and footer slots in stable order', () => {
    const html = renderToStaticMarkup(
      createElement(IndustrialWorkbenchTemplate, {
        pageTestId: 'erp-workbench-root',
        toolbar: createElement('nav', { 'data-testid': 'standard-toolbar' }, '?????'),
        header: createElement('header', { 'data-testid': 'standard-header' }, '????'),
        tableContent: createElement('main', { 'data-testid': 'standard-main-grid' }, '????'),
        tableFooter: createElement('footer', { 'data-testid': 'standard-footer' }, '???'),
        actionBar: createElement('section', { 'data-testid': 'standard-action-bar' }, '????'),
      }),
    );

    expect(html).toContain('data-testid="erp-workbench-root"');
    expect(html).toContain('data-workbench-layout="industrial"');
    expect(html).toContain('data-testid="standard-toolbar"');
    expect(html).toContain('data-testid="industrial-workbench-body"');
    expect(html).toContain('data-testid="industrial-workbench-main"');
    expect(html).toContain('data-testid="industrial-workbench-header"');
    expect(html).toContain('data-testid="standard-header"');
    expect(html).toContain('data-testid="industrial-workbench-table"');
    expect(html).toContain('data-testid="industrial-workbench-table-content"');
    expect(html).toContain('data-testid="standard-main-grid"');
    expect(html).toContain('data-testid="standard-footer"');
    expect(html).toContain('data-testid="industrial-workbench-action"');
    expect(html).toContain('data-testid="standard-action-bar"');

    const toolbarIndex = html.indexOf('data-testid="standard-toolbar"');
    const bodyIndex = html.indexOf('data-testid="industrial-workbench-body"');
    const tableIndex = html.indexOf('data-testid="industrial-workbench-table"');
    const contentIndex = html.indexOf('data-testid="industrial-workbench-table-content"');
    const footerIndex = html.indexOf('data-testid="standard-footer"');
    const actionIndex = html.indexOf('data-testid="industrial-workbench-action"');

    expect(toolbarIndex).toBeGreaterThan(-1);
    expect(bodyIndex).toBeGreaterThan(toolbarIndex);
    expect(tableIndex).toBeGreaterThan(bodyIndex);
    expect(contentIndex).toBeGreaterThan(tableIndex);
    expect(footerIndex).toBeGreaterThan(contentIndex);
    expect(actionIndex).toBeGreaterThan(footerIndex);
  });

  it('keeps WorkbenchPageTemplate backed by the generic industrial template', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchPageTemplate.tsx'), 'utf8');

    expect(source).toContain('IndustrialWorkbenchTemplate');
    expect(source).not.toContain('OrderReviewWorkbenchTemplate');
    expect(source).toContain('toolbar 必须使用 WorkbenchToolbar');
  });
});
