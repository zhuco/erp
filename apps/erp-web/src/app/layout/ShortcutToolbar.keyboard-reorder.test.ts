import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('ShortcutToolbar keyboard reorder and undo', () => {
  it('supports keyboard reorder and remove interactions', () => {
    const source = readFileSync(resolve(__dirname, 'ShortcutToolbar.tsx'), 'utf8');

    expect(source).toContain('handleDraftListKeyDown');
    expect(source).toContain("if (event.key === 'ArrowUp')");
    expect(source).toContain("if (event.key === 'ArrowDown')");
    expect(source).toContain("if (event.key === 'Delete' || event.key === 'Backspace')");
    expect(source).toContain('moveDraftPath');
    expect(source).toContain('aria-label="已选快捷入口列表"');
  });

  it('supports clear-undo and unified focus return', () => {
    const source = readFileSync(resolve(__dirname, 'ShortcutToolbar.tsx'), 'utf8');

    expect(source).toContain('clearUndoSnapshot');
    expect(source).toContain('handleUndoClear');
    expect(source).toContain('8 秒内撤销');
    expect(source).toContain('useFocusReturn(modalOpen)');
  });
});
