import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WarehouseDirectoryPage source contract', () => {
  it('keeps warehouses on the full industrial workbench shell', () => {
    const source = readFileSync(resolve(__dirname, './WarehouseDirectoryPage.tsx'), 'utf8');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('className="order-review-page warehouse-directory-page"');
    expect(source).toContain('leftPanel={');
    expect(source).toContain('leftPanelVisible={true}');
    expect(source).toContain('rightPanel={rightPanel}');
    expect(source).toContain('rightPanelVisible={true}');
    expect(source).toContain('bottomPanel={bottomPanel}');
    expect(source).toContain('bottomPanelVisible={true}');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('onGridFilterModelChange={(model) => {');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });
});
