import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

export type OrderAuditMode = 'auto' | 'manual';

export type OrderAuditSettingsState = {
  enabled: boolean;
  mode: OrderAuditMode;
  setEnabled: (enabled: boolean) => void;
  setMode: (mode: OrderAuditMode) => void;
};

const defaultState = {
  enabled: false,
  mode: 'manual' as OrderAuditMode
};

export const useOrderAuditSettingsStore = create<OrderAuditSettingsState>()(
  persist(
    (set) => ({
      ...defaultState,
      setEnabled: (enabled) => set({ enabled }),
      setMode: (mode) => set({ mode })
    }),
    {
      name: 'erp.order-audit-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ enabled: state.enabled, mode: state.mode })
    }
  )
);
