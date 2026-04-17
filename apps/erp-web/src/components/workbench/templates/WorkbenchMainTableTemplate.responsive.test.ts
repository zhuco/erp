import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('Workbench template responsive ergonomics', () => {
  it('uses larger splitter hit targets and non-absolute toolbar center layout', () => {
    const layoutCss = readFileSync(resolve(__dirname, '../../../app/layout/workbenchLayout.css'), 'utf8');
    const templateCss = readFileSync(resolve(__dirname, 'WorkbenchMainTableTemplate.css'), 'utf8');

    expect(layoutCss).toContain('.workbench-resizer {');
    expect(layoutCss).toContain('width: 8px;');
    expect(layoutCss).toContain('.workbench-resizer-horizontal {');
    expect(layoutCss).toContain('height: 8px;');
    expect(layoutCss).toContain('.workbench-resizer-toggle {');
    expect(layoutCss).toContain('width: 20px;');
    expect(layoutCss).toContain('.workbench-resizer-horizontal-toggle {');
    expect(layoutCss).toContain('height: 20px;');

    expect(templateCss).toContain('.order-workbench-toolbar {');
    expect(templateCss).toContain('display: grid;');
    expect(templateCss).toContain('.order-workbench-toolbar-center {');
    expect(templateCss).toContain('justify-self: center;');
    expect(templateCss).not.toContain('position: absolute;');
  });
});
