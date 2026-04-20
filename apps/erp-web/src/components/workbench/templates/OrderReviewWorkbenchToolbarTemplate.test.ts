import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createElement } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { UiModule02ActionSearchToolbar } from '@/components/ui-modules';
import { OrderReviewWorkbenchToolbarTemplate } from './OrderReviewWorkbenchToolbarTemplate';

const buildProps = () => {
  const onSearch = vi.fn();
  const onSearchChange = vi.fn();
  const onExport = vi.fn();
  const onPrint = vi.fn();
  const onHelp = vi.fn();

  return {
    className: 'custom-toolbar',
    leftContent: createElement('span', { 'data-testid': 'left-action' }, 'left'),
    searchValue: 'SO-001',
    searchPlaceholder: 'SO No.',
    searchInputClassName: 'input-class',
    searchButtonClassName: 'button-class',
    searchInputTestId: 'search-input',
    searchButtonTestId: 'search-button',
    onSearch,
    onSearchChange,
    onExport,
    onPrint,
    onHelp,
    exportButtonTestId: 'export-id',
    printButtonTestId: 'print-id',
    helpButtonTestId: 'help-id',
  };
};

describe('OrderReviewWorkbenchToolbarTemplate', () => {
  it('wraps UiModule02ActionSearchToolbar and forwards key props with defaults', () => {
    const props = buildProps();

    const element = OrderReviewWorkbenchToolbarTemplate(props);

    expect(element.type).toBe(UiModule02ActionSearchToolbar);
    expect(element.props.leftContent).toBe(props.leftContent);
    expect(element.props.searchValue).toBe('SO-001');
    expect(element.props.searchPlaceholder).toBe('SO No.');
    expect(element.props.searchInputClassName).toBe('order-workbench-toolbar-search-input input-class');
    expect(element.props.searchButtonClassName).toBe('order-workbench-toolbar-search-button button-class');
    expect(element.props.searchInputTestId).toBe('search-input');
    expect(element.props.searchButtonTestId).toBe('search-button');
    expect(element.props.onSearch).toBe(props.onSearch);
    expect(element.props.onSearchChange).toBe(props.onSearchChange);
    expect(element.props.onExport).toBe(props.onExport);
    expect(element.props.onPrint).toBe(props.onPrint);
    expect(element.props.onHelp).toBe(props.onHelp);
    expect(element.props.exportButtonTestId).toBe('export-id');
    expect(element.props.printButtonTestId).toBe('print-id');
    expect(element.props.helpButtonTestId).toBe('help-id');

    expect(element.props.searchButtonAriaLabel).toBe('search');
    expect(element.props.canExport).toBe(true);
    expect(element.props.exporting).toBe(false);
    expect(element.props.className).toContain('order-workbench-toolbar');
  });

  it('allows overriding className, exporting state, and aria label', () => {
    const onSearch = vi.fn();
    const onSearchChange = vi.fn();
    const element = OrderReviewWorkbenchToolbarTemplate({
      className: 'override-toolbar',
      leftContent: createElement('span'),
      searchValue: 'SO-002',
      onSearch,
      onSearchChange,
      searchPlaceholder: 'New placeholder',
      searchButtonAriaLabel: '查找',
      canExport: false,
      exporting: true,
    });

    expect(element.props.className).toContain('order-workbench-toolbar');
    expect(element.props.className).toContain('override-toolbar');
    expect(element.props.searchInputClassName).toBe('order-workbench-toolbar-search-input');
    expect(element.props.searchButtonClassName).toBe('order-workbench-toolbar-search-button');
    expect(element.props.searchButtonAriaLabel).toBe('查找');
    expect(element.props.canExport).toBe(false);
    expect(element.props.exporting).toBe(true);
    expect(element.props.onSearch).toBe(onSearch);
    expect(element.props.onSearchChange).toBe(onSearchChange);
  });

  it('keeps the shared toolbar vertically compact with 2px breathing room', () => {
    const css = readFileSync(resolve(__dirname, 'OrderReviewWorkbenchTemplate.css'), 'utf8');

    expect(css).toContain('.order-workbench-toolbar {');
    expect(css).toContain('padding-block: 2px;');
  });
});
