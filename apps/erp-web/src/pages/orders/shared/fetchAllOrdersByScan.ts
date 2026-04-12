import { listOrders, type OrderListQuery, type OrderSummary } from '@/api/orders';

export const fetchAllOrdersByScan = async (
  tenantId: string,
  query: OrderListQuery,
  options?: { signal?: AbortSignal },
) => {
  const pageSize = 500;
  let offset = 0;
  let total = 0;
  const items: OrderSummary[] = [];
  let hasNextPage = true;

  while (hasNextPage) {
    const page = await listOrders(
      tenantId,
      {
        ...query,
        offset,
        limit: pageSize,
      },
      { signal: options?.signal },
    );
    const chunk = page.items ?? [];
    if (offset === 0) {
      total = Number(page.total ?? 0);
    }
    if (!chunk.length) {
      break;
    }
    items.push(...chunk);
    offset += chunk.length;
    if (total > 0 && offset >= total) {
      hasNextPage = false;
    }
    if (chunk.length < pageSize) {
      hasNextPage = false;
    }
  }

  return items;
};
