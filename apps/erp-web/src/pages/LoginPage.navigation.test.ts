import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('LoginPage navigation target', () => {
  it('redirects the default post-login entry to the sales order query workbench', () => {
    const source = readFileSync(resolve(__dirname, 'LoginPage.tsx'), 'utf8');

    expect(source).toContain("'/sales/order-query'");
    expect(source).not.toContain("'/orders'");
  });
});
