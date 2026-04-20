import type { ReactNode } from 'react';

export type WorkbenchPanelVisibilityConfig = {
  leftPanel?: ReactNode;
  leftPanelVisible?: boolean;
  leftPanelSyncKey?: string | number;
  leftPanelWidth?: number;
  rightPanel?: ReactNode;
  rightPanelVisible?: boolean;
  rightPanelSyncKey?: string | number;
  rightPanelWidth?: number;
  bottomPanel?: ReactNode;
  bottomPanelVisible?: boolean;
  bottomPanelSyncKey?: string | number;
  bottomPanelHeight?: number;
};

export type WorkbenchKeyboardShortcut = {
  id: string;
  key: string;
  altKey?: boolean;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  metaKey?: boolean;
  preventDefault?: boolean;
  when?: (event: KeyboardEvent) => boolean;
  handler: (event: KeyboardEvent) => void;
};

export type WorkbenchContextMenuState<TRow = unknown> = {
  open: boolean;
  position: { x: number; y: number } | null;
  row: TRow | null;
};

export type WorkbenchSelectionState<TKey extends string = string> = {
  selectedRowKeys: TKey[];
  focusedRowKey: TKey | null;
  activeRowKey: TKey | null;
  selectionAnchorKey: TKey | null;
};
