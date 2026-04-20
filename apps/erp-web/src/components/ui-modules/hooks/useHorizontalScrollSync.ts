import type { RefObject } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

type HorizontalScrollSyncOptions = {
  containerRef: RefObject<HTMLElement | null>;
  syncKey?: string | number;
  selectors?: string[];
};

type HorizontalScrollSyncResult = {
  scrollBarRef: RefObject<HTMLDivElement>;
  scrollInnerRef: RefObject<HTMLDivElement>;
  hasHorizontalOverflow: boolean;
};

const defaultSelectors = [
  '.ag-center-cols-viewport',
  '.ag-body-viewport',
  '.ag-body-horizontal-scroll-viewport',
  '.ant-table-body',
  '.ant-table-content',
];
const HORIZONTAL_OVERFLOW_TOLERANCE_PX = 8;
const isElementMeasurable = (element: HTMLElement) =>
  element.clientWidth > 0 && element.getClientRects().length > 0;

const resolveTableContent = (container: HTMLElement, selectors: string[]) => {
  for (const selector of selectors) {
    const element = container.querySelector<HTMLElement>(selector);
    if (element) {
      return element;
    }
  }
  return null;
};

export const useHorizontalScrollSync = ({
  containerRef,
  syncKey,
  selectors
}: HorizontalScrollSyncOptions): HorizontalScrollSyncResult => {
  const scrollBarRef = useRef<HTMLDivElement | null>(null);
  const scrollInnerRef = useRef<HTMLDivElement | null>(null);
  const [hasHorizontalOverflow, setHasHorizontalOverflow] = useState(false);
  const ignoreTableScrollRef = useRef(false);
  const ignoreBarScrollRef = useRef(false);
  const barDraggingRef = useRef(false);
  const tableContentRef = useRef<HTMLElement | null>(null);
  const cleanupRef = useRef<(() => void) | null>(null);
  const tableSyncFrameRef = useRef<number | null>(null);
  const barSyncFrameRef = useRef<number | null>(null);
  const attachFrameRef = useRef<number | null>(null);
  const lastMetricsRef = useRef({ scrollWidth: 0, clientWidth: 0, scrollLeft: 0 });

  const resolvedSelectors = useMemo(
    () => (selectors && selectors.length ? selectors : defaultSelectors),
    [selectors]
  );

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    const container = containerRef.current;
    const scrollBar = scrollBarRef.current;
    const scrollInner = scrollInnerRef.current;
    if (!container || !scrollBar || !scrollInner) {
      return;
    }

    if (scrollInner.style.width !== '0px') {
      scrollInner.style.width = '0px';
    }
    setHasHorizontalOverflow(false);
    lastMetricsRef.current = { scrollWidth: 0, clientWidth: 0, scrollLeft: 0 };

    let frameId: number | null = null;

    const handlePointerDown = (event: PointerEvent) => {
      if (event.button !== 0) {
        return;
      }
      barDraggingRef.current = true;
      const maxScrollLeft = scrollBar.scrollWidth - scrollBar.clientWidth;
      if (maxScrollLeft <= 0) {
        return;
      }
      const rect = scrollBar.getBoundingClientRect();
      if (!rect.width) {
        return;
      }
      const offsetX = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
      const nextScrollLeft = Math.round((offsetX / rect.width) * maxScrollLeft);
      if (scrollBar.scrollLeft !== nextScrollLeft) {
        scrollBar.scrollLeft = nextScrollLeft;
      }
    };

    const handlePointerUp = () => {
      barDraggingRef.current = false;
    };

    scrollBar.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);

    const updateWidth = (tableContent: HTMLElement) => {
      if (!tableContent.isConnected) {
        if (scrollInner.style.width !== '0px') {
          scrollInner.style.width = '0px';
        }
        setHasHorizontalOverflow(false);
        scheduleAttach();
        return;
      }
      if (!isElementMeasurable(tableContent)) {
        if (scrollInner.style.width !== '0px') {
          scrollInner.style.width = '0px';
        }
        setHasHorizontalOverflow(false);
        lastMetricsRef.current = { scrollWidth: 0, clientWidth: 0, scrollLeft: 0 };
        scheduleAttach();
        return;
      }
      const scrollWidth = tableContent.scrollWidth;
      const clientWidth = tableContent.clientWidth;
      const scrollLeft = tableContent.scrollLeft;
      const nextHasHorizontalOverflow =
        scrollWidth - clientWidth > HORIZONTAL_OVERFLOW_TOLERANCE_PX;
      const lastMetrics = lastMetricsRef.current;
      if (lastMetrics.scrollWidth !== scrollWidth || lastMetrics.clientWidth !== clientWidth) {
        const nextWidth = `${scrollWidth}px`;
        if (scrollInner.style.width !== nextWidth) {
          scrollInner.style.width = nextWidth;
        }
      }
      setHasHorizontalOverflow((current) =>
        current === nextHasHorizontalOverflow ? current : nextHasHorizontalOverflow,
      );
      if (lastMetrics.scrollLeft !== scrollLeft && scrollBar.scrollLeft !== scrollLeft) {
        ignoreBarScrollRef.current = true;
        scrollBar.scrollLeft = scrollLeft;
      }
      lastMetricsRef.current = { scrollWidth, clientWidth, scrollLeft };
    };

    const scheduleUpdate = (tableContent: HTMLElement) => {
      if (frameId !== null) {
        window.cancelAnimationFrame(frameId);
      }
      frameId = window.requestAnimationFrame(() => {
        frameId = null;
        updateWidth(tableContent);
      });
    };

    const attachToTable = () => {
      const tableContent = resolveTableContent(container, resolvedSelectors);
      if (!tableContent) {
        if (scrollInner.style.width !== '0px') {
          scrollInner.style.width = '0px';
        }
        setHasHorizontalOverflow(false);
        return;
      }
      if (tableContentRef.current === tableContent) {
        scheduleUpdate(tableContent);
        return;
      }

      cleanupRef.current?.();
      tableContentRef.current = tableContent;
      lastMetricsRef.current = { scrollWidth: 0, clientWidth: 0, scrollLeft: 0 };

      const syncFromTable = () => {
        if (!tableContent.isConnected) {
          scheduleAttach();
          return;
        }
        if (barDraggingRef.current) {
          return;
        }
        if (scrollBar.scrollLeft === tableContent.scrollLeft) {
          return;
        }
        if (ignoreTableScrollRef.current) {
          ignoreTableScrollRef.current = false;
          return;
        }
        if (barSyncFrameRef.current !== null) {
          return;
        }
        barSyncFrameRef.current = window.requestAnimationFrame(() => {
          barSyncFrameRef.current = null;
          if (scrollBar.scrollLeft === tableContent.scrollLeft) {
            return;
          }
          ignoreBarScrollRef.current = true;
          scrollBar.scrollLeft = tableContent.scrollLeft;
        });
      };

      const syncFromBar = () => {
        if (!tableContent.isConnected) {
          scheduleAttach();
          return;
        }
        if (tableContent.scrollLeft === scrollBar.scrollLeft) {
          return;
        }
        if (ignoreBarScrollRef.current) {
          ignoreBarScrollRef.current = false;
          return;
        }
        if (tableSyncFrameRef.current !== null) {
          return;
        }
        tableSyncFrameRef.current = window.requestAnimationFrame(() => {
          tableSyncFrameRef.current = null;
          if (tableContent.scrollLeft === scrollBar.scrollLeft) {
            return;
          }
          ignoreTableScrollRef.current = true;
          tableContent.scrollLeft = scrollBar.scrollLeft;
        });
      };

      tableContent.addEventListener('scroll', syncFromTable, { passive: true });
      scrollBar.addEventListener('scroll', syncFromBar, { passive: true });

      const resizeObserver =
        typeof ResizeObserver === 'undefined'
          ? null
          : new ResizeObserver(() => scheduleUpdate(tableContent));
      resizeObserver?.observe(tableContent);
      const tableElement =
        tableContent.querySelector<HTMLElement>('.ag-center-cols-container') ??
        tableContent.querySelector<HTMLElement>('table');
      if (tableElement) {
        resizeObserver?.observe(tableElement);
      }

      scheduleUpdate(tableContent);

      cleanupRef.current = () => {
        tableContent.removeEventListener('scroll', syncFromTable);
        scrollBar.removeEventListener('scroll', syncFromBar);
        resizeObserver?.disconnect();
        if (tableSyncFrameRef.current !== null) {
          window.cancelAnimationFrame(tableSyncFrameRef.current);
          tableSyncFrameRef.current = null;
        }
        if (barSyncFrameRef.current !== null) {
          window.cancelAnimationFrame(barSyncFrameRef.current);
          barSyncFrameRef.current = null;
        }
      };
    };

    const scheduleAttach = () => {
      if (attachFrameRef.current !== null) {
        return;
      }
      attachFrameRef.current = window.requestAnimationFrame(() => {
        attachFrameRef.current = null;
        attachToTable();
      });
    };

    const handleWindowResize = () => {
      const tableContent = tableContentRef.current;
      if (tableContent) {
        scheduleUpdate(tableContent);
      } else {
        scheduleAttach();
      }
    };

    scheduleAttach();

    const mutationObserver =
      typeof MutationObserver === 'undefined'
        ? null
        : new MutationObserver(() => {
            scheduleAttach();
          });
    mutationObserver?.observe(container, {
      childList: true,
      attributes: true,
      subtree: true,
    });
    window.addEventListener('resize', handleWindowResize);

    return () => {
      cleanupRef.current?.();
      cleanupRef.current = null;
      tableContentRef.current = null;
      mutationObserver?.disconnect();
      scrollBar.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);
      window.removeEventListener('resize', handleWindowResize);
      if (frameId !== null) {
        window.cancelAnimationFrame(frameId);
      }
      if (attachFrameRef.current !== null) {
        window.cancelAnimationFrame(attachFrameRef.current);
        attachFrameRef.current = null;
      }
      if (tableSyncFrameRef.current !== null) {
        window.cancelAnimationFrame(tableSyncFrameRef.current);
        tableSyncFrameRef.current = null;
      }
      if (barSyncFrameRef.current !== null) {
        window.cancelAnimationFrame(barSyncFrameRef.current);
        barSyncFrameRef.current = null;
      }
    };
  }, [containerRef, resolvedSelectors, syncKey]);

  return { scrollBarRef, scrollInnerRef, hasHorizontalOverflow };
};
