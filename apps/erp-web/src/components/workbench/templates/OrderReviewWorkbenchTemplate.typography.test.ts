import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('OrderReviewWorkbenchTemplate typography', () => {
  it('routes workbench body text through the shared 13px body scale and shared 15/18 title scales', () => {
    const cssSource = readFileSync(resolve(__dirname, 'OrderReviewWorkbenchTemplate.css'), 'utf8');

    expect(cssSource).toContain('font-size: var(--erp-font-size-body);');
    expect(cssSource).toContain('line-height: var(--erp-line-height);');
    expect(cssSource).toContain('--ag-font-size: var(--erp-font-size-body);');
    expect(cssSource).toContain('.order-workbench-page :where(');
    expect(cssSource).toContain('font-size: var(--erp-font-size-title);');
    expect(cssSource).toContain('font-size: var(--erp-font-size-title-display);');
  });
});
