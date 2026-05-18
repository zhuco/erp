import { PlusOutlined } from '@ant-design/icons';
import {
  Button,
  Descriptions,
  Empty,
  Space,
  Tag,
  Tabs,
} from 'antd';
import type { CellClickedEvent } from 'ag-grid-community';
import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom';

import {
  getKitDetail,
  listKits,
  type KitComponent,
  type KitListQuery,
  type KitSummary,
} from '@/api/kits';
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
} from '@/pages/shared/directoryWorkbenchUtils';

import {
  buildGoodsKitCreatePath,
  buildGoodsKitEditPath,
  createGoodsEditSearchParams,
} from '../shared/goodsRoutes';
import { IndustrialTable } from '@/components/industrial';

const GOODS_KITS_QUERY_PATH = '/goods/kits/query';
const GOODS_PAGE_SIZE_OPTIONS = ['20', '50', '100'];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const parseKitLeftFilterKey = (value: string | null): 'view:all' | 'component:multi' =>
  value === 'component:multi' ? value : 'view:all';

const parsePositiveInteger = (value: string | null, fallback: number) => {
  const nextValue = Number.parseInt(String(value ?? ''), 10);
  return Number.isFinite(nextValue) && nextValue > 0 ? nextValue : fallback;
};

export const GoodsKitsQueryPage = () => {
  const navigate = useNavigate();
  const tenantId = useTenantId();
  const [searchParams, setSearchParams] = useSearchParams();
  const editKitSkuId = searchParams.get('edit');
  const initialKeyword = (searchParams.get('keyword') ?? '').trim();
  const initialLeftFilterKey = parseKitLeftFilterKey(searchParams.get('leftFilterKey'));
  const initialPage = parsePositiveInteger(searchParams.get('page'), 1);
  const initialPageSize = parsePositiveInteger(searchParams.get('pageSize'), 20);
  const initialActiveKitSkuId = (searchParams.get('activeId') ?? '').trim() || null;
  const [searchInput, setSearchInput] = useState(initialKeyword);
  const [searchKeyword, setSearchKeyword] = useState(initialKeyword);
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<'view:all' | 'component:multi'>(
    initialLeftFilterKey,
  );
  const [activeKitSkuId, setActiveKitSkuId] = useState<string | null>(initialActiveKitSkuId);

  const { pagination, setPagination, handlePaginationChange, resetPagination } =
    useWorkbenchTablePagination({
      defaultPageSize: 20,
      pageSizeOptions: GOODS_PAGE_SIZE_OPTIONS,
      storageKey: tenantId ? `goods.kits.query.pagination.${tenantId}` : undefined,
    });
  const legacyEditRedirectTarget = editKitSkuId
    ? buildGoodsKitEditPath(editKitSkuId, createGoodsEditSearchParams(searchParams.toString()))
    : null;

  useEffect(() => {
    if (legacyEditRedirectTarget) {
      return;
    }
    setSearchInput(initialKeyword);
    setSearchKeyword(initialKeyword);
    setLeftFilterKey(initialLeftFilterKey);
    setActiveKitSkuId(initialActiveKitSkuId);
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
    initialActiveKitSkuId,
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

    if (activeKitSkuId) {
      next.set('activeId', activeKitSkuId);
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
    activeKitSkuId,
    leftFilterKey,
    pagination.current,
    pagination.pageSize,
    searchKeyword,
    searchParams,
    setSearchParams,
    legacyEditRedirectTarget,
  ]);

  const query = useMemo<KitListQuery>(
    () => ({
      keyword: searchKeyword || undefined,
      offset: 0,
      limit: 500,
    }),
    [searchKeyword],
  );

  const kitsQuery = useQuery({
    queryKey: ['goods-kits-query', GOODS_KITS_QUERY_PATH, tenantId, query],
    queryFn: () => listKits(tenantId, query),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const rows = useMemo(() => kitsQuery.data?.items ?? [], [kitsQuery.data?.items]);

  const activeKit = useMemo(
    () => rows.find((row) => row.kit_sku_id === activeKitSkuId) ?? null,
    [activeKitSkuId, rows],
  );

  const detailQuery = useQuery({
    queryKey: ['goods-kit-detail-panel', tenantId, activeKit?.kit_sku_id],
    queryFn: () => getKitDetail(tenantId, activeKit?.kit_sku_id ?? ''),
    enabled: Boolean(tenantId && activeKit?.kit_sku_id),
  });

  useEffect(() => {
    const firstRow = rows[0];
    if (!activeKit && firstRow) {
      setActiveKitSkuId(firstRow.kit_sku_id);
    }
  }, [activeKit, rows]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部组合装', value: rows.length, color: '#2f6fed' },
      {
        key: 'multi',
        label: '多组件',
        value: rows.filter((row) => row.component_count >= 2).length,
        color: '#0f9d58',
      },
      {
        key: 'single',
        label: '单组件',
        value: rows.filter((row) => row.component_count < 2).length,
        color: '#d48806',
      },
    ],
    [rows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部组合装',
        count: rows.length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'component:multi',
        label: '多组件组合装',
        count: rows.filter((row) => row.component_count >= 2).length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'component:multi',
      },
    ],
    [leftFilterKey, rows],
  );

  const displayRows = useMemo(
    () =>
      rows.filter((row) => {
        if (leftFilterKey === 'component:multi' && row.component_count < 2) {
          return false;
        }
        return true;
      }),
    [leftFilterKey, rows],
  );

  const pagedRows = useMemo(
    () => sliceRowsByPagination(displayRows, pagination),
    [displayRows, pagination],
  );

  const detailPanels = useMemo(
    () => [
      {
        key: 'summary',
        label: '组合装摘要',
        children: activeKit ? (
          <Descriptions column={1} size="small" bordered>
            <Descriptions.Item label="KIT SKU">
              {activeKit.kit_sku_code}
            </Descriptions.Item>
            <Descriptions.Item label="组合装名称">
              {activeKit.kit_name ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="组件数量">
              <Tag color="blue">{activeKit.component_count}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="最近更新时间">
              {formatDateTime(activeKit.updated_at)}
            </Descriptions.Item>
          </Descriptions>
        ) : (
          <Empty description="请选择组合装查看摘要" />
        ),
      },
    ],
    [activeKit],
  );

  const bottomItems = useMemo(
    () => [
      {
        key: 'components',
        label: 'BOM 组件',
        children: detailQuery.data ? (
          <UiModule04BottomTable syncKey={detailQuery.data.kit_sku.sku_id}>
            <IndustrialTable<KitComponent>
              size="small"
              rowKey="component_sku_id"
              pagination={false}
              scroll={{ x: 'max-content' }}
              dataSource={detailQuery.data.components}
              columns={[
                {
                  title: '组件 SKU',
                  dataIndex: 'component_sku_code',
                  key: 'component_sku_code',
                  width: 180,
                },
                {
                  title: '组件名称',
                  dataIndex: 'component_name',
                  key: 'component_name',
                  width: 220,
                  render: (value) => value ?? '-',
                },
                {
                  title: '数量',
                  dataIndex: 'qty',
                  key: 'qty',
                  width: 100,
                },
                {
                  title: '效期时间',
                  dataIndex: 'expire_at',
                  key: 'expire_at',
                  width: 180,
                  render: (value) => formatDateTime(value),
                },
              ]}
            />
          </UiModule04BottomTable>
        ) : (
          <Empty description="请选择组合装查看 BOM 组件" />
        ),
      },
    ],
    [detailQuery.data],
  );

  if (legacyEditRedirectTarget) {
    return <Navigate replace to={legacyEditRedirectTarget} />;
  }

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="goods-kits-query-page"
        toolbar={(
          <WorkbenchToolbar
            leftContent={(
              <Space>
                <Button
                  type="primary"
                  icon={<PlusOutlined />}
                  onClick={() => navigate(buildGoodsKitCreatePath(searchParams.toString()))}
                >
                  新建组合装
                </Button>
                <Button
                  disabled={!activeKit}
                  onClick={() => {
                    if (!activeKit) {
                      return;
                    }
                    navigate(
                      buildGoodsKitEditPath(
                        activeKit.kit_sku_id,
                        searchParams.toString(),
                      ),
                    );
                  }}
                >
                  修改组合装
                </Button>
              </Space>
            )}
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => setSearchKeyword(searchInput.trim())}
            searchPlaceholder="搜索组合装 SKU、名称"
          />
        )}
        leftPanel={(
          <WorkbenchLeftPanelTemplate
            leftTab={leftTab}
            onLeftTabChange={(key) => setLeftTab(key)}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              setLeftFilterKey(
                (event.data?.key as 'view:all' | 'component:multi') ?? 'view:all',
              );
              resetPagination();
            }}
            screenContent={<Empty description="组合装查询暂不提供额外筛选面板" />}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(
              leftStatsItems.map((item) => ({
                value: item.value,
                color: item.color,
              })),
            )}
            globalOrderTotal={rows.length}
            statsScopeText="组合装主档实时统计"
          />
        )}
        leftPanelVisible
        rightPanel={<UiModule05DetailPanel panels={detailPanels} />}
        rightPanelVisible
        bottomPanel={
          <Tabs activeKey="components" items={bottomItems} />
        }
        bottomPanelVisible
        tableContent={(
          <ErpDataGridNext<KitSummary>
            rows={pagedRows}
            rowKey={(row) => row.kit_sku_id}
            dataTestId="goods-kits-grid"
            emptyText="暂无组合装数据"
            loading={kitsQuery.isLoading}
            error={kitsQuery.error}
            onRetry={() => kitsQuery.refetch()}
            activeRowKey={activeKit?.kit_sku_id ?? null}
            onActiveRowChange={(row) => setActiveKitSkuId(row.kit_sku_id)}
            onRowDoubleClick={(row) => {
              navigate(buildGoodsKitEditPath(row.kit_sku_id, searchParams.toString()));
            }}
            columns={[
              {
                key: 'kit_sku_code',
                field: 'kit_sku_code',
                header: '组合装 SKU',
                width: 160,
                render: (row) => row.kit_sku_code,
              },
              {
                key: 'kit_name',
                field: 'kit_name',
                header: '组合装名称',
                width: 240,
                render: (row) => row.kit_name ?? '-',
              },
              {
                key: 'component_count',
                field: 'component_count',
                header: '组件数',
                width: 120,
                render: (row) => <Tag color="blue">{row.component_count}</Tag>,
              },
              {
                key: 'updated_at',
                field: 'updated_at',
                header: '更新时间',
                width: 180,
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
