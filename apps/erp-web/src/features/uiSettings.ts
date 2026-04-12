import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

export type ThemeMode = 'light' | 'dark';

export type UiSettingsState = {
  themeMode: ThemeMode;
  denseMode: boolean;
  setThemeMode: (mode: ThemeMode) => void;
  setDenseMode: (denseMode: boolean) => void;
  toggleDenseMode: () => void;
};

const baseState = {
  themeMode: 'light' as ThemeMode,
  denseMode: false
};

export const useUiSettingsStore = create<UiSettingsState>()(
  persist(
    (set) => ({
      ...baseState,
      setThemeMode: (themeMode) => set({ themeMode }),
      setDenseMode: (denseMode) => set({ denseMode }),
      toggleDenseMode: () => set((state) => ({ denseMode: !state.denseMode }))
    }),
    {
      name: 'erp.ui-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ themeMode: state.themeMode, denseMode: state.denseMode })
    }
  )
);
