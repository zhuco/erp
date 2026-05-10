import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('TagDirectory unified workbench adoption', () => {
  it('pulls order, sku, and builtin tags into a single shared workbench', () => {
    const source = readFileSync(resolve(__dirname, './TagDirectoryWorkbench.tsx'), 'utf8');

    expect(source).toContain('WorkbenchPageTemplate');
    expect(source).toContain('WorkbenchToolbar');
    expect(source).toContain('ErpDataGridNext');
    expect(source).toContain('listOrderTagDefinitions');
    expect(source).toContain('listSkuTagDefinitions');
    expect(source).toContain('builtinOrderTags');
  });

  it('keeps tag directory on the same full industrial workbench shell as order review', () => {
    const source = readFileSync(resolve(__dirname, './TagDirectoryWorkbench.tsx'), 'utf8');

    expect(source).toContain('className="order-review-page tag-directory-page"');
    expect(source).toContain('leftPanel={leftPanel}');
    expect(source).toContain('leftPanelVisible={true}');
    expect(source).toContain('rightPanel={rightPanel}');
    expect(source).toContain('rightPanelVisible={true}');
    expect(source).toContain('bottomPanel={bottomPanel}');
    expect(source).toContain('bottomPanelVisible={true}');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps the route page as a thin tab-state wrapper instead of nesting subpages', () => {
    const source = readFileSync(resolve(__dirname, './TagDirectoryPage.tsx'), 'utf8');

    expect(source).toContain('TagDirectoryWorkbench');
    expect(source).not.toContain('OrderTagDefinitionsPage');
    expect(source).not.toContain('SkuTagDefinitionsPage');
  });

  it('aligns the order and sku subpage exports to the shared workbench shell', () => {
    const orderSource = readFileSync(resolve(__dirname, './OrderTagDefinitionsPage.tsx'), 'utf8');
    const skuSource = readFileSync(resolve(__dirname, './SkuTagDefinitionsPage.tsx'), 'utf8');

    expect(orderSource).toContain('TagDirectoryWorkbench');
    expect(orderSource).not.toContain('UiModule03MainTableSection');
    expect(skuSource).toContain('TagDirectoryWorkbench');
    expect(skuSource).not.toContain('UiModule03MainTableSection');
  });
});
