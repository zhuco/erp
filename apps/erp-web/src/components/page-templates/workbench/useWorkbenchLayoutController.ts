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
    if (!workbenchPanels) {
      return;
    }
    if (typeof workbenchPanels.setLeftPanelCollapsed === 'function') {
      workbenchPanels.setLeftPanelCollapsed(false);
    }
    workbenchPanels.setLeftPanel(layoutState.left.visible ? layoutState.left.panel : undefined);
    workbenchPanels.setLeftPanelVisible(layoutState.left.visible);
    return () => {
      workbenchPanels.setLeftPanel(undefined);
      workbenchPanels.setLeftPanelVisible(false);
    };
  }, [layoutState.left.panel, layoutState.left.visible, leftPanelSyncKey, workbenchPanels]);

  useEffect(() => {
    if (!workbenchPanels) {
      return;
    }
    workbenchPanels.setRightPanel(layoutState.right.visible ? layoutState.right.panel : undefined);
    workbenchPanels.setRightPanelVisible(layoutState.right.visible);
    return () => {
      workbenchPanels.setRightPanel(undefined);
      workbenchPanels.setRightPanelVisible(false);
    };
  }, [layoutState.right.panel, layoutState.right.visible, rightPanelSyncKey, workbenchPanels]);

  useEffect(() => {
    if (!workbenchPanels) {
      return;
    }
    workbenchPanels.setBottomPanel(layoutState.bottom.visible ? layoutState.bottom.panel : undefined);
    workbenchPanels.setBottomPanelVisible(layoutState.bottom.visible);
    return () => {
      workbenchPanels.setBottomPanel(undefined);
      workbenchPanels.setBottomPanelVisible(false);
    };
  }, [bottomPanelSyncKey, layoutState.bottom.panel, layoutState.bottom.visible, workbenchPanels]);

  useEffect(() => {
    if (!workbenchPanels || leftPanelWidth == null) {
      return;
    }
    workbenchPanels.setLeftPanelWidth(leftPanelWidth);
  }, [leftPanelWidth, workbenchPanels]);

  useEffect(() => {
    if (!workbenchPanels || rightPanelWidth == null) {
      return;
    }
    workbenchPanels.setRightPanelWidth(rightPanelWidth);
  }, [rightPanelWidth, workbenchPanels]);

  useEffect(() => {
    if (!workbenchPanels || bottomPanelHeight == null) {
      return;
    }
    workbenchPanels.setBottomPanelHeight(bottomPanelHeight);
  }, [bottomPanelHeight, workbenchPanels]);

  return layoutState;
};
