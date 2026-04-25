import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

import { WorkbenchColumnSettingsFloatingControl } from './WorkbenchColumnSettingsFloatingControl';

describe('WorkbenchColumnSettingsFloatingControl', () => {
  it('renders as a pure icon button with accessibility labels when no text is configured', () => {
    const markup = renderToStaticMarkup(
      createElement(WorkbenchColumnSettingsFloatingControl, {
        ariaLabel: '\u5217\u8bbe\u7f6e',
        buttonLabel: '',
        onOpenColumnSettings: vi.fn(),
        testId: 'workbench-column-settings-trigger',
      }),
    );

    expect(markup).toContain('workbench-column-settings-floating-button');
    expect(markup).toContain('aria-label="\u5217\u8bbe\u7f6e"');
    expect(markup).toContain('title="\u5217\u8bbe\u7f6e"');
    expect(markup).not.toContain('<span>\u5217\u8bbe\u7f6e</span>');
  });

  it('supports additional wrapper and button classes for shared structural column alignment', () => {
    const markup = renderToStaticMarkup(
      createElement(WorkbenchColumnSettingsFloatingControl, {
        controlClassName: 'custom-control',
        buttonClassName: 'custom-button',
        onOpenColumnSettings: vi.fn(),
      }),
    );

    expect(markup).toContain('workbench-column-settings-floating-control custom-control');
    expect(markup).toContain('workbench-column-settings-floating-button custom-button');
  });

  it('keeps the row-index floating settings trigger explicitly centered inside the narrow structural column', () => {
    const cssSource = readFileSync(resolve(__dirname, 'workbenchColumnSettings.css'), 'utf8');

    expect(cssSource).toContain('.workbench-column-settings-floating-control--row-index');
    expect(cssSource).toContain('.workbench-column-settings-floating-button--row-index');
    expect(cssSource).toContain('.order-workbench-row-index-header-centered');
    expect(cssSource).toContain('justify-content: center;');
  });
});
