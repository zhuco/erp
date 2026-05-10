import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('LoginPage', () => {
  it('uses Space orientation instead of the deprecated direction prop', () => {
    const source = readFileSync(resolve(__dirname, 'LoginPage.tsx'), 'utf8');

    expect(source).toContain('<Space orientation="vertical" size={16} style={{ width: \'100%\' }}>'.replace(/'/g, "'"));
    expect(source).not.toContain('<Space direction="vertical" size={16} style={{ width: \'100%\' }}>'.replace(/'/g, "'"));
  });
});
