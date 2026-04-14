# After-sales Import Template Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Provide a real, testable after-sales-import CSV template download for the bulk-edit page.

**Architecture:** Introduce a dedicated helper module that owns the header/filename logic plus a download trigger, keep the BulkEdit layout unchanged, and wire the download button to the helper so Vitest can verify the pure functions without touching the UI. Processing stays client-side with standard browser blobs.

**Tech Stack:** React + TypeScript, Ant Design, Vitest, pnpm (apps/erp-web workspace)

---

### Task 1: Template helper + vitest suite

**Files:**
- Create: pps/erp-web/src/pages/afterSales/afterSalesImportTemplate.ts
- Create: pps/erp-web/src/pages/afterSales/afterSalesImportTemplate.test.ts

- [ ] **Step 1: Write the failing test**
  `	s
  import { getAfterSalesTemplateCsv, getAfterSalesTemplateFilename } from './afterSalesImportTemplate';

  describe('after sales import template helper', () => {
    it('serializes the required header row', () => {
      expect(getAfterSalesTemplateCsv()).toBe(
        'order_id,after_sales_type,sku_id,qty,refund_cent,amount_cent,reason_code\n'
      );
    });

    it('appends a sample row when requested', () => {
      expect(getAfterSalesTemplateCsv({ includeSampleRow: true })).toContain('SAMPLE_ORDER');
    });

    it('exposes a deterministic filename', () => {
      expect(getAfterSalesTemplateFilename()).toBe('after-sales-import-template.csv');
    });
  });
  `
- [ ] **Step 2: Run test to confirm failure**
  Run: pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesImportTemplate.test.ts
  Expected: FAIL (fterSalesImportTemplate module/file is missing or exports aren’t defined yet).
- [ ] **Step 3: Implement minimal helper**
  `	s
  const HEADER_COLUMNS = [
    'order_id',
    'after_sales_type',
    'sku_id',
    'qty',
    'refund_cent',
    'amount_cent',
    'reason_code'
  ];

  export const getAfterSalesTemplateCsv = ({ includeSampleRow = false } = {}) => {
    const rows = [HEADER_COLUMNS.join(',')];
    if (includeSampleRow) {
      rows.push('SAMPLE_ORDER,RETURN,SAMPLE_SKU,1,0,0,DEFECT');
    }
    return ${rows.join('\n')}\n;
  };

  export const getAfterSalesTemplateFilename = () => 'after-sales-import-template.csv';
  `
  (Also add downloadAfterSalesTemplate that builds a UTF-8 Blob, creates an <a> tag with download, clicks it, and revokes the URL.)
- [ ] **Step 4: Run the same vitest command and expect PASS**
- [ ] **Step 5: Commit**
  `ash
  git add apps/erp-web/src/pages/afterSales/afterSalesImportTemplate.ts \
    apps/erp-web/src/pages/afterSales/afterSalesImportTemplate.test.ts
  git commit -m "feat: add after sales import template helper"
  `

### Task 2: Wire BulkEdit page download button

**Files:**
- Modify: pps/erp-web/src/pages/afterSales/AfterSalesBulkEditPage.tsx

- [ ] **Step 1: Update the download button**
  - Import downloadAfterSalesTemplate from the new helper.
  - Replace the message.info call on the template button with an onClick={downloadAfterSalesTemplate}.
  - Keep the surrounding <Space>/Button structure and permissions guard.
- [ ] **Step 2: Run the vitest suite again (same command) to keep TDD discipline.**
- [ ] **Step 3: Commit**
  `ash
  git add apps/erp-web/src/pages/afterSales/AfterSalesBulkEditPage.tsx
  git commit -m "feat: wire bulk edit template download"
  `

### Review & follow-up
- Document: docs/superpowers/specs/2026-04-14-after-sales-import-template-design.md (already drafted) provides the context reviewers need.
- Plan review: no plan-document-reviewer subagent is available, so manual review is required (note for final report).
- After implementation, rerun the vitest command once more to verify the suite still passes before pushing.
