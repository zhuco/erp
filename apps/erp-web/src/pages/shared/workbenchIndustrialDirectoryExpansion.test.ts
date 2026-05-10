import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const readSource = (relativePath: string) =>
  readFileSync(resolve(__dirname, relativePath), 'utf8');

describe('workbench industrial directory expansion', () => {
  it('keeps containers on the full industrial workbench shell', () => {
    const source = readSource('../wms/ContainersPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps sender addresses on the full industrial workbench shell', () => {
    const source = readSource('../wms/SenderAddressesPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps warehouse areas on the full industrial workbench shell', () => {
    const source = readSource('../wms/WarehouseAreasPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps warehouse locations on the full industrial workbench shell', () => {
    const source = readSource('../wms/WarehouseLocationsPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps permissions on the full industrial workbench shell', () => {
    const source = readSource('../system/PermissionsPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps roles on the full industrial workbench shell', () => {
    const source = readSource('../system/RolesPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps customer categories on the full industrial workbench shell', () => {
    const source = readSource('../customers/CustomerCategoriesPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps member levels on the full industrial workbench shell', () => {
    const source = readSource('../customers/MemberLevelsPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });

  it('keeps customer tags on the full industrial workbench shell', () => {
    const source = readSource('../customers/CustomerTagsPage.tsx');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('UiModule03MainTableSection');
  });
});
