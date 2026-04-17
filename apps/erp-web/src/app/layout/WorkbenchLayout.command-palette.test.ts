import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchLayout command/help overlay upgrade', () => {
  it('adds command palette and shortcut help overlays', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain('commandPaletteOpen');
    expect(source).toContain('shortcutHelpOpen');
    expect(source).toContain('title="Command Palette"');
    expect(source).toContain('title="Shortcut Help"');
    expect(source).toContain('filteredWorkbenchCommands');
    expect(source).toContain('runWorkbenchCommand');
    expect(source).toContain('disableCtrlKSearchHotkey');
  });

  it('supports workbench announce event and focus-return flow', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchLayout.tsx'), 'utf8');

    expect(source).toContain("WORKBENCH_ANNOUNCE_EVENT = 'erp:workbench-announce'");
    expect(source).toContain('useFocusReturn(commandPaletteOpen)');
    expect(source).toContain('useFocusReturn(shortcutHelpOpen)');
    expect(source).toContain('workbench-shortcut-help-trigger');
    expect(source).toContain('openCommandPalette');
    expect(source).toContain('openShortcutHelp');
  });
});
