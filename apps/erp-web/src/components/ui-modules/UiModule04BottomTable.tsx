import type { ReactNode } from 'react';
import { useEffect, useLayoutEffect, useRef } from 'react';

import { useHorizontalScrollSync } from './hooks/useHorizontalScrollSync';
import './UiModule04BottomTable.css';

type UiModule04BottomTableProps = {
  className?: string;
  children: ReactNode;
  syncKey?: string | number;
};

export const UiModule04BottomTable = ({
  className,
  children,
  syncKey
}: UiModule04BottomTableProps) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const verticalScrollRef = useRef<number | null>(null);
  const verticalTargetRef = useRef<HTMLElement | null>(null);
  const verticalCleanupRef = useRef<(() => void) | null>(null);
  const { scrollBarRef, scrollInnerRef } = useHorizontalScrollSync({
    containerRef,
    syncKey
  });
  const useSafeLayoutEffect = typeof window === 'undefined' ? useEffect : useLayoutEffect;

  const isScrollable = (element: HTMLElement) =>
    element.scrollHeight - element.clientHeight > 1;

  const resolveScrollableParent = (container: HTMLElement) => {
    if (typeof window === 'undefined') {
      return null;
    }
    const tabPane = container.closest<HTMLElement>('.ant-tabs-tabpane');
    if (tabPane && isScrollable(tabPane)) {
      return tabPane;
    }
    let current = container.parentElement;
    while (current && current !== document.body) {
      const style = window.getComputedStyle(current);
      const canScroll = ['auto', 'scroll', 'overlay'].includes(style.overflowY);
      if (canScroll && isScrollable(current)) {
        return current;
      }
      current = current.parentElement;
    }
    return null;
  };

  const resolveVerticalTarget = () => {
    const container = containerRef.current;
    if (!container) {
      return null;
    }
    const tableTarget =
      container.querySelector<HTMLElement>('.ag-body-viewport') ??
      container.querySelector<HTMLElement>('.ant-table-body') ??
      container.querySelector<HTMLElement>('.ant-table-content')
    if (tableTarget && isScrollable(tableTarget)) {
      return tableTarget;
    }
    return resolveScrollableParent(container) ?? tableTarget;
  };

  useSafeLayoutEffect(() => {
    const target = resolveVerticalTarget();
    if (!target) {
      return;
    }
    if (verticalTargetRef.current !== target) {
      verticalCleanupRef.current?.();
      verticalCleanupRef.current = null;
      verticalTargetRef.current = target;
      const handleScroll = () => {
        verticalScrollRef.current = target.scrollTop;
      };
      target.addEventListener('scroll', handleScroll, { passive: true });
      verticalCleanupRef.current = () => {
        target.removeEventListener('scroll', handleScroll);
      };
    }
    if (verticalScrollRef.current == null) {
      verticalScrollRef.current = target.scrollTop;
      return;
    }
    const nextTop = verticalScrollRef.current;
    if (Math.abs(target.scrollTop - nextTop) > 1) {
      target.scrollTop = nextTop;
    }
  });

  useEffect(() => () => {
    verticalCleanupRef.current?.();
    verticalCleanupRef.current = null;
    verticalTargetRef.current = null;
  }, []);

  const rootClassName = [
    'order-workbench-bottom-table',
    'order-review-bottom-table',
    className
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div className={rootClassName} ref={containerRef}>
      <div className="order-workbench-bottom-scrollbar-row order-review-bottom-scrollbar-row">
        <div
          className="order-workbench-bottom-scrollbar order-review-bottom-scrollbar"
          ref={scrollBarRef}
        >
          <div
            className="order-workbench-bottom-scrollbar-inner order-review-bottom-scrollbar-inner"
            ref={scrollInnerRef}
          />
        </div>
      </div>
      {children}
    </div>
  );
};
