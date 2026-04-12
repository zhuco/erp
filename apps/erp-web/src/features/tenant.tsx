import { createContext, useCallback, useContext, useMemo } from 'react';
import type { ReactNode } from 'react';
import { z } from 'zod';

import { useSessionStore } from '@/features/sessionStore';

type TenantContextValue = {
  tenantId: string;
  shopIds: string[];
  warehouseId?: string;
  setTenantId: (tenantId: string) => void;
  setShopIds: (shopIds: string[]) => void;
  setWarehouseId: (warehouseId?: string) => void;
  clearContext: () => void;
};

const tenantIdSchema = z.string().uuid();
const TenantContext = createContext<TenantContextValue | null>(null);

export const TenantProvider = ({ children }: { children: ReactNode }) => {
  const tenantId = useSessionStore((state) => state.selectedTenantId);
  const setTenantId = useSessionStore((state) => state.setSelectedTenantId);
  const shopIds = useSessionStore((state) => state.selectedShopIds);
  const setShopIds = useSessionStore((state) => state.setSelectedShopIds);
  const warehouseId = useSessionStore((state) => state.selectedWarehouseId);
  const setWarehouseId = useSessionStore((state) => state.setSelectedWarehouseId);
  const clearContext = useSessionStore((state) => state.clearContext);

  const updateTenantId = useCallback(
    (nextTenantId: string) => {
      if (!nextTenantId) {
        clearContext();
        return;
      }
      const parsed = tenantIdSchema.safeParse(nextTenantId);
      setTenantId(parsed.success ? parsed.data : '');
    },
    [clearContext, setTenantId]
  );

  const updateShopIds = useCallback(
    (nextShopIds: string[]) => {
      const normalized = Array.from(
        new Set((nextShopIds ?? []).map((shopId) => shopId.trim()).filter(Boolean))
      );
      setShopIds(normalized);
    },
    [setShopIds]
  );

  const updateWarehouseId = useCallback(
    (nextWarehouseId?: string) => {
      const trimmed = nextWarehouseId?.trim();
      setWarehouseId(trimmed || undefined);
    },
    [setWarehouseId]
  );

  const value = useMemo(
    () => ({
      tenantId,
      shopIds,
      warehouseId,
      setTenantId: updateTenantId,
      setShopIds: updateShopIds,
      setWarehouseId: updateWarehouseId,
      clearContext
    }),
    [tenantId, shopIds, warehouseId, updateTenantId, updateShopIds, updateWarehouseId, clearContext]
  );

  return <TenantContext.Provider value={value}>{children}</TenantContext.Provider>;
};

export const useTenant = (): TenantContextValue => {
  const context = useContext(TenantContext);
  if (!context) {
    throw new Error('TenantProvider is missing');
  }
  return context;
};

export const useTenantId = (): string => useTenant().tenantId;
export const useShopIds = (): string[] => useTenant().shopIds;
export const useWarehouseId = (): string | undefined => useTenant().warehouseId;
export const isTenantSelected = (tenantId: string): boolean =>
  tenantIdSchema.safeParse(tenantId).success;

export const resolveShopId = (shopIds: string[], overrideShopId?: string): string | undefined => {
  const trimmedOverride = overrideShopId?.trim();
  if (trimmedOverride) {
    return trimmedOverride;
  }
  return shopIds.length === 1 ? shopIds[0] : undefined;
};
