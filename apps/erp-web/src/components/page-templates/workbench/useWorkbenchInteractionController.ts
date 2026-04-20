import { useCallback, useEffect, useState } from 'react';

import type { WorkbenchContextMenuState, WorkbenchKeyboardShortcut } from './contracts';

type EditableTargetLike = {
  tagName?: string;
  isContentEditable?: boolean;
  closest?: (selector: string) => unknown;
};

type UseWorkbenchInteractionControllerOptions = {
  keyboardShortcuts?: WorkbenchKeyboardShortcut[];
  onAnnounce?: (message: string) => void;
};

const normalizeKey = (value: string) => value.trim().toLowerCase();

export const isEditableWorkbenchTarget = (target: unknown) => {
  if (!target || typeof target !== "object") {
    return false;
  }

  const candidate = target as EditableTargetLike;
  const tagName = String(candidate.tagName ?? '').toUpperCase();
  return (
    candidate.isContentEditable === true ||
    tagName === 'INPUT' ||
    tagName === 'TEXTAREA' ||
    tagName === 'SELECT' ||
    Boolean(candidate.closest?.('.ant-select-dropdown, .ant-picker-dropdown'))
  );
};

export const matchWorkbenchShortcut = (
  shortcut: Pick<
    WorkbenchKeyboardShortcut,
    'key' | 'altKey' | 'ctrlKey' | 'shiftKey' | 'metaKey'
  >,
  event: Pick<KeyboardEvent, 'key' | 'altKey' | 'ctrlKey' | 'shiftKey' | 'metaKey'>,
) =>
  normalizeKey(shortcut.key) === normalizeKey(String(event.key ?? '')) &&
  Boolean(shortcut.altKey) === Boolean(event.altKey) &&
  Boolean(shortcut.ctrlKey) === Boolean(event.ctrlKey) &&
  Boolean(shortcut.shiftKey) === Boolean(event.shiftKey) &&
  Boolean(shortcut.metaKey) === Boolean(event.metaKey);

export const createWorkbenchContextMenuState = <TRow,>(
  row: TRow | null = null,
): WorkbenchContextMenuState<TRow> => ({
  open: false,
  position: null,
  row,
});

export const useWorkbenchInteractionController = <
  TRow,
>({
  keyboardShortcuts,
  onAnnounce,
}: UseWorkbenchInteractionControllerOptions = {}) => {
  const [contextMenuState, setContextMenuState] = useState<WorkbenchContextMenuState<TRow>>(
    () => createWorkbenchContextMenuState<TRow>(),
  );

  const announce = useCallback(
    (message: string) => {
      onAnnounce?.(message);
    },
    [onAnnounce],
  );

  const openContextMenu = useCallback((row: TRow, event: MouseEvent) => {
    setContextMenuState({
      open: true,
      position: { x: event.clientX, y: event.clientY },
      row,
    });
  }, []);

  const closeContextMenu = useCallback(() => {
    setContextMenuState(createWorkbenchContextMenuState<TRow>());
  }, []);

  useEffect(() => {
    if (!keyboardShortcuts?.length || typeof window === 'undefined') {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (isEditableWorkbenchTarget(event.target)) {
        return;
      }

      const matchedShortcut = keyboardShortcuts.find((shortcut) => {
        if (!matchWorkbenchShortcut(shortcut, event)) {
          return false;
        }
        return shortcut.when ? shortcut.when(event) : true;
      });

      if (!matchedShortcut) {
        return;
      }

      if (matchedShortcut.preventDefault !== false) {
        event.preventDefault();
      }
      matchedShortcut.handler(event);
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [keyboardShortcuts]);

  return {
    announce,
    contextMenuState,
    openContextMenu,
    closeContextMenu,
    setContextMenuState,
  };
};
