import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

export type FastModeSettingsState = {
  enabled: boolean;
  setEnabled: (enabled: boolean) => void;
};

const defaultState = {
  enabled: false
};

export const useFastModeSettingsStore = create<FastModeSettingsState>()(
  persist(
    (set) => ({
      ...defaultState,
      setEnabled: (enabled) => set({ enabled })
    }),
    {
      name: 'erp.fast-mode-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ enabled: state.enabled })
    }
  )
);
