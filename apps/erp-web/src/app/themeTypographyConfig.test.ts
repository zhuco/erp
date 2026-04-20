import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('theme typography config', () => {
  it('keeps the default body font size in a shared typography entry that drives tokens and CSS variables', () => {
    const appProvidersSource = readFileSync(resolve(__dirname, 'AppProviders.tsx'), 'utf8');
    const themeCssSource = readFileSync(resolve(__dirname, '../styles/theme.css'), 'utf8');
    const typographySource = readFileSync(resolve(__dirname, '../styles/typography.ts'), 'utf8');

    expect(typographySource).toContain('body: 12.8');
    expect(typographySource).toContain('bodyStrong: 12.8');
    expect(typographySource).toContain('title: 15');
    expect(typographySource).toContain('titleDisplay: 18');
    expect(appProvidersSource).toContain('resolveTypographyScale');
    expect(themeCssSource).toContain('--erp-font-size-body: 12.8px;');
    expect(themeCssSource).toContain('--erp-font-size-body-strong: 12.8px;');
    expect(themeCssSource).toContain('--erp-font-size-title: 15px;');
    expect(themeCssSource).toContain('--erp-font-size-title-display: 18px;');
    expect(themeCssSource).toContain('--erp-font-size-base: var(--erp-font-size-body);');
  });
});
