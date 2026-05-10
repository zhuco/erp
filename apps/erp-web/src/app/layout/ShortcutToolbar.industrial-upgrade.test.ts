import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('ShortcutToolbar industrial upgrade', () => {
  it('uses non-deprecated modal lifecycle props', () => {
    const source = readFileSync(resolve(__dirname, 'ShortcutToolbar.tsx'), 'utf8');

    expect(source).toContain('destroyOnHidden');
    expect(source).not.toContain('destroyOnClose');
    expect(source).toContain('orientation="vertical"');
    expect(source).not.toContain('direction="vertical"');
  });

  it('renders the toolbar as an industrial panel instead of an inline white strip', () => {
    const source = readFileSync(resolve(__dirname, 'ShortcutToolbar.tsx'), 'utf8');

    expect(source).toContain('className="shortcut-toolbar"');
    expect(source).toContain('className="shortcut-toolbar-items"');
    expect(source).not.toContain("background: '#fff'");
  });

  it('keeps the shortcut toolbar title readable on the industrial panel', () => {
    const source = readFileSync(resolve(__dirname, 'ShortcutToolbar.css'), 'utf8');

    expect(source).toContain('.shortcut-toolbar-title.ant-typography');
    expect(source).toContain('color: #fff;');
  });
});
