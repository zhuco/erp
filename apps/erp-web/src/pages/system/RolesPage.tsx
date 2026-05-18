import { Button, Card, Descriptions, Form, Input, Modal, Select, Space, Tabs, Typography, message } from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  createRole,
  listPermissions,
  listRolePermissions,
  listRoles,
  updateRole,
  updateRolePermissions,
  type PermissionDefinition,
  type Role,
} from '@/api/system';
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
import { isTenantSelected, useTenantId } from '@/features/tenant';
import {
  buildStatsPieBackground,
  normalizeSelectionKeys,
  renderDirectoryBottomTable,
  sliceRowsByPagination,
  trimText,
  type DirectoryBottomDetailRow,
} from '@/pages/shared/directoryWorkbenchUtils';

type RoleFormValues = {
  name: string;
  permissions?: string[];
};

type RolesGridFilters = {
  name: string;
};

type RoleLeftFilterKey =
  | 'view:all'
  | 'group:system'
  | 'group:ops'
  | 'group:wms'
  | 'group:procurement'
  | 'group:service';

const pageSizeOptions = ['20', '50', '100'];

const formatDateTime = (value?: string) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

const createDefaultGridFilters = (): RolesGridFilters => ({
  name: '',
});

const buildGridFilterModel = (filters: RolesGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): RolesGridFilters => ({
  name:
    model?.name && typeof model.name === 'object' && 'filter' in model.name
      ? trimText(String(model.name.filter ?? ''))
      : '',
});

const resolveRoleGroup = (roleName: string): RoleLeftFilterKey => {
  if (roleName.includes('系统')) return 'group:system';
  if (roleName.includes('运营')) return 'group:ops';
  if (roleName.includes('仓储')) return 'group:wms';
  if (roleName.includes('采购')) return 'group:procurement';
  if (roleName.includes('客服')) return 'group:service';
  return 'view:all';
};

const resolveRoleGroupLabel = (roleName: string) => {
  const group = resolveRoleGroup(roleName);
  if (group === 'group:system') return '系统体系';
  if (group === 'group:ops') return '运营体系';
  if (group === 'group:wms') return '仓储体系';
  if (group === 'group:procurement') return '采购体系';
  if (group === 'group:service') return '客服体系';
  return '通用角色';
};

