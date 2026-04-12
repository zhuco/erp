import { useEffect } from 'react';
import type { ReactNode } from 'react';

import { fetchCapabilityMatrix } from '@/features/capabilities';
import { fetchFeatureFlags } from '@/features/featureFlags';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';

export const FeatureGateProvider = ({ children }: { children: ReactNode }) => {
  const tenantId = useTenantId();
  const setFeatureFlags = useSessionStore((state) => state.setFeatureFlags);
  const setCapabilityMatrix = useSessionStore((state) => state.setCapabilityMatrix);

  useEffect(() => {
    let active = true;
    const load = async () => {
      if (!tenantId) {
        return;
      }
      const flags = await fetchFeatureFlags(tenantId);
      if (active) {
        setFeatureFlags(flags);
      }
    };
    void load();
    return () => {
      active = false;
    };
  }, [tenantId, setFeatureFlags]);

  useEffect(() => {
    let active = true;
    const load = async () => {
      if (!tenantId) {
        return;
      }
      const matrix = await fetchCapabilityMatrix(tenantId);
      if (active) {
        setCapabilityMatrix(matrix);
      }
    };
    void load();
    return () => {
      active = false;
    };
  }, [tenantId, setCapabilityMatrix]);

  return <>{children}</>;
};
