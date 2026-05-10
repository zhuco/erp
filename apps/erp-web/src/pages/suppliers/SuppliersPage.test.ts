import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('SuppliersPage source contract', () => {
  it('uses the unified industrial workbench instead of the legacy card-and-table layout', () => {
    const source = readFileSync(resolve(__dirname, './SuppliersPage.tsx'), 'utf8');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('className="order-review-page suppliers-page"');
    expect(source).toContain('leftPanel={');
    expect(source).toContain('rightPanel={rightPanel}');
    expect(source).toContain('bottomPanel={bottomPanel}');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
    expect(source).not.toContain("title: 'Actions'");
  });
});
