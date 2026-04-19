import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('OrderReviewPage keyboard efficiency upgrade', () => {
  it('supports bulk action hotkeys', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewPage.tsx'), 'utf8');

    expect(source).toContain("if (key === 'a')");
    expect(source).toContain("if (key === 'm')");
    expect(source).toContain("if (key === 's')");
    expect(source).toContain('aria-keyshortcuts="Alt+Shift+A"');
    expect(source).toContain('aria-keyshortcuts="Alt+Shift+M"');
    expect(source).toContain('aria-keyshortcuts="Alt+Shift+S"');
  });

  it('supports local filter snapshot save/replay', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewPage.tsx'), 'utf8');

    expect(source).toContain('ORDER_REVIEW_FILTER_SNAPSHOT_VERSION');
    expect(source).toContain('filterSnapshotStorageKey');
    expect(source).toContain('persistFilterSnapshot');
    expect(source).toContain('replayFilterSnapshot');
    expect(source).not.toContain('Alt+1..9 回放');
    expect(source).not.toContain('Ctrl+Alt+1..9 保存');
  });
});
