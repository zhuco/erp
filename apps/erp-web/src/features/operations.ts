import type { Capability, CapabilityMatrix } from '@/features/capabilities';
import { getCapabilityDecision, useCapability } from '@/features/capabilities';
import type { FeatureFlagKey, FeatureFlags } from '@/features/featureFlags';
import { getFeatureFlagDecision, useFeatureFlag } from '@/features/featureFlags';
import type { PlatformCode } from '@/features/platform';
import type { Permission } from '@/features/rbac';
import { hasPermission } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';

export type OperationKey = 'orders.ship' | 'orders.print_waybill';

type OperationConfig = {
  permissions: Permission[];
  feature: FeatureFlagKey;
  capability: Capability;
};

const operationConfig: Record<OperationKey, OperationConfig> = {
  'orders.ship': {
    permissions: ['orders.write', 'shipping.write'],
    feature: 'orders.ship',
    capability: 'ship'
  },
  'orders.print_waybill': {
    permissions: ['print_jobs.write'],
    feature: 'orders.print_waybill',
    capability: 'print_waybill'
  }
};

export type OperationDecision = {
  allowed: boolean;
  reason?: string;
};

export const evaluateOperation = (
  operation: OperationKey,
  platform: PlatformCode,
  options: {
    permissions: Permission[];
    featureFlags: FeatureFlags;
    capabilityMatrix: CapabilityMatrix;
  }
): OperationDecision => {
  const config = operationConfig[operation];
  const missingPermission = config.permissions.find(
    (permission) => !hasPermission(options.permissions, permission)
  );
  if (missingPermission) {
    return { allowed: false, reason: 'permission denied' };
  }
  const featureDecision = getFeatureFlagDecision(options.featureFlags, config.feature);
  if (!featureDecision.enabled) {
    return { allowed: false, reason: featureDecision.reason };
  }
  const capabilityDecision = getCapabilityDecision(
    options.capabilityMatrix,
    platform,
    config.capability
  );
  if (!capabilityDecision.enabled) {
    return { allowed: false, reason: capabilityDecision.reason };
  }
  return { allowed: true };
};

export const useOperationAccess = (
  operation: OperationKey,
  platform: PlatformCode
): OperationDecision => {
  const permissions = useSessionStore((state) => state.permissions);
  const config = operationConfig[operation];
  const featureDecision = useFeatureFlag(config.feature);
  const capabilityDecision = useCapability(platform, config.capability);
  const missingPermission = config.permissions.find(
    (permission) => !hasPermission(permissions, permission)
  );
  if (missingPermission) {
    return { allowed: false, reason: 'permission denied' };
  }
  if (!featureDecision.enabled) {
    return { allowed: false, reason: featureDecision.reason };
  }
  if (!capabilityDecision.enabled) {
    return { allowed: false, reason: capabilityDecision.reason };
  }
  return { allowed: true };
};
