import { createElement } from 'react';
import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';

import { UiModule02WorkbenchTemplate } from './UiModule02WorkbenchTemplate';

const assertTestIdRenderedInsideContainerClass = (
  markup: string,
  testId: string,
  containerClass: string,
) => {
  const voidTags = new Set([
    'area',
    'base',
    'br',
    'col',
    'embed',
    'hr',
    'img',
    'input',
    'link',
    'meta',
    'param',
    'source',
    'track',
    'wbr',
  ]);

  const stack: Array<{ tag: string; classes: string[] }> = [];
  const tokenRe = /<\/?[a-zA-Z][^>]*>/g;
  const classRe = /\bclass="([^"]*)"/;
  const testIdRe = /\bdata-testid="([^"]+)"/;
  const tagNameRe = /^<\/?\s*([a-zA-Z][a-zA-Z0-9:-]*)/;
  const isSelfClosing = (token: string, tag: string) =>
    token.endsWith('/>') || voidTags.has(tag.toLowerCase());

  let match: RegExpExecArray | null;
  while ((match = tokenRe.exec(markup))) {
    const token = match[0];

    const nameMatch = tagNameRe.exec(token);
    const tag = nameMatch?.[1]?.toLowerCase();
    if (!tag) {
      continue;
    }

    if (token.startsWith('</')) {
      for (let idx = stack.length - 1; idx >= 0; idx -= 1) {
        const entry = stack.pop();
        if (entry?.tag === tag) {
          break;
        }
      }
      continue;
    }

    const classMatch = classRe.exec(token);
    const className = classMatch ? classMatch[1] : '';
    const classes = className.split(' ').filter(Boolean);

    const tidMatch = testIdRe.exec(token);
    if (!isSelfClosing(token, tag)) {
      stack.push({ tag, classes });
    }
    if (tidMatch?.[1] === testId) {
      const insideContainer =
        classes.includes(containerClass) || stack.some((entry) => entry.classes.includes(containerClass));
      expect(
        insideContainer,
        `data-testid="${testId}" should be inside .${containerClass}`,
      ).toBe(true);
      return;
    }
  }

  throw new Error(`data-testid="${testId}" not found in markup`);
};

const assertTestIdRenderedInsideMain = (markup: string, testId: string) =>
  assertTestIdRenderedInsideContainerClass(markup, testId, 'order-workbench-main');

describe('UiModule02WorkbenchTemplate', () => {
  it('keeps root class and renders WorkbenchPageTemplate skeleton wrappers', () => {
    const markup = renderToStaticMarkup(
      createElement(UiModule02WorkbenchTemplate, {
        className: 'custom-root',
        header: createElement('div', { 'data-testid': 'header-slot' }, 'header'),
        tableContent: createElement('div', { 'data-testid': 'table-slot' }, 'table'),
        tableFooter: createElement('div', { 'data-testid': 'table-footer-slot' }, 'footer'),
        actionBar: createElement('div', { 'data-testid': 'action-slot' }, 'action'),
      }),
    );

    expect(markup).toContain('order-workbench-page');
    expect(markup).toContain('custom-root');
    expect(markup).toContain('order-workbench-body');
    expect(markup).toContain('order-workbench-main');

    assertTestIdRenderedInsideMain(markup, 'header-slot');
    assertTestIdRenderedInsideMain(markup, 'table-slot');
    assertTestIdRenderedInsideMain(markup, 'table-footer-slot');
    assertTestIdRenderedInsideMain(markup, 'action-slot');
  });

  it('renders toolbar left/center/right when toolbar prop is omitted', () => {
    const markup = renderToStaticMarkup(
      createElement(UiModule02WorkbenchTemplate, {
        toolbarLeft: createElement('span', { 'data-testid': 'toolbar-left' }, 'left'),
        toolbarCenter: createElement('span', { 'data-testid': 'toolbar-center' }, 'center'),
        toolbarRight: createElement('span', { 'data-testid': 'toolbar-right' }, 'right'),
        tableContent: createElement('div', { 'data-testid': 'table-slot' }, 'table'),
      }),
    );

    expect(markup).toContain('order-workbench-toolbar');
    expect(markup).toContain('order-workbench-toolbar-left');
    expect(markup).toContain('data-testid="toolbar-left"');
    expect(markup).toContain('order-workbench-toolbar-center');
    expect(markup).toContain('data-testid="toolbar-center"');
    expect(markup).toContain('order-workbench-toolbar-right');
    expect(markup).toContain('data-testid="toolbar-right"');
  });

  it('forwards custom toolbar when toolbar prop is provided', () => {
    const markup = renderToStaticMarkup(
      createElement(UiModule02WorkbenchTemplate, {
        toolbar: createElement('section', { 'data-testid': 'custom-toolbar' }, 'toolbar'),
        toolbarLeft: createElement('span', { 'data-testid': 'toolbar-left' }, 'left'),
        toolbarCenter: createElement('span', { 'data-testid': 'toolbar-center' }, 'center'),
        toolbarRight: createElement('span', { 'data-testid': 'toolbar-right' }, 'right'),
        tableContent: createElement('div', { 'data-testid': 'table-slot' }, 'table'),
      }),
    );

    expect(markup).toContain('data-testid="custom-toolbar"');
    expect(markup).not.toContain('order-workbench-toolbar-left');
    expect(markup).not.toContain('order-workbench-toolbar-center');
    expect(markup).not.toContain('order-workbench-toolbar-right');
  });

  it('keeps tableFooter inside the .order-workbench-table container', () => {
    const markup = renderToStaticMarkup(
      createElement(UiModule02WorkbenchTemplate, {
        tableContent: createElement('div', { 'data-testid': 'table-slot' }, 'table'),
        tableFooter: createElement('footer', { 'data-testid': 'table-footer-slot' }, 'footer'),
      }),
    );

    assertTestIdRenderedInsideContainerClass(markup, 'table-footer-slot', 'order-workbench-table');

    const contentIndex = markup.indexOf('data-testid="table-slot"');
    const footerIndex = markup.indexOf('data-testid="table-footer-slot"');
    expect(contentIndex).toBeGreaterThanOrEqual(0);
    expect(footerIndex).toBeGreaterThan(contentIndex);
  });
});
