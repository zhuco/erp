import { describe, expect, it } from 'vitest';

import {
  ENTERPRISE_DEMO_MODULES,
  ERP_DATA_GRID_ENTERPRISE_MODULES,
  ORDER_WORKBENCH_ENTERPRISE_MODULES,
} from './agGridEnterpriseModules';

const toModuleNames = (modules: ReadonlyArray<{ moduleName?: string }>) =>
  modules.map((module) => module.moduleName).filter(Boolean);

describe('agGridEnterpriseModules', () => {
  it('通用数据表模块必须包含客户端行模型', () => {
    const moduleNames = toModuleNames(ERP_DATA_GRID_ENTERPRISE_MODULES);

    expect(moduleNames).toEqual(
      expect.arrayContaining([
        'AllCommunity',
        'Menu',
        'SetFilter',
        'SideBar',
        'ColumnsToolPanel',
        'FiltersToolPanel',
      ]),
    );
    expect(moduleNames).not.toEqual(
      expect.arrayContaining(['AllEnterprise', 'Pivot', 'IntegratedCharts']),
    );
  });

  it('订单工作台模块必须包含客户端行模型与基础交互模块', () => {
    const moduleNames = toModuleNames(ORDER_WORKBENCH_ENTERPRISE_MODULES);

    expect(moduleNames).toEqual(
      expect.arrayContaining(['AllCommunity', 'Menu', 'SetFilter', 'CellSelection']),
    );
    expect(moduleNames).not.toEqual(
      expect.arrayContaining(['Pivot', 'IntegratedCharts', 'SideBar']),
    );
  });

  it('演示页保留客户端行模型与分组透视模块，但不引入图表模块', () => {
    const moduleNames = toModuleNames(ENTERPRISE_DEMO_MODULES);

    expect(moduleNames).toEqual(
      expect.arrayContaining([
        'AllCommunity',
        'Menu',
        'SetFilter',
        'SideBar',
        'RowGrouping',
        'Pivot',
      ]),
    );
    expect(moduleNames).not.toContain('IntegratedCharts');
  });
});
