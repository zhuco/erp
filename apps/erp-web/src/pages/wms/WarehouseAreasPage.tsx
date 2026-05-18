import { Button, Card, Descriptions, Drawer, Form, Input, InputNumber, Modal, Select, Space, Tabs, Tag, Typography, message } from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type {
  WarehouseArea,
  WarehouseAreaCreatePayload,
  WarehouseAreaListQuery,
  WarehouseAreaUpdatePayload,
} from '@/api/warehouseAreas';
import {
  createWarehouseArea,
  deleteWarehouseArea,
  listWarehouseAreas,
  updateWarehouseArea,
} from '@/api/warehouseAreas';
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
import { UiModule05DetailPanel } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';
import {
  buildStatsPieBackground,
  normalizeSelectionKeys,
  renderDirectoryBottomTable,
  sliceRowsByPagination,
  trimText,
  type DirectoryBottomDetailRow,
} from '@/pages/shared/directoryWorkbenchUtils';

type WarehouseAreaFormValues = {
  warehouse_id: string;
  code: string;
  name: string;
  area_type?: string;
  status?: string;
  priority?: number;
};

type WarehouseAreasGridFilters = {
  warehouse_id: string;
  code: string;
  area_type: string;
};

type WarehouseAreaLeftFilterKey =
  | 'view:all'
  | 'status:ACTIVE'
  | 'status:DISABLED'
  | 'type:STORAGE'
  | 'type:PICKING'
  | 'type:RECEIVING'
  | 'type:SHIPPING';

const pageSizeOptions = ['20', '50', '100'];

const areaTypeOptions = [
  { label: '存储', value: 'STORAGE' },
  { label: '拣货', value: 'PICKING' },
  { label: '收货', value: 'RECEIVING' },
  { label: '发货', value: 'SHIPPING' },
];

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '禁用', value: 'DISABLED' },
];

const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

const renderAreaType = (value?: string | null) => {
  const match = areaTypeOptions.find((item) => item.value === value);
  return match?.label ?? value ?? '-';
};

const createDefaultGridFilters = (): WarehouseAreasGridFilters => ({
  warehouse_id: '',
  code: '',
  area_type: '',
});

const buildGridFilterModel = (filters: WarehouseAreasGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.warehouse_id)) {
    model.warehouse_id = { filterType: 'text', type: 'contains', filter: trimText(filters.warehouse_id) };
  }
  if (trimText(filters.code)) {
    model.code = { filterType: 'text', type: 'contains', filter: trimText(filters.code) };
  }
  if (trimText(filters.area_type)) {
    model.area_type = { filterType: 'text', type: 'contains', filter: trimText(filters.area_type) };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): WarehouseAreasGridFilters => ({
  warehouse_id:
    model?.warehouse_id && typeof model.warehouse_id === 'object' && 'filter' in model.warehouse_id
      ? trimText(String(model.warehouse_id.filter ?? ''))
      : '',
  code:
    model?.code && typeof model.code === 'object' && 'filter' in model.code
      ? trimText(String(model.code.filter ?? ''))
      : '',
  area_type:
    model?.area_type && typeof model.area_type === 'object' && 'filter' in model.area_type
      ? trimText(String(model.area_type.filter ?? ''))
      : '',
});

