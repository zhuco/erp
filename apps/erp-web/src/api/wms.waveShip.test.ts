import { afterEach, describe, expect, it, vi } from 'vitest';

import { mockTenantId } from './mocks/data';

const loadWmsApiWithMocks = async () => {
  vi.resetModules();
  vi.doMock('@/utils/env', async () => {
    const actual = await vi.importActual<typeof import('@/utils/env')>('@/utils/env');
    return {
      ...actual,
      useMocks: true,
    };
  });

  return import('./wms');
};

afterEach(() => {
  vi.doUnmock('@/utils/env');
  vi.resetModules();
});

describe('WMS wave ship mock API', () => {
  it('ships wave and returns summary', async () => {
    const { listWmsWaves, getWmsWave, shipWmsWave } = await loadWmsApiWithMocks();

    const waves = await listWmsWaves(mockTenantId, {});
    expect(waves.items.length).toBeGreaterThan(0);

    const target = waves.items[0];
    expect(target).toBeDefined();

    const detail = await getWmsWave(mockTenantId, target.wave_id);
    const result = await shipWmsWave(mockTenantId, target.wave_id, {
      continue_on_error: true,
    });

    expect(result.wave_id).toBe(target.wave_id);
    expect(result.total).toBe(detail.docs.length);
    expect(result.success + result.failed).toBe(result.total);
    expect(result.items.length).toBe(result.total);

    const updated = await getWmsWave(mockTenantId, target.wave_id);
    updated.docs.forEach((doc) => {
      expect(doc.status).toBe('SHIPPED');
    });
  });
});
