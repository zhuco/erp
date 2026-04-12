import { useQuery } from '@tanstack/react-query';

import { buildCursorQuery, type CursorPagination } from '@/api/cursor';
import type { OrderListQuery } from '@/api/orders';
import type { OrderDetail } from '@/api/orders';
import { getOrderDetail, listOrders } from '@/api/orders';
import type { SkuListQuery } from '@/api/skus';
import { listSkus } from '@/api/skus';
import type { MappingListQuery } from '@/api/mappings';
import { listMappings } from '@/api/mappings';
import type { ShopProductSkuListQuery, ShopProductTaskListQuery } from '@/api/shopProducts';
import { listShopProductSkus, listShopProductTasks } from '@/api/shopProducts';
import { useTenantId } from '@/features/tenant';

type OrdersListParams = {
  query: OrderListQuery;
  enabled?: boolean;
};

export const useOrdersList = ({ query, enabled = true }: OrdersListParams) => {
  const tenantId = useTenantId();
  return useQuery({
    queryKey: ['orders', tenantId, query],
    queryFn: () => listOrders(tenantId, query),
    placeholderData: (previousData) => previousData,
    enabled,
    retry: false
  });
};

export const useOrderDetail = (orderId?: string, enabled = true) => {
  const tenantId = useTenantId();
  return useQuery<OrderDetail>({
    queryKey: ['order-detail', tenantId, orderId],
    queryFn: () => getOrderDetail(tenantId, orderId ?? ''),
    enabled: enabled && Boolean(orderId)
  });
};

type SkusListParams = CursorPagination & {
  product_id?: SkuListQuery['product_id'];
  sku_code?: SkuListQuery['sku_code'];
  barcode?: SkuListQuery['barcode'];
  keyword?: SkuListQuery['keyword'];
  is_active?: SkuListQuery['is_active'];
  enabled?: boolean;
};

export const useSkusList = ({
  product_id,
  sku_code,
  barcode,
  keyword,
  is_active,
  pageToken,
  pageSize,
  enabled = true
}: SkusListParams = {}) => {
  const tenantId = useTenantId();
  const query: SkuListQuery = {
    product_id,
    sku_code,
    barcode,
    keyword,
    is_active,
    ...buildCursorQuery({ pageToken, pageSize })
  };
  return useQuery({
    queryKey: ['skus', tenantId, query],
    queryFn: () => listSkus(tenantId, query),
    placeholderData: (previousData) => previousData,
    enabled
  });
};

type MappingsListParams = {
  query: MappingListQuery;
  enabled?: boolean;
};

export const useMappingsList = ({ query, enabled = true }: MappingsListParams) => {
  const tenantId = useTenantId();
  return useQuery({
    queryKey: ['mappings', tenantId, query],
    queryFn: () => listMappings(tenantId, query),
    placeholderData: (previousData) => previousData,
    enabled
  });
};

type ShopProductSkusListParams = {
  query: ShopProductSkuListQuery;
  enabled?: boolean;
};

export const useShopProductSkus = ({ query, enabled = true }: ShopProductSkusListParams) => {
  const tenantId = useTenantId();
  return useQuery({
    queryKey: ['shop-products', 'skus', tenantId, query],
    queryFn: () => listShopProductSkus(tenantId, query),
    placeholderData: (previousData) => previousData,
    enabled
  });
};

type ShopProductTasksListParams = {
  query: ShopProductTaskListQuery;
  enabled?: boolean;
};

export const useShopProductTasks = ({ query, enabled = true }: ShopProductTasksListParams) => {
  const tenantId = useTenantId();
  return useQuery({
    queryKey: ['shop-products', 'tasks', tenantId, query],
    queryFn: () => listShopProductTasks(tenantId, query),
    placeholderData: (previousData) => previousData,
    enabled
  });
};
