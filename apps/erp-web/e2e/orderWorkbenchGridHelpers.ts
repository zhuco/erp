import { expect, type Locator, type Page } from '@playwright/test';

export const getCenterRows = (grid: Locator) =>
  grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');

export const getBusinessRowById = (grid: Locator, rowId: string) =>
  grid.locator(`.ag-center-cols-container .ag-row[row-id="${rowId}"]`).first();

export const getStructuralRowById = (grid: Locator, rowId: string) =>
  grid
    .locator(
      [
        `.ag-pinned-left-cols-container .ag-row[row-id="${rowId}"]`,
        `.ag-center-cols-container .ag-row[row-id="${rowId}"]`,
      ].join(', '),
    )
    .first();

export const getStructuralCell = (grid: Locator, rowId: string, colId: string) =>
  grid
    .locator(
      [
        `.ag-pinned-left-cols-container .ag-row[row-id="${rowId}"] .ag-cell[col-id="${colId}"]`,
        `.ag-center-cols-container .ag-row[row-id="${rowId}"] .ag-cell[col-id="${colId}"]`,
      ].join(', '),
    )
    .first();

export const getSelectionCell = (grid: Locator, rowId: string) =>
  getStructuralCell(grid, rowId, '__erpSelection');

export const getSelectionVisual = (grid: Locator, rowId: string) =>
  grid
    .locator(
      [
        `.ag-pinned-left-cols-container .ag-row[row-id="${rowId}"] .ag-cell[col-id="__erpSelection"] .erp-selection-checkbox-visual`,
        `.ag-center-cols-container .ag-row[row-id="${rowId}"] .ag-cell[col-id="__erpSelection"] .erp-selection-checkbox-visual`,
      ].join(', '),
    )
    .first();

export const getHeaderCell = (grid: Locator, colId: string) =>
  grid.locator(`.ag-header-cell[col-id="${colId}"]`).first();

export const clickSelectionCheckboxCenter = async (
  page: Page,
  grid: Locator,
  rowId: string,
  modifiers: Array<'Shift' | 'Control'> = [],
) => {
  const cell = getSelectionCell(grid, rowId);
  const box = await cell.boundingBox();
  expect(box).not.toBeNull();
  for (const modifier of modifiers) {
    await page.keyboard.down(modifier);
  }
  try {
    await page.mouse.click(box!.x + box!.width / 2, box!.y + box!.height / 2);
  } finally {
    for (const modifier of modifiers.slice().reverse()) {
      await page.keyboard.up(modifier);
    }
  }
};

export const clickSelectionCheckboxHotArea = async (
  page: Page,
  grid: Locator,
  rowId: string,
  modifiers: Array<'Shift' | 'Control'> = [],
) => {
  const cell = getSelectionCell(grid, rowId);
  const box = await cell.boundingBox();
  expect(box).not.toBeNull();
  for (const modifier of modifiers) {
    await page.keyboard.down(modifier);
  }
  try {
    await page.mouse.click(box!.x + Math.min(box!.width / 2, 12), box!.y + box!.height / 2);
  } finally {
    for (const modifier of modifiers.slice().reverse()) {
      await page.keyboard.up(modifier);
    }
  }
};

export const clickSelectionColumnBlankArea = async (
  page: Page,
  grid: Locator,
  rowId: string,
  modifiers: Array<'Shift' | 'Control'> = [],
) => {
  const cell = getSelectionCell(grid, rowId);
  const box = await cell.boundingBox();
  expect(box).not.toBeNull();
  for (const modifier of modifiers) {
    await page.keyboard.down(modifier);
  }
  try {
    await page.mouse.click(box!.x + box!.width - 3, box!.y + box!.height / 2);
  } finally {
    for (const modifier of modifiers.slice().reverse()) {
      await page.keyboard.up(modifier);
    }
  }
};
