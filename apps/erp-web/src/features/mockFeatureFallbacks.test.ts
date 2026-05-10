import { describe, expect, it, vi } from 'vitest';

vi.mock('@/utils/env', () => ({
  apiBaseUrl: 'http://localhost:8080',
  useMocks: true,
}));

vi.mock('@/api/client', () => ({
  fetchWithAuth: vi.fn(async () => {
    throw new Error('network should not be called in mock mode');
  }),
}));

import { fetchCapabilityMatrix, defaultCapabilityMatrix } from './capabilities';
import { fetchFeatureFlags, defaultFeatureFlags } from './featureFlags';

describe('mock feature fallbacks', () => {
  it('returns default feature flags without issuing backend requests in mock mode', async () => {
    await expect(fetchFeatureFlags('tenant-1')).resolves.toEqual(defaultFeatureFlags);
  });

  it('returns default capability matrix without issuing backend requests in mock mode', async () => {
    await expect(fetchCapabilityMatrix('tenant-1')).resolves.toEqual(defaultCapabilityMatrix);
  });
});
