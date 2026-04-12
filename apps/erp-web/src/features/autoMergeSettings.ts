import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

export type AutoMergeSettingsState = {
  enabled: boolean;
  setEnabled: (enabled: boolean) => void;
};

const defaultState = {
  enabled: false
};

export const useAutoMergeSettingsStore = create<AutoMergeSettingsState>()(
  persist(
    (set) => ({
      ...defaultState,
      setEnabled: (enabled) => set({ enabled })
    }),
    {
      name: 'erp.auto-merge-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ enabled: state.enabled })
    }
  )
);
