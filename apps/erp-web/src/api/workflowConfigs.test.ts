import { afterEach, describe, expect, it, vi } from 'vitest';

import { mockTenantId, mockWarehouses } from './mocks/data';

const loadWorkflowConfigApiWithMocks = async () => {
  vi.resetModules();
  vi.doMock('@/utils/env', async () => {
    const actual = await vi.importActual<typeof import('@/utils/env')>('@/utils/env');
    return {
      ...actual,
      useMocks: true,
    };
  });

  return import('./workflowConfigs');
};

afterEach(() => {
  vi.doUnmock('@/utils/env');
  vi.resetModules();
});

describe('WMS 流程配置 mock API', () => {
  it('mock 模式下更新后再次读取仍返回最新配置', async () => {
    const { getWarehouseWorkflowConfig, updateWarehouseWorkflowConfig } =
      await loadWorkflowConfigApiWithMocks();
    const warehouseId = mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT';

    const initial = await getWarehouseWorkflowConfig(mockTenantId, warehouseId);
    expect(initial.enable_wave).toBe(false);
    expect(initial.require_pack).toBe(true);

    await updateWarehouseWorkflowConfig(mockTenantId, warehouseId, {
      enable_wave: true,
      require_pack: false,
      require_inspection: false,
      require_weighing: false,
    });

    const updated = await getWarehouseWorkflowConfig(mockTenantId, warehouseId);

    expect(updated).toEqual({
      warehouse_id: warehouseId,
      enable_wave: true,
      require_pack: false,
      require_inspection: false,
      require_weighing: false,
    });
  });
});
