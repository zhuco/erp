import type { TablePaginationConfig } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { readWorkbenchViewStateCache, writeWorkbenchViewStateCache } from './workbenchViewStateCache';

type UseWorkbenchTablePaginationOptions = {
  defaultPageSize?: number;
  pageSizeOptions?: string[];
  storageKey?: string;
};

const buildInitialPagination = ({
  defaultPageSize,
  pageSizeOptions,
  storageKey,
}: Required<Pick<UseWorkbenchTablePaginationOptions, 'defaultPageSize'>> &
  Pick<UseWorkbenchTablePaginationOptions, 'pageSizeOptions' | 'storageKey'>): TablePaginationConfig => {
  const persistedPagination = readWorkbenchViewStateCache(storageKey).pagination;

  return {
    current: persistedPagination?.current ?? 1,
    pageSize: persistedPagination?.pageSize ?? defaultPageSize,
    pageSizeOptions,
  };
};

export const useWorkbenchTablePagination = ({
  defaultPageSize = 100,
  pageSizeOptions,
  storageKey,
}: UseWorkbenchTablePaginationOptions = {}) => {
  const initialPagination = useMemo(
    () => buildInitialPagination({ defaultPageSize, pageSizeOptions, storageKey }),
    [defaultPageSize, pageSizeOptions, storageKey],
  );

  const [pagination, setPagination] = useState<TablePaginationConfig>(initialPagination);

  useEffect(() => {
    setPagination((current) => {
      const nextPagination = buildInitialPagination({ defaultPageSize, pageSizeOptions, storageKey });
      if (
        (current.current ?? 1) === (nextPagination.current ?? 1) &&
        (current.pageSize ?? defaultPageSize) === (nextPagination.pageSize ?? defaultPageSize) &&
        current.pageSizeOptions === pageSizeOptions
      ) {
        return current;
      }
      return {
        ...current,
        current: nextPagination.current,
        pageSize: nextPagination.pageSize,
        pageSizeOptions,
      };
    });
  }, [defaultPageSize, pageSizeOptions, storageKey]);

  useEffect(() => {
    writeWorkbenchViewStateCache(storageKey, {
      pagination: {
        current: Number(pagination.current ?? 1),
        pageSize: Number(pagination.pageSize ?? defaultPageSize),
      },
    });
  }, [defaultPageSize, pagination.current, pagination.pageSize, storageKey]);

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
      setPagination((current) => {
        const currentPageSize = Number(current.pageSize ?? defaultPageSize);
        const nextPageSize = Number(next.pageSize ?? current.pageSize ?? defaultPageSize);
        const pageSizeChanged = nextPageSize !== currentPageSize;

        return {
          ...current,
          current: pageSizeChanged ? 1 : (next.current ?? current.current ?? 1),
          pageSize: nextPageSize,
          pageSizeOptions: current.pageSizeOptions ?? pageSizeOptions,
        };
      });
    },
    [defaultPageSize, pageSizeOptions],
  );

  return {
    pagination,
    setPagination,
    resetPagination,
    handlePaginationChange,
  };
};
