import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('WorkbenchListPage localization fallbacks', () => {
  it('contains readable Chinese fallback labels and no mojibake fragments', () => {
    const source = readFileSync(resolve(__dirname, 'WorkbenchListPage.tsx'), 'utf8');

    expect(source).toContain("label: '标记'");
    expect(source).toContain("label: '默认过滤'");
    expect(source).toContain('请配置底部子表 Tabs');

    expect(source).not.toMatch(/鏆|琛ㄦ牸|榛樿|鏍囪|甯哥敤鍔熻兘|璇︽儏/);
  });
});
