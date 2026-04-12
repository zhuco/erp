import { create } from 'zustand';

export type AppTab = {
  key: string;
  path: string;
  title: string;
  closable: boolean;
};

type TabsState = {
  tabs: AppTab[];
  activeKey?: string;
  refreshTokens: Record<string, number>;
  setTabs: (tabs: AppTab[], activeKey?: string) => void;
  setActiveKey: (activeKey?: string) => void;
  addOrUpdateTab: (tab: AppTab) => void;
  closeTab: (key: string) => string | undefined;
  closeOtherTabs: (key: string) => string | undefined;
  closeAllTabs: () => void;
  refreshTab: (key: string) => void;
};

export const useTabsStore = create<TabsState>((set, get) => ({
  tabs: [],
  activeKey: undefined,
  refreshTokens: {},
  setTabs: (tabs, activeKey) => set({ tabs, activeKey }),
  setActiveKey: (activeKey) => set({ activeKey }),
  addOrUpdateTab: (tab) => {
    const { tabs } = get();
    const existingIndex = tabs.findIndex((item) => item.key === tab.key);
    if (existingIndex >= 0) {
      const nextTabs = [...tabs];
      nextTabs[existingIndex] = { ...nextTabs[existingIndex], ...tab };
      set({ tabs: nextTabs, activeKey: tab.key });
      return;
    }
    set({ tabs: [...tabs, tab], activeKey: tab.key });
  },
  closeTab: (key) => {
    const { tabs, activeKey } = get();
    const removeIndex = tabs.findIndex((item) => item.key === key);
    const nextTabs = tabs.filter((item) => item.key !== key);
    let nextActiveKey = activeKey;
    if (activeKey === key) {
      if (nextTabs.length === 0) {
        nextActiveKey = undefined;
      } else if (removeIndex >= 0 && removeIndex < nextTabs.length) {
        nextActiveKey = nextTabs[removeIndex]?.key;
      } else {
        nextActiveKey = nextTabs[nextTabs.length - 1]?.key;
      }
    }
    set({ tabs: nextTabs, activeKey: nextActiveKey });
    return nextActiveKey;
  },
  closeOtherTabs: (key) => {
    const { tabs } = get();
    const nextTabs = tabs.filter((item) => item.key === key);
    const nextActiveKey = nextTabs[0]?.key;
    set({ tabs: nextTabs, activeKey: nextActiveKey });
    return nextActiveKey;
  },
  closeAllTabs: () => {
    set({ tabs: [], activeKey: undefined });
  },
  refreshTab: (key) => {
    set((state) => ({
      refreshTokens: {
        ...state.refreshTokens,
        [key]: (state.refreshTokens[key] ?? 0) + 1
      }
    }));
  }
}));
