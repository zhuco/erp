import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('LogisticsFeeDirectoryPage source contract', () => {
  it('uses the unified industrial directory workbench instead of a placeholder page', () => {
    const source = readFileSync(resolve(__dirname, './LogisticsFeeDirectoryPage.tsx'), 'utf8');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('className="order-review-page logistics-fee-directory-page"');
    expect(source).toContain('leftPanel={leftPanel}');
    expect(source).toContain('rightPanel={rightPanel}');
    expect(source).toContain('bottomPanel={bottomPanel}');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('<PlaceholderPage');
  });
});
