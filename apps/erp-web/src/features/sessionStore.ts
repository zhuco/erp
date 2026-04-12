import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { defaultCapabilityMatrix, type CapabilityMatrix } from '@/features/capabilities';
import { defaultFeatureFlags, type FeatureFlags } from '@/features/featureFlags';
import { defaultPermissions, type Permission } from '@/features/rbac';

export type SessionState = {
  selectedTenantId: string;
  selectedShopIds: string[];
  selectedWarehouseId?: string;
  contextExpiresAt?: number;
  token?: string;
  userId: string;
  userName: string;
  permissions: Permission[];
  featureFlags: FeatureFlags;
  capabilityMatrix: CapabilityMatrix;
  setSelectedTenantId: (tenantId: string) => void;
  setSelectedShopIds: (shopIds: string[]) => void;
  setSelectedWarehouseId: (warehouseId?: string) => void;
  setToken: (token?: string) => void;
  setUserId: (userId: string) => void;
  setUserName: (userName: string) => void;
  setPermissions: (permissions: Permission[]) => void;
  setFeatureFlags: (flags: FeatureFlags) => void;
  setCapabilityMatrix: (matrix: CapabilityMatrix) => void;
  clearContext: () => void;
  resetSession: () => void;
};

const CONTEXT_TTL_MS = 1000 * 60 * 60 * 24 * 7;
const buildContextExpiry = () => Date.now() + CONTEXT_TTL_MS;

const baseState = {
  selectedTenantId: '',
  selectedShopIds: [],
  selectedWarehouseId: undefined,
  contextExpiresAt: undefined,
  token: undefined,
  userId: '',
  userName: '衡策云操作员',
  permissions: defaultPermissions,
  featureFlags: defaultFeatureFlags,
  capabilityMatrix: defaultCapabilityMatrix
};

export const useSessionStore = create<SessionState>()(
  persist(
    (set) => ({
      ...baseState,
      setSelectedTenantId: (selectedTenantId) =>
        set({ selectedTenantId, contextExpiresAt: buildContextExpiry() }),
      setSelectedShopIds: (selectedShopIds) =>
        set({ selectedShopIds, contextExpiresAt: buildContextExpiry() }),
      setSelectedWarehouseId: (selectedWarehouseId) =>
        set({ selectedWarehouseId, contextExpiresAt: buildContextExpiry() }),
      setToken: (token) => set({ token }),
      setUserId: (userId) => set({ userId }),
      setUserName: (userName) => set({ userName }),
      setPermissions: (permissions) => set({ permissions }),
      setFeatureFlags: (featureFlags) => set({ featureFlags }),
      setCapabilityMatrix: (capabilityMatrix) => set({ capabilityMatrix }),
      clearContext: () =>
        set({
          selectedTenantId: '',
          selectedShopIds: [],
          selectedWarehouseId: undefined,
          contextExpiresAt: undefined
        }),
      resetSession: () =>
        set((state) => {
          const contextStillValid =
            typeof state.contextExpiresAt === 'number' && state.contextExpiresAt > Date.now();
          return {
            ...baseState,
            selectedTenantId: contextStillValid ? state.selectedTenantId : '',
            selectedShopIds: contextStillValid ? state.selectedShopIds : [],
            selectedWarehouseId: contextStillValid ? state.selectedWarehouseId : undefined,
            contextExpiresAt: contextStillValid ? state.contextExpiresAt : undefined
          };
        })
    }),
    {
      name: 'erp.session',
      storage: createJSONStorage(() => localStorage),
      version: 2,
      migrate: (persistedState) => {
        const state = persistedState as Partial<SessionState> & {
          tenantId?: string;
          shopId?: string;
          warehouseId?: string;
        };
        return {
          ...baseState,
          ...state,
          selectedTenantId: state.selectedTenantId ?? state.tenantId ?? '',
          selectedShopIds: Array.isArray(state.selectedShopIds)
            ? state.selectedShopIds
            : state.shopId
              ? [state.shopId]
              : baseState.selectedShopIds,
          selectedWarehouseId: state.selectedWarehouseId ?? state.warehouseId,
          contextExpiresAt: state.contextExpiresAt
        };
      },
      partialize: (state) => ({
        selectedTenantId: state.selectedTenantId,
        selectedShopIds: state.selectedShopIds,
        selectedWarehouseId: state.selectedWarehouseId,
        contextExpiresAt: state.contextExpiresAt,
        token: state.token,
        userId: state.userId,
        userName: state.userName,
        permissions: state.permissions,
        featureFlags: state.featureFlags,
        capabilityMatrix: state.capabilityMatrix
      }),
      onRehydrateStorage: () => (state) => {
        if (!state?.contextExpiresAt) {
          return;
        }
        if (state.contextExpiresAt < Date.now()) {
          state.clearContext();
        }
      }
    }
  )
);
