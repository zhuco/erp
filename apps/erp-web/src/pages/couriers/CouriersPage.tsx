import {
  Button,
  Card,
  Descriptions,
  Drawer,
  Empty,
  Form,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  Typography,
  message,
} from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type { Courier, CourierCreatePayload } from '@/api/couriers';
import { createCourier, deleteCourier, listCouriers, updateCourier } from '@/api/couriers';
import type { ErpDataGridColumn } from '@/components/ErpDataGrid';
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
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';
import { useLeftFilterModule } from '@/pages/orders/shared/useLeftFilterModule';

type CourierFormValues = {
  name: string;
  code: string;
  status: string;
};

type CourierLeftFilters = {
  selectedFilterId: 'view:all' | 'status:ACTIVE' | 'status:DISABLED' | 'timeline:recent';
  keyword: string;
};

type CourierGridFilters = {
  nameKeyword: string;
  codeKeyword: string;
  status: 'ALL' | 'ACTIVE' | 'DISABLED';
  updatedAtFrom?: string;
  updatedAtTo?: string;
};

type CourierDetailRow = {
  key: string;
  field: string;
  value: string;
};

const pageSizeOptions = ['20', '50', '100', '200'];
const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' },
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const trimText = (value?: string | null) => String(value ?? '').trim();
const normalizeDateInput = (value?: string | null) => trimText(value).slice(0, 10);

const createDefaultCourierLeftFilters = (): CourierLeftFilters => ({
  selectedFilterId: 'view:all',
  keyword: '',
});

const createDefaultCourierGridFilters = (): CourierGridFilters => ({
  nameKeyword: '',
  codeKeyword: '',
  status: 'ALL',
  updatedAtFrom: '',
  updatedAtTo: '',
});

const normalizeSetValues = (values: Array<string | null | undefined>) =>
  Array.from(new Set(values.map((value) => trimText(value)).filter(Boolean)));

const buildCourierGridFilterModel = (filters: CourierGridFilters): FilterModel => {
  const model: FilterModel = {};

  if (trimText(filters.nameKeyword)) {
    model.name = {
      filterType: 'text',
      type: 'contains',
      filter: trimText(filters.nameKeyword),
    };
  }

  if (trimText(filters.codeKeyword)) {
    model.code = {
      filterType: 'text',
      type: 'contains',
      filter: trimText(filters.codeKeyword),
    };
  }

  if (filters.status !== 'ALL') {
    model.status = {
      filterType: 'set',
      values: [filters.status],
    };
  }

  const from = normalizeDateInput(filters.updatedAtFrom);
  const to = normalizeDateInput(filters.updatedAtTo);
  if (from && to) {
    model.updated_at = {
      filterType: 'date',
      type: 'inRange',
      dateFrom: `${from} 00:00:00`,
      dateTo: `${to} 23:59:59`,
    };
  } else if (from) {
    model.updated_at = {
      filterType: 'date',
      type: 'greaterThanOrEqual',
      dateFrom: `${from} 00:00:00`,
    };
  } else if (to) {
    model.updated_at = {
      filterType: 'date',
      type: 'lessThanOrEqual',
      dateTo: `${to} 23:59:59`,
    };
  }

  return model;
};

const extractCourierGridFilterModel = (model: FilterModel | null | undefined): CourierGridFilters => {
  const statusValues =
    model?.status && typeof model.status === 'object' && 'values' in model.status && Array.isArray(model.status.values)
      ? normalizeSetValues(model.status.values as string[])
      : [];

  return {
    nameKeyword:
      model?.name && typeof model.name === 'object' && 'filter' in model.name
        ? trimText(String(model.name.filter ?? ''))
        : '',
    codeKeyword:
      model?.code && typeof model.code === 'object' && 'filter' in model.code
        ? trimText(String(model.code.filter ?? ''))
        : '',
    status: (statusValues[0] as CourierGridFilters['status'] | undefined) ?? 'ALL',
    updatedAtFrom:
      model?.updated_at && typeof model.updated_at === 'object' && 'dateFrom' in model.updated_at
        ? normalizeDateInput(String(model.updated_at.dateFrom ?? ''))
        : '',
    updatedAtTo:
      model?.updated_at && typeof model.updated_at === 'object' && 'dateTo' in model.updated_at
        ? normalizeDateInput(String(model.updated_at.dateTo ?? ''))
        : '',
  };
};

