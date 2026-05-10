import { Button, Card, Descriptions, Drawer, Form, Input, InputNumber, Modal, Space, Tabs, Typography, message } from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type { Container, ContainerCreatePayload, ContainerListQuery } from '@/api/containers';
import { createContainer, deleteContainer, listContainers, updateContainer } from '@/api/containers';
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

type ContainerFormValues = {
  code: string;
  container_type: string;
  capacity?: number;
};

type ContainersGridFilters = {
  code: string;
  container_type: string;
};

type ContainerLeftFilterKey =
  | 'view:all'
  | 'type:托盘'
  | 'type:周转箱'
  | 'type:拣货筐'
  | 'type:冷链箱'
  | 'type:料箱';

const pageSizeOptions = ['20', '50', '100'];

const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

const createDefaultGridFilters = (): ContainersGridFilters => ({
  code: '',
  container_type: '',
});

const buildGridFilterModel = (filters: ContainersGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.code)) {
    model.code = { filterType: 'text', type: 'contains', filter: trimText(filters.code) };
  }
  if (trimText(filters.container_type)) {
    model.container_type = {
      filterType: 'text',
      type: 'contains',
      filter: trimText(filters.container_type),
    };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): ContainersGridFilters => ({
  code:
    model?.code && typeof model.code === 'object' && 'filter' in model.code
      ? trimText(String(model.code.filter ?? ''))
      : '',
  container_type:
    model?.container_type &&
    typeof model.container_type === 'object' &&
    'filter' in model.container_type
      ? trimText(String(model.container_type.filter ?? ''))
      : '',
});

