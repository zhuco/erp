import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, 'UiModule03MainTableSection.tsx'), 'utf8');

const industrialGridContractProps = [
  'gridPreset',
  'selectionCellMode',
  'selectionHeaderMode',
  'showRowIndex',
  'rowIndexWidth',
  'rowHeight',
  'headerHeight',
  'floatingFilter',
  'animateRows'
] as const;

describe('UiModule03MainTableSection 工业主表契约', () => {
  it('声明 P0 页面工业表格可选契约属性', () => {
    for (const propName of industrialGridContractProps) {
      expect(source).toContain(`${propName}?:`);
    }

    expect(source).toContain("gridPreset?: OrderQueryMainGridProps<T>['gridPreset'];");
  });

  it('将工业表格契约透传到 query variant 的 OrderWorkbenchMainGridEntry', () => {
    const entryStart = source.indexOf('<OrderWorkbenchMainGridEntry');
    const entryEnd = source.indexOf('/>', entryStart);
    const entrySource = source.slice(entryStart, entryEnd);

    expect(entrySource).toContain('variant="query"');
    expect(entrySource).toContain('{...industrialGridContractProps}');

    for (const propName of industrialGridContractProps) {
      expect(source).toContain(`${propName},`);
    }
  });

  it('defaults every legacy UiModule03 table to the industrial keyboard selection contract', () => {
    expect(source).toContain("showColumnSettings = true");
    expect(source).toContain("gridPreset = 'industrial-review-compact'");
    expect(source).toContain("selectionCellMode = 'custom'");
    expect(source).toContain("selectionHeaderMode = 'allInvert'");
  });

});
