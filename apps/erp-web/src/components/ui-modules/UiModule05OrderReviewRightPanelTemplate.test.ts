import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('UiModule05OrderReviewRightPanelTemplate', () => {
  it('upgrades the mark area to a unified compact popover-based manager', () => {
    const source = readFileSync(resolve(__dirname, 'UiModule05OrderReviewRightPanelTemplate.tsx'), 'utf8');
    const css = readFileSync(resolve(__dirname, 'UiModule05OrderReviewRightPanelTemplate.css'), 'utf8');

    expect(source).toContain('Popover');
    expect(source).toContain('管理记号');
    expect(source).toContain('输入新记号');
    expect(source).toContain('快捷添加');
    expect(source).toContain('记号库');
    expect(source).toContain('当前订单');
    expect(source).toContain('ui-module-05-order-mark-manager-popover');
    expect(source).toContain('destroyOnHidden');
    expect(source).not.toContain('destroyTooltipOnHide');
    expect(source).toContain('onMarkRemove');
    expect(source).toContain('onMarkCreate');
    expect(source).toContain('onMarkDefinitionDelete');
    expect(source).toContain('onMarkLibraryRefresh');
    expect(css).toContain('.ui-module-05-order-mark-manager-popover');
  });

  it('keeps the inline mark badges single-line and without dash placeholders', () => {
    const source = readFileSync(resolve(__dirname, 'UiModule05OrderReviewRightPanelTemplate.tsx'), 'utf8');

    expect(source).toContain('emptyText={null}');
    expect(source).toContain('singleLine');
  });
});
