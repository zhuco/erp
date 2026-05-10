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

import type { Supplier, SupplierCreatePayload } from '@/api/suppliers';
import { createSupplier, deleteSupplier, listSuppliers, updateSupplier } from '@/api/suppliers';
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

type SupplierFormValues = {
  name: string;
  contact?: string;
  settlement_terms?: string;
  status?: string;
};

type SupplierGridFilters = {
  name: string;
  contact: string;
  settlement: string;
  status: 'ALL' | 'ACTIVE' | 'INACTIVE';
};

type SupplierLeftFilterKey = 'view:all' | 'status:ACTIVE' | 'status:INACTIVE';

type SupplierDetailRow = {
  key: string;
  field: string;
  value: string;
};

const pageSizeOptions = ['20', '50', '100', '200'];
const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'INACTIVE' },
];

const trimText = (value?: string | null) => String(value ?? '').trim();
const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const createDefaultGridFilters = (): SupplierGridFilters => ({ name: '', contact: '', settlement: '', status: 'ALL' });

const buildGridFilterModel = (filters: SupplierGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  if (trimText(filters.contact)) {
    model.contact = { filterType: 'text', type: 'contains', filter: trimText(filters.contact) };
  }
  if (trimText(filters.settlement)) {
    model.settlement_terms = { filterType: 'text', type: 'contains', filter: trimText(filters.settlement) };
  }
  if (filters.status !== 'ALL') {
    model.status = { filterType: 'set', values: [filters.status] };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): SupplierGridFilters => {
  const statusValues =
    model?.status && typeof model.status === 'object' && 'values' in model.status && Array.isArray(model.status.values)
      ? model.status.values.map((value) => String(value ?? '').trim()).filter(Boolean)
      : [];
  return {
    name:
      model?.name && typeof model.name === 'object' && 'filter' in model.name
        ? trimText(String(model.name.filter ?? ''))
        : '',
    contact:
      model?.contact && typeof model.contact === 'object' && 'filter' in model.contact
        ? trimText(String(model.contact.filter ?? ''))
        : '',
    settlement:
      model?.settlement_terms && typeof model.settlement_terms === 'object' && 'filter' in model.settlement_terms
        ? trimText(String(model.settlement_terms.filter ?? ''))
        : '',
    status: (statusValues[0] as SupplierGridFilters['status'] | undefined) ?? 'ALL',
  };
};

const buildPieBackground = (rows: Supplier[]) => {
  const active = rows.filter((row) => row.status === 'ACTIVE').length;
  const inactive = rows.filter((row) => row.status === 'INACTIVE').length;
  const total = rows.length || 1;
  const activeEnd = (active / total) * 360;
  const inactiveEnd = activeEnd + (inactive / total) * 360;
  return `conic-gradient(#2f6fed 0deg ${activeEnd.toFixed(2)}deg, #d0d7de ${activeEnd.toFixed(2)}deg ${inactiveEnd.toFixed(2)}deg, #eef2f7 ${inactiveEnd.toFixed(2)}deg 360deg)`;
};

export const SuppliersPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('suppliers.write');
  const queryClient = useQueryClient();
  const [drawerForm] = Form.useForm<SupplierFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<SupplierLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<SupplierGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingSupplier, setEditingSupplier] = useState<Supplier | null>(null);
  const [activeSupplierId, setActiveSupplierId] = useState<string | null>(null);
  const [selectedSupplierIds, setSelectedSupplierIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantId ? `suppliers.directory.pagination.v2.${tenantId}` : undefined,
  });

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['suppliers', tenantId, searchKeyword],
    queryFn: () => listSuppliers(tenantId, { keyword: searchKeyword || undefined, offset: 0, limit: 500 }),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const searchScopedRows = data?.items ?? [];

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部供应商', value: searchScopedRows.length, color: '#2f6fed' },
      { key: 'active', label: '启用', value: searchScopedRows.filter((row) => row.status === 'ACTIVE').length, color: '#0f9d58' },
      { key: 'inactive', label: '停用', value: searchScopedRows.filter((row) => row.status === 'INACTIVE').length, color: '#d0d7de' },
    ],
    [searchScopedRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      { key: 'view:all', label: '全部供应商', count: searchScopedRows.length, nodeType: 'shop', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'view:all' },
      { key: 'status:ACTIVE', label: '启用中', count: searchScopedRows.filter((row) => row.status === 'ACTIVE').length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'status:ACTIVE' },
      { key: 'status:INACTIVE', label: '已停用', count: searchScopedRows.filter((row) => row.status === 'INACTIVE').length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'status:INACTIVE' },
    ],
    [leftFilterKey, searchScopedRows],
  );

  const leftFilteredRows = useMemo(
    () =>
      searchScopedRows.filter((row) => {
        if (leftFilterKey === 'status:ACTIVE' && row.status !== 'ACTIVE') return false;
        if (leftFilterKey === 'status:INACTIVE' && row.status !== 'INACTIVE') return false;
        if (!trimText(leftKeyword)) return true;
        return [row.name, row.contact, row.settlement_terms]
          .map((value) => trimText(value).toLowerCase())
          .some((value) => value.includes(trimText(leftKeyword).toLowerCase()));
      }),
    [leftFilterKey, leftKeyword, searchScopedRows],
  );

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (trimText(gridFilters.name) && !trimText(row.name).toLowerCase().includes(trimText(gridFilters.name).toLowerCase())) return false;
        if (trimText(gridFilters.contact) && !trimText(row.contact).toLowerCase().includes(trimText(gridFilters.contact).toLowerCase())) return false;
        if (trimText(gridFilters.settlement) && !trimText(row.settlement_terms).toLowerCase().includes(trimText(gridFilters.settlement).toLowerCase())) return false;
        if (gridFilters.status !== 'ALL' && row.status !== gridFilters.status) return false;
        return true;
      }),
    [gridFilters, leftFilteredRows],
  );

  const currentPage = Number(pagination.current ?? 1);
  const currentPageSize = Number(pagination.pageSize ?? 20);
  const suppliers = useMemo(() => {
    const start = (currentPage - 1) * currentPageSize;
    return displayRows.slice(start, start + currentPageSize);
  }, [currentPage, currentPageSize, displayRows]);

  useEffect(() => {
    setActiveSupplierId((previous) => (previous && suppliers.some((item) => item.supplier_id === previous) ? previous : suppliers[0]?.supplier_id ?? null));
    setSelectedSupplierIds((previous) => previous.filter((supplierId) => suppliers.some((item) => item.supplier_id === supplierId)));
  }, [suppliers]);

  const activeSupplier = useMemo(() => suppliers.find((item) => item.supplier_id === activeSupplierId) ?? null, [activeSupplierId, suppliers]);

  const saveMutation = useMutation({
    mutationFn: async (values: SupplierFormValues) => {
      if (!tenantId) throw new Error('请先选择租户');
      const payload: SupplierCreatePayload = {
        name: values.name.trim(),
        contact: values.contact?.trim() || undefined,
        settlement_terms: values.settlement_terms?.trim() || undefined,
        status: values.status,
      };
      if (editingSupplier) return updateSupplier(tenantId, editingSupplier.supplier_id, payload);
      return createSupplier(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingSupplier ? '供应商已更新' : '供应商已创建');
      setDrawerOpen(false);
      setEditingSupplier(null);
      await queryClient.invalidateQueries({ queryKey: ['suppliers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存供应商失败');
    },
  });

  const openCreateDrawer = useCallback(() => {
    setEditingSupplier(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ status: 'ACTIVE' });
    setDrawerOpen(true);
  }, [drawerForm]);

  const openEditDrawer = useCallback((supplier: Supplier) => {
    setEditingSupplier(supplier);
    drawerForm.setFieldsValue({
      name: supplier.name,
      contact: supplier.contact ?? undefined,
      settlement_terms: supplier.settlement_terms ?? undefined,
      status: supplier.status ?? undefined,
    });
    setDrawerOpen(true);
  }, [drawerForm]);

  const handleSave = useCallback(async () => {
    try {
      const values = await drawerForm.validateFields();
      await saveMutation.mutateAsync(values);
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  }, [drawerForm, saveMutation]);

  const handleDeleteSuppliers = useCallback(async (supplierIds: string[]) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (!supplierIds.length) {
      message.warning('请先选择要删除的供应商');
      return;
    }
    try {
      await Promise.all(supplierIds.map((supplierId) => deleteSupplier(tenantId, supplierId)));
      message.success(`已删除 ${supplierIds.length} 个供应商`);
      setSelectedSupplierIds([]);
      await queryClient.invalidateQueries({ queryKey: ['suppliers', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除供应商失败');
    }
  }, [queryClient, tenantId]);

  const handleUpdateStatus = useCallback(async (nextStatus: 'ACTIVE' | 'INACTIVE') => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    const targetIds = selectedSupplierIds.length > 0 ? selectedSupplierIds : activeSupplierId ? [activeSupplierId] : [];
    if (!targetIds.length) {
      message.warning(nextStatus === 'ACTIVE' ? '请选择要启用的供应商' : '请选择要停用的供应商');
      return;
    }
    try {
      await Promise.all(targetIds.map((supplierId) => updateSupplier(tenantId, supplierId, { status: nextStatus })));
      message.success(nextStatus === 'ACTIVE' ? '供应商已批量启用' : '供应商已批量停用');
      await queryClient.invalidateQueries({ queryKey: ['suppliers', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新供应商状态失败');
    }
  }, [activeSupplierId, queryClient, selectedSupplierIds, tenantId]);

  const columns = useMemo<ErpDataGridColumn<Supplier>[]>(
    () => [
      { key: 'name', field: 'name', title: '供应商名称', width: 220, minWidth: 180, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'contact', field: 'contact', title: '联系人', width: 180, minWidth: 150, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'settlement_terms', field: 'settlement_terms', title: '结算条件', width: 160, minWidth: 140, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'status', field: 'status', title: '状态', width: 100, filter: 'agSetColumnFilter', filterParams: { values: ['ACTIVE', 'INACTIVE'] }, renderCell: (value) => <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value === 'ACTIVE' ? '启用' : '停用'}</Tag> },
      { key: 'updated_at', field: 'updated_at', title: '更新时间', width: 170, renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at) },
    ],
    [],
  );

  const toolbarLeftContent = useMemo(() => {
    const targetIds = selectedSupplierIds.length > 0 ? selectedSupplierIds : activeSupplierId ? [activeSupplierId] : [];
    const targetSupplier = suppliers.find((item) => item.supplier_id === targetIds[0]) ?? activeSupplier;
    return (
      <Space size={8} wrap>
        <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>新建供应商</Button>
        <Button size="small" disabled={!canWrite || targetIds.length !== 1 || !targetSupplier} onClick={() => targetSupplier && openEditDrawer(targetSupplier)}>编辑</Button>
        <Button size="small" disabled={!canWrite || targetIds.length === 0} onClick={() => void handleUpdateStatus('ACTIVE')}>启用</Button>
        <Button size="small" disabled={!canWrite || targetIds.length === 0} onClick={() => void handleUpdateStatus('INACTIVE')}>停用</Button>
        <Button size="small" danger disabled={!canWrite || targetIds.length === 0} onClick={() => { void Modal.confirm({ title: `确认删除选中的 ${targetIds.length} 个供应商吗？`, okText: '删除', cancelText: '取消', onOk: async () => handleDeleteSuppliers(targetIds) }); }}>删除</Button>
      </Space>
    );
  }, [activeSupplier, activeSupplierId, canWrite, handleDeleteSuppliers, handleUpdateStatus, openCreateDrawer, openEditDrawer, selectedSupplierIds, suppliers]);

  const rightPanel = useMemo(() => {
    if (!activeSupplier) {
      return (
        <Card size="small" title="供应商详情">
          <Typography.Text type="secondary">请选择一条供应商记录查看右侧详情。</Typography.Text>
        </Card>
      );
    }
    return (
      <UiModule05DetailPanel
        defaultActiveKeys={['detail', 'ops']}
        panels={[
          {
            key: 'detail',
            label: '供应商详情',
            children: (
              <Descriptions column={1} size="small">
                <Descriptions.Item label="供应商名称">{activeSupplier.name}</Descriptions.Item>
                <Descriptions.Item label="联系人">{activeSupplier.contact || '-'}</Descriptions.Item>
                <Descriptions.Item label="结算条件">{activeSupplier.settlement_terms || '-'}</Descriptions.Item>
                <Descriptions.Item label="状态">{activeSupplier.status === 'ACTIVE' ? '启用' : '停用'}</Descriptions.Item>
              </Descriptions>
            ),
          },
          {
            key: 'ops',
            label: '操作约束',
            children: <Typography.Text type="secondary">编辑、启停用和删除动作统一通过工具栏执行，主表保持紧凑浏览节奏。</Typography.Text>,
          },
        ]}
      />
    );
  }, [activeSupplier]);

  const bottomColumns = useMemo(
    () => [
      { title: '字段', dataIndex: 'field', key: 'field', width: 180 },
      { title: '值', dataIndex: 'value', key: 'value', width: 320 },
    ],
    [],
  );

  const profileRows: SupplierDetailRow[] = activeSupplier
    ? [
        { key: 'name', field: '供应商名称', value: activeSupplier.name },
        { key: 'contact', field: '联系人', value: activeSupplier.contact || '-' },
        { key: 'settlement', field: '结算条件', value: activeSupplier.settlement_terms || '-' },
      ]
    : [];
  const timelineRows: SupplierDetailRow[] = activeSupplier
    ? [
        { key: 'created_at', field: '创建时间', value: formatDateTime(activeSupplier.created_at) },
        { key: 'updated_at', field: '更新时间', value: formatDateTime(activeSupplier.updated_at ?? activeSupplier.created_at) },
        { key: 'query', field: '当前检索关键字', value: searchKeyword.trim() || '-' },
      ]
    : [];
  const opsRows: SupplierDetailRow[] = activeSupplier
    ? [
        { key: 'edit', field: '编辑能力', value: canWrite ? '可编辑' : '只读' },
        { key: 'delete', field: '删除能力', value: canWrite ? '可删除' : '只读' },
        { key: 'status', field: '状态变更方式', value: canWrite ? '通过工具栏执行' : '只读' },
      ]
    : [];

  const renderBottomTable = (rows: SupplierDetailRow[], emptyDescription: string, syncKey: string) =>
    activeSupplier ? (
      <UiModule04BottomTable syncKey={syncKey}>
        <Table<SupplierDetailRow> size="small" rowKey="key" columns={bottomColumns} dataSource={rows} pagination={false} scroll={{ x: 'max-content' }} locale={{ emptyText: <Empty description={emptyDescription} /> }} />
      </UiModule04BottomTable>
    ) : (
      <Empty description="请选择供应商查看底部明细" />
    );

  const bottomPanel = (
    <Tabs
      size="small"
      type="card"
      activeKey={activeBottomTab}
      onChange={setActiveBottomTab}
      items={[
        { key: 'profile', label: `基础资料 (${profileRows.length})`, children: renderBottomTable(profileRows, '暂无基础资料', `supplier-profile-${activeSupplier?.supplier_id ?? 'empty'}`) },
        { key: 'timeline', label: `时间轨迹 (${timelineRows.length})`, children: renderBottomTable(timelineRows, '暂无时间轨迹', `supplier-timeline-${activeSupplier?.supplier_id ?? 'empty'}`) },
        { key: 'ops', label: `操作约束 (${opsRows.length})`, children: renderBottomTable(opsRows, '暂无操作约束信息', `supplier-ops-${activeSupplier?.supplier_id ?? 'empty'}`) },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="suppliers-workbench-page"
        className="order-review-page suppliers-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="suppliers-left-panel"
            dataTestId="suppliers-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as SupplierLeftFilterKey | undefined;
              if (!nextKey) return;
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildPieBackground(searchScopedRows)}
            globalOrderTotal={searchScopedRows.length}
            statsScopeText="口径：当前查询结果中的供应商统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">供应商检索</Typography.Text>
                  <Input size="small" allowClear value={leftKeyword} placeholder="筛选名称 / 联系人 / 结算条件" onChange={(event) => { resetPagination(); setLeftKeyword(event.target.value); }} />
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
        rightPanelSyncKey={activeSupplier?.supplier_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={toolbarLeftContent}
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => { setSearchKeyword(searchInput.trim()); resetPagination(); }}
            searchPlaceholder="搜索供应商名称 / 联系人 / 结算条件"
            onExport={() => message.info('供应商目录导出能力待接入')}
            onPrint={() => message.info('供应商目录打印能力待接入')}
            onHelp={() => message.info('供应商目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<Supplier>
            rowKey={(record) => record.supplier_id}
            columns={columns}
            rows={suppliers}
            gridPreset="industrial-review-compact"
            loading={isLoading || isFetching}
            error={error}
            onRetry={() => refetch()}
            errorTitle="供应商目录加载失败"
            emptyText="暂无供应商数据"
            columnSettingsKey="suppliers.directory.main-grid"
            dataTestId="suppliers-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => { resetPagination(); setGridFilters(extractGridFilterModel(model)); }}
            selectedRowKeys={selectedSupplierIds}
            activeRowKey={activeSupplierId}
            onSelectionChange={(keys) => setSelectedSupplierIds(Array.from(new Set(keys.map((key) => String(key ?? '').trim()).filter(Boolean))))}
            onActiveRowChange={(record) => setActiveSupplierId(record.supplier_id)}
            onRowDoubleClick={(record) => { if (canWrite) openEditDrawer(record); }}
          />
        }
        footerSummary={
          <>
            <span>{`总数 ${displayRows.length} 条，本页 ${suppliers.length} 条，已选 ${selectedSupplierIds.length} 条`}</span>
            <span>{activeSupplier ? `当前焦点：${activeSupplier.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`suppliers-${currentPage}-${currentPageSize}-${displayRows.length}-${selectedSupplierIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingSupplier ? '编辑供应商' : '新建供应商'}
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
        <Form form={drawerForm} layout="vertical">
          <Form.Item name="name" label="供应商名称" rules={[{ required: true, message: '请输入供应商名称' }]}>
            <Input placeholder="供应商名称" />
          </Form.Item>
          <Form.Item name="contact" label="联系人">
            <Input placeholder="联系人 / 电话" />
          </Form.Item>
          <Form.Item name="settlement_terms" label="结算条件">
            <Input placeholder="例如：月结 / 周结" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="请选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