export const RolesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantReady ? `system.roles.pagination.${tenantId}` : undefined,
  });
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<RoleLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<RolesGridFilters>(createDefaultGridFilters);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<Role | null>(null);
  const [saving, setSaving] = useState(false);
  const [loadingRolePerms, setLoadingRolePerms] = useState(false);
  const [activeRoleId, setActiveRoleId] = useState<string | null>(null);
  const [selectedRoleIds, setSelectedRoleIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');
  const [form] = Form.useForm<RoleFormValues>();

  const rolesQuery = useQuery({
    queryKey: ['roles', tenantId],
    queryFn: () => listRoles(tenantId),
    enabled: tenantReady,
  });

  const permissionsQuery = useQuery({
    queryKey: ['permission-catalog', tenantId],
    queryFn: () => listPermissions(tenantId),
    enabled: tenantReady,
  });

  const permissionOptions = useMemo(
    () =>
      (permissionsQuery.data ?? []).map((perm: PermissionDefinition) => ({
        label: `${perm.module} / ${perm.name}`,
        value: perm.code,
      })),
    [permissionsQuery.data],
  );

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const sourceRows = useMemo(() => {
    const keyword = trimText(searchKeyword).toLowerCase();
    const rows = rolesQuery.data ?? [];
    if (!keyword) {
      return rows;
    }
    return rows.filter((row) => row.name.toLowerCase().includes(keyword));
  }, [rolesQuery.data, searchKeyword]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部角色', value: sourceRows.length, color: '#2f6fed' },
      {
        key: 'ops',
        label: '运营体系',
        value: sourceRows.filter((row) => resolveRoleGroup(row.name) === 'group:ops').length,
        color: '#0f9d58',
      },
      {
        key: 'system',
        label: '系统体系',
        value: sourceRows.filter((row) => resolveRoleGroup(row.name) === 'group:system').length,
        color: '#d48806',
      },
    ],
    [sourceRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部角色',
        count: sourceRows.length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'group:system',
        label: '系统体系',
        count: sourceRows.filter((row) => resolveRoleGroup(row.name) === 'group:system').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'group:system',
      },
      {
        key: 'group:ops',
        label: '运营体系',
        count: sourceRows.filter((row) => resolveRoleGroup(row.name) === 'group:ops').length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'group:ops',
      },
      {
        key: 'group:wms',
        label: '仓储体系',
        count: sourceRows.filter((row) => resolveRoleGroup(row.name) === 'group:wms').length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'group:wms',
      },
      {
        key: 'group:procurement',
        label: '采购体系',
        count: sourceRows.filter((row) => resolveRoleGroup(row.name) === 'group:procurement').length,
        nodeType: 'shop',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'group:procurement',
      },
      {
        key: 'group:service',
        label: '客服体系',
        count: sourceRows.filter((row) => resolveRoleGroup(row.name) === 'group:service').length,
        nodeType: 'remark',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'group:service',
      },
    ],
    [leftFilterKey, sourceRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return sourceRows.filter((row) => {
      if (leftFilterKey !== 'view:all' && resolveRoleGroup(row.name) !== leftFilterKey) {
        return false;
      }
      if (!keyword) {
        return true;
      }
      return row.name.toLowerCase().includes(keyword);
    });
  }, [leftFilterKey, leftKeyword, sourceRows]);

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (
          trimText(gridFilters.name) &&
          !row.name.toLowerCase().includes(trimText(gridFilters.name).toLowerCase())
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
    setActiveRoleId((previous) =>
      previous && pageRows.some((row) => row.role_id === previous) ? previous : pageRows[0]?.role_id ?? null,
    );
    setSelectedRoleIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.role_id === item)),
    );
  }, [pageRows]);

  const activeRole = useMemo(
    () => pageRows.find((row) => row.role_id === activeRoleId) ?? null,
    [activeRoleId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<Role>[]>(
    () => [
      {
        key: 'name',
        field: 'name',
        title: '角色名称',
        width: 220,
        minWidth: 180,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'created_at',
        field: 'created_at',
        title: '创建时间',
        width: 180,
        minWidth: 160,
        renderCell: (value) => formatDateTime(String(value ?? '')),
      },
    ],
    [],
  );

  const openCreate = () => {
    setEditingRole(null);
    form.resetFields();
    form.setFieldsValue({ permissions: [] });
    setModalOpen(true);
  };

  const openEdit = useCallback(
    async (role: Role) => {
      setEditingRole(role);
      setModalOpen(true);
      setLoadingRolePerms(true);
      form.setFieldsValue({ name: role.name, permissions: [] });
      try {
        const perms = await listRolePermissions(tenantId, role.role_id);
        form.setFieldsValue({ name: role.name, permissions: perms });
      } catch (error) {
        message.error(error instanceof Error ? error.message : '加载角色权限失败');
      } finally {
        setLoadingRolePerms(false);
      }
    },
    [form, tenantId],
  );

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);
      if (editingRole) {
        await updateRole(tenantId, editingRole.role_id, { name: values.name });
        await updateRolePermissions(tenantId, editingRole.role_id, {
          permissions: values.permissions ?? [],
        });
        message.success('角色已更新');
      } else {
        await createRole(tenantId, {
          name: values.name,
          permissions: values.permissions ?? [],
        });
        message.success('角色已创建');
      }
      setModalOpen(false);
      await rolesQuery.refetch();
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const rightPanel = activeRole ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'ops']}
      panels={[
        {
          key: 'detail',
          label: '角色详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="角色名称">{activeRole.name}</Descriptions.Item>
              <Descriptions.Item label="角色分组">
                {resolveRoleGroupLabel(activeRole.name)}
              </Descriptions.Item>
              <Descriptions.Item label="创建时间">{formatDateTime(activeRole.created_at)}</Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'ops',
          label: '工作台约束',
          children: (
            <Typography.Text type="secondary">
              角色目录与订单审核共享同一套工业工作台结构，编辑动作统一从工具栏和主表双击入口触发，布局不再按页面各自定义。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="角色详情">
      <Typography.Text type="secondary">请选择一条角色记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeRole
    ? [
        { key: 'name', field: '角色名称', value: activeRole.name },
        { key: 'created', field: '创建时间', value: formatDateTime(activeRole.created_at) },
        { key: 'group', field: '角色分组', value: resolveRoleGroupLabel(activeRole.name) },
      ]
    : [];

  const permissionRows: DirectoryBottomDetailRow[] = activeRole
    ? [
        { key: 'perm-loading', field: '权限装载', value: loadingRolePerms ? '加载中' : '按需读取角色权限' },
        { key: 'perm-source', field: '权限来源', value: '系统权限目录' },
      ]
    : [];

  const opsRows: DirectoryBottomDetailRow[] = activeRole
    ? [
        { key: 'search', field: '当前搜索关键字', value: trimText(searchKeyword) || '-' },
        { key: 'selected', field: '当前勾选数', value: String(selectedRoleIds.length) },
        { key: 'entry', field: '维护入口', value: '工具栏新建 / 主表双击 / 编辑弹窗' },
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
            Boolean(activeRole),
            profileRows,
            '暂无基础资料',
            `roles-profile-${activeRole?.role_id ?? 'empty'}`,
          ),
        },
        {
          key: 'permissions',
          label: `权限上下文 (${permissionRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeRole),
            permissionRows,
            '暂无权限上下文',
            `roles-permissions-${activeRole?.role_id ?? 'empty'}`,
          ),
        },
        {
          key: 'ops',
          label: `操作上下文 (${opsRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeRole),
            opsRows,
            '暂无操作上下文',
            `roles-ops-${activeRole?.role_id ?? 'empty'}`,
          ),
        },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="roles-workbench-page"
        className="order-review-page roles-directory-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="roles-left-panel"
            dataTestId="roles-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as RoleLeftFilterKey | undefined;
              if (!nextKey) {
                return;
              }
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems)}
            globalOrderTotal={sourceRows.length}
            statsScopeText="口径：当前角色目录查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">角色检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按角色名称缩小左侧结果"
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
        rightPanelSyncKey={activeRole?.role_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={
              <Space size={8} wrap>
                <Button type="primary" size="small" onClick={openCreate} disabled={!tenantReady}>
                  新建角色
                </Button>
                <Button
                  size="small"
                  disabled={!activeRole}
                  onClick={() => activeRole && void openEdit(activeRole)}
                >
                  编辑
                </Button>
              </Space>
            }
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => {
              setSearchKeyword(trimText(searchInput));
              resetPagination();
            }}
            searchPlaceholder="搜索角色名称"
            onExport={() => message.info('角色目录导出能力待接入')}
            onPrint={() => message.info('角色目录打印能力待接入')}
            onHelp={() => message.info('角色目录帮助文档待接入')}
          />
        }
        tableContent={
          <ErpDataGridNext<Role>
            rowKey={(record) => record.role_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="system.roles.main-grid"
            dataTestId="roles-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={rolesQuery.isFetching}
            error={rolesQuery.error}
            onRetry={() => {
              void rolesQuery.refetch();
            }}
            errorTitle="加载角色失败"
            emptyText="暂无角色"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedRoleIds}
            activeRowKey={activeRoleId}
            onSelectionChange={(keys) => {
              setSelectedRoleIds(normalizeSelectionKeys(keys));
            }}
            onActiveRowChange={(record) => {
              setActiveRoleId(record.role_id);
            }}
            onRowDoubleClick={(record) => {
              void openEdit(record);
            }}
          />
        }
        footerSummary={
          <>
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedRoleIds.length} 条`}</span>
            <span>{activeRole ? `当前焦点：${activeRole.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`roles-${displayRows.length}-${pageRows.length}-${selectedRoleIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Modal
        title={editingRole ? '编辑角色' : '新建角色'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={() => void handleSubmit()}
        confirmLoading={saving}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label="角色名称"
            rules={[{ required: true, message: '请输入角色名称' }]}
          >
            <Input placeholder="例如：运营主管" />
          </Form.Item>
          <Form.Item name="permissions" label="权限列表">
            <Select
              mode="multiple"
              allowClear
              placeholder={loadingRolePerms ? '加载中...' : '选择权限'}
              options={permissionOptions}
              loading={permissionsQuery.isFetching || loadingRolePerms}
              filterOption={(input, option) =>
                (option?.label ?? '').toString().toLowerCase().includes(input.toLowerCase())
              }
            />
          </Form.Item>
        </Form>
        {permissionsQuery.error && (
          <Space direction="vertical">
            <span>权限目录加载失败</span>
          </Space>
        )}
      </Modal>
    </>
  );
};
