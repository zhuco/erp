import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ReconcilePage.tsx'), 'utf8');
const reconcileJobTable = source.match(/<UiModule03MainTableSection<ReconcileJob>[\s\S]*?\n {6}\/>/)?.[0] ?? '';
const reconcileItemTable = source.match(/<UiModule03MainTableSection<ReconcileJobItem>[\s\S]*?\n {6}\/>/)?.[0] ?? '';
const paymentJobTable = source.match(/<UiModule03MainTableSection<PaymentReconcileJob>[\s\S]*?\n {6}\/>/)?.[0] ?? '';
const paymentItemTable = source.match(/<UiModule03MainTableSection<PaymentReconcileItem>[\s\S]*?\n {6}\/>/)?.[0] ?? '';

describe('ReconcilePage 工业表格契约', () => {
  it('将对账任务主列表迁移到统一工业主表组件', () => {
    expect(reconcileJobTable).toContain('dataTestId="finance-reconcile-main-grid"');
    expect(reconcileJobTable).toContain('columnSettingsKey="finance.reconcile.main-grid"');
    expect(reconcileJobTable).toContain('toolbarLeft={reconcileJobHeader}');
  });

  it('显式接入工业紧凑表格、选择列和列设置入口', () => {
    expect(reconcileJobTable).toContain('gridPreset="industrial-review-compact"');
    expect(reconcileJobTable).toContain('selectionCellMode="custom"');
    expect(reconcileJobTable).toContain('selectionHeaderMode="allInvert"');
    expect(reconcileJobTable).toContain('showColumnSettings');
  });

  it('差异明细和支付对账列表不再使用裸 Table', () => {
    expect(reconcileItemTable).toContain('dataTestId="finance-reconcile-items-grid"');
    expect(paymentJobTable).toContain('dataTestId="finance-payment-reconcile-jobs-grid"');
    expect(paymentItemTable).toContain('dataTestId="finance-payment-reconcile-items-grid"');
    expect(source).not.toMatch(/<Table\b/);
  });

  it('二级表格也使用统一工具栏与工业紧凑表格契约', () => {
    for (const table of [reconcileItemTable, paymentJobTable, paymentItemTable]) {
      expect(table).toContain('toolbarLeft=');
      expect(table).toContain('gridPreset="industrial-review-compact"');
      expect(table).toContain('selectionCellMode="custom"');
      expect(table).toContain('selectionHeaderMode="allInvert"');
    }
  });
});
