import { Outlet } from 'react-router-dom';
import { useEffect, useMemo } from 'react';

import { TenantSelectionModal } from '@/components/TenantSelectionModal';
import { isTenantSelected, useTenant } from '@/features/tenant';
import { tenantOptions } from '@/features/tenantOptions';

export const TenantGuard = () => {
  const { tenantId, setTenantId } = useTenant();
  const availableTenantIds = useMemo(
    () => new Set(tenantOptions.map((tenant) => tenant.value)),
    []
  );
  const tenantSelected = isTenantSelected(tenantId);
  const tenantReady = tenantSelected && availableTenantIds.has(tenantId);

  useEffect(() => {
    if (tenantSelected && !availableTenantIds.has(tenantId)) {
      setTenantId('');
    }
  }, [availableTenantIds, setTenantId, tenantId, tenantSelected]);

  useEffect(() => {
    if (!tenantReady && tenantOptions.length === 1) {
      setTenantId(tenantOptions[0].value);
    }
  }, [tenantReady, setTenantId]);

  const shouldPromptTenant = !tenantReady && tenantOptions.length !== 1;

  return (
    <>
      {tenantReady ? <Outlet /> : null}
      <TenantSelectionModal
        open={shouldPromptTenant}
        options={tenantOptions}
        selectedTenantId={tenantId}
        onConfirm={(nextTenantId) => setTenantId(nextTenantId)}
      />
    </>
  );
};
