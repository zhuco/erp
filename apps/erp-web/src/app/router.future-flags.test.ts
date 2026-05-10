import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('router future flags', () => {
  it('opts RouterProvider into the v7 startTransition behavior', () => {
    const source = readFileSync(resolve(__dirname, '../main.tsx'), 'utf8');

    expect(source).toContain('<RouterProvider router={router} future={{ v7_startTransition: true }} />');
  });
});
