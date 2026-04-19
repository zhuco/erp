import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const projectRoot = process.cwd();

const retainedTemplateFiles = [
  'src/components/page-templates/index.ts',
  'src/components/page-templates/workbench/index.ts',
  'src/components/page-templates/workbench/WorkbenchPageTemplate.tsx',
  'src/components/page-templates/workbench/WorkbenchToolbar.tsx',
  'src/components/page-templates/workbench/WorkbenchDetailPanelTemplate.tsx',
  'src/components/page-templates/workbench/WorkbenchLeftPanelTemplate.tsx',
  'src/components/page-templates/workbench/useWorkbenchSavedViewState.ts',
  'src/components/page-templates/workbench/useWorkbenchTablePagination.ts',
  'src/components/page-templates/settings/index.ts',
  'src/components/page-templates/settings/SettingsPageTemplate.tsx',
];

const removedTemplateFiles = [
  'src/components/workbench/OrderReviewWorkbenchListPage.tsx',
  'src/components/workbench/OrderReviewWorkbenchListPage.localization.test.ts',
  'src/components/ui-modules/UiModule03MainTableSection.template.test.ts',
  'src/pages/afterSales/afterSalesImportTemplate.ts',
  'src/pages/afterSales/afterSalesImportTemplate.test.ts',
  'src/pages/listPagesTemplateUnification.test.ts',
  'src/pages/shopProducts/ShopProductDownloadPage.template.test.ts',
  'src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts',
];

describe('reduced template inventory', () => {
  it('keeps the unified template entry surface for visible pages', () => {
    retainedTemplateFiles.forEach((relativePath) => {
      expect(existsSync(resolve(projectRoot, relativePath))).toBe(true);
    });
  });

  it('removes template files that only served hidden pages', () => {
    removedTemplateFiles.forEach((relativePath) => {
      expect(existsSync(resolve(projectRoot, relativePath))).toBe(false);
    });
  });
});
