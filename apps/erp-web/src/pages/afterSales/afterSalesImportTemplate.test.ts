import { describe, expect, it } from 'vitest';
import { getAfterSalesTemplateCsv, getAfterSalesTemplateFilename } from './afterSalesImportTemplate';

describe('after sales import template helper', () => {
  it('produces the required header row', () => {
    expect(getAfterSalesTemplateCsv()).toBe(
      'order_id,after_sales_type,sku_id,qty,refund_cent,amount_cent,reason_code\n'
    );
  });

  it('appends a sample row only when requested', () => {
    expect(getAfterSalesTemplateCsv({ includeSampleRow: true })).toContain('SAMPLE_ORDER');
    expect(getAfterSalesTemplateCsv({ includeSampleRow: false })).not.toContain('SAMPLE_ORDER');
  });

  it('returns the expected filename', () => {
    expect(getAfterSalesTemplateFilename()).toBe('after-sales-import-template.csv');
  });
});
