import './OrderReviewWorkbenchTemplate.css';

import type { TablePaginationConfig } from 'antd';
import type { MutableRefObject, ReactNode, Ref } from 'react';
import { useCallback, useEffect, useMemo, useRef } from 'react';

import { useWorkbenchLayoutController } from '@/components/page-templates/workbench/useWorkbenchLayoutController';
import { UiModule03TableFooter } from '@/components/ui-modules';

import { readWorkbenchViewStateCache, writeWorkbenchViewStateCache } from './workbenchViewStateCache';

export type OrderReviewWorkbenchTemplateProps = {
  pageTestId?: string;
  className?: string;
  toolbar?: ReactNode;
  toolbarLeft?: ReactNode;
  toolbarCenter?: ReactNode;
  toolbarRight?: ReactNode;
  header?: ReactNode;
  actionBar?: ReactNode;
  leftPanel?: ReactNode;
  leftPanelVisible?: boolean;
  leftPanelSyncKey?: string | number;
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
  stateCacheKey?: string;
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

const verticalScrollSelectors = ['.ag-body-viewport', '.ag-center-cols-viewport', '.ant-table-body'];
const horizontalScrollSelectors = [
  '.ag-body-horizontal-scroll-viewport',
  '.ag-center-cols-viewport',
  '.ant-table-body',
  '.ant-table-content',
];

const resolveScrollTarget = (container: HTMLElement | null, selectors: string[]) => {
  if (!container) {
    return null;
  }
  for (const selector of selectors) {
    const element = container.querySelector<HTMLElement>(selector);
    if (element) {
      return element;
    }
  }
  return null;
};

export const OrderReviewWorkbenchTemplate = ({
  pageTestId = 'order-review-page',
  className = 'order-review-page',
  toolbar,
  toolbarLeft,
  toolbarCenter,
  toolbarRight,
  header,
  actionBar,
  leftPanel,
  leftPanelVisible,
  leftPanelSyncKey,
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
  stateCacheKey,
}: OrderReviewWorkbenchTemplateProps) => {
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

  useWorkbenchLayoutController({
    leftPanel,
    leftPanelVisible,
    leftPanelSyncKey,
    leftPanelWidth,
    rightPanel,
    rightPanelVisible,
    rightPanelSyncKey,
    rightPanelWidth,
    bottomPanel: wrappedBottomPanel,
    bottomPanelVisible: shouldShowBottomPanel,
    bottomPanelSyncKey,
    bottomPanelHeight,
  });

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

  useEffect(() => {
    if (!stateCacheKey || typeof window === 'undefined') {
      return;
    }

    const tableContainer = internalTableContainerRef.current;
    if (!tableContainer) {
      return;
    }

    const persistScrollState = () => {
      const verticalTarget = resolveScrollTarget(tableContainer, verticalScrollSelectors);
      const horizontalTarget = resolveScrollTarget(tableContainer, horizontalScrollSelectors);
      writeWorkbenchViewStateCache(stateCacheKey, {
        scroll: {
          scrollTop: verticalTarget?.scrollTop ?? 0,
          scrollLeft: horizontalTarget?.scrollLeft ?? 0,
        },
      });
    };

    const verticalTarget = resolveScrollTarget(tableContainer, verticalScrollSelectors);
    const horizontalTarget = resolveScrollTarget(tableContainer, horizontalScrollSelectors);
    verticalTarget?.addEventListener('scroll', persistScrollState, { passive: true });
    if (horizontalTarget && horizontalTarget !== verticalTarget) {
      horizontalTarget.addEventListener('scroll', persistScrollState, { passive: true });
    }

    return () => {
      verticalTarget?.removeEventListener('scroll', persistScrollState);
      if (horizontalTarget && horizontalTarget !== verticalTarget) {
        horizontalTarget.removeEventListener('scroll', persistScrollState);
      }
    };
  }, [stateCacheKey, footerSyncKey]);

  useEffect(() => {
    if (!stateCacheKey || typeof window === 'undefined') {
      return;
    }

    const tableContainer = internalTableContainerRef.current;
    if (!tableContainer) {
      return;
    }

    const persistedScroll = readWorkbenchViewStateCache(stateCacheKey).scroll;
    if (!persistedScroll) {
      return;
    }

    let frameId = 0;
    const restoreScroll = () => {
      const verticalTarget = resolveScrollTarget(tableContainer, verticalScrollSelectors);
      const horizontalTarget = resolveScrollTarget(tableContainer, horizontalScrollSelectors);

      if (verticalTarget && persistedScroll.scrollTop != null) {
        verticalTarget.scrollTop = persistedScroll.scrollTop;
      }
      if (horizontalTarget && persistedScroll.scrollLeft != null) {
        horizontalTarget.scrollLeft = persistedScroll.scrollLeft;
      }
    };

    frameId = window.requestAnimationFrame(restoreScroll);
    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [footerSyncKey, stateCacheKey]);

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

  return (
    <div data-testid={pageTestId} style={pageShellStyle}>
      <div className={['order-workbench-page', className].filter(Boolean).join(' ')}>
        {resolvedToolbar}
        <div className="order-workbench-body">
          <div className="order-workbench-main">
            {header ? <div className="order-workbench-header">{header}</div> : null}
            <div className={resolvedTableClassName} ref={handleTableContainerRef}>
              <div className="order-workbench-table-content">{tableContent}</div>
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