const filterCouriersByKeyword = (rows: Courier[], keyword: string) => {
  const normalized = trimText(keyword).toLowerCase();
  if (!normalized) {
    return rows;
  }
  return rows.filter((row) =>
    [row.name, row.code, row.status]
      .map((value) => trimText(value).toLowerCase())
      .some((value) => value.includes(normalized)),
  );
};

const filterCouriersByLeftFilters = (rows: Courier[], filters: CourierLeftFilters) => {
  const normalized = trimText(filters.keyword).toLowerCase();
  return rows.filter((row) => {
    if (filters.selectedFilterId === 'status:ACTIVE' && row.status !== 'ACTIVE') {
      return false;
    }
    if (filters.selectedFilterId === 'status:DISABLED' && row.status !== 'DISABLED') {
      return false;
    }
    if (filters.selectedFilterId === 'timeline:recent') {
      const timestamp = Date.parse(String(row.updated_at ?? row.created_at ?? ''));
      if (!Number.isFinite(timestamp) || Date.now() - timestamp > 7 * 24 * 60 * 60 * 1000) {
        return false;
      }
    }
    if (!normalized) {
      return true;
    }
    return [row.name, row.code, row.status]
      .map((value) => trimText(value).toLowerCase())
      .some((value) => value.includes(normalized));
  });
};

const filterCouriersByGridFilters = (rows: Courier[], filters: CourierGridFilters) => {
  const nameKeyword = trimText(filters.nameKeyword).toLowerCase();
  const codeKeyword = trimText(filters.codeKeyword).toLowerCase();
  const from = normalizeDateInput(filters.updatedAtFrom);
  const to = normalizeDateInput(filters.updatedAtTo);

  return rows.filter((row) => {
    if (nameKeyword && !trimText(row.name).toLowerCase().includes(nameKeyword)) {
      return false;
    }
    if (codeKeyword && !trimText(row.code).toLowerCase().includes(codeKeyword)) {
      return false;
    }
    if (filters.status !== 'ALL' && row.status !== filters.status) {
      return false;
    }
    if (from || to) {
      const updatedAt = normalizeDateInput(row.updated_at ?? row.created_at);
      if (!updatedAt) {
        return false;
      }
      if (from && updatedAt < from) {
        return false;
      }
      if (to && updatedAt > to) {
        return false;
      }
    }
    return true;
  });
};

const buildCourierLeftNodes = (rows: Courier[]): OrderReviewAgLeftNode[] => {
  const recentCount = rows.filter((row) => {
    const timestamp = Date.parse(String(row.updated_at ?? row.created_at ?? ''));
    return Number.isFinite(timestamp) && Date.now() - timestamp <= 7 * 24 * 60 * 60 * 1000;
  }).length;

  return [
    {
      key: 'group:overview',
      label: '目录视图',
      nodeType: 'group',
      count: rows.length,
      children: [
        { key: 'view:all', label: '全部物流公司', nodeType: 'status', count: rows.length },
        {
          key: 'status:ACTIVE',
          label: '启用中',
          nodeType: 'status',
          count: rows.filter((row) => row.status === 'ACTIVE').length,
        },
        {
          key: 'status:DISABLED',
          label: '已停用',
          nodeType: 'status',
          count: rows.filter((row) => row.status === 'DISABLED').length,
        },
      ],
    },
    {
      key: 'group:timeline',
      label: '维护节奏',
      nodeType: 'group',
      count: rows.length,
      children: [
        { key: 'timeline:recent', label: '7日内有更新', nodeType: 'time', count: recentCount },
      ],
    },
  ];
};

const buildCourierPieBackground = (rows: Courier[]) => {
  const active = rows.filter((row) => row.status === 'ACTIVE').length;
  const disabled = rows.filter((row) => row.status === 'DISABLED').length;
  const total = rows.length || 1;
  const activeEnd = (active / total) * 360;
  const disabledEnd = activeEnd + (disabled / total) * 360;
  return `conic-gradient(#2f6fed 0deg ${activeEnd.toFixed(2)}deg, #d0d7de ${activeEnd.toFixed(
    2,
  )}deg ${disabledEnd.toFixed(2)}deg, #eef2f7 ${disabledEnd.toFixed(2)}deg 360deg)`;
};

