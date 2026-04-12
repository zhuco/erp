import type { GridApi } from 'ag-grid-community';

import type { OrderSummary } from '@/api/orders';

type PrintWindowLike = {
  document: {
    write: (html: string) => void;
    close: () => void;
  };
  focus: () => void;
  print: () => void;
  close: () => void;
  onafterprint: null | (() => void);
};

type OpenPrintWindow = () => PrintWindowLike | null;

const INTERNAL_COLUMN_PREFIX = '__erp';

const getExportableColumnKeys = (api: Pick<GridApi<OrderSummary>, 'getAllDisplayedColumns'>) =>
  api
    .getAllDisplayedColumns()
    .map((column) => column.getColId())
    .filter((colId) => colId && !colId.startsWith(INTERNAL_COLUMN_PREFIX));

const buildExportFileName = (title: string) => {
  const timestamp = new Date().toISOString().replace(/[-:]/g, '').replace(/\..+$/, '');
  return `${title}-${timestamp}.xlsx`;
};

const escapeHtml = (value: string) =>
  value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

const parseCsv = (csv: string) => {
  const rows: string[][] = [];
  let currentRow: string[] = [];
  let currentCell = '';
  let insideQuotes = false;

  for (let index = 0; index < csv.length; index += 1) {
    const char = csv[index];
    const nextChar = csv[index + 1];

    if (char === '"') {
      if (insideQuotes && nextChar === '"') {
        currentCell += '"';
        index += 1;
        continue;
      }
      insideQuotes = !insideQuotes;
      continue;
    }

    if (char === ',' && !insideQuotes) {
      currentRow.push(currentCell);
      currentCell = '';
      continue;
    }

    if ((char === '\n' || char === '\r') && !insideQuotes) {
      if (char === '\r' && nextChar === '\n') {
        index += 1;
      }
      currentRow.push(currentCell);
      rows.push(currentRow);
      currentRow = [];
      currentCell = '';
      continue;
    }

    currentCell += char;
  }

  if (currentCell.length > 0 || currentRow.length > 0) {
    currentRow.push(currentCell);
    rows.push(currentRow);
  }

  return rows.filter((row) => row.length > 0 && row.some((cell) => cell.length > 0));
};

const buildPrintHtml = (title: string, headers: string[], bodyRows: string[][]) => {
  const tableHead = headers.map((header) => `<th>${escapeHtml(header)}</th>`).join('');
  const tableBody = bodyRows
    .map((row) => `<tr>${row.map((cell) => `<td>${escapeHtml(cell)}</td>`).join('')}</tr>`)
    .join('');

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>${escapeHtml(title)}</title>
    <style>
      body { font-family: Arial, sans-serif; padding: 16px; }
      h1 { margin: 0 0 12px; font-size: 18px; }
      table { width: 100%; border-collapse: collapse; font-size: 12px; }
      th, td { border: 1px solid #ddd; padding: 6px 8px; text-align: left; vertical-align: top; }
      th { background: #f5f5f5; }
    </style>
  </head>
  <body>
    <h1>${escapeHtml(title)}</h1>
    <table>
      <thead><tr>${tableHead}</tr></thead>
      <tbody>${tableBody}</tbody>
    </table>
  </body>
</html>`;
};

const defaultOpenPrintWindow: OpenPrintWindow = () =>
  window.open('', '_blank', 'width=1200,height=800') as PrintWindowLike | null;

export const exportOrderReviewGridAsExcel = (
  api: Pick<GridApi<OrderSummary>, 'exportDataAsExcel' | 'getAllDisplayedColumns'>,
  title: string,
) => {
  const columnKeys = getExportableColumnKeys(api);
  if (!columnKeys.length) {
    throw new Error('NO_EXPORTABLE_COLUMNS');
  }

  api.exportDataAsExcel({
    columnKeys,
    fileName: buildExportFileName(title),
    sheetName: title,
  });
};

export const printOrderReviewGrid = (
  api: Pick<GridApi<OrderSummary>, 'getAllDisplayedColumns' | 'getDataAsCsv'>,
  title: string,
  openPrintWindow: OpenPrintWindow = defaultOpenPrintWindow,
) => {
  const columnKeys = getExportableColumnKeys(api);
  if (!columnKeys.length) {
    throw new Error('NO_PRINTABLE_COLUMNS');
  }

  const csv = api.getDataAsCsv({ columnKeys });
  const rows = parseCsv(csv);
  if (!rows.length) {
    throw new Error('NO_PRINTABLE_ROWS');
  }

  const [headers, ...bodyRows] = rows;
  const printWindow = openPrintWindow();
  if (!printWindow) {
    throw new Error('PRINT_WINDOW_BLOCKED');
  }

  printWindow.document.write(buildPrintHtml(title, headers, bodyRows));
  printWindow.document.close();
  printWindow.focus();
  printWindow.print();
  printWindow.onafterprint = () => {
    printWindow.close();
  };
};
