import './OrderReviewWorkbenchTemplate.css';

import type { TablePaginationConfig } from 'antd';
import type { MutableRefObject, ReactNode, Ref } from 'react';
import { useCallback, useEffect, useMemo, useRef } from 'react';

import { useWorkbenchPanelsOptional } from '@/app/layout/workbenchPanels';
import { UiModule03TableFooter } from '@/components/ui-modules';

export type OrderReviewWorkbenchTemplateProps = {
  pageTestId?: string;
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
  footerSummary?: ReactNode;
  footerSyncKey?: string | number;
  total?: number;
  pagination?: TablePaginationConfig;
  onPaginationChange?: (pagination: TablePaginationConfig) => void;
  showFooterScrollbarRow?: boolean;
  rightPanel?: ReactNode;
  rightPanelVisible?: boolean;
  rightPanelSyncKey?: string | number;
  bottomPanel?: ReactNode;
  bottomPanelVisible?: boolean;
  bottomPanelSyncKey?: string | number;
  leftPanelWidth?: number;
  rightPanelWidth?: number;
  bottomPanelHeight?: number;
  overlays?: ReactNode;
};

const pageShellStyle = {
  display: 'flex',
  flexDirection: 'column',
  flex: 1,
  minHeight: 0,
  width: '100%',
  maxWidth: '100%',
  height: '100%',
} as const;

export const OrderReviewWorkbenchTemplate = ({
  pageTestId = 'order-review-page',
  className = 'order-review-page',
  toolbar,
  toolbarLeft,
  toolbarCenter,
  toolbarRight,
  header,
  actionBar,
  tableContainerRef,
  tableClassName = 'order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar',
  tableContent,
  tableFooter,
  footerSummary,
  footerSyncKey,
  total,
  pagination,
  onPaginationChange,
  showFooterScrollbarRow = true,
  rightPanel,
  rightPanelVisible = true,
  rightPanelSyncKey,
  bottomPanel,
  bottomPanelVisible,
  bottomPanelSyncKey,
  leftPanelWidth,
  rightPanelWidth,
  bottomPanelHeight,
  overlays,
}: OrderReviewWorkbenchTemplateProps) => {
  const workbenchPanels = useWorkbenchPanelsOptional();
  const internalTableContainerRef = useRef<HTMLDivElement | null>(null);

  const handleTableContainerRef = useCallback(
    (node: HTMLDivElement | null) => {
      internalTableContainerRef.current = node;
      if (typeof tableContainerRef === 'function') {
        tableContainerRef(node);
        return;
      }
      if (tableContainerRef && typeof tableContainerRef === 'object') {
        (tableContainerRef as MutableRefObject<HTMLDivElement | null>).current = node;
      }
    },
    [tableContainerRef],
  );

  const shouldShowBottomPanel = useMemo(() => {
    if (!bottomPanel) {
      return false;
    }
    if (typeof bottomPanelVisible === 'boolean') {
      return bottomPanelVisible;
    }
    return true;
  }, [bottomPanel, bottomPanelVisible]);

  const wrappedBottomPanel = useMemo(
    () =>
      shouldShowBottomPanel && bottomPanel ? (
        <div className="order-workbench-bottom-panel">{bottomPanel}</div>
      ) : undefined,
    [bottomPanel, shouldShowBottomPanel],
  );

  const hasToolbarSlots = Boolean(toolbarLeft || toolbarCenter || toolbarRight);
  const resolvedToolbar = toolbar ? (
    toolbar
  ) : hasToolbarSlots ? (
    <div className="order-workbench-toolbar">
      {toolbarLeft ? <div className="order-workbench-toolbar-left">{toolbarLeft}</div> : null}
      {toolbarCenter ? <div className="order-workbench-toolbar-center">{toolbarCenter}</div> : null}
      {toolbarRight ? <div className="order-workbench-toolbar-right">{toolbarRight}</div> : null}
    </div>
  ) : null;

  const resolvedTableFooter = useMemo<ReactNode>(() => {
    if (tableFooter !== undefined) {
      return tableFooter;
    }
    if (!pagination || !onPaginationChange || total == null) {
      return undefined;
    }
    return (
      <UiModule03TableFooter
        tableContainerRef={internalTableContainerRef}
        syncKey={footerSyncKey}
        summary={footerSummary}
        total={total}
        pagination={pagination}
        showScrollbarRow={showFooterScrollbarRow}
        onPaginationChange={onPaginationChange}
      />
    );
  }, [
    footerSummary,
    footerSyncKey,
    onPaginationChange,
    pagination,
    showFooterScrollbarRow,
    tableFooter,
    total,
  ]);

  const resolvedTableClassName = [
    'order-workbench-table',
    tableClassName,
    resolvedTableFooter ? 'order-workbench-table-with-footer' : null,
  ]
    .filter(Boolean)
    .join(' ');

  useEffect(() => {
    if (!workbenchPanels) {
      return;
    }
    const shouldShow = Boolean(rightPanel) && rightPanelVisible;
    workbenchPanels.setRightPanel(shouldShow ? rightPanel : undefined);
    workbenchPanels.setRightPanelVisible(shouldShow);
    return () => {
      workbenchPanels.setRightPanel(undefined);
      workbenchPanels.setRightPanelVisible(false);
    };
  }, [rightPanel, rightPanelSyncKey, rightPanelVisible, workbenchPanels]);

  useEffect(() => {
    if (!workbenchPanels) {
      return;
    }
    workbenchPanels.setBottomPanel(wrappedBottomPanel);
    workbenchPanels.setBottomPanelVisible(Boolean(wrappedBottomPanel));
    return () => {
      workbenchPanels.setBottomPanel(undefined);
      workbenchPanels.setBottomPanelVisible(false);
    };
  }, [bottomPanelSyncKey, workbenchPanels, wrappedBottomPanel]);

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

  return (
    <div data-testid={pageTestId} style={pageShellStyle}>
      <div className={['order-workbench-page', className].filter(Boolean).join(' ')}>
        {resolvedToolbar}
        <div className="order-workbench-body">
          <div className="order-workbench-main">
            {header ? <div className="order-workbench-header">{header}</div> : null}
            <div className={resolvedTableClassName} ref={handleTableContainerRef}>
              {tableContent}
              {resolvedTableFooter}
            </div>
            {actionBar ? <div className="order-workbench-action">{actionBar}</div> : null}
          </div>
        </div>
      </div>
      {overlays}
    </div>
  );
};
