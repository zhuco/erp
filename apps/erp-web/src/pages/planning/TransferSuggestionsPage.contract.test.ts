import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './TransferSuggestionsPage.tsx'), 'utf8');

describe('TransferSuggestionsPage 统一表格契约', () => {
  it('使用 ErpDataGrid 替代页面级裸 Table', () => {
    expect(source).toContain('ErpDataGrid<PlanningTransferSuggestion>');
    expect(source).toContain('data-testid="planning-transfer-table"');
    expect(source).not.toMatch(/<Table\b/);
  });

  it('禁止页面级裸 Card 容器', () => {
    expect(source).not.toMatch(/<Card\b/);
  });
});
