import { useEffect, useMemo } from 'react';
import type { ReactNode } from 'react';

import { useWorkbenchPanelsOptional } from '@/app/layout/workbenchPanels';

import type { WorkbenchPanelVisibilityConfig } from './contracts';

export const resolveWorkbenchPanelVisible = (
  panel: ReactNode | undefined,
  visible?: boolean,
) => Boolean(panel) && (typeof visible === 'boolean' ? visible : true);

export const buildWorkbenchLayoutState = ({
  leftPanel,
  leftPanelVisible,
  rightPanel,
  rightPanelVisible,
  bottomPanel,
  bottomPanelVisible,
}: Pick<
  WorkbenchPanelVisibilityConfig,
  | 'leftPanel'
  | 'leftPanelVisible'
  | 'rightPanel'
  | 'rightPanelVisible'
  | 'bottomPanel'
  | 'bottomPanelVisible'
>) => ({
  left: { panel: leftPanel, visible: resolveWorkbenchPanelVisible(leftPanel, leftPanelVisible) },
  right: { panel: rightPanel, visible: resolveWorkbenchPanelVisible(rightPanel, rightPanelVisible) },
  bottom: {
    panel: bottomPanel,
    visible: resolveWorkbenchPanelVisible(bottomPanel, bottomPanelVisible),
  },
});

export const useWorkbenchLayoutController = ({
  leftPanel,
  leftPanelVisible,
  leftPanelSyncKey,
  leftPanelWidth,
  rightPanel,
  rightPanelVisible,
  rightPanelSyncKey,
  rightPanelWidth,
  bottomPanel,
  bottomPanelVisible,
  bottomPanelSyncKey,
  bottomPanelHeight,
}: WorkbenchPanelVisibilityConfig) => {
  const workbenchPanels = useWorkbenchPanelsOptional();
  const setLeftPanel = workbenchPanels?.setLeftPanel;
  const setRightPanel = workbenchPanels?.setRightPanel;
  const setBottomPanel = workbenchPanels?.setBottomPanel;
  const setLeftPanelWidth = workbenchPanels?.setLeftPanelWidth;
  const setRightPanelWidth = workbenchPanels?.setRightPanelWidth;
  const setBottomPanelHeight = workbenchPanels?.setBottomPanelHeight;
  const setLeftPanelCollapsed = workbenchPanels?.setLeftPanelCollapsed;
  const setLeftPanelVisible = workbenchPanels?.setLeftPanelVisible;
  const setRightPanelVisible = workbenchPanels?.setRightPanelVisible;
  const setBottomPanelVisible = workbenchPanels?.setBottomPanelVisible;

  const layoutState = useMemo(
    () =>
      buildWorkbenchLayoutState({
        leftPanel,
        leftPanelVisible,
        rightPanel,
        rightPanelVisible,
        bottomPanel,
        bottomPanelVisible,
      }),
    [bottomPanel, bottomPanelVisible, leftPanel, leftPanelVisible, rightPanel, rightPanelVisible],
  );

  useEffect(() => {
    if (!setLeftPanel || !setLeftPanelVisible) {
      return;
    }
    setLeftPanelCollapsed?.(false);
    setLeftPanel(layoutState.left.visible ? layoutState.left.panel : undefined);
    setLeftPanelVisible(layoutState.left.visible);
    return () => {
      setLeftPanel(undefined);
      setLeftPanelVisible(false);
    };
  }, [
    layoutState.left.panel,
    layoutState.left.visible,
    leftPanelSyncKey,
    setLeftPanel,
    setLeftPanelCollapsed,
    setLeftPanelVisible,
  ]);

  useEffect(() => {
    if (!setRightPanel || !setRightPanelVisible) {
      return;
    }
    setRightPanel(layoutState.right.visible ? layoutState.right.panel : undefined);
    setRightPanelVisible(layoutState.right.visible);
    return () => {
      setRightPanel(undefined);
      setRightPanelVisible(false);
    };
  }, [
    layoutState.right.panel,
    layoutState.right.visible,
    rightPanelSyncKey,
    setRightPanel,
    setRightPanelVisible,
  ]);

  useEffect(() => {
    if (!setBottomPanel || !setBottomPanelVisible) {
      return;
    }
    setBottomPanel(layoutState.bottom.visible ? layoutState.bottom.panel : undefined);
    setBottomPanelVisible(layoutState.bottom.visible);
    return () => {
      setBottomPanel(undefined);
      setBottomPanelVisible(false);
    };
  }, [
    bottomPanelSyncKey,
    layoutState.bottom.panel,
    layoutState.bottom.visible,
    setBottomPanel,
    setBottomPanelVisible,
  ]);

  useEffect(() => {
    if (!setLeftPanelWidth || leftPanelWidth == null) {
      return;
    }
    setLeftPanelWidth(leftPanelWidth);
  }, [leftPanelWidth, setLeftPanelWidth]);

  useEffect(() => {
    if (!setRightPanelWidth || rightPanelWidth == null) {
      return;
    }
    setRightPanelWidth(rightPanelWidth);
  }, [rightPanelWidth, setRightPanelWidth]);

  useEffect(() => {
    if (!setBottomPanelHeight || bottomPanelHeight == null) {
      return;
    }
    setBottomPanelHeight(bottomPanelHeight);
  }, [bottomPanelHeight, setBottomPanelHeight]);

  return layoutState;
};
