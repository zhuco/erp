import type { TablePaginationConfig } from 'antd';
import { useCallback, useState } from 'react';

type UseWorkbenchTablePaginationOptions = {
  defaultPageSize?: number;
  pageSizeOptions?: string[];
};

export const useWorkbenchTablePagination = ({
  defaultPageSize = 100,
  pageSizeOptions,
}: UseWorkbenchTablePaginationOptions = {}) => {
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: defaultPageSize,
    pageSizeOptions,
  });

  const resetPagination = useCallback(() => {
    setPagination((current) => {
      if ((current.current ?? 1) === 1) {
        return current;
      }
      return { ...current, current: 1 };
    });
  }, []);

  const handlePaginationChange = useCallback(
    (next: TablePaginationConfig) => {
      setPagination((current) => ({
        ...current,
        current: next.current ?? 1,
        pageSize: next.pageSize ?? current.pageSize,
        pageSizeOptions: current.pageSizeOptions ?? pageSizeOptions,
      }));
    },
    [pageSizeOptions],
  );

  return {
    pagination,
    setPagination,
    resetPagination,
    handlePaginationChange,
  };
};
