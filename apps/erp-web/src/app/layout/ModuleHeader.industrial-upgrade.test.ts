import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('ModuleHeader industrial upgrade', () => {
  it('uses anchor semantics for module navigation and submenu links', () => {
    const source = readFileSync(resolve(__dirname, 'ModuleHeader.tsx'), 'utf8');

    expect(source).toContain('className={`workbench-module-link');
    expect(source).toContain('href={item.path}');
    expect(source).toContain('href={module.defaultPath ?? undefined}');
    expect(source).toContain('event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0');
  });

  it('uses action-oriented search microcopy for ERP scenarios', () => {
    const source = readFileSync(resolve(__dirname, 'ModuleHeader.tsx'), 'utf8');

    expect(source).toContain('placeholder="搜索功能（模块名 / 路由关键词）"');
    expect(source).toContain('未找到匹配功能。支持模块名、分组名、页面名称或路由关键词。');
    expect(source).toContain("classNames={{ popup: { root: 'workbench-function-search-dropdown' } }}");
    expect(source).not.toContain('popupClassName=');
  });

  it('keeps the channel fallback top-level label renamed to 目录', () => {
    const source = readFileSync(resolve(__dirname, 'ModuleHeader.tsx'), 'utf8');

    expect(source).toContain("{ key: 'channels', label: '目录', leafPaths: [], groups: [] }");
    expect(source).not.toContain("{ key: 'channels', label: '渠道', leafPaths: [], groups: [] }");
  });
});
