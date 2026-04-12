import { createElement, isValidElement } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { UiModule02ActionSearchToolbar } from './index';

const findElement = (
  node: ReactNode,
  predicate: (element: ReactElement) => boolean,
): ReactElement | null => {
  if (Array.isArray(node)) {
    for (const child of node) {
      const match = findElement(child, predicate);
      if (match) {
        return match;
      }
    }
    return null;
  }

  if (!isValidElement(node)) {
    return null;
  }

  if (predicate(node)) {
    return node;
  }

  return findElement(node.props.children, predicate);
};

describe('UiModule02ActionSearchToolbar', () => {
  it('renders the search area and forwards toolbar actions', () => {
    const onSearch = vi.fn();
    const onSearchChange = vi.fn();
    const onExport = vi.fn();
    const onPrint = vi.fn();
    const onHelp = vi.fn();

    const element = UiModule02ActionSearchToolbar({
      leftContent: createElement('span', { 'data-testid': 'left-action' }, 'left'),
      searchValue: 'SO-001',
      onSearch,
      onSearchChange,
      searchPlaceholder: 'order no/account/phone',
      searchInputClassName: 'review-search-input',
      searchButtonClassName: 'review-search-trigger',
      searchInputTestId: 'review-search-input',
      searchButtonTestId: 'review-search-trigger',
      searchButtonAriaLabel: 'search',
      onExport,
      onPrint,
      onHelp,
      exportButtonTestId: 'toolbar-export',
      printButtonTestId: 'toolbar-print',
      helpButtonTestId: 'toolbar-help',
    });

    expect(element.props.className).toContain('order-workbench-toolbar');
    expect(findElement(element, (child) => child.props['data-testid'] === 'left-action')).not.toBeNull();

    const searchInput = findElement(
      element,
      (child) => child.props['data-testid'] === 'review-search-input',
    );
    expect(searchInput).not.toBeNull();
    expect(searchInput?.props.placeholder).toBe('order no/account/phone');
    expect(searchInput?.props.value).toBe('SO-001');
    expect(searchInput?.props.className).toContain('review-search-input');

    searchInput?.props.onChange({ target: { value: 'SO-002' } });
    searchInput?.props.onPressEnter();
    expect(onSearchChange).toHaveBeenCalledWith('SO-002');
    expect(onSearch).toHaveBeenCalledTimes(1);

    const searchButton = findElement(
      element,
      (child) => child.props['data-testid'] === 'review-search-trigger',
    );
    expect(searchButton).not.toBeNull();
    expect(searchButton?.props.className).toContain('review-search-trigger');
    searchButton?.props.onClick();
    expect(onSearch).toHaveBeenCalledTimes(2);

    const rightToolbar = findElement(
      element,
      (child) => child.props.exportButtonTestId === 'toolbar-export',
    );
    expect(rightToolbar).not.toBeNull();
    expect(rightToolbar?.props.onExport).toBe(onExport);
    expect(rightToolbar?.props.onPrint).toBe(onPrint);
    expect(rightToolbar?.props.onHelp).toBe(onHelp);
  });
});
