import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';

import { listProductBrands } from '@/api/productBrands';
import { listProductCategories } from '@/api/productCategories';
import { listProductUnits } from '@/api/productUnits';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { useTenantId } from '@/features/tenant';

export type GoodsOption = {
  label: string;
  value: string;
};

const mapNameOptions = (items: Array<{ name: string }>): GoodsOption[] =>
  items.map((item) => ({
    label: item.name,
    value: item.name,
  }));

export const useGoodsMasterData = () => {
  const tenantId = useTenantId();
  const enabled = Boolean(tenantId);

  const brandsQuery = useQuery({
    queryKey: ['goods-master-data', tenantId, 'brands'],
    queryFn: () => listProductBrands(tenantId, { offset: 0, limit: 200 }),
    enabled,
    staleTime: 60_000,
  });

  const categoriesQuery = useQuery({
    queryKey: ['goods-master-data', tenantId, 'categories'],
    queryFn: () => listProductCategories(tenantId, { offset: 0, limit: 200 }),
    enabled,
    staleTime: 60_000,
  });

  const unitsQuery = useQuery({
    queryKey: ['goods-master-data', tenantId, 'units'],
    queryFn: () => listProductUnits(tenantId, { offset: 0, limit: 200 }),
    enabled,
    staleTime: 60_000,
  });

  const suppliersQuery = useQuery({
    queryKey: ['goods-master-data', tenantId, 'suppliers'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled,
    staleTime: 60_000,
  });

  const warehousesQuery = useQuery({
    queryKey: ['goods-master-data', tenantId, 'warehouses'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled,
    staleTime: 60_000,
  });

  const brandOptions = useMemo(
    () => mapNameOptions(brandsQuery.data?.items ?? []),
    [brandsQuery.data?.items],
  );
  const categoryOptions = useMemo(
    () => mapNameOptions(categoriesQuery.data?.items ?? []),
    [categoriesQuery.data?.items],
  );
  const unitOptions = useMemo(
    () => mapNameOptions(unitsQuery.data?.items ?? []),
    [unitsQuery.data?.items],
  );
  const supplierOptions = useMemo(
    () =>
      (suppliersQuery.data?.items ?? []).map((item) => ({
        label: item.name,
        value: item.supplier_id,
      })),
    [suppliersQuery.data?.items],
  );
  const warehouseOptions = useMemo(
    () =>
      (warehousesQuery.data?.items ?? []).map((item) => ({
        label: `${item.name} (${item.code})`,
        value: item.warehouse_id,
      })),
    [warehousesQuery.data?.items],
  );

  return {
    tenantId,
    enabled,
    brandsQuery,
    categoriesQuery,
    unitsQuery,
    suppliersQuery,
    warehousesQuery,
    brandOptions,
    categoryOptions,
    unitOptions,
    supplierOptions,
    warehouseOptions,
  };
};
