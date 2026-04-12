import { useEffect, useMemo } from 'react';
import type { ReactNode, Ref } from 'react';

import { useWorkbenchPanelsOptional } from '@/app/layout/workbenchPanels';

import { UiModule02WorkbenchTemplate } from './UiModule02WorkbenchTemplate';

type UiModule02WorkbenchTableStackProps = {
  className?: string;
  toolbar?: ReactNode;
  toolbarLeft?: ReactNode;
  toolbarCenter?: ReactNode;
  toolbarRight?: ReactNode;
  header?: ReactNode;
  actionBar?: ReactNode;
  tableContainerRef?: Ref<HTMLDivElement>;
  tableClassName?: string;
  tableContent: ReactNode;
  tableFooter?: ReactNode;
  bottomPanel?: ReactNode;
  bottomPanelVisible?: boolean;
};

export const UiModule02WorkbenchTableStack = ({
  bottomPanel,
  bottomPanelVisible = Boolean(bottomPanel),
  ...templateProps
}: UiModule02WorkbenchTableStackProps) => {
  const workbenchPanels = useWorkbenchPanelsOptional();
  const wrappedBottomPanel = useMemo(
    () =>
      bottomPanel ? (
        <div className="order-workbench-bottom-panel">{bottomPanel}</div>
      ) : undefined,
    [bottomPanel]
  );

  useEffect(() => {
    if (!workbenchPanels) {
      return;
    }
    if (!wrappedBottomPanel) {
      workbenchPanels.setBottomPanel(undefined);
      workbenchPanels.setBottomPanelVisible(false);
      return;
    }
    workbenchPanels.setBottomPanel(wrappedBottomPanel);
    workbenchPanels.setBottomPanelVisible(bottomPanelVisible);
    return () => {
      workbenchPanels.setBottomPanel(undefined);
      workbenchPanels.setBottomPanelVisible(false);
    };
  }, [bottomPanelVisible, workbenchPanels, wrappedBottomPanel]);

  return <UiModule02WorkbenchTemplate {...templateProps} />;
};
