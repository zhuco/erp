import { Pagination } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ReactNode, RefObject } from 'react';

import { useHorizontalScrollSync } from './hooks/useHorizontalScrollSync';
import './UiModule03TableFooter.css';

type UiModule03TableFooterProps = {
  tableContainerRef: RefObject<HTMLDivElement>;
  syncKey?: string | number;
  summary?: ReactNode;
  total: number;
  pagination: TablePaginationConfig;
  onPaginationChange: (pagination: TablePaginationConfig) => void;
  showScrollbarRow?: boolean;
};

export const UiModule03TableFooter = ({
  tableContainerRef,
  syncKey,
  summary,
  total,
  pagination,
  onPaginationChange,
  showScrollbarRow = true
}: UiModule03TableFooterProps) => {
  const { scrollBarRef, scrollInnerRef } = useHorizontalScrollSync({
    containerRef: tableContainerRef,
    syncKey
  });

  return (
    <div className="order-workbench-table-footer order-review-table-footer">
      {showScrollbarRow ? (
        <div className="order-workbench-scrollbar-row order-review-scrollbar-row">
          <div className="order-workbench-scrollbar order-review-scrollbar" ref={scrollBarRef}>
            <div
              className="order-workbench-scrollbar-inner order-review-scrollbar-inner"
              ref={scrollInnerRef}
            />
          </div>
        </div>
      ) : null}
      <div className="order-workbench-pagination order-review-pagination">
        {summary ? (
          <span className="order-workbench-pagination-summary order-review-pagination-summary">
            {summary}
          </span>
        ) : null}
        <span className="order-workbench-pagination-controls order-review-pagination-controls">
          <Pagination
            size="small"
            current={pagination.current ?? 1}
            pageSize={pagination.pageSize ?? 100}
            total={total}
            showSizeChanger={{
              placement: 'topRight',
              getPopupContainer: () => document.body
            }}
            pageSizeOptions={pagination.pageSizeOptions}
            onChange={(page, pageSize) => onPaginationChange({ current: page, pageSize })}
          />
        </span>
      </div>
    </div>
  );
};
