import type { ReactNode } from 'react';
import { createContext, useContext } from 'react';

export type WorkbenchPanelsContextValue = {
  leftPanel?: ReactNode;
  rightPanel?: ReactNode;
  bottomPanel?: ReactNode;
  leftPanelWidth: number;
  rightPanelWidth: number;
  bottomPanelHeight: number;
  leftPanelVisible: boolean;
  rightPanelVisible: boolean;
  bottomPanelVisible: boolean;
  setLeftPanel: (panel?: ReactNode) => void;
  setRightPanel: (panel?: ReactNode) => void;
  setBottomPanel: (panel?: ReactNode) => void;
  setLeftPanelWidth: (width: number) => void;
  setRightPanelWidth: (width: number) => void;
  setBottomPanelHeight: (height: number) => void;
  setLeftPanelCollapsed: (collapsed: boolean) => void;
  setLeftPanelVisible: (visible: boolean) => void;
  setRightPanelVisible: (visible: boolean) => void;
  setBottomPanelVisible: (visible: boolean) => void;
  clearPanels: () => void;
};

const WorkbenchPanelsContext = createContext<WorkbenchPanelsContextValue | null>(null);

export const WorkbenchPanelsProvider = WorkbenchPanelsContext.Provider;

export const useWorkbenchPanelsOptional = () => useContext(WorkbenchPanelsContext);

export const useWorkbenchPanels = () => {
  const context = useWorkbenchPanelsOptional();
  if (!context) {
    throw new Error('useWorkbenchPanels must be used within WorkbenchPanelsProvider');
  }
  return context;
};