export const CouriersPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('couriers.write');
  const queryClient = useQueryClient();
  const [editorForm] = Form.useForm<CourierFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingCourier, setEditingCourier] = useState<Courier | null>(null);
  const [activeCourierId, setActiveCourierId] = useState<string | null>(null);
  const [selectedCourierIds, setSelectedCourierIds] = useState<string[]>([]);
  const [leftFilters, setLeftFilters] = useState<CourierLeftFilters>(createDefaultCourierLeftFilters);
  const [gridFilters, setGridFilters] = useState<CourierGridFilters>(createDefaultCourierGridFilters);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const gridFilterModel = useMemo(() => buildCourierGridFilterModel(gridFilters), [gridFilters]);

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantId ? `couriers.directory.pagination.v1.${tenantId}` : undefined,
  });

  const query = useMemo(
    () => ({
      keyword: searchKeyword || undefined,
      offset: 0,
      limit: 500,
    }),
    [searchKeyword],
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['couriers', tenantId, query],
    queryFn: () => listCouriers(tenantId, query),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const sourceRows = data?.items ?? [];
  const searchScopedRows = useMemo(
    () => filterCouriersByKeyword(sourceRows, searchKeyword),
    [searchKeyword, sourceRows],
  );

  const courierStats = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部物流', value: searchScopedRows.length, color: '#2f6fed' },
      {
        key: 'active',
        label: '启用',
        value: searchScopedRows.filter((row) => row.status === 'ACTIVE').length,
        color: '#0f9d58',
      },
      {
        key: 'disabled',
        label: '停用',
        value: searchScopedRows.filter((row) => row.status === 'DISABLED').length,
        color: '#d0d7de',
      },
    ],
    [searchScopedRows],
  );

  const { filteredRows, leftPanel } = useLeftFilterModule<Courier, CourierLeftFilters>({
    rows: searchScopedRows,
    filters: leftFilters,
    onFiltersChange: (action) =>
      setLeftFilters((previous) => (typeof action === 'function' ? action(previous) : action)),
    initialExpandedKeys: ['group:overview', 'group:timeline'],
    emptyRowLabel: '没有可用筛选项',
    hasManualFilter: (filters) =>
      filters.selectedFilterId !== 'view:all' || Boolean(trimText(filters.keyword)),
    filterRows: filterCouriersByLeftFilters,
    buildCheckedKeys: (filters) => new Set<string>([filters.selectedFilterId]),
    buildLeftNodes: buildCourierLeftNodes,
    reduceFiltersByNode: (previous, node) => ({
      ...previous,
      selectedFilterId:
        previous.selectedFilterId === node.key
          ? 'view:all'
          : (node.key as CourierLeftFilters['selectedFilterId']),
    }),
    onBeforeLeafFilterApply: resetPagination,
    renderPanel: ({
      leftTab,
      onLeftTabChange,
      leftRowsForRender,
      onLeftRowClick,
      hasManualFilterActive,
    }) => (
      <WorkbenchLeftPanelTemplate
        className="couriers-left-panel"
        dataTestId="couriers-left-panel"
        leftTab={leftTab}
        onLeftTabChange={onLeftTabChange}
        leftRowsForRender={leftRowsForRender as WorkbenchLeftRenderRow[]}
        onLeftRowClick={onLeftRowClick as (event: CellClickedEvent<WorkbenchLeftRenderRow>) => void}
        leftStatsItems={courierStats}
        leftStatsPieBackground={buildCourierPieBackground(searchScopedRows)}
        globalOrderTotal={searchScopedRows.length}
        statsScopeText={
          isFetching
            ? '统计更新中...'
            : hasManualFilterActive
              ? '口径：当前查询结果叠加左侧筛选后的物流公司'
              : '口径：当前查询结果中的全部物流公司'
        }
        screenContent={
          <Space direction="vertical" size={10} style={{ width: '100%' }}>
            <div>
              <Typography.Text type="secondary">公司检索</Typography.Text>
              <Input
                size="small"
                allowClear
                value={leftFilters.keyword}
                placeholder="筛选名称 / 编码 / 状态"
                onChange={(event) => {
                  resetPagination();
                  setLeftFilters((previous) => ({ ...previous, keyword: event.target.value }));
                }}
              />
            </div>
            <Button
              size="small"
              onClick={() => {
                resetPagination();
                setLeftFilters(createDefaultCourierLeftFilters());
              }}
            >
              重置左侧筛选
            </Button>
          </Space>
        }
      />
    ),
  });

  const gridFilteredRows = useMemo(
    () => filterCouriersByGridFilters(filteredRows, gridFilters),
    [filteredRows, gridFilters],
  );

  const currentPage = Number(pagination.current ?? 1);
  const currentPageSize = Number(pagination.pageSize ?? 20);
  const couriers = useMemo(() => {
    const start = (currentPage - 1) * currentPageSize;
    return gridFilteredRows.slice(start, start + currentPageSize);
  }, [currentPage, currentPageSize, gridFilteredRows]);

  useEffect(() => {
    setActiveCourierId((previous) => {
      if (previous && couriers.some((item) => item.courier_id === previous)) {
        return previous;
      }
      return couriers[0]?.courier_id ?? null;
    });
    setSelectedCourierIds((previous) =>
      previous.filter((courierId) => couriers.some((item) => item.courier_id === courierId)),
    );
  }, [couriers]);

  const activeCourier = useMemo(
    () => couriers.find((item) => item.courier_id === activeCourierId) ?? null,
    [activeCourierId, couriers],
  );

  const saveMutation = useMutation({
    mutationFn: async (values: CourierFormValues) => {
      if (!tenantId) {
        throw new Error('请先选择租户');
      }

      const payload: CourierCreatePayload = {
        name: values.name.trim(),
        code: values.code.trim(),
        status: values.status,
      };

      if (editingCourier) {
        return updateCourier(tenantId, editingCourier.courier_id, payload);
      }
      return createCourier(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingCourier ? '物流公司已更新' : '物流公司已创建');
      setDrawerOpen(false);
      setEditingCourier(null);
      await queryClient.invalidateQueries({ queryKey: ['couriers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存物流公司失败');
    },
  });

  const openCreateDrawer = useCallback(() => {
    setEditingCourier(null);
    editorForm.resetFields();
    editorForm.setFieldsValue({
      status: 'ACTIVE',
    });
    setDrawerOpen(true);
  }, [editorForm]);

  const openEditDrawer = useCallback(
    (courier: Courier) => {
      setEditingCourier(courier);
      editorForm.setFieldsValue({
        name: courier.name,
        code: courier.code,
        status: courier.status,
      });
      setDrawerOpen(true);
    },
    [editorForm],
  );

  const handleSave = useCallback(async () => {
    try {
      const values = await editorForm.validateFields();
      await saveMutation.mutateAsync(values);
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  }, [editorForm, saveMutation]);

  const handleSearch = useCallback(() => {
    setSearchKeyword(searchInput.trim());
    resetPagination();
  }, [resetPagination, searchInput]);

  const handleDeleteCouriers = useCallback(
    async (courierIds: string[]) => {
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      if (courierIds.length === 0) {
        message.warning('请先选择要删除的物流公司');
        return;
      }
      try {
        await Promise.all(courierIds.map((courierId) => deleteCourier(tenantId, courierId)));
        message.success(`已删除 ${courierIds.length} 个物流公司`);
        setSelectedCourierIds([]);
        await queryClient.invalidateQueries({ queryKey: ['couriers', tenantId] });
      } catch (err) {
        message.error(err instanceof Error ? err.message : '删除物流公司失败');
      }
    },
    [queryClient, tenantId],
  );

  const handleUpdateCourierStatus = useCallback(
    async (nextStatus: 'ACTIVE' | 'DISABLED') => {
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      const targetIds = selectedCourierIds.length > 0 ? selectedCourierIds : activeCourierId ? [activeCourierId] : [];
      if (targetIds.length === 0) {
        message.warning(nextStatus === 'ACTIVE' ? '请选择要启用的物流公司' : '请选择要停用的物流公司');
        return;
      }
      try {
        await Promise.all(
          targetIds.map((courierId) =>
            updateCourier(tenantId, courierId, {
              status: nextStatus,
            }),
          ),
        );
        message.success(nextStatus === 'ACTIVE' ? '已批量启用物流公司' : '已批量停用物流公司');
        await queryClient.invalidateQueries({ queryKey: ['couriers', tenantId] });
      } catch (err) {
        message.error(err instanceof Error ? err.message : '更新物流公司状态失败');
      }
    },
    [activeCourierId, queryClient, selectedCourierIds, tenantId],
  );

  const columns = useMemo<ErpDataGridColumn<Courier>[]>(
    () => [
      {
        key: 'name',
        field: 'name',
        title: '物流公司名称',
        width: 220,
        minWidth: 180,
        filter: 'agTextColumnFilter',
        filterParams: {
          filterOptions: ['contains'],
          defaultOption: 'contains',
          maxNumConditions: 1,
        },
      },
      {
        key: 'code',
        field: 'code',
        title: '物流公司编码',
        width: 160,
        minWidth: 140,
        filter: 'agTextColumnFilter',
        filterParams: {
          filterOptions: ['contains'],
          defaultOption: 'contains',
          maxNumConditions: 1,
        },
      },
      {
        key: 'status',
        field: 'status',
        title: '状态',
        width: 100,
        minWidth: 90,
        filter: 'agSetColumnFilter',
        filterParams: {
          values: ['ACTIVE', 'DISABLED'],
        },
        renderCell: (value) => (
          <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>
            {value === 'ACTIVE' ? '启用' : '停用'}
          </Tag>
        ),
      },
      {
        key: 'updated_at',
        field: 'updated_at',
        title: '更新时间',
        width: 170,
        minWidth: 160,
        filter: 'agDateColumnFilter',
        filterParams: {
          filterOptions: ['inRange', 'greaterThanOrEqual', 'lessThanOrEqual'],
          defaultOption: 'inRange',
          maxNumConditions: 1,
        },
        filterValueGetter: (params) => {
          const raw = trimText(params.data?.updated_at ?? params.data?.created_at);
          if (!raw) {
            return null;
          }
          const value = new Date(raw);
          return Number.isNaN(value.getTime()) ? null : value;
        },
        renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at),
      },
    ],
    [],
  );

  const toolbarLeftContent = useMemo(() => {
    const targetIds = selectedCourierIds.length > 0 ? selectedCourierIds : activeCourierId ? [activeCourierId] : [];
    const targetCourier = couriers.find((item) => item.courier_id === targetIds[0]) ?? activeCourier;

    return (
      <Space size={8} wrap>
        <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>
          新建物流公司
        </Button>
        <Button
          size="small"
          disabled={!canWrite || targetIds.length !== 1 || !targetCourier}
          onClick={() => {
            if (targetCourier) {
              openEditDrawer(targetCourier);
            }
          }}
        >
          编辑
        </Button>
        <Button
          size="small"
          disabled={!canWrite || targetIds.length === 0}
          onClick={() => {
            void handleUpdateCourierStatus('ACTIVE');
          }}
        >
          启用
        </Button>
        <Button
          size="small"
          disabled={!canWrite || targetIds.length === 0}
          onClick={() => {
            void handleUpdateCourierStatus('DISABLED');
          }}
        >
          停用
        </Button>
        <Button
          size="small"
          danger
          disabled={!canWrite || targetIds.length === 0}
          onClick={() => {
            void Modal.confirm({
              title: `确认删除选中的 ${targetIds.length} 个物流公司吗？`,
              okText: '删除',
              cancelText: '取消',
              onOk: async () => handleDeleteCouriers(targetIds),
            });
          }}
        >
          删除
        </Button>
      </Space>
    );
  }, [
    activeCourier,
    activeCourierId,
    canWrite,
    couriers,
    handleDeleteCouriers,
    handleUpdateCourierStatus,
    openCreateDrawer,
    openEditDrawer,
    selectedCourierIds,
  ]);

  const rightPanel = useMemo(
    () =>
      activeCourier ? (
        <UiModule05DetailPanel
          defaultActiveKeys={['detail', 'ops']}
          panels={[
            {
              key: 'detail',
              label: '物流详情',
              children: (
                <Descriptions column={1} size="small">
                  <Descriptions.Item label="物流公司名称">{activeCourier.name}</Descriptions.Item>
                  <Descriptions.Item label="物流公司编码">{activeCourier.code}</Descriptions.Item>
                  <Descriptions.Item label="状态">
                    <Tag color={activeCourier.status === 'ACTIVE' ? 'green' : 'default'}>
                      {activeCourier.status === 'ACTIVE' ? '启用' : '停用'}
                    </Tag>
                  </Descriptions.Item>
                  <Descriptions.Item label="创建时间">
                    {formatDateTime(activeCourier.created_at)}
                  </Descriptions.Item>
                  <Descriptions.Item label="更新时间">
                    {formatDateTime(activeCourier.updated_at ?? activeCourier.created_at)}
                  </Descriptions.Item>
                </Descriptions>
              ),
            },
            {
              key: 'ops',
              label: '操作说明',
              children: (
                <Space direction="vertical" size={10} style={{ width: '100%' }}>
                  <Typography.Text type="secondary">
                    物流目录已切换到与订单审核一致的工业工作台骨架，主表、左侧筛选、底部明细和右侧详情保持统一交互。
                  </Typography.Text>
                  <Typography.Text type="secondary">
                    主表只负责检索与浏览，启用/停用等业务动作统一放在工具栏，避免行内控件打断表格节奏。
                  </Typography.Text>
                </Space>
              ),
            },
          ]}
        />
      ) : (
        <Card size="small" title="物流详情">
          <Typography.Text type="secondary">请选择一条物流记录查看详情与底部明细。</Typography.Text>
        </Card>
      ),
    [activeCourier],
  );

  const rightPanelSyncKey = useMemo(
    () =>
      JSON.stringify({
        activeCourierId: activeCourier?.courier_id ?? '',
        activeCourierStatus: activeCourier?.status ?? '',
        activeCourierUpdatedAt: activeCourier?.updated_at ?? '',
      }),
    [activeCourier?.courier_id, activeCourier?.status, activeCourier?.updated_at],
  );

  const bottomDetailColumns = useMemo(
    () => [
      { title: '字段', dataIndex: 'field', key: 'field', width: 180 },
      { title: '值', dataIndex: 'value', key: 'value', width: 320 },
    ],
    [],
  );

  const courierProfileRows = useMemo<CourierDetailRow[]>(
    () =>
      activeCourier
        ? [
            { key: 'name', field: '物流公司名称', value: activeCourier.name },
            { key: 'code', field: '物流公司编码', value: activeCourier.code },
            {
              key: 'status',
              field: '状态',
              value: activeCourier.status === 'ACTIVE' ? '启用' : '停用',
            },
          ]
        : [],
    [activeCourier],
  );

  const courierTimelineRows = useMemo<CourierDetailRow[]>(
    () =>
      activeCourier
        ? [
            {
              key: 'created_at',
              field: '创建时间',
              value: formatDateTime(activeCourier.created_at),
            },
            {
              key: 'updated_at',
              field: '更新时间',
              value: formatDateTime(activeCourier.updated_at ?? activeCourier.created_at),
            },
            {
              key: 'query_scope',
              field: '当前检索关键词',
              value: searchKeyword.trim() || '-',
            },
          ]
        : [],
    [activeCourier, searchKeyword],
  );

  const courierOpsRows = useMemo<CourierDetailRow[]>(
    () =>
      activeCourier
        ? [
            { key: 'edit', field: '编辑能力', value: canWrite ? '可编辑' : '只读' },
            { key: 'delete', field: '删除能力', value: canWrite ? '可删除' : '只读' },
            {
              key: 'status_change',
              field: '状态变更方式',
              value: canWrite ? '通过工具栏批量动作执行' : '只读',
            },
          ]
        : [],
    [activeCourier, canWrite],
  );

  const renderBottomTable = useCallback(
    (rows: CourierDetailRow[], emptyDescription: string, syncKey: string) =>
      activeCourier ? (
        <UiModule04BottomTable syncKey={syncKey}>
          <Table<CourierDetailRow>
            size="small"
            rowKey="key"
            columns={bottomDetailColumns}
            dataSource={rows}
            pagination={false}
            scroll={{ x: 'max-content' }}
            locale={{ emptyText: <Empty description={emptyDescription} /> }}
          />
        </UiModule04BottomTable>
      ) : (
        <Empty description="请选择物流公司查看底部明细" />
      ),
    [activeCourier, bottomDetailColumns],
  );

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        type="card"
        activeKey={activeBottomTab}
        onChange={setActiveBottomTab}
        items={[
          {
            key: 'profile',
            label: `基础资料 (${courierProfileRows.length})`,
            children: renderBottomTable(
              courierProfileRows,
              '暂无基础资料',
              `courier-profile-${activeCourier?.courier_id ?? 'empty'}-${courierProfileRows.length}`,
            ),
          },
          {
            key: 'timeline',
            label: `时间轨迹 (${courierTimelineRows.length})`,
            children: renderBottomTable(
              courierTimelineRows,
              '暂无时间轨迹',
              `courier-timeline-${activeCourier?.courier_id ?? 'empty'}-${courierTimelineRows.length}`,
            ),
          },
          {
            key: 'ops',
            label: `操作约束 (${courierOpsRows.length})`,
            children: renderBottomTable(
              courierOpsRows,
              '暂无操作约束信息',
              `courier-ops-${activeCourier?.courier_id ?? 'empty'}-${courierOpsRows.length}`,
            ),
          },
        ]}
      />
    ),
    [
      activeBottomTab,
      activeCourier?.courier_id,
      courierOpsRows,
      courierProfileRows,
      courierTimelineRows,
      renderBottomTable,
    ],
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="couriers-workbench-page"
        className="order-review-page couriers-page"
        leftPanel={leftPanel}
        leftPanelVisible={true}
        rightPanel={rightPanel}
        rightPanelVisible={true}
        rightPanelSyncKey={rightPanelSyncKey}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={toolbarLeftContent}
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={handleSearch}
            searchPlaceholder="搜索物流公司名称 / 编码"
            onExport={() => message.info('物流目录导出能力待接入')}
            onPrint={() => message.info('物流目录打印能力待接入')}
            onHelp={() => message.info('物流目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<Courier>
            rowKey={(record) => record.courier_id}
            columns={columns}
            rows={couriers}
            gridPreset="industrial-review-compact"
            loading={isLoading || isFetching}
            error={error}
            onRetry={() => refetch()}
            errorTitle="物流目录加载失败"
            emptyText="暂无物流公司数据"
            columnSettingsKey="couriers.directory.main-grid"
            dataTestId="couriers-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractCourierGridFilterModel(model));
            }}
            selectedRowKeys={selectedCourierIds}
            activeRowKey={activeCourierId}
            onSelectionChange={(keys) => {
              const normalized = Array.from(
                new Set(keys.map((key) => String(key ?? '').trim()).filter(Boolean)),
              );
              setSelectedCourierIds(normalized);
            }}
            onActiveRowChange={(record) => {
              setActiveCourierId(record.courier_id);
            }}
            onRowDoubleClick={(record) => {
              if (!canWrite) {
                return;
              }
              openEditDrawer(record);
            }}
          />
        }
        footerSummary={
          <>
            <span>{`总数 ${gridFilteredRows.length} 条，本页 ${couriers.length} 条，已选 ${selectedCourierIds.length} 条`}</span>
            <span>{activeCourier ? `当前焦点：${activeCourier.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`couriers-${currentPage}-${currentPageSize}-${gridFilteredRows.length}-${selectedCourierIds.length}`}
        total={gridFilteredRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingCourier ? '编辑物流公司' : '新建物流公司'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saveMutation.isPending} onClick={() => void handleSave()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={editorForm} layout="vertical">
          <Form.Item
            name="name"
            label="物流公司名称"
            rules={[{ required: true, message: '请输入物流公司名称' }]}
          >
            <Input placeholder="请输入物流公司名称" />
          </Form.Item>
          <Form.Item
            name="code"
            label="物流公司编码"
            rules={[{ required: true, message: '请输入物流公司编码' }]}
          >
            <Input placeholder="请输入物流公司编码" />
          </Form.Item>
          <Form.Item name="status" label="状态" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
