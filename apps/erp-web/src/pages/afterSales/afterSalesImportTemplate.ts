const HEADER_COLUMNS = [
  'order_id',
  'after_sales_type',
  'sku_id',
  'qty',
  'refund_cent',
  'amount_cent',
  'reason_code'
] as const;

const SAMPLE_ROW = 'SAMPLE_ORDER,RETURN_REFUND,SAMPLE_SKU,1,0,0,DEFECT';
const CSV_BOM = '\uFEFF';

export interface AfterSalesTemplateOptions {
  includeSampleRow?: boolean;
}

export const getAfterSalesTemplateCsv = (
  options: AfterSalesTemplateOptions = {}
): string => {
  const rows = [HEADER_COLUMNS.join(',')];
  if (options.includeSampleRow) {
    rows.push(SAMPLE_ROW);
  }
  return `${rows.join('\n')}\n`;
};

export const getAfterSalesTemplateFilename = (): string => 'after-sales-import-template.csv';

export const downloadAfterSalesTemplate = (options?: AfterSalesTemplateOptions) => {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    throw new Error('downloadAfterSalesTemplate can only run in the browser');
  }
  const csv = getAfterSalesTemplateCsv(options);
  const blob = new Blob([CSV_BOM + csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = getAfterSalesTemplateFilename();
  anchor.style.display = 'none';
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  URL.revokeObjectURL(url);
};
