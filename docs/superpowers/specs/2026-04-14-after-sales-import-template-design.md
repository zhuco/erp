# After-sales Import Template Landing
**Date**: 2026-04-14

## Context
- The after-sales bulk edit page currently advertises a template download but just records a message.info stub.
- The operational need is to provide a CSV template that crews can actually download and populate so the backend import receives consistent data.
- Requirements already narrow down the core columns to order_id, fter_sales_type, sku_id, qty, efund_cent, mount_cent, and eason_code.

## Goals
1. Replace the placeholder download action with a real CSV file generation/ download.
2. Encapsulate all template text and filename decisions in pure, easily testable functions.
3. Leave the UI layout intact while wiring the existing button to the new helper.
4. Keep the experience in the bulk-edit page in sync with the rest of the product (no new visual treatments or messaging).

## Assumptions
- The template only needs a single header row at this point; we can revisit sample rows once we have input from the requestor.
- UTF-8 (with BOM) and comma-separated columns are acceptable for target customers who open the files in Excel.

## Solution Overview
### Template module (fterSalesImportTemplate.ts)
- Export a constant array of the canonical column keys and two combinators: getAfterSalesTemplateCsv() and getAfterSalesTemplateFilename().
- getAfterSalesTemplateCsv() joins the columns with commas, optionally appending a sample row when includeSampleRow is true, and always terminates the string with a newline so Excel sees a complete row.
- getAfterSalesTemplateFilename() returns a deterministic fter-sales-import-template.csv so tests can assert on the exact file name.
- Export an imperative helper (downloadAfterSalesTemplate()) that creates a UTF-8 blob (with BOM), instantiates an <a> element, populates href/download, clicks it, and revokes the URL, keeping that logic separate from the page.

### UI Integration (AfterSalesBulkEditPage.tsx)
- Import downloadAfterSalesTemplate from the new module and hook it straight to the Button that currently calls message.info.
- Leave the surrounding form, upload logic, and spacing untouched so the page retains its existing structure.
- No API changes or backend work is required; all logic runs in the browser.

### Testing Strategy
- fterSalesImportTemplate.test.ts targets the pure functions:
  - Confirm the default CSV string equals the header row plus newline.
  - Confirm toggling includeSampleRow appends a predictable example row.
  - Confirm getAfterSalesTemplateFilename() returns the expected string.
- The vitest command supplied in the task card (pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesImportTemplate.test.ts) will execute this suite.
- Following TDD, the suite will be authored before the helper implementation so the first run fails and subsequent runs verify the fix.

## Risks & Open Questions
- No dedicated spec-reviewer subagent is available in this environment, so the review loop noted in the brainstorming skill can’t be automated; I’ll document this for later follow-up.
- If the requirement changes to include sample rows or descriptive headers, the helper already accepts an option to append them, but we’ll want to re-validate the content with stakeholders.

## File Impact Summary
- pps/erp-web/src/pages/afterSales/afterSalesImportTemplate.ts (new helper module)
- pps/erp-web/src/pages/afterSales/afterSalesImportTemplate.test.ts (vitest suite)
- pps/erp-web/src/pages/afterSales/AfterSalesBulkEditPage.tsx (download button wiring)

## Next Steps
1. Get the spec documented and reviewed (this file).
2. After the spec is approved, use the writing-plans skill to break the implementation into TDD steps.
3. Implement tests first, then the helper, and finally update the UI button.
