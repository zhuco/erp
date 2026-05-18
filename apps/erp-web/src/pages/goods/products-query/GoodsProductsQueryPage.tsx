import { PlusOutlined } from '@ant-design/icons';
import {
  Button,
  Descriptions,
  Empty,
  Form,
  Select,
  Space,
  Tag,
  Tabs,
} from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom';

import { listProducts, type Product, type ProductListQuery } from '@/api/products';
import { ErpDataGridNext } from '@/components/ErpDataGridNext';
import {
  WorkbenchLeftPanelTemplate,
  WorkbenchPageTemplate,
  WorkbenchToolbar,
  type WorkbenchLeftRenderRow,
  type WorkbenchLeftStatsItem,
  useWorkbenchTablePagination,
} from '@/components/page-templates';
import { UiModule04BottomTable, UiModule05DetailPanel } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';
import {
  buildStatsPieBackground,
  sliceRowsByPagination,
  trimText,
} from '@/pages/shared/directoryWorkbenchUtils';

import {
  buildGoodsProductCreatePath,
  buildGoodsProductEditPath,
  createGoodsEditSearchParams,
} from '../shared/goodsRoutes';
import { useGoodsMasterData } from '../shared/useGoodsMasterData';
import { IndustrialTable } from '@/components/industrial';

type ProductGridFilters = {
  name: string;
  brand: string;
  category: string;
  status: 'ALL' | 'ACTIVE' | 'INACTIVE';
};

type ProductLeftFilterKey = 'view:all' | 'status:ACTIVE' | 'status:INACTIVE';
const GOODS_PRODUCTS_QUERY_PATH = '/goods/products/query';
const GOODS_PAGE_SIZE_OPTIONS = ['20', '50', '100'];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const createDefaultGridFilters = (): ProductGridFilters => ({
  name: '',
  brand: '',
  category: '',
  status: 'ALL',
});

const buildGridFilterModel = (filters: ProductGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = {
      filterType: 'text',
      type: 'contains',
      filter: trimText(filters.name),
    };
  }
  if (trimText(filters.brand)) {
    model.brand = {
      filterType: 'text',
      type: 'contains',
      filter: trimText(filters.brand),
    };
  }
  if (trimText(filters.category)) {
    model.category = {
      filterType: 'text',
      type: 'contains',
      filter: trimText(filters.category),
    };
  }
  if (filters.status !== 'ALL') {
    model.status = { filterType: 'set', values: [filters.status] };
  }
  return model;
};

const extractGridFilterModel = (
  model: FilterModel | null | undefined,
): ProductGridFilters => {
  const statusValues =
    model?.status &&
    typeof model.status === 'object' &&
    'values' in model.status &&
    Array.isArray(model.status.values)
      ? model.status.values
          .map((value: unknown) => String(value ?? '').trim())
          .filter(Boolean)
      : [];

  return {
    name:
      model?.name && typeof model.name === 'object' && 'filter' in model.name
        ? trimText(String(model.name.filter ?? ''))
        : '',
    brand:
      model?.brand && typeof model.brand === 'object' && 'filter' in model.brand
        ? trimText(String(model.brand.filter ?? ''))
        : '',
    category:
      model?.category && typeof model.category === 'object' && 'filter' in model.category
        ? trimText(String(model.category.filter ?? ''))
        : '',
    status: (statusValues[0] as ProductGridFilters['status'] | undefined) ?? 'ALL',
  };
};

const productStatusText = (status?: string | null) =>
  status === 'INACTIVE' || status === 'DISABLED' ? '停用' : '启用';

const productStatusColor = (status?: string | null) =>
  status === 'INACTIVE' || status === 'DISABLED' ? 'default' : 'green';

const isSameProductGridFilters = (
  current: ProductGridFilters,
  next: ProductGridFilters,
) =>
  current.name === next.name &&
  current.brand === next.brand &&
  current.category === next.category &&
  current.status === next.status;

const parseProductLeftFilterKey = (value: string | null): ProductLeftFilterKey =>
  value === 'status:ACTIVE' || value === 'status:INACTIVE' ? value : 'view:all';

const parsePositiveInteger = (value: string | null, fallback: number) => {
  const nextValue = Number.parseInt(String(value ?? ''), 10);
  return Number.isFinite(nextValue) && nextValue > 0 ? nextValue : fallback;
};

