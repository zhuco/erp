import { describe, expect, it } from 'vitest';

import { appMenuItems, flattenMenuItems } from './menu';

describe('系统菜单调试入口收口', () => {
  it('保留 AG Grid 入口并移除 EDG 调试入口', () => {
    const leafPaths = flattenMenuItems(appMenuItems).map((item) => item.path);

    expect(leafPaths).toContain('/system/ag-grid-enterprise');
    expect(leafPaths).not.toContain('/system/erp-data-grid-next');
    expect(leafPaths).not.toContain('/system/erp-data-grid');
    expect(leafPaths).not.toContain('/system/erp-data-grid-review');
  });

  it('销售菜单暴露独立的对账与异常 SLA 作业台', () => {
    const leafPaths = flattenMenuItems(appMenuItems).map((item) => item.path);

    expect(leafPaths).toContain('/orders/split-merge');
    expect(leafPaths).toContain('/orders/reconciliations');
    expect(leafPaths).toContain('/orders/exception-sla');
  });
});
