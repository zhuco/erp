import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

export type MenuCacheSettingsState = {
  enabled: boolean;
  setEnabled: (enabled: boolean) => void;
};

const defaultState = {
  enabled: false
};

export const useMenuCacheSettingsStore = create<MenuCacheSettingsState>()(
  persist(
    (set) => ({
      ...defaultState,
      setEnabled: (enabled) => set({ enabled })
    }),
    {
      name: 'erp.menu-cache-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ enabled: state.enabled })
    }
  )
);
