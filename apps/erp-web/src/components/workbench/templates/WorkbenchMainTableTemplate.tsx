import type { TablePaginationConfig } from 'antd';
import type { MutableRefObject, ReactNode, Ref } from 'react';
import { useCallback, useEffect, useMemo, useRef } from 'react';

import { useWorkbenchPanelsOptional } from '@/app/layout/workbenchPanels';
import { UiModule03TableFooter } from '@/components/ui-modules';

import { WorkbenchPageTemplate } from './WorkbenchPageTemplate';

const hasOwn = Object.prototype.hasOwnProperty;

type WorkbenchMainTableTemplateProps = {
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
  bottomPanel?: ReactNode;
  bottomPanelVisible?: boolean;
  leftPanelWidth?: number;
  rightPanelWidth?: number;
  bottomPanelHeight?: number;
};

export const WorkbenchMainTableTemplate = (props: WorkbenchMainTableTemplateProps) => {
  const {
    className,
    toolbar,
    toolbarLeft,
    toolbarCenter,
    toolbarRight,
    header,
    actionBar,
    tableContainerRef,
    tableClassName,
    tableContent,
    tableFooter,
    footerSummary,
    footerSyncKey,
    total,
    pagination,
    onPaginationChange,
    showFooterScrollbarRow = false,
    bottomPanel,
    bottomPanelVisible = Boolean(bottomPanel),
    leftPanelWidth,
    rightPanelWidth,
    bottomPanelHeight,
  } = props;
  const workbenchPanels = useWorkbenchPanelsOptional();
  const internalTableContainerRef = useRef<HTMLDivElement | null>(null);
  const shouldSyncBottomPanel =
    hasOwn.call(props, 'bottomPanel') ||
    hasOwn.call(props, 'bottomPanelVisible') ||
    hasOwn.call(props, 'bottomPanelHeight');

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

  const wrappedBottomPanel = useMemo(
    () =>
      bottomPanel ? (
        <div className="order-workbench-bottom-panel">{bottomPanel}</div>
      ) : undefined,
    [bottomPanel],
  );

  useEffect(() => {
    if (!workbenchPanels || !shouldSyncBottomPanel) {
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
  }, [bottomPanelVisible, shouldSyncBottomPanel, workbenchPanels, wrappedBottomPanel]);

  useEffect(() => {
    if (!workbenchPanels || !leftPanelWidth) {
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

  const resolvedToolbar = toolbar ? (
    toolbar
  ) : (
    <div className="order-workbench-toolbar">
      {toolbarLeft ? <div className="order-workbench-toolbar-left">{toolbarLeft}</div> : null}
      {toolbarCenter ? <div className="order-workbench-toolbar-center">{toolbarCenter}</div> : null}
      {toolbarRight ? <div className="order-workbench-toolbar-right">{toolbarRight}</div> : null}
    </div>
  );

  return (
    <WorkbenchPageTemplate
      className={className}
      toolbar={resolvedToolbar}
      mainContent={
        <>
          {header ? <div className="order-workbench-header">{header}</div> : null}
          <div className={resolvedTableClassName} ref={handleTableContainerRef}>
            {tableContent}
            {resolvedTableFooter}
          </div>
          {actionBar ? <div className="order-workbench-action">{actionBar}</div> : null}
        </>
      }
    />
  );
};
