export type CursorPagination = {
  pageToken?: string | null;
  pageSize?: number;
};

export type CursorQuery = {
  cursor?: string;
  limit?: number;
};

export const DEFAULT_PAGE_SIZE = 20;

export const buildCursorQuery = (pagination: CursorPagination = {}): CursorQuery => ({
  cursor: pagination.pageToken ?? undefined,
  limit: pagination.pageSize ?? DEFAULT_PAGE_SIZE
});

export const getNextCursor = (
  pagination: CursorPagination,
  nextCursor?: string | null
): CursorPagination => ({
  pageToken: nextCursor ?? undefined,
  pageSize: pagination.pageSize ?? DEFAULT_PAGE_SIZE
});
