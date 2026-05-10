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

import type { Warehouse, WarehouseCreatePayload } from '@/api/warehouses';
import {
  createWarehouse,
  deleteWarehouse,
  listWarehouses,
  setDefaultWarehouse,
  updateWarehouse,
} from '@/api/warehouses';
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

type WarehouseFormValues = {
  name: string;
  code: string;
  status?: string;
  is_default?: boolean;
};

type WarehouseGridFilters = {
  name: string;
  code: string;
  status: 'ALL' | 'ACTIVE' | 'DISABLED';
};

type WarehouseLeftFilterKey = 'view:all' | 'status:ACTIVE' | 'status:DISABLED' | 'default:yes' | 'default:no';

type WarehouseDetailRow = {
  key: string;
  field: string;
  value: string;
};

const pageSizeOptions = ['20', '50', '100', '200'];
const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' },
];

const trimText = (value?: string | null) => String(value ?? '').trim();
const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const createDefaultGridFilters = (): WarehouseGridFilters => ({ name: '', code: '', status: 'ALL' });
const resolveTargetIds = (selected: string[], active: string | null) => (selected.length > 0 ? selected : active ? [active] : []);

const buildGridFilterModel = (filters: WarehouseGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  if (trimText(filters.code)) {
    model.code = { filterType: 'text', type: 'contains', filter: trimText(filters.code) };
  }
  if (filters.status !== 'ALL') {
    model.status = { filterType: 'set', values: [filters.status] };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): WarehouseGridFilters => {
  const statusValues =
    model?.status && typeof model.status === 'object' && 'values' in model.status && Array.isArray(model.status.values)
      ? model.status.values.map((value) => String(value ?? '').trim()).filter(Boolean)
      : [];
  return {
    name:
      model?.name && typeof model.name === 'object' && 'filter' in model.name
        ? trimText(String(model.name.filter ?? ''))
        : '',
    code:
      model?.code && typeof model.code === 'object' && 'filter' in model.code
        ? trimText(String(model.code.filter ?? ''))
        : '',
    status: (statusValues[0] as WarehouseGridFilters['status'] | undefined) ?? 'ALL',
  };
};

const buildPieBackground = (rows: Warehouse[]) => {
  const active = rows.filter((row) => row.status === 'ACTIVE').length;
  const disabled = rows.filter((row) => row.status === 'DISABLED').length;
  const total = rows.length || 1;
  const activeEnd = (active / total) * 360;
  const disabledEnd = activeEnd + (disabled / total) * 360;
  return `conic-gradient(#2f6fed 0deg ${activeEnd.toFixed(2)}deg, #d0d7de ${activeEnd.toFixed(2)}deg ${disabledEnd.toFixed(2)}deg, #eef2f7 ${disabledEnd.toFixed(2)}deg 360deg)`;
};

export const WarehouseDirectoryPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('warehouses.write');
  const queryClient = useQueryClient();
  const [editorForm] = Form.useForm<WarehouseFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<WarehouseLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<WarehouseGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingWarehouse, setEditingWarehouse] = useState<Warehouse | null>(null);
  const [activeWarehouseId, setActiveWarehouseId] = useState<string | null>(null);
  const [selectedWarehouseIds, setSelectedWarehouseIds] = useState<string[]>([]);
  const [settingDefaultId, setSettingDefaultId] = useState<string | null>(null);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantId ? `warehouses.directory.pagination.v2.${tenantId}` : undefined,
  });

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['warehouses', tenantId, searchKeyword],
    queryFn: () => listWarehouses(tenantId, { keyword: searchKeyword || undefined, offset: 0, limit: 500 }),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const searchScopedRows = data?.items ?? [];

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部仓库', value: searchScopedRows.length, color: '#2f6fed' },
      { key: 'active', label: '启用', value: searchScopedRows.filter((row) => row.status === 'ACTIVE').length, color: '#0f9d58' },
      { key: 'default', label: '默认仓', value: searchScopedRows.filter((row) => row.is_default).length, color: '#d48806' },
    ],
    [searchScopedRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      { key: 'view:all', label: '全部仓库', count: searchScopedRows.length, nodeType: 'warehouse', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'view:all' },
      { key: 'status:ACTIVE', label: '启用中', count: searchScopedRows.filter((row) => row.status === 'ACTIVE').length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'status:ACTIVE' },
      { key: 'status:DISABLED', label: '已停用', count: searchScopedRows.filter((row) => row.status === 'DISABLED').length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'status:DISABLED' },
      { key: 'default:yes', label: '默认仓', count: searchScopedRows.filter((row) => row.is_default).length, nodeType: 'warehouse', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'default:yes' },
      { key: 'default:no', label: '普通仓', count: searchScopedRows.filter((row) => !row.is_default).length, nodeType: 'warehouse', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'default:no' },
    ],
    [leftFilterKey, searchScopedRows],
  );

  const leftFilteredRows = useMemo(
    () =>
      searchScopedRows.filter((row) => {
        if (leftFilterKey === 'status:ACTIVE' && row.status !== 'ACTIVE') return false;
        if (leftFilterKey === 'status:DISABLED' && row.status !== 'DISABLED') return false;
        if (leftFilterKey === 'default:yes' && !row.is_default) return false;
        if (leftFilterKey === 'default:no' && row.is_default) return false;
        if (!trimText(leftKeyword)) return true;
        return [row.name, row.code, row.status]
          .map((value) => trimText(value).toLowerCase())
          .some((value) => value.includes(trimText(leftKeyword).toLowerCase()));
      }),
    [leftFilterKey, leftKeyword, searchScopedRows],
  );

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (trimText(gridFilters.name) && !trimText(row.name).toLowerCase().includes(trimText(gridFilters.name).toLowerCase())) {
          return false;
        }
        if (trimText(gridFilters.code) && !trimText(row.code).toLowerCase().includes(trimText(gridFilters.code).toLowerCase())) {
          return false;
        }
        if (gridFilters.status !== 'ALL' && row.status !== gridFilters.status) {
          return false;
        }
        return true;
      }),
    [gridFilters, leftFilteredRows],
  );

  const currentPage = Number(pagination.current ?? 1);
  const currentPageSize = Number(pagination.pageSize ?? 20);
  const warehouses = useMemo(() => {
    const start = (currentPage - 1) * currentPageSize;
    return displayRows.slice(start, start + currentPageSize);
  }, [currentPage, currentPageSize, displayRows]);

  useEffect(() => {
    setActiveWarehouseId((previous) => (previous && warehouses.some((item) => item.warehouse_id === previous) ? previous : warehouses[0]?.warehouse_id ?? null));
    setSelectedWarehouseIds((previous) => previous.filter((warehouseId) => warehouses.some((item) => item.warehouse_id === warehouseId)));
  }, [warehouses]);

  const activeWarehouse = useMemo(
    () => warehouses.find((item) => item.warehouse_id === activeWarehouseId) ?? null,
    [activeWarehouseId, warehouses],
  );

  const saveMutation = useMutation({
    mutationFn: async (values: WarehouseFormValues) => {
      if (!tenantId) throw new Error('请先选择租户');
      const payload: WarehouseCreatePayload = {
        name: values.name.trim(),
        code: values.code.trim(),
        status: values.status,
        is_default: Boolean(values.is_default),
      };
      if (editingWarehouse) {
        const updated = await updateWarehouse(tenantId, editingWarehouse.warehouse_id, payload);
        if (payload.is_default) {
          await setDefaultWarehouse(tenantId, editingWarehouse.warehouse_id);
        }
        return updated;
      }
      const created = await createWarehouse(tenantId, payload);
      if (payload.is_default) {
        await setDefaultWarehouse(tenantId, created.warehouse_id);
      }
      return created;
    },
    onSuccess: async () => {
      message.success(editingWarehouse ? '仓库已更新' : '仓库已创建');
      setDrawerOpen(false);
      setEditingWarehouse(null);
      await queryClient.invalidateQueries({ queryKey: ['warehouses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存仓库失败');
    },
  });

  const defaultMutation = useMutation({
    mutationFn: async (warehouseId: string) => {
      if (!tenantId) throw new Error('请先选择租户');
      return setDefaultWarehouse(tenantId, warehouseId);
    },
    onMutate: (warehouseId) => setSettingDefaultId(warehouseId),
    onSuccess: async () => {
      message.success('默认仓库已更新');
      await queryClient.invalidateQueries({ queryKey: ['warehouses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '设置默认仓库失败');
    },
    onSettled: () => setSettingDefaultId(null),
  });

  const openCreateDrawer = useCallback(() => {
    setEditingWarehouse(null);
    editorForm.resetFields();
    editorForm.setFieldsValue({ status: 'ACTIVE', is_default: false });
    setDrawerOpen(true);
  }, [editorForm]);

  const openEditDrawer = useCallback(
    (warehouse: Warehouse) => {
      setEditingWarehouse(warehouse);
      editorForm.setFieldsValue({
        name: warehouse.name,
        code: warehouse.code,
        status: warehouse.status ?? undefined,
        is_default: warehouse.is_default,
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

  const handleDeleteWarehouses = useCallback(
    async (warehouseIds: string[]) => {
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      if (warehouseIds.length === 0) {
        message.warning('请先选择要删除的仓库');
        return;
      }
      try {
        await Promise.all(warehouseIds.map((warehouseId) => deleteWarehouse(tenantId, warehouseId)));
        message.success(`已删除 ${warehouseIds.length} 个仓库`);
        setSelectedWarehouseIds([]);
        await queryClient.invalidateQueries({ queryKey: ['warehouses', tenantId] });
      } catch (err) {
        message.error(err instanceof Error ? err.message : '删除仓库失败');
      }
    },
    [queryClient, tenantId],
  );

  const handleSetDefaultWarehouse = useCallback(
    async (warehouseId: string | null) => {
      if (!warehouseId) {
        message.warning('请先选择一个仓库');
        return;
      }
      await defaultMutation.mutateAsync(warehouseId);
    },
    [defaultMutation],
  );

  const columns = useMemo<ErpDataGridColumn<Warehouse>[]>(
    () => [
      { key: 'name', field: 'name', title: '仓库名称', width: 220, minWidth: 180, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'code', field: 'code', title: '仓库编码', width: 160, minWidth: 140, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'is_default', field: 'is_default', title: '仓库类型', width: 120, renderCell: (value) => (value ? <Tag color="gold">默认仓</Tag> : <Tag>普通仓</Tag>) },
      { key: 'status', field: 'status', title: '状态', width: 100, filter: 'agSetColumnFilter', filterParams: { values: ['ACTIVE', 'DISABLED'] }, renderCell: (value) => <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value === 'ACTIVE' ? '启用' : '停用'}</Tag> },
      { key: 'updated_at', field: 'updated_at', title: '更新时间', width: 170, renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at) },
    ],
    [],
  );

  const toolbarLeftContent = useMemo(() => {
    const targetIds = resolveTargetIds(selectedWarehouseIds, activeWarehouseId);
    const targetWarehouse = warehouses.find((item) => item.warehouse_id === targetIds[0]) ?? activeWarehouse;
    return (
      <Space size={8} wrap>
        <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>新建仓库</Button>
        <Button size="small" disabled={!canWrite || targetIds.length !== 1 || !targetWarehouse} onClick={() => targetWarehouse && openEditDrawer(targetWarehouse)}>编辑</Button>
        <Button size="small" disabled={!canWrite || targetIds.length !== 1 || !targetWarehouse || Boolean(targetWarehouse?.is_default)} loading={Boolean(targetWarehouse?.warehouse_id) && settingDefaultId === targetWarehouse?.warehouse_id} onClick={() => void handleSetDefaultWarehouse(targetWarehouse?.warehouse_id ?? null)}>设为默认</Button>
        <Button size="small" danger disabled={!canWrite || targetIds.length === 0} onClick={() => { void Modal.confirm({ title: `确认删除选中的 ${targetIds.length} 个仓库吗？`, okText: '删除', cancelText: '取消', onOk: async () => handleDeleteWarehouses(targetIds) }); }}>删除</Button>
      </Space>
    );
  }, [activeWarehouse, activeWarehouseId, canWrite, handleDeleteWarehouses, handleSetDefaultWarehouse, openCreateDrawer, openEditDrawer, selectedWarehouseIds, settingDefaultId, warehouses]);

  const rightPanel = useMemo(() => {
    if (!activeWarehouse) {
      return (
        <Card size="small" title="仓库详情">
          <Typography.Text type="secondary">请选择一条仓库记录查看右侧详情。</Typography.Text>
        </Card>
      );
    }
    return (
      <UiModule05DetailPanel
        defaultActiveKeys={['detail', 'ops']}
        panels={[
          {
            key: 'detail',
            label: '仓库详情',
            children: (
              <Descriptions column={1} size="small">
                <Descriptions.Item label="仓库名称">{activeWarehouse.name}</Descriptions.Item>
                <Descriptions.Item label="仓库编码">{activeWarehouse.code}</Descriptions.Item>
                <Descriptions.Item label="仓库类型">{activeWarehouse.is_default ? '默认仓' : '普通仓'}</Descriptions.Item>
                <Descriptions.Item label="状态">{activeWarehouse.status === 'ACTIVE' ? '启用' : activeWarehouse.status === 'DISABLED' ? '停用' : '-'}</Descriptions.Item>
              </Descriptions>
            ),
          },
          {
            key: 'ops',
            label: '操作约束',
            children: (
              <Typography.Text type="secondary">
                默认仓、删除和编辑动作统一通过工具栏执行，主表保持紧凑浏览节奏。
              </Typography.Text>
            ),
          },
        ]}
      />
    );
  }, [activeWarehouse]);

  const bottomColumns = useMemo(
    () => [
      { title: '字段', dataIndex: 'field', key: 'field', width: 180 },
      { title: '值', dataIndex: 'value', key: 'value', width: 320 },
    ],
    [],
  );

  const profileRows: WarehouseDetailRow[] = activeWarehouse
    ? [
        { key: 'name', field: '仓库名称', value: activeWarehouse.name },
        { key: 'code', field: '仓库编码', value: activeWarehouse.code },
        { key: 'type', field: '仓库类型', value: activeWarehouse.is_default ? '默认仓' : '普通仓' },
      ]
    : [];
  const timelineRows: WarehouseDetailRow[] = activeWarehouse
    ? [
        { key: 'created_at', field: '创建时间', value: formatDateTime(activeWarehouse.created_at) },
        { key: 'updated_at', field: '更新时间', value: formatDateTime(activeWarehouse.updated_at ?? activeWarehouse.created_at) },
        { key: 'query', field: '当前检索关键字', value: searchKeyword.trim() || '-' },
      ]
    : [];
  const opsRows: WarehouseDetailRow[] = activeWarehouse
    ? [
        { key: 'edit', field: '编辑能力', value: canWrite ? '可编辑' : '只读' },
        { key: 'default', field: '默认仓动作', value: canWrite ? '通过工具栏执行' : '只读' },
        { key: 'delete', field: '删除能力', value: canWrite ? '可删除' : '只读' },
      ]
    : [];

  const renderBottomTable = (rows: WarehouseDetailRow[], emptyDescription: string, syncKey: string) =>
    activeWarehouse ? (
      <UiModule04BottomTable syncKey={syncKey}>
        <Table<WarehouseDetailRow> size="small" rowKey="key" columns={bottomColumns} dataSource={rows} pagination={false} scroll={{ x: 'max-content' }} locale={{ emptyText: <Empty description={emptyDescription} /> }} />
      </UiModule04BottomTable>
    ) : (
      <Empty description="请选择仓库查看底部明细" />
    );

  const bottomPanel = (
    <Tabs
      size="small"
      type="card"
      activeKey={activeBottomTab}
      onChange={setActiveBottomTab}
      items={[
        { key: 'profile', label: `基础资料 (${profileRows.length})`, children: renderBottomTable(profileRows, '暂无基础资料', `warehouse-profile-${activeWarehouse?.warehouse_id ?? 'empty'}`) },
        { key: 'timeline', label: `时间轨迹 (${timelineRows.length})`, children: renderBottomTable(timelineRows, '暂无时间轨迹', `warehouse-timeline-${activeWarehouse?.warehouse_id ?? 'empty'}`) },
        { key: 'ops', label: `操作约束 (${opsRows.length})`, children: renderBottomTable(opsRows, '暂无操作约束信息', `warehouse-ops-${activeWarehouse?.warehouse_id ?? 'empty'}`) },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="warehouse-directory-workbench-page"
        className="order-review-page warehouse-directory-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="warehouse-directory-left-panel"
            dataTestId="warehouse-directory-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as WarehouseLeftFilterKey | undefined;
              if (!nextKey) return;
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildPieBackground(searchScopedRows)}
            globalOrderTotal={searchScopedRows.length}
            statsScopeText="口径：当前查询结果中的仓库统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">仓库检索</Typography.Text>
                  <Input size="small" allowClear value={leftKeyword} placeholder="筛选名称 / 编码 / 状态" onChange={(event) => { resetPagination(); setLeftKeyword(event.target.value); }} />
                </div>
                <Button size="small" onClick={() => { resetPagination(); setLeftKeyword(''); setLeftFilterKey('view:all'); }}>
                  重置左侧筛选
                </Button>
              </Space>
            }
          />
        }
        leftPanelVisible={true}
        rightPanel={rightPanel}
        rightPanelVisible={true}
        rightPanelSyncKey={activeWarehouse?.warehouse_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={toolbarLeftContent}
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => { setSearchKeyword(searchInput.trim()); resetPagination(); }}
            searchPlaceholder="搜索仓库名称 / 编码"
            onExport={() => message.info('仓库目录导出能力待接入')}
            onPrint={() => message.info('仓库目录打印能力待接入')}
            onHelp={() => message.info('仓库目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<Warehouse>
            rowKey={(record) => record.warehouse_id}
            columns={columns}
            rows={warehouses}
            gridPreset="industrial-review-compact"
            loading={isLoading || isFetching}
            error={error}
            onRetry={() => refetch()}
            errorTitle="仓库目录加载失败"
            emptyText="暂无仓库数据"
            columnSettingsKey="warehouses.directory.main-grid"
            dataTestId="warehouse-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => { resetPagination(); setGridFilters(extractGridFilterModel(model)); }}
            selectedRowKeys={selectedWarehouseIds}
            activeRowKey={activeWarehouseId}
            onSelectionChange={(keys) => setSelectedWarehouseIds(Array.from(new Set(keys.map((key) => String(key ?? '').trim()).filter(Boolean))))}
            onActiveRowChange={(record) => setActiveWarehouseId(record.warehouse_id)}
            onRowDoubleClick={(record) => { if (canWrite) openEditDrawer(record); }}
          />
        }
        footerSummary={
          <>
            <span>{`总数 ${displayRows.length} 条，本页 ${warehouses.length} 条，已选 ${selectedWarehouseIds.length} 条`}</span>
            <span>{activeWarehouse ? `当前焦点：${activeWarehouse.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`warehouses-${currentPage}-${currentPageSize}-${displayRows.length}-${selectedWarehouseIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingWarehouse ? '编辑仓库' : '新建仓库'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saveMutation.isPending} disabled={!canWrite} onClick={() => void handleSave()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={editorForm} layout="vertical">
          <Form.Item name="name" label="仓库名称" rules={[{ required: true, message: '请输入仓库名称' }]}>
            <Input placeholder="请输入仓库名称" />
          </Form.Item>
          <Form.Item name="code" label="仓库编码" rules={[{ required: true, message: '请输入仓库编码' }]}>
            <Input placeholder="请输入仓库编码" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="请选择状态" />
          </Form.Item>
          <Form.Item name="is_default" label="设为默认仓">
            <Select options={[{ label: '是', value: true }, { label: '否', value: false }]} placeholder="请选择是否默认仓" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
