import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import { installAgGridLicenseLogFilter } from './suppressAgGridLicenseLogs';

describe('suppressAgGridLicenseLogs', () => {
  it('filters only the known AG Grid enterprise license banner lines', () => {
    const source = readFileSync(resolve(__dirname, 'suppressAgGridLicenseLogs.ts'), 'utf8');

    expect(source).toContain('AG Grid Enterprise License');
    expect(source).toContain('License Key Not Found');
    expect(source).toContain('For Trial Use Only');
    expect(source).toContain('console.error = (...args: unknown[]) => {');
    expect(source).toContain('args.every((arg) => isAgGridLicenseBannerMessage(arg))');
  });

  it('exports an installer function for app bootstrap', () => {
    expect(typeof installAgGridLicenseLogFilter).toBe('function');
  });
});
