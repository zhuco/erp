import { fetchWithAuth } from '@/api/client';
import type { PlatformCode } from '@/features/platform';
import { platformValues } from '@/features/platform';
import { useSessionStore } from '@/features/sessionStore';
import { apiBaseUrl } from '@/utils/env';

export type Capability = 'ship' | 'print_waybill' | 'refund_sync' | 'template_fetch';

export type CapabilityMatrix = Record<PlatformCode, Partial<Record<Capability, boolean>>>;

export const defaultCapabilityMatrix: CapabilityMatrix = {
  taobao: {
    ship: true,
    print_waybill: true,
    refund_sync: true,
    template_fetch: true
  },
  jd: {
    ship: true,
    print_waybill: true,
    refund_sync: true,
    template_fetch: true
  },
  pdd: {
    ship: true,
    print_waybill: true,
    refund_sync: true,
    template_fetch: true
  },
  douyin: {
    ship: true,
    print_waybill: true,
    refund_sync: true,
    template_fetch: true
  }
};

const normalizeCapabilityMatrix = (
  matrix?: Partial<CapabilityMatrix>
): CapabilityMatrix =>
  platformValues.reduce<CapabilityMatrix>((acc, platform) => {
    acc[platform] = {
      ...defaultCapabilityMatrix[platform],
      ...(matrix?.[platform] ?? {})
    };
    return acc;
  }, {} as CapabilityMatrix);

export const fetchCapabilityMatrix = async (tenantId: string): Promise<CapabilityMatrix> => {
  try {
    // TODO: replace with backend /feature_flags/capabilities API once available.
    const response = await fetchWithAuth(`${apiBaseUrl}/v1/tenants/${tenantId}/capabilities`);
    if (!response.ok) {
      throw new Error('fetch capability matrix failed');
    }
    const data = (await response.json()) as Partial<CapabilityMatrix>;
    return normalizeCapabilityMatrix(data);
  } catch {
    return defaultCapabilityMatrix;
  }
};

export type CapabilityDecision = {
  enabled: boolean;
  reason?: string;
};

export const getCapabilityDecision = (
  matrix: CapabilityMatrix,
  platform: PlatformCode,
  capability: Capability
): CapabilityDecision => {
  const enabled = Boolean(matrix[platform]?.[capability]);
  return enabled ? { enabled } : { enabled, reason: '平台不支持' };
};

export const hasCapability = (
  matrix: CapabilityMatrix,
  platform: PlatformCode,
  capability: Capability
): boolean => Boolean(matrix[platform]?.[capability]);

export const useCapability = (
  platform: PlatformCode,
  capability: Capability
): CapabilityDecision => {
  const capabilityMatrix = useSessionStore((state) => state.capabilityMatrix);
  return getCapabilityDecision(capabilityMatrix, platform, capability);
};