export const WarehouseAreasPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<WarehouseAreaFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<WarehouseAreaLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<WarehouseAreasGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingArea, setEditingArea] = useState<WarehouseArea | null>(null);
  const [activeAreaId, setActiveAreaId] = useState<string | null>(null);
  const [selectedAreaIds, setSelectedAreaIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantReady ? `wms.warehouse-areas.pagination.${tenantId}` : undefined,
  });

  const queryPayload = useMemo<WarehouseAreaListQuery>(
    () => ({
      keyword: trimText(searchKeyword) || undefined,
      offset: 0,
      limit: 500,
    }),
    [searchKeyword],
  );

  const query = useQuery({
    queryKey: ['warehouse-areas', tenantId, queryPayload],
    queryFn: () => listWarehouseAreas(tenantId, queryPayload),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData,
  });

  const saveMutation = useMutation({
    mutationFn: async (values: WarehouseAreaFormValues) => {
      if (editingArea) {
        const payload: WarehouseAreaUpdatePayload = {
          name: values.name.trim(),
          area_type: values.area_type,
          status: values.status,
          priority: values.priority ?? 0,
        };
        return updateWarehouseArea(tenantId, editingArea.area_id, payload);
      }
      const payload: WarehouseAreaCreatePayload = {
        warehouse_id: values.warehouse_id.trim(),
        code: values.code.trim(),
        name: values.name.trim(),
        area_type: values.area_type,
        status: values.status,
        priority: values.priority ?? 0,
      };
      return createWarehouseArea(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingArea ? '库区已更新' : '库区已创建');
      setDrawerOpen(false);
      setEditingArea(null);
      await queryClient.invalidateQueries({ queryKey: ['warehouse-areas', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存库区失败');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (areaIds: string[]) => {
      await Promise.all(areaIds.map((areaId) => deleteWarehouseArea(tenantId, areaId)));
    },
    onSuccess: async (_data, areaIds) => {
      message.success(`已删除 ${areaIds.length} 个库区`);
      setSelectedAreaIds([]);
      await queryClient.invalidateQueries({ queryKey: ['warehouse-areas', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '删除库区失败');
    },
  });

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);
  const sourceRows = useMemo(() => query.data?.items ?? [], [query.data]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部库区', value: sourceRows.length, color: '#2f6fed' },
      {
        key: 'active',
        label: '启用',
        value: sourceRows.filter((row) => row.status === 'ACTIVE').length,
        color: '#0f9d58',
      },
      {
        key: 'storage',
        label: '存储区',
        value: sourceRows.filter((row) => row.area_type === 'STORAGE').length,
        color: '#d48806',
      },
    ],
    [sourceRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部库区',
        count: sourceRows.length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'status:ACTIVE',
        label: '启用中',
        count: sourceRows.filter((row) => row.status === 'ACTIVE').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'status:ACTIVE',
      },
      {
        key: 'status:DISABLED',
        label: '已禁用',
        count: sourceRows.filter((row) => row.status === 'DISABLED').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'status:DISABLED',
      },
      {
        key: 'type:STORAGE',
        label: '存储区',
        count: sourceRows.filter((row) => row.area_type === 'STORAGE').length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:STORAGE',
      },
      {
        key: 'type:PICKING',
        label: '拣货区',
        count: sourceRows.filter((row) => row.area_type === 'PICKING').length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:PICKING',
      },
      {
        key: 'type:RECEIVING',
        label: '收货区',
        count: sourceRows.filter((row) => row.area_type === 'RECEIVING').length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:RECEIVING',
      },
      {
        key: 'type:SHIPPING',
        label: '发货区',
        count: sourceRows.filter((row) => row.area_type === 'SHIPPING').length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:SHIPPING',
      },
    ],
    [leftFilterKey, sourceRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return sourceRows.filter((row) => {
      if (leftFilterKey === 'status:ACTIVE' && row.status !== 'ACTIVE') return false;
      if (leftFilterKey === 'status:DISABLED' && row.status !== 'DISABLED') return false;
      if (leftFilterKey.startsWith('type:') && row.area_type !== leftFilterKey.replace('type:', '')) return false;
      if (!keyword) {
        return true;
      }
      return [row.code, row.name, row.warehouse_id]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword));
    });
  }, [leftFilterKey, leftKeyword, sourceRows]);

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (
          trimText(gridFilters.warehouse_id) &&
          !row.warehouse_id.toLowerCase().includes(trimText(gridFilters.warehouse_id).toLowerCase())
        ) {
          return false;
        }
        if (
          trimText(gridFilters.code) &&
          !row.code.toLowerCase().includes(trimText(gridFilters.code).toLowerCase())
        ) {
          return false;
        }
        if (
          trimText(gridFilters.area_type) &&
          !row.area_type.toLowerCase().includes(trimText(gridFilters.area_type).toLowerCase())
        ) {
          return false;
        }
        return true;
      }),
    [gridFilters, leftFilteredRows],
  );

  const pageRows = useMemo(
    () => sliceRowsByPagination(displayRows, pagination),
    [displayRows, pagination],
  );

  useEffect(() => {
    setActiveAreaId((previous) =>
      previous && pageRows.some((row) => row.area_id === previous) ? previous : pageRows[0]?.area_id ?? null,
    );
    setSelectedAreaIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.area_id === item)),
    );
  }, [pageRows]);

  const activeArea = useMemo(
    () => pageRows.find((row) => row.area_id === activeAreaId) ?? null,
    [activeAreaId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<WarehouseArea>[]>(
    () => [
      {
        key: 'code',
        field: 'code',
        title: '库区编码',
        width: 150,
        minWidth: 120,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'name',
        field: 'name',
        title: '库区名称',
        width: 180,
        minWidth: 150,
      },
      {
        key: 'warehouse_id',
        field: 'warehouse_id',
        title: '仓库ID',
        width: 160,
        minWidth: 140,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'area_type',
        field: 'area_type',
        title: '类型',
        width: 120,
        minWidth: 100,
        renderCell: (value) => renderAreaType(String(value ?? '')),
      },
      {
        key: 'status',
        field: 'status',
        title: '状态',
        width: 100,
        minWidth: 90,
        renderCell: (value) =>
          value === 'DISABLED' ? <Tag color="red">禁用</Tag> : <Tag color="green">启用</Tag>,
      },
      {
        key: 'priority',
        field: 'priority',
        title: '优先级',
        width: 90,
        minWidth: 80,
      },
      {
        key: 'updated_at',
        field: 'updated_at',
        title: '更新时间',
        width: 170,
        minWidth: 160,
        renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at),
      },
    ],
    [],
  );

  const openCreateDrawer = useCallback(() => {
    setEditingArea(null);
    form.resetFields();
    form.setFieldsValue({
      area_type: 'STORAGE',
      status: 'ACTIVE',
      priority: 0,
    });
    setDrawerOpen(true);
  }, [form]);

  const openEditDrawer = useCallback(
    (area: WarehouseArea) => {
      setEditingArea(area);
      form.setFieldsValue({
        warehouse_id: area.warehouse_id,
        code: area.code,
        name: area.name,
        area_type: area.area_type,
        status: area.status,
        priority: area.priority,
      });
      setDrawerOpen(true);
    },
    [form],
  );

  const handleSave = useCallback(async () => {
    try {
      const values = await form.validateFields();
      await saveMutation.mutateAsync(values);
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    }
  }, [form, saveMutation]);

  const handleDelete = useCallback(
    async (areaIds: string[]) => {
      if (!areaIds.length) {
        message.warning('请先选择要删除的库区');
        return;
      }
      await deleteMutation.mutateAsync(areaIds);
    },
    [deleteMutation],
  );

  const targetIds = selectedAreaIds.length > 0 ? selectedAreaIds : activeAreaId ? [activeAreaId] : [];
  const targetArea = pageRows.find((row) => row.area_id === targetIds[0]) ?? activeArea;

  const rightPanel = activeArea ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'ops']}
      panels={[
        {
          key: 'detail',
          label: '库区详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="库区编码">{activeArea.code}</Descriptions.Item>
              <Descriptions.Item label="库区名称">{activeArea.name}</Descriptions.Item>
              <Descriptions.Item label="仓库ID">{activeArea.warehouse_id}</Descriptions.Item>
              <Descriptions.Item label="类型">{renderAreaType(activeArea.area_type)}</Descriptions.Item>
              <Descriptions.Item label="状态">
                {activeArea.status === 'ACTIVE' ? '启用' : '禁用'}
              </Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'ops',
          label: '操作约束',
          children: (
            <Typography.Text type="secondary">
              库区目录已切换到统一工业工作台结构，左侧突出状态和类型筛选，主表、右侧详情与底部明细保持与订单审核一致的工业密度。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="库区详情">
      <Typography.Text type="secondary">请选择一条库区记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeArea
    ? [
        { key: 'code', field: '库区编码', value: activeArea.code },
        { key: 'name', field: '库区名称', value: activeArea.name },
        { key: 'warehouse', field: '仓库ID', value: activeArea.warehouse_id },
      ]
    : [];

  const statusRows: DirectoryBottomDetailRow[] = activeArea
    ? [
        { key: 'type', field: '类型', value: renderAreaType(activeArea.area_type) },
        { key: 'status', field: '状态', value: activeArea.status === 'ACTIVE' ? '启用' : '禁用' },
        { key: 'priority', field: '优先级', value: String(activeArea.priority) },
      ]
    : [];

  const timelineRows: DirectoryBottomDetailRow[] = activeArea
    ? [
        { key: 'created', field: '创建时间', value: formatDateTime(activeArea.created_at) },
        { key: 'updated', field: '更新时间', value: formatDateTime(activeArea.updated_at ?? activeArea.created_at) },
        { key: 'selected', field: '当前勾选数', value: String(selectedAreaIds.length) },
      ]
    : [];

  const bottomPanel = (
    <Tabs
      size="small"
      type="card"
      activeKey={activeBottomTab}
      onChange={setActiveBottomTab}
      items={[
        {
          key: 'profile',
          label: `基础资料 (${profileRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeArea),
            profileRows,
            '暂无基础资料',
            `warehouse-area-profile-${activeArea?.area_id ?? 'empty'}`,
          ),
        },
        {
          key: 'status',
          label: `状态配置 (${statusRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeArea),
            statusRows,
            '暂无状态配置',
            `warehouse-area-status-${activeArea?.area_id ?? 'empty'}`,
          ),
        },
        {
          key: 'timeline',
          label: `维护上下文 (${timelineRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeArea),
            timelineRows,
            '暂无维护上下文',
            `warehouse-area-timeline-${activeArea?.area_id ?? 'empty'}`,
          ),
        },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="warehouse-areas-workbench-page"
        className="order-review-page warehouse-areas-directory-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="warehouse-areas-left-panel"
            dataTestId="warehouse-areas-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as WarehouseAreaLeftFilterKey | undefined;
              if (!nextKey) {
                return;
              }
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems)}
            globalOrderTotal={sourceRows.length}
            statsScopeText="口径：当前库区目录查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">库区检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按编码 / 名称 / 仓库缩小左侧结果"
                    onChange={(event) => {
                      resetPagination();
                      setLeftKeyword(event.target.value);
                    }}
                  />
                </div>
                <Button
                  size="small"
                  onClick={() => {
                    resetPagination();
                    setLeftKeyword('');
                    setLeftFilterKey('view:all');
                  }}
                >
                  重置左侧筛选
                </Button>
              </Space>
            }
          />
        }
        leftPanelVisible={true}
        rightPanel={rightPanel}
        rightPanelVisible={true}
        rightPanelSyncKey={activeArea?.area_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={
              <Space size={8} wrap>
                <Button type="primary" size="small" disabled={!tenantReady || !canWrite} onClick={openCreateDrawer}>
                  新建库区
                </Button>
                <Button
                  size="small"
                  disabled={!tenantReady || !canWrite || targetIds.length !== 1 || !targetArea}
                  onClick={() => targetArea && openEditDrawer(targetArea)}
                >
                  编辑
                </Button>
                <Button
                  size="small"
                  danger
                  disabled={!tenantReady || !canWrite || targetIds.length === 0}
                  loading={deleteMutation.isPending}
                  onClick={() => {
                    void Modal.confirm({
                      title: `确认删除选中的 ${targetIds.length} 个库区吗？`,
                      okText: '删除',
                      cancelText: '取消',
                      onOk: async () => handleDelete(targetIds),
                    });
                  }}
                >
                  删除
                </Button>
              </Space>
            }
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => {
              setSearchKeyword(trimText(searchInput));
              resetPagination();
            }}
            searchPlaceholder="搜索库区编码 / 名称 / 仓库ID"
            onExport={() => message.info('库区目录导出能力待接入')}
            onPrint={() => message.info('库区目录打印能力待接入')}
            onHelp={() => message.info('库区目录帮助文档待接入')}
          />
        }
        tableContent={
          <ErpDataGridNext<WarehouseArea>
            rowKey={(record) => record.area_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="wms.warehouse-areas.main-grid"
            dataTestId="warehouse-areas-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={query.isLoading || query.isFetching}
            error={query.error}
            onRetry={() => {
              void query.refetch();
            }}
            errorTitle="加载库区失败"
            emptyText="暂无库区"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedAreaIds}
            activeRowKey={activeAreaId}
            onSelectionChange={(keys) => {
              setSelectedAreaIds(normalizeSelectionKeys(keys));
            }}
            onActiveRowChange={(record) => {
              setActiveAreaId(record.area_id);
            }}
            onRowDoubleClick={(record) => {
              if (canWrite) {
                openEditDrawer(record);
              }
            }}
          />
        }
        footerSummary={
          <>
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedAreaIds.length} 条`}</span>
            <span>{activeArea ? `当前焦点：${activeArea.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`warehouse-areas-${displayRows.length}-${pageRows.length}-${selectedAreaIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingArea ? '编辑库区' : '新建库区'}
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
        <Form form={form} layout="vertical">
          <Form.Item
            name="warehouse_id"
            label="仓库ID"
            rules={[{ required: true, message: '请输入仓库ID' }]}
          >
            <Input disabled={Boolean(editingArea)} />
          </Form.Item>
          <Form.Item name="code" label="库区编码" rules={[{ required: true, message: '请输入库区编码' }]}>
            <Input disabled={Boolean(editingArea)} />
          </Form.Item>
          <Form.Item name="name" label="库区名称" rules={[{ required: true, message: '请输入库区名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="area_type" label="类型">
            <Select options={areaTypeOptions} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="priority" label="优先级">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
