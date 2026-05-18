import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('SalesSettingsPage contract', () => {
  it('declares the order recheck toggle in initial values and processing form section', () => {
    const source = readFileSync(resolve(__dirname, 'SalesSettingsPage.tsx'), 'utf8');

    expect(source).toContain('enableOrderRecheck: true');
    expect(source).toContain('label="是否启用订单复核"');
    expect(source).toContain('name="enableOrderRecheck"');
  });
});
