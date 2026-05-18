import './industrialWorkbenchTemplate.css';

import type { TablePaginationConfig } from 'antd';
import type { MutableRefObject, ReactNode, Ref } from 'react';
import { useCallback, useEffect, useMemo, useRef } from 'react';

import { UiModule03TableFooter } from '@/components/ui-modules';

import { useWorkbenchLayoutController } from './useWorkbenchLayoutController';
import {
  readWorkbenchViewStateCache,
  writeWorkbenchViewStateCache,
} from '@/components/workbench/templates/workbenchViewStateCache';

export type IndustrialWorkbenchTemplateProps = {
  pageTestId?: string;
  className?: string;
  toolbar?: ReactNode;
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

export const DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME =
  'industrial-workbench-table-module-03 industrial-workbench-table-external-scrollbar order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar';

export const INDUSTRIAL_WORKBENCH_TEST_IDS = {
  shell: 'industrial-workbench-shell',
  body: 'industrial-workbench-body',
  main: 'industrial-workbench-main',
  header: 'industrial-workbench-header',
  table: 'industrial-workbench-table',
  tableContent: 'industrial-workbench-table-content',
  action: 'industrial-workbench-action',
  overlays: 'industrial-workbench-overlays',
  bottomPanel: 'industrial-workbench-bottom-panel',
} as const;

export const INDUSTRIAL_WORKBENCH_REGIONS = {
  shell: 'shell',
  body: 'body',
  main: 'main',
  header: 'header',
  table: 'table',
  tableContent: 'table-content',
  action: 'action',
  overlays: 'overlays',
  bottomPanel: 'bottom-panel',
} as const;

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

export const IndustrialWorkbenchTemplate = ({
  pageTestId = 'industrial-workbench-page',
  className = 'industrial-workbench-page',
  toolbar,
  header,
  actionBar,
  leftPanel,
  leftPanelVisible,
  leftPanelSyncKey,
  tableContainerRef,
  tableClassName = DEFAULT_INDUSTRIAL_WORKBENCH_TABLE_CLASS_NAME,
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
}: IndustrialWorkbenchTemplateProps) => {
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
        <div
          className="industrial-workbench-bottom-panel order-workbench-bottom-panel"
          data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.bottomPanel}
          data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.bottomPanel}
        >
          {bottomPanel}
        </div>
      ) : undefined,
    [bottomPanel, shouldShowBottomPanel],
  );

  const layoutState = useWorkbenchLayoutController({
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
    'industrial-workbench-table',
    'order-workbench-table',
    tableClassName,
    resolvedTableFooter ? 'industrial-workbench-table-with-footer order-workbench-table-with-footer' : null,
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      data-testid={pageTestId}
      data-workbench-layout="industrial"
      data-left-panel-visible={layoutState.left.visible ? 'true' : 'false'}
      data-right-panel-visible={layoutState.right.visible ? 'true' : 'false'}
      data-bottom-panel-visible={layoutState.bottom.visible ? 'true' : 'false'}
      style={pageShellStyle}
    >
      <div
        className={['industrial-workbench-page-shell', 'order-workbench-page', className]
          .filter(Boolean)
          .join(' ')}
        data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.shell}
        data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.shell}
      >
        {toolbar ?? null}
        <div
          className="industrial-workbench-body order-workbench-body"
          data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.body}
          data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.body}
        >
          <div
            className="industrial-workbench-main order-workbench-main"
            data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.main}
            data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.main}
          >
            {header ? (
              <div
                className="industrial-workbench-header"
                data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.header}
                data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.header}
              >
                {header}
              </div>
            ) : null}
            <div
              className={resolvedTableClassName}
              data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.table}
              data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.table}
              ref={handleTableContainerRef}
            >
              <div
                className="industrial-workbench-table-content order-workbench-table-content"
                data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.tableContent}
                data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.tableContent}
              >
                {tableContent}
              </div>
              {resolvedTableFooter}
            </div>
            {actionBar ? (
              <div
                className="industrial-workbench-action"
                data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.action}
                data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.action}
              >
                {actionBar}
              </div>
            ) : null}
          </div>
        </div>
      </div>
      {overlays ? (
        <div
          className="industrial-workbench-overlays"
          data-testid={INDUSTRIAL_WORKBENCH_TEST_IDS.overlays}
          data-workbench-region={INDUSTRIAL_WORKBENCH_REGIONS.overlays}
        >
          {overlays}
        </div>
      ) : null}
    </div>
  );
};
