import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('ModuleHeader accessibility labels', () => {
  it('uses Chinese aria labels for workbench header controls', () => {
    const source = readFileSync(resolve(__dirname, 'ModuleHeader.tsx'), 'utf8');

    expect(source).toContain('aria-label="功能搜索输入"');
    expect(source).toContain('aria-label="功能搜索"');
    expect(source).toContain("denseMode ? '关闭紧凑模式' : '开启紧凑模式'");
    expect(source).toContain('aria-label="通知"');
    expect(source).toContain('aria-label="设置"');
    expect(source).toContain('aria-label="退出登录"');
    expect(source).toContain('aria-label={`用户菜单：${userName}`}');

    expect(source).not.toContain('Function search input');
    expect(source).not.toContain('Function search');
    expect(source).not.toContain('Enable compact mode');
    expect(source).not.toContain('Disable compact mode');
    expect(source).not.toContain('Notifications');
    expect(source).not.toContain('Settings');
    expect(source).not.toContain('Sign out');
    expect(source).not.toContain('User menu:');
  });
});
