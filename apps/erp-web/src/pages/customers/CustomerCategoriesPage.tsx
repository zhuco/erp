import { Button, Card, Descriptions, Drawer, Form, Input, Modal, Select, Space, Tabs, Tag, Typography, message } from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';

import type { CustomerCategory, CustomerCategoryCreatePayload } from '@/api/customerCategories';
import {
  createCustomerCategory,
  deleteCustomerCategory,
  listCustomerCategories,
  updateCustomerCategory,
} from '@/api/customerCategories';
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
import { useTenantId } from '@/features/tenant';
import {
  buildStatsPieBackground,
  normalizeSelectionKeys,
  renderDirectoryBottomTable,
  sliceRowsByPagination,
  trimText,
  type DirectoryBottomDetailRow,
} from '@/pages/shared/directoryWorkbenchUtils';

type CustomerCategoryFormValues = {
  name: string;
  description?: string;
  status?: 'ACTIVE' | 'INACTIVE';
};

type CustomerCategoryGridFilters = {
  name: string;
  description: string;
  status: 'ALL' | 'ACTIVE' | 'INACTIVE';
};

type CustomerCategoryLeftFilterKey =
  | 'view:all'
  | 'status:ACTIVE'
  | 'status:INACTIVE'
  | 'description:yes'
  | 'description:no';

const pageSizeOptions = ['20', '50', '100', '200'];
const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'INACTIVE' },
];

const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const createDefaultGridFilters = (): CustomerCategoryGridFilters => ({
  name: '',
  description: '',
  status: 'ALL',
});

