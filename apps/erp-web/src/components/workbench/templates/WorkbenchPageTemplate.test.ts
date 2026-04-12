import { createElement } from 'react';
import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';

import { WorkbenchPageTemplate } from './WorkbenchPageTemplate';

describe('WorkbenchPageTemplate', () => {
  it('renders the root classes and optional wrappers when slot props are provided', () => {
    const markup = renderToStaticMarkup(
      createElement(WorkbenchPageTemplate, {
        className: 'order-review-page custom-root',
        toolbar: createElement('div', { 'data-testid': 'toolbar-slot' }, 'toolbar'),
        leftPanel: createElement('span', { 'data-testid': 'left-slot' }, 0),
        mainContent: createElement('div', { 'data-testid': 'main-slot' }, 'main'),
        rightPanel: createElement('span', { 'data-testid': 'right-slot' }, ''),
        bottomPanel: createElement('div', { 'data-testid': 'bottom-slot' }, 'bottom'),
        footer: createElement('div', { 'data-testid': 'footer-slot' }, 'footer'),
      }),
    );

    expect(markup).toContain('order-workbench-page order-review-page custom-root');
    expect(markup).toContain('data-testid="toolbar-slot"');
    expect(markup).toContain('order-workbench-body');
    expect(markup).toContain('order-workbench-left');
    expect(markup).toContain('data-testid="left-slot"');
    expect(markup).toContain('order-workbench-main');
    expect(markup).toContain('data-testid="main-slot"');
    expect(markup).toContain('order-workbench-right');
    expect(markup).toContain('data-testid="right-slot"');
    expect(markup).toContain('order-workbench-bottom');
    expect(markup).toContain('data-testid="bottom-slot"');
    expect(markup).toContain('order-workbench-footer');
    expect(markup).toContain('data-testid="footer-slot"');
  });

  it('renders only the main area when optional slots are omitted', () => {
    const markup = renderToStaticMarkup(
      createElement(WorkbenchPageTemplate, {
        mainContent: createElement('div', { 'data-testid': 'main-slot' }, 'main'),
      }),
    );

    expect(markup).toContain('order-workbench-page');
    expect(markup).toContain('order-workbench-body');
    expect(markup).toContain('order-workbench-main');
    expect(markup).not.toContain('order-workbench-left');
    expect(markup).not.toContain('order-workbench-right');
    expect(markup).not.toContain('order-workbench-bottom');
    expect(markup).not.toContain('order-workbench-footer');
  });
});
