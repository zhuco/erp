import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('ModuleHeader search focus bridge', () => {
  it('listens to external focus-search event and cleans up listeners', () => {
    const source = readFileSync(resolve(__dirname, 'ModuleHeader.tsx'), 'utf8');

    expect(source).toContain("'erp:focus-function-search'");
    expect(source).toContain('window.addEventListener');
    expect(source).toContain('window.removeEventListener');
    expect(source).toContain('focusSearchInput()');
  });
});