const buildGridFilterModel = (filters: CustomerCategoryGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  if (trimText(filters.description)) {
    model.description = { filterType: 'text', type: 'contains', filter: trimText(filters.description) };
  }
  if (filters.status !== 'ALL') {
    model.status = { filterType: 'set', values: [filters.status] };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): CustomerCategoryGridFilters => {
  const statusValues =
    model?.status &&
    typeof model.status === 'object' &&
    'values' in model.status &&
    Array.isArray(model.status.values)
      ? model.status.values.map((value) => String(value ?? '').trim()).filter(Boolean)
      : [];

  return {
    name:
      model?.name && typeof model.name === 'object' && 'filter' in model.name
        ? trimText(String(model.name.filter ?? ''))
        : '',
    description:
      model?.description && typeof model.description === 'object' && 'filter' in model.description
        ? trimText(String(model.description.filter ?? ''))
        : '',
    status: (statusValues[0] as CustomerCategoryGridFilters['status'] | undefined) ?? 'ALL',
  };
};

export const CustomerCategoriesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('customers.write');
  const queryClient = useQueryClient();
  const [drawerForm] = Form.useForm<CustomerCategoryFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<CustomerCategoryLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<CustomerCategoryGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingCategory, setEditingCategory] = useState<CustomerCategory | null>(null);
  const [activeCategoryId, setActiveCategoryId] = useState<string | null>(null);
  const [selectedCategoryIds, setSelectedCategoryIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');
  const [saving, setSaving] = useState(false);

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantId ? `customers.categories.pagination.v2.${tenantId}` : undefined,
  });

  const query = useQuery({
    queryKey: ['customer-categories', tenantId, searchKeyword],
    queryFn: () => listCustomerCategories(tenantId, { keyword: searchKeyword || undefined, offset: 0, limit: 500 }),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const searchScopedRows = useMemo(() => {
    const keyword = trimText(searchKeyword).toLowerCase();
    const rows = query.data?.items ?? [];
    if (!keyword) {
      return rows;
    }
    return rows.filter((row) =>
      [row.name, row.description ?? '', row.status ?? '']
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword)),
    );
  }, [query.data, searchKeyword]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部分类', value: searchScopedRows.length, color: '#2f6fed' },
      {
        key: 'active',
        label: '启用',
        value: searchScopedRows.filter((row) => row.status === 'ACTIVE').length,
        color: '#0f9d58',
      },
      {
        key: 'described',
        label: '已配描述',
        value: searchScopedRows.filter((row) => trimText(row.description)).length,
        color: '#d48806',
      },
    ],
    [searchScopedRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部分类',
        count: searchScopedRows.length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'status:ACTIVE',
        label: '启用中',
        count: searchScopedRows.filter((row) => row.status === 'ACTIVE').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'status:ACTIVE',
      },
      {
        key: 'status:INACTIVE',
        label: '已停用',
        count: searchScopedRows.filter((row) => row.status === 'INACTIVE').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'status:INACTIVE',
      },
      {
        key: 'description:yes',
        label: '已配描述',
        count: searchScopedRows.filter((row) => trimText(row.description)).length,
        nodeType: 'remark',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'description:yes',
      },
      {
        key: 'description:no',
        label: '待补描述',
        count: searchScopedRows.filter((row) => !trimText(row.description)).length,
        nodeType: 'remark',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'description:no',
      },
    ],
    [leftFilterKey, searchScopedRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return searchScopedRows.filter((row) => {
      if (leftFilterKey === 'status:ACTIVE' && row.status !== 'ACTIVE') return false;
      if (leftFilterKey === 'status:INACTIVE' && row.status !== 'INACTIVE') return false;
      if (leftFilterKey === 'description:yes' && !trimText(row.description)) return false;
      if (leftFilterKey === 'description:no' && trimText(row.description)) return false;
      if (!keyword) {
        return true;
      }
      return [row.name, row.description ?? '', row.status ?? '']
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword));
    });
  }, [leftFilterKey, leftKeyword, searchScopedRows]);

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (trimText(gridFilters.name) && !row.name.toLowerCase().includes(trimText(gridFilters.name).toLowerCase())) {
          return false;
        }
        if (
          trimText(gridFilters.description) &&
          !trimText(row.description).toLowerCase().includes(trimText(gridFilters.description).toLowerCase())
        ) {
          return false;
        }
        if (gridFilters.status !== 'ALL' && row.status !== gridFilters.status) {
          return false;
        }
        return true;
      }),
    [gridFilters, leftFilteredRows],
  );

  const pageRows = useMemo(() => sliceRowsByPagination(displayRows, pagination), [displayRows, pagination]);

  useEffect(() => {
    setActiveCategoryId((previous) =>
      previous && pageRows.some((item) => item.category_id === previous) ? previous : pageRows[0]?.category_id ?? null,
    );
    setSelectedCategoryIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.category_id === item)),
    );
  }, [pageRows]);

  const activeCategory = useMemo(
    () => pageRows.find((item) => item.category_id === activeCategoryId) ?? null,
    [activeCategoryId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<CustomerCategory>[]>(
    () => [
      {
        key: 'name',
        field: 'name',
        title: '分类名称',
        width: 220,
        minWidth: 180,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'description',
        field: 'description',
        title: '描述',
        width: 360,
        minWidth: 240,
        renderCell: (value) => trimText(String(value ?? '')) || '-',
      },
      {
        key: 'status',
        field: 'status',
        title: '状态',
        width: 110,
        filter: 'agSetColumnFilter',
        filterParams: { values: ['ACTIVE', 'INACTIVE'] },
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
        renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at),
      },
    ],
    [],
  );

  const openCreateDrawer = () => {
    setEditingCategory(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ status: 'ACTIVE' });
    setDrawerOpen(true);
  };

  const openEditDrawer = (category: CustomerCategory) => {
    setEditingCategory(category);
    drawerForm.setFieldsValue({
      name: category.name,
      description: category.description ?? undefined,
      status: (category.status as 'ACTIVE' | 'INACTIVE' | undefined) ?? undefined,
    });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      const values = await drawerForm.validateFields();
      const payload: CustomerCategoryCreatePayload = {
        name: values.name.trim(),
        description: trimText(values.description) || undefined,
        status: values.status,
      };
      setSaving(true);
      if (editingCategory) {
        await updateCustomerCategory(tenantId, editingCategory.category_id, payload);
        message.success('客户分类已更新');
      } else {
        await createCustomerCategory(tenantId, payload);
        message.success('客户分类已创建');
      }
      setDrawerOpen(false);
      setEditingCategory(null);
      await queryClient.invalidateQueries({ queryKey: ['customer-categories', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteCategories = async (categoryIds: string[]) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (!categoryIds.length) {
      message.warning('请选择要删除的客户分类');
      return;
    }
    try {
      await Promise.all(categoryIds.map((categoryId) => deleteCustomerCategory(tenantId, categoryId)));
      message.success(`已删除 ${categoryIds.length} 个客户分类`);
      setSelectedCategoryIds([]);
      await queryClient.invalidateQueries({ queryKey: ['customer-categories', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除客户分类失败');
    }
  };

  const handleUpdateStatus = async (nextStatus: 'ACTIVE' | 'INACTIVE') => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    const targetIds = selectedCategoryIds.length > 0 ? selectedCategoryIds : activeCategoryId ? [activeCategoryId] : [];
    if (!targetIds.length) {
      message.warning(nextStatus === 'ACTIVE' ? '请选择要启用的客户分类' : '请选择要停用的客户分类');
      return;
    }
    try {
      await Promise.all(targetIds.map((categoryId) => updateCustomerCategory(tenantId, categoryId, { status: nextStatus })));
      message.success(nextStatus === 'ACTIVE' ? '客户分类已批量启用' : '客户分类已批量停用');
      await queryClient.invalidateQueries({ queryKey: ['customer-categories', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新客户分类状态失败');
    }
  };

  const toolbarLeftContent = useMemo(() => {
    const targetIds = selectedCategoryIds.length > 0 ? selectedCategoryIds : activeCategoryId ? [activeCategoryId] : [];
    const targetCategory = pageRows.find((item) => item.category_id === targetIds[0]) ?? activeCategory;
    return (
      <Space size={8} wrap>
        <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>
          新建分类
        </Button>
        <Button size="small" disabled={!canWrite || targetIds.length !== 1 || !targetCategory} onClick={() => targetCategory && openEditDrawer(targetCategory)}>
          编辑
        </Button>
        <Button size="small" disabled={!canWrite || targetIds.length === 0} onClick={() => void handleUpdateStatus('ACTIVE')}>
          启用
        </Button>
        <Button size="small" disabled={!canWrite || targetIds.length === 0} onClick={() => void handleUpdateStatus('INACTIVE')}>
          停用
        </Button>
        <Button
          size="small"
          danger
          disabled={!canWrite || targetIds.length === 0}
          onClick={() => {
            Modal.confirm({
              title: `确认删除选中的 ${targetIds.length} 个客户分类吗？`,
              okText: '删除',
              cancelText: '取消',
              onOk: async () => handleDeleteCategories(targetIds),
            });
          }}
        >
          删除
        </Button>
      </Space>
    );
  }, [activeCategory, activeCategoryId, canWrite, pageRows, selectedCategoryIds]);

  const rightPanel = activeCategory ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'rule']}
      panels={[
        {
          key: 'detail',
          label: '分类详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="分类名称">{activeCategory.name}</Descriptions.Item>
              <Descriptions.Item label="状态">
                {activeCategory.status === 'ACTIVE' ? '启用' : '停用'}
              </Descriptions.Item>
              <Descriptions.Item label="描述">{trimText(activeCategory.description) || '-'}</Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'rule',
          label: '工作台约束',
          children: (
            <Typography.Text type="secondary">
              客户分类目录与订单审核复用同一套工业工作台骨架，左侧做结构筛选，中间只保留紧凑主表，右侧展示分类说明，底部承载补充明细。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="分类详情">
      <Typography.Text type="secondary">请选择一条客户分类记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeCategory
    ? [
        { key: 'name', field: '分类名称', value: activeCategory.name },
        { key: 'status', field: '状态', value: activeCategory.status === 'ACTIVE' ? '启用' : '停用' },
        { key: 'description', field: '描述', value: trimText(activeCategory.description) || '-' },
      ]
    : [];

  const timelineRows: DirectoryBottomDetailRow[] = activeCategory
    ? [
        { key: 'created_at', field: '创建时间', value: formatDateTime(activeCategory.created_at) },
        { key: 'updated_at', field: '更新时间', value: formatDateTime(activeCategory.updated_at ?? activeCategory.created_at) },
        { key: 'search', field: '当前检索词', value: trimText(searchKeyword) || '-' },
      ]
    : [];

  const queryRows: DirectoryBottomDetailRow[] = activeCategory
    ? [
        { key: 'selected', field: '当前勾选数', value: String(selectedCategoryIds.length) },
        { key: 'left-filter', field: '左侧筛选', value: leftFilterKey },
        { key: 'layout', field: '工作台结构', value: '左筛选 / 主表 / 右详情 / 底部明细' },
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
            Boolean(activeCategory),
            profileRows,
            '暂无基础资料',
            `customer-category-profile-${activeCategory?.category_id ?? 'empty'}`,
          ),
        },
        {
          key: 'timeline',
          label: `时间轨迹 (${timelineRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeCategory),
            timelineRows,
            '暂无时间轨迹',
            `customer-category-timeline-${activeCategory?.category_id ?? 'empty'}`,
          ),
        },
        {
          key: 'query',
          label: `查询上下文 (${queryRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeCategory),
            queryRows,
            '暂无查询上下文',
            `customer-category-query-${activeCategory?.category_id ?? 'empty'}`,
          ),
        },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="customer-categories-workbench-page"
        className="order-review-page customer-categories-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="customer-categories-left-panel"
            dataTestId="customer-categories-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as CustomerCategoryLeftFilterKey | undefined;
              if (!nextKey) return;
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems.map(({ color, value }) => ({ color, value })))}
            globalOrderTotal={searchScopedRows.length}
            statsScopeText="口径：当前客户分类查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">分类检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按名称 / 描述 / 状态缩小左侧结果"
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
        rightPanelSyncKey={activeCategory?.category_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={toolbarLeftContent}
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => {
              setSearchKeyword(trimText(searchInput));
              resetPagination();
            }}
            searchPlaceholder="搜索分类名称 / 描述 / 状态"
            onExport={() => message.info('客户分类目录导出能力待接入')}
            onPrint={() => message.info('客户分类目录打印能力待接入')}
            onHelp={() => message.info('客户分类目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<CustomerCategory>
            rowKey={(record) => record.category_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="customers.categories.main-grid"
            dataTestId="customer-categories-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={query.isLoading || query.isFetching}
            error={query.error}
            onRetry={() => void query.refetch()}
            errorTitle="加载客户分类失败"
            emptyText="暂无客户分类"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedCategoryIds}
            activeRowKey={activeCategoryId}
            onSelectionChange={(keys) => {
              setSelectedCategoryIds(normalizeSelectionKeys(keys));
            }}
            onActiveRowChange={(record) => {
              setActiveCategoryId(record.category_id);
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
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedCategoryIds.length} 条`}</span>
            <span>{activeCategory ? `当前焦点：${activeCategory.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`customer-categories-${displayRows.length}-${pageRows.length}-${selectedCategoryIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingCategory ? '编辑客户分类' : '新建客户分类'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={() => void handleSave()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item name="name" label="分类名称" rules={[{ required: true, message: '请输入分类名称' }]}>
            <Input placeholder="分类名称" />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={4} placeholder="分类描述" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
