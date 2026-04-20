import { readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

const repoRoot = path.resolve(__dirname, '../../../../../..');

const readRepoFile = (relativePath: string) =>
  readFileSync(path.resolve(repoRoot, relativePath), 'utf8');

describe('workbench state persistence regression', () => {
  it('adds persistent pagination support to the shared workbench pagination hook', () => {
    const source = readRepoFile('apps/erp-web/src/components/workbench/templates/useWorkbenchTablePagination.ts');

    expect(source).toContain('storageKey?: string;');
    expect(source).toContain('readWorkbenchViewStateCache(storageKey)');
    expect(source).toContain('writeWorkbenchViewStateCache(storageKey');
    expect(source).toContain('pagination: {');
  });

  it('adds scroll position persistence to the shared workbench page template', () => {
    const source = readRepoFile('apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx');

    expect(source).toContain('stateCacheKey?: string;');
    expect(source).toContain('readWorkbenchViewStateCache(stateCacheKey)');
    expect(source).toContain('writeWorkbenchViewStateCache(stateCacheKey');
    expect(source).toContain('scrollTop');
    expect(source).toContain('scrollLeft');
  });
});
