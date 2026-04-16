import type { HeaderModuleItem } from './headerMenuModel';

export const SHARED_SUBMENU_VERTICAL_GAP = 4;

export type SharedAnchorRect = {
  left: number;
  right: number;
  top: number;
  bottom: number;
  width: number;
  height: number;
};

export type SharedPanelToggleResult = {
  nextOpenModuleKey: string | null;
  navigateTo?: string;
};

export const resolveSharedAnchorRect = (
  anchorRect: SharedAnchorRect | null,
  verticalGap = SHARED_SUBMENU_VERTICAL_GAP,
) => {
  if (!anchorRect) {
    return null;
  }

  return {
    ...anchorRect,
    bottom: anchorRect.bottom + verticalGap,
  };
};

export const resolveSharedPanelToggle = (
  currentOpenModuleKey: string | null,
  module: HeaderModuleItem,
): SharedPanelToggleResult => {
  if (module.groups.length === 0) {
    return {
      nextOpenModuleKey: null,
      navigateTo: module.defaultPath,
    };
  }

  return {
    nextOpenModuleKey: currentOpenModuleKey === module.key ? null : module.key,
  };
};

export const resolveSharedPanelGeometry = (anchorRect: SharedAnchorRect | null) => {
  if (!anchorRect) {
    return null;
  }

  return {
    left: anchorRect.left,
    right: anchorRect.right,
    top: anchorRect.bottom,
    width: anchorRect.width,
  };
};

export const resolveSharedPanelMaxHeight = (
  viewportHeight: number,
  panelTop: number,
  viewportBottomGap = 24,
) => {
  return Math.max(0, viewportHeight - panelTop - viewportBottomGap);
};

export const resolveSharedPanelVisibleHeight = (contentHeight: number, maxHeight: number) => {
  return Math.max(0, Math.min(contentHeight, maxHeight));
};