export const GoodsProductsQueryPage = () => {
  const navigate = useNavigate();
  const tenantId = useTenantId();
  const [searchParams, setSearchParams] = useSearchParams();
  const searchSnapshot = searchParams.toString();
  const editProductId = searchParams.get('edit');
  const initialKeyword = trimText(searchParams.get('keyword'));
  const initialLeftFilterKey = parseProductLeftFilterKey(searchParams.get('leftFilterKey'));
  const initialGridFilters = useMemo<ProductGridFilters>(
    () => ({
      ...createDefaultGridFilters(),
      brand: trimText(searchParams.get('brand')),
      category: trimText(searchParams.get('category')),
      status:
        searchParams.get('status') === 'ACTIVE' || searchParams.get('status') === 'INACTIVE'
          ? (searchParams.get('status') as ProductGridFilters['status'])
          : 'ALL',
    }),
    [searchSnapshot],
  );
  const initialPage = parsePositiveInteger(searchParams.get('page'), 1);
  const initialPageSize = parsePositiveInteger(searchParams.get('pageSize'), 20);
  const initialActiveProductId = trimText(searchParams.get('activeId')) || null;
  const [searchInput, setSearchInput] = useState(initialKeyword);
  const [searchKeyword, setSearchKeyword] = useState(initialKeyword);
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<ProductLeftFilterKey>(initialLeftFilterKey);
  const [gridFilters, setGridFilters] = useState<ProductGridFilters>(initialGridFilters);
  const [activeProductId, setActiveProductId] = useState<string | null>(initialActiveProductId);
  const {
    brandOptions,
    categoryOptions,
  } = useGoodsMasterData();

  const { pagination, setPagination, handlePaginationChange, resetPagination } =
    useWorkbenchTablePagination({
      defaultPageSize: 20,
      pageSizeOptions: GOODS_PAGE_SIZE_OPTIONS,
      storageKey: tenantId ? `goods.products.query.pagination.${tenantId}` : undefined,
    });
  const legacyEditRedirectTarget = editProductId
    ? buildGoodsProductEditPath(editProductId, createGoodsEditSearchParams(searchParams.toString()))
    : null;

  useEffect(() => {
    if (legacyEditRedirectTarget) {
      return;
    }
    setSearchInput(initialKeyword);
    setSearchKeyword(initialKeyword);
    setLeftFilterKey(initialLeftFilterKey);
    setGridFilters((current) =>
      isSameProductGridFilters(current, initialGridFilters) ? current : initialGridFilters,
    );
    setActiveProductId(initialActiveProductId);
    setPagination((current) => {
      const currentPage = Number(current.current ?? 1);
      const currentPageSize = Number(current.pageSize ?? 20);
      if (currentPage === initialPage && currentPageSize === initialPageSize) {
        return current;
      }
      return {
        ...current,
        current: initialPage,
        pageSize: initialPageSize,
      };
    });
  }, [
    initialActiveProductId,
    initialGridFilters,
    initialKeyword,
    initialLeftFilterKey,
    initialPage,
    initialPageSize,
    legacyEditRedirectTarget,
    setPagination,
  ]);

  useEffect(() => {
    if (legacyEditRedirectTarget) {
      return;
    }
    const timer = window.setTimeout(() => {
      const nextKeyword = searchInput.trim();
      if (nextKeyword === searchKeyword) {
        return;
      }
      setSearchKeyword(nextKeyword);
      resetPagination();
    }, 250);
    return () => window.clearTimeout(timer);
  }, [legacyEditRedirectTarget, resetPagination, searchInput, searchKeyword]);

  useEffect(() => {
    if (legacyEditRedirectTarget) {
      return;
    }
    const next = new URLSearchParams(searchParams);
    const keyword = searchKeyword.trim();

    if (keyword) {
      next.set('keyword', keyword);
    } else {
      next.delete('keyword');
    }

    if (leftFilterKey !== 'view:all') {
      next.set('leftFilterKey', leftFilterKey);
    } else {
      next.delete('leftFilterKey');
    }

    if (gridFilters.brand) {
      next.set('brand', gridFilters.brand);
    } else {
      next.delete('brand');
    }

    if (gridFilters.category) {
      next.set('category', gridFilters.category);
    } else {
      next.delete('category');
    }

    if (gridFilters.status !== 'ALL') {
      next.set('status', gridFilters.status);
    } else {
      next.delete('status');
    }

    if (activeProductId) {
      next.set('activeId', activeProductId);
    } else {
      next.delete('activeId');
    }

    next.set('page', String(pagination.current ?? 1));
    next.set('pageSize', String(pagination.pageSize ?? 20));

    const serialized = next.toString();
    const currentSerialized = searchParams.toString();
    if (serialized !== currentSerialized) {
      setSearchParams(next, { replace: true });
    }
  }, [
    activeProductId,
    gridFilters.brand,
    gridFilters.category,
    gridFilters.status,
    leftFilterKey,
    pagination.current,
    pagination.pageSize,
    searchKeyword,
    searchParams,
    setSearchParams,
    legacyEditRedirectTarget,
  ]);

  const query = useMemo<ProductListQuery>(
    () => ({
      keyword: searchKeyword || undefined,
      limit: 500,
    }),
    [searchKeyword],
  );

  const productsQuery = useQuery({
    queryKey: ['goods-products-query', GOODS_PRODUCTS_QUERY_PATH, tenantId, query],
    queryFn: () => listProducts(tenantId, query),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const rows = useMemo(() => productsQuery.data?.data ?? [], [productsQuery.data?.data]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部货品', value: rows.length, color: '#2f6fed' },
      {
        key: 'active',
        label: '启用',
        value: rows.filter((row) => productStatusText(row.status) === '启用').length,
        color: '#0f9d58',
      },
      {
        key: 'inactive',
        label: '停用',
        value: rows.filter((row) => productStatusText(row.status) === '停用').length,
        color: '#8c8c8c',
      },
    ],
    [rows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部货品',
        count: rows.length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'status:ACTIVE',
        label: '启用货品',
        count: rows.filter((row) => productStatusText(row.status) === '启用').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'status:ACTIVE',
      },
      {
        key: 'status:INACTIVE',
        label: '停用货品',
        count: rows.filter((row) => productStatusText(row.status) === '停用').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'status:INACTIVE',
      },
    ],
    [leftFilterKey, rows],
  );

  const displayRows = useMemo(() => {
    return rows.filter((row) => {
      if (leftFilterKey === 'status:ACTIVE' && productStatusText(row.status) !== '启用') {
        return false;
      }
      if (leftFilterKey === 'status:INACTIVE' && productStatusText(row.status) !== '停用') {
        return false;
      }
      if (trimText(gridFilters.name) && !row.name.toLowerCase().includes(trimText(gridFilters.name).toLowerCase())) {
        return false;
      }
      if (trimText(gridFilters.brand) && !trimText(row.brand).toLowerCase().includes(trimText(gridFilters.brand).toLowerCase())) {
        return false;
      }
      if (trimText(gridFilters.category) && !trimText(row.category).toLowerCase().includes(trimText(gridFilters.category).toLowerCase())) {
        return false;
      }
      if (gridFilters.status !== 'ALL' && row.status !== gridFilters.status) {
        return false;
      }
      return true;
    });
  }, [gridFilters, leftFilterKey, rows]);

  const pagedRows = useMemo(
    () => sliceRowsByPagination(displayRows, pagination),
    [displayRows, pagination],
  );

  const activeProduct = useMemo(
    () => displayRows.find((row) => row.product_id === activeProductId) ?? null,
    [activeProductId, displayRows],
  );

  useEffect(() => {
    const firstRow = pagedRows[0];
    if (!activeProduct && firstRow) {
      setActiveProductId(firstRow.product_id);
    }
  }, [activeProduct, pagedRows]);

  const detailPanels = useMemo(
    () => [
      {
        key: 'basic',
        label: '货品摘要',
        children: activeProduct ? (
          <Descriptions column={1} size="small" bordered>
            <Descriptions.Item label="货品编码">
              {activeProduct.product_code ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="货品名称">
              {activeProduct.name}
            </Descriptions.Item>
            <Descriptions.Item label="品牌">
              {activeProduct.brand ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="分类">
              {activeProduct.category ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="状态">
              <Tag color={productStatusColor(activeProduct.status)}>
                {productStatusText(activeProduct.status)}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="更新时间">
              {formatDateTime(activeProduct.updated_at)}
            </Descriptions.Item>
          </Descriptions>
        ) : (
          <Empty description="请选择货品查看摘要" />
        ),
      },
    ],
    [activeProduct],
  );

  const bottomItems = useMemo(
    () => [
      {
        key: 'profile',
        label: '主档字段',
        children: activeProduct ? (
          <UiModule04BottomTable syncKey={activeProduct.product_id}>
            <IndustrialTable
              size="small"
              rowKey="field"
              pagination={false}
              scroll={{ x: 'max-content' }}
              dataSource={[
                { field: '货品编码', value: activeProduct.product_code ?? '-' },
                { field: '主单位', value: activeProduct.base_unit ?? '-' },
                { field: '副单位', value: activeProduct.second_unit ?? '-' },
                { field: '默认供应商', value: activeProduct.default_supplier_id ?? '-' },
                { field: '默认仓库', value: activeProduct.default_warehouse_id ?? '-' },
                { field: '主条码', value: activeProduct.primary_barcode ?? '-' },
                {
                  field: '货品标签',
                  value: activeProduct.product_tags.join('、') || '-',
                },
                { field: '备注', value: activeProduct.remark ?? '-' },
              ]}
              columns={[
                { title: '字段', dataIndex: 'field', key: 'field', width: 180 },
                { title: '值', dataIndex: 'value', key: 'value', width: 420 },
              ]}
            />
          </UiModule04BottomTable>
        ) : (
          <Empty description="请选择货品查看明细" />
        ),
      },
    ],
    [activeProduct],
  );

  if (legacyEditRedirectTarget) {
    return <Navigate replace to={legacyEditRedirectTarget} />;
  }

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="goods-products-query-page"
        toolbar={(
          <WorkbenchToolbar
            leftContent={(
              <Space>
                <Button
                  type="primary"
                  icon={<PlusOutlined />}
                  onClick={() => navigate(buildGoodsProductCreatePath(searchParams.toString()))}
                >
                  新建货品
                </Button>
                <Button
                  disabled={!activeProduct}
                  onClick={() => {
                    if (!activeProduct) {
                      return;
                    }
                    navigate(
                      buildGoodsProductEditPath(
                        activeProduct.product_id,
                        searchParams.toString(),
                      ),
                    );
                  }}
                >
                  修改货品
                </Button>
              </Space>
            )}
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => setSearchKeyword(searchInput.trim())}
            searchPlaceholder="搜索货品编码、名称、条码"
          />
        )}
        leftPanel={(
          <WorkbenchLeftPanelTemplate
            leftTab={leftTab}
            onLeftTabChange={(key) => setLeftTab(key)}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              setLeftFilterKey(event.data?.key as ProductLeftFilterKey);
              resetPagination();
            }}
            screenContent={(
              <Form layout="vertical">
                <Form.Item label="品牌">
                  <Select
                    allowClear
                    showSearch
                    options={brandOptions}
                    optionFilterProp="label"
                    value={gridFilters.brand || undefined}
                    placeholder="筛选品牌"
                    onChange={(value) => {
                      setGridFilters((prev) => ({ ...prev, brand: String(value ?? '') }));
                      resetPagination();
                    }}
                  />
                </Form.Item>
                <Form.Item label="分类">
                  <Select
                    allowClear
                    showSearch
                    options={categoryOptions}
                    optionFilterProp="label"
                    value={gridFilters.category || undefined}
                    placeholder="筛选分类"
                    onChange={(value) => {
                      setGridFilters((prev) => ({ ...prev, category: String(value ?? '') }));
                      resetPagination();
                    }}
                  />
                </Form.Item>
              </Form>
            )}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(
              leftStatsItems.map((item) => ({
                value: item.value,
                color: item.color,
              })),
            )}
            globalOrderTotal={rows.length}
            statsScopeText="货品主档实时统计"
          />
        )}
        leftPanelVisible
        rightPanel={<UiModule05DetailPanel panels={detailPanels} />}
        rightPanelVisible
        bottomPanel={<Tabs activeKey="profile" items={bottomItems} />}
        bottomPanelVisible
        tableContent={(
          <ErpDataGridNext<Product>
            rows={pagedRows}
            rowKey={(row) => row.product_id}
            dataTestId="goods-products-grid"
            emptyText="暂无货品数据"
            loading={productsQuery.isLoading}
            error={productsQuery.error}
            onRetry={() => productsQuery.refetch()}
            activeRowKey={activeProduct?.product_id ?? null}
            onActiveRowChange={(row) => setActiveProductId(row.product_id)}
            onRowDoubleClick={(row) => {
              navigate(buildGoodsProductEditPath(row.product_id, searchParams.toString()));
            }}
            gridFilterModel={buildGridFilterModel(gridFilters)}
            onGridFilterModelChange={(model) => {
              setGridFilters(extractGridFilterModel(model));
              resetPagination();
            }}
            columns={[
              {
                key: 'product_code',
                field: 'product_code',
                header: '货品编码',
                width: 140,
                render: (row) => row.product_code ?? '-',
              },
              {
                key: 'name',
                field: 'name',
                header: '货品名称',
                width: 220,
                render: (row) => row.name,
              },
              {
                key: 'brand',
                field: 'brand',
                header: '品牌',
                width: 140,
                render: (row) => row.brand ?? '-',
              },
              {
                key: 'category',
                field: 'category',
                header: '分类',
                width: 140,
                render: (row) => row.category ?? '-',
              },
              {
                key: 'status',
                field: 'status',
                header: '状态',
                width: 100,
                render: (row) => (
                  <Tag color={productStatusColor(row.status)}>
                    {productStatusText(row.status)}
                  </Tag>
                ),
              },
              {
                key: 'updated_at',
                field: 'updated_at',
                header: '更新时间',
                width: 160,
                render: (row) => formatDateTime(row.updated_at),
              },
            ]}
          />
        )}
        total={displayRows.length}
        pagination={pagination as TablePaginationConfig}
        onPaginationChange={handlePaginationChange}
      />
    </>
  );
};