export const ContainersPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<ContainerFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<ContainerLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<ContainersGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingContainer, setEditingContainer] = useState<Container | null>(null);
  const [activeContainerId, setActiveContainerId] = useState<string | null>(null);
  const [selectedContainerIds, setSelectedContainerIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantReady ? `wms.containers.pagination.${tenantId}` : undefined,
  });

  const queryPayload = useMemo<ContainerListQuery>(
    () => ({
      keyword: trimText(searchKeyword) || undefined,
      offset: 0,
      limit: 500,
    }),
    [searchKeyword],
  );

  const query = useQuery({
    queryKey: ['containers', tenantId, queryPayload],
    queryFn: () => listContainers(tenantId, queryPayload),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData,
  });

  const saveMutation = useMutation({
    mutationFn: async (values: ContainerFormValues) => {
      if (editingContainer) {
        return updateContainer(tenantId, editingContainer.container_id, {
          container_type: values.container_type.trim(),
          capacity: values.capacity,
        });
      }
      const payload: ContainerCreatePayload = {
        code: values.code.trim(),
        container_type: values.container_type.trim(),
        capacity: values.capacity,
      };
      return createContainer(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingContainer ? '容器已更新' : '容器已创建');
      setDrawerOpen(false);
      setEditingContainer(null);
      await queryClient.invalidateQueries({ queryKey: ['containers', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存容器失败');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (containerIds: string[]) => {
      await Promise.all(containerIds.map((containerId) => deleteContainer(tenantId, containerId)));
    },
    onSuccess: async (_data, containerIds) => {
      message.success(`已删除 ${containerIds.length} 个容器`);
      setSelectedContainerIds([]);
      await queryClient.invalidateQueries({ queryKey: ['containers', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '删除容器失败');
    },
  });

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const sourceRows = useMemo(() => query.data?.items ?? [], [query.data]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部容器', value: sourceRows.length, color: '#2f6fed' },
      {
        key: 'pallet',
        label: '托盘',
        value: sourceRows.filter((row) => row.container_type === '托盘').length,
        color: '#0f9d58',
      },
      {
        key: 'turnover',
        label: '周转箱',
        value: sourceRows.filter((row) => row.container_type === '周转箱').length,
        color: '#d48806',
      },
    ],
    [sourceRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部容器',
        count: sourceRows.length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'type:托盘',
        label: '托盘',
        count: sourceRows.filter((row) => row.container_type === '托盘').length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:托盘',
      },
      {
        key: 'type:周转箱',
        label: '周转箱',
        count: sourceRows.filter((row) => row.container_type === '周转箱').length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:周转箱',
      },
      {
        key: 'type:拣货筐',
        label: '拣货筐',
        count: sourceRows.filter((row) => row.container_type === '拣货筐').length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:拣货筐',
      },
      {
        key: 'type:冷链箱',
        label: '冷链箱',
        count: sourceRows.filter((row) => row.container_type === '冷链箱').length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:冷链箱',
      },
      {
        key: 'type:料箱',
        label: '料箱',
        count: sourceRows.filter((row) => row.container_type === '料箱').length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'type:料箱',
      },
    ],
    [leftFilterKey, sourceRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return sourceRows.filter((row) => {
      if (leftFilterKey !== 'view:all') {
        const type = leftFilterKey.replace('type:', '');
        if (row.container_type !== type) {
          return false;
        }
      }
      if (!keyword) {
        return true;
      }
      return [row.code, row.container_type]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword));
    });
  }, [leftFilterKey, leftKeyword, sourceRows]);

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (
          trimText(gridFilters.code) &&
          !row.code.toLowerCase().includes(trimText(gridFilters.code).toLowerCase())
        ) {
          return false;
        }
        if (
          trimText(gridFilters.container_type) &&
          !row.container_type.toLowerCase().includes(trimText(gridFilters.container_type).toLowerCase())
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
    setActiveContainerId((previous) =>
      previous && pageRows.some((row) => row.container_id === previous)
        ? previous
        : pageRows[0]?.container_id ?? null,
    );
    setSelectedContainerIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.container_id === item)),
    );
  }, [pageRows]);

  const activeContainer = useMemo(
    () => pageRows.find((row) => row.container_id === activeContainerId) ?? null,
    [activeContainerId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<Container>[]>(
    () => [
      {
        key: 'code',
        field: 'code',
        title: '容器编码',
        width: 180,
        minWidth: 150,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'container_type',
        field: 'container_type',
        title: '容器类型',
        width: 160,
        minWidth: 140,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'capacity',
        field: 'capacity',
        title: '容量',
        width: 100,
        minWidth: 90,
        renderCell: (value) => String(value ?? '-'),
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
    setEditingContainer(null);
    form.resetFields();
    setDrawerOpen(true);
  }, [form]);

  const openEditDrawer = useCallback(
    (container: Container) => {
      setEditingContainer(container);
      form.setFieldsValue({
        code: container.code,
        container_type: container.container_type,
        capacity: container.capacity ?? undefined,
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
    async (containerIds: string[]) => {
      if (!containerIds.length) {
        message.warning('请先选择要删除的容器');
        return;
      }
      await deleteMutation.mutateAsync(containerIds);
    },
    [deleteMutation],
  );

  const targetIds = selectedContainerIds.length > 0 ? selectedContainerIds : activeContainerId ? [activeContainerId] : [];
  const targetContainer = pageRows.find((row) => row.container_id === targetIds[0]) ?? activeContainer;

  const rightPanel = activeContainer ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'ops']}
      panels={[
        {
          key: 'detail',
          label: '容器详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="容器编码">{activeContainer.code}</Descriptions.Item>
              <Descriptions.Item label="容器类型">{activeContainer.container_type}</Descriptions.Item>
              <Descriptions.Item label="容量">{String(activeContainer.capacity ?? '-')}</Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(activeContainer.updated_at ?? activeContainer.created_at)}
              </Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'ops',
          label: '操作约束',
          children: (
            <Typography.Text type="secondary">
              容器目录已切换到统一工业工作台结构，新建、编辑、删除动作统一从工具栏进入，主表保持紧凑规范的审核式密度。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="容器详情">
      <Typography.Text type="secondary">请选择一条容器记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeContainer
    ? [
        { key: 'code', field: '容器编码', value: activeContainer.code },
        { key: 'type', field: '容器类型', value: activeContainer.container_type },
        { key: 'capacity', field: '容量', value: String(activeContainer.capacity ?? '-') },
      ]
    : [];

  const timelineRows: DirectoryBottomDetailRow[] = activeContainer
    ? [
        { key: 'created', field: '创建时间', value: formatDateTime(activeContainer.created_at) },
        { key: 'updated', field: '更新时间', value: formatDateTime(activeContainer.updated_at ?? activeContainer.created_at) },
        { key: 'search', field: '当前搜索关键字', value: trimText(searchKeyword) || '-' },
      ]
    : [];

  const opsRows: DirectoryBottomDetailRow[] = activeContainer
    ? [
        { key: 'selected', field: '当前勾选数', value: String(selectedContainerIds.length) },
        { key: 'edit', field: '编辑能力', value: canWrite ? '可编辑' : '只读' },
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
            Boolean(activeContainer),
            profileRows,
            '暂无基础资料',
            `container-profile-${activeContainer?.container_id ?? 'empty'}`,
          ),
        },
        {
          key: 'timeline',
          label: `时间轴迹 (${timelineRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeContainer),
            timelineRows,
            '暂无时间轴迹',
            `container-timeline-${activeContainer?.container_id ?? 'empty'}`,
          ),
        },
        {
          key: 'ops',
          label: `操作上下文 (${opsRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeContainer),
            opsRows,
            '暂无操作上下文',
            `container-ops-${activeContainer?.container_id ?? 'empty'}`,
          ),
        },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="containers-workbench-page"
        className="order-review-page containers-directory-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="containers-left-panel"
            dataTestId="containers-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as ContainerLeftFilterKey | undefined;
              if (!nextKey) {
                return;
              }
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems)}
            globalOrderTotal={sourceRows.length}
            statsScopeText="口径：当前容器目录查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">容器检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按编码 / 类型缩小左侧结果"
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
        rightPanelSyncKey={activeContainer?.container_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={
              <Space size={8} wrap>
                <Button type="primary" size="small" disabled={!tenantReady || !canWrite} onClick={openCreateDrawer}>
                  新建容器
                </Button>
                <Button
                  size="small"
                  disabled={!tenantReady || !canWrite || targetIds.length !== 1 || !targetContainer}
                  onClick={() => targetContainer && openEditDrawer(targetContainer)}
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
                      title: `确认删除选中的 ${targetIds.length} 个容器吗？`,
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
            searchPlaceholder="搜索容器编码 / 类型"
            onExport={() => message.info('容器目录导出能力待接入')}
            onPrint={() => message.info('容器目录打印能力待接入')}
            onHelp={() => message.info('容器目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<Container>
            rowKey={(record) => record.container_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="wms.containers.main-grid"
            dataTestId="containers-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={query.isLoading || query.isFetching}
            error={query.error}
            onRetry={() => {
              void query.refetch();
            }}
            errorTitle="加载容器失败"
            emptyText="暂无容器"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedContainerIds}
            activeRowKey={activeContainerId}
            onSelectionChange={(keys) => {
              setSelectedContainerIds(normalizeSelectionKeys(keys));
            }}
            onActiveRowChange={(record) => {
              setActiveContainerId(record.container_id);
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
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedContainerIds.length} 条`}</span>
            <span>{activeContainer ? `当前焦点：${activeContainer.code}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`containers-${displayRows.length}-${pageRows.length}-${selectedContainerIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingContainer ? '编辑容器' : '新建容器'}
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
          <Form.Item name="code" label="容器编码" rules={[{ required: true, message: '请输入容器编码' }]}>
            <Input disabled={Boolean(editingContainer)} />
          </Form.Item>
          <Form.Item
            name="container_type"
            label="容器类型"
            rules={[{ required: true, message: '请输入容器类型' }]}
          >
            <Input placeholder="例如：托盘 / 周转箱 / 拣货筐" />
          </Form.Item>
          <Form.Item name="capacity" label="容量">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
