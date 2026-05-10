import {
  Button,
  Card,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Tabs,
  Tag,
  Typography,
  message,
} from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';

import type { MemberLevel, MemberLevelCreatePayload } from '@/api/memberLevels';
import {
  createMemberLevel,
  deleteMemberLevel,
  listMemberLevels,
  updateMemberLevel,
} from '@/api/memberLevels';
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

type MemberLevelFormValues = {
  name: string;
  level_rank?: number;
  discount_rate?: number;
  rules?: string;
  status?: 'ACTIVE' | 'INACTIVE';
};

type MemberLevelGridFilters = {
  name: string;
  rules: string;
  status: 'ALL' | 'ACTIVE' | 'INACTIVE';
};

type MemberLevelLeftFilterKey =
  | 'view:all'
  | 'status:ACTIVE'
  | 'status:INACTIVE'
  | 'discount:yes'
  | 'discount:no';

const pageSizeOptions = ['20', '50', '100', '200'];
const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'INACTIVE' },
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatDiscount = (value?: number | null) =>
  value == null ? '-' : `${(value * 100).toFixed(0)}%`;

const createDefaultGridFilters = (): MemberLevelGridFilters => ({
  name: '',
  rules: '',
  status: 'ALL',
});

const buildGridFilterModel = (filters: MemberLevelGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  if (trimText(filters.rules)) {
    model.rules = { filterType: 'text', type: 'contains', filter: trimText(filters.rules) };
  }
  if (filters.status !== 'ALL') {
    model.status = { filterType: 'set', values: [filters.status] };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): MemberLevelGridFilters => {
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
    rules:
      model?.rules && typeof model.rules === 'object' && 'filter' in model.rules
        ? trimText(String(model.rules.filter ?? ''))
        : '',
    status: (statusValues[0] as MemberLevelGridFilters['status'] | undefined) ?? 'ALL',
  };
};

export const MemberLevelsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('customers.write');
  const queryClient = useQueryClient();
  const [drawerForm] = Form.useForm<MemberLevelFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<MemberLevelLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<MemberLevelGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingLevel, setEditingLevel] = useState<MemberLevel | null>(null);
  const [activeLevelId, setActiveLevelId] = useState<string | null>(null);
  const [selectedLevelIds, setSelectedLevelIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');
  const [saving, setSaving] = useState(false);

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantId ? `customers.member-levels.pagination.v2.${tenantId}` : undefined,
  });

  const query = useQuery({
    queryKey: ['member-levels', tenantId, searchKeyword],
    queryFn: () =>
      listMemberLevels(tenantId, { keyword: searchKeyword || undefined, offset: 0, limit: 500 }),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const searchScopedRows = useMemo(() => {
    const rows = query.data?.items ?? [];
    const keyword = trimText(searchKeyword).toLowerCase();
    if (!keyword) {
      return rows;
    }
    return rows.filter((row) =>
      [row.name, row.rules ?? '', row.status ?? '']
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword)),
    );
  }, [query.data, searchKeyword]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部等级', value: searchScopedRows.length, color: '#2f6fed' },
      {
        key: 'active',
        label: '启用',
        value: searchScopedRows.filter((row) => row.status === 'ACTIVE').length,
        color: '#0f9d58',
      },
      {
        key: 'discount',
        label: '已配折扣',
        value: searchScopedRows.filter((row) => row.discount_rate != null).length,
        color: '#d48806',
      },
    ],
    [searchScopedRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部等级',
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
        key: 'discount:yes',
        label: '已配折扣',
        count: searchScopedRows.filter((row) => row.discount_rate != null).length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'discount:yes',
      },
      {
        key: 'discount:no',
        label: '无折扣规则',
        count: searchScopedRows.filter((row) => row.discount_rate == null).length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'discount:no',
      },
    ],
    [leftFilterKey, searchScopedRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return searchScopedRows.filter((row) => {
      if (leftFilterKey === 'status:ACTIVE' && row.status !== 'ACTIVE') return false;
      if (leftFilterKey === 'status:INACTIVE' && row.status !== 'INACTIVE') return false;
      if (leftFilterKey === 'discount:yes' && row.discount_rate == null) return false;
      if (leftFilterKey === 'discount:no' && row.discount_rate != null) return false;
      if (!keyword) return true;
      return [row.name, row.rules ?? '', row.status ?? '']
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
        if (trimText(gridFilters.rules) && !trimText(row.rules).toLowerCase().includes(trimText(gridFilters.rules).toLowerCase())) {
          return false;
        }
        if (gridFilters.status !== 'ALL' && row.status !== gridFilters.status) {
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
    setActiveLevelId((previous) =>
      previous && pageRows.some((item) => item.level_id === previous) ? previous : pageRows[0]?.level_id ?? null,
    );
    setSelectedLevelIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.level_id === item)),
    );
  }, [pageRows]);

  const activeLevel = useMemo(
    () => pageRows.find((item) => item.level_id === activeLevelId) ?? null,
    [activeLevelId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<MemberLevel>[]>(
    () => [
      {
        key: 'name',
        field: 'name',
        title: '等级名称',
        width: 200,
        minWidth: 160,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      { key: 'level_rank', field: 'level_rank', title: '等级排序', width: 110 },
      {
        key: 'discount_rate',
        field: 'discount_rate',
        title: '折扣',
        width: 110,
        renderCell: (value) => formatDiscount(Number.isFinite(Number(value)) ? Number(value) : null),
      },
      {
        key: 'rules',
        field: 'rules',
        title: '规则',
        width: 320,
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
    setEditingLevel(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ level_rank: 1, status: 'ACTIVE' });
    setDrawerOpen(true);
  };

  const openEditDrawer = (level: MemberLevel) => {
    setEditingLevel(level);
    drawerForm.setFieldsValue({
      name: level.name,
      level_rank: level.level_rank,
      discount_rate: level.discount_rate ?? undefined,
      rules: level.rules ?? undefined,
      status: (level.status as 'ACTIVE' | 'INACTIVE' | undefined) ?? undefined,
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
      const payload: MemberLevelCreatePayload = {
        name: values.name.trim(),
        level_rank: values.level_rank ?? 0,
        discount_rate: values.discount_rate,
        rules: trimText(values.rules) || undefined,
        status: values.status,
      };
      setSaving(true);
      if (editingLevel) {
        await updateMemberLevel(tenantId, editingLevel.level_id, payload);
        message.success('会员等级已更新');
      } else {
        await createMemberLevel(tenantId, payload);
        message.success('会员等级已创建');
      }
      setDrawerOpen(false);
      setEditingLevel(null);
      await queryClient.invalidateQueries({ queryKey: ['member-levels', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteLevels = async (levelIds: string[]) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (!levelIds.length) {
      message.warning('请选择要删除的会员等级');
      return;
    }
    try {
      await Promise.all(levelIds.map((levelId) => deleteMemberLevel(tenantId, levelId)));
      message.success(`已删除 ${levelIds.length} 个会员等级`);
      setSelectedLevelIds([]);
      await queryClient.invalidateQueries({ queryKey: ['member-levels', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除会员等级失败');
    }
  };

  const handleUpdateStatus = async (nextStatus: 'ACTIVE' | 'INACTIVE') => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    const targetIds = selectedLevelIds.length > 0 ? selectedLevelIds : activeLevelId ? [activeLevelId] : [];
    if (!targetIds.length) {
      message.warning(nextStatus === 'ACTIVE' ? '请选择要启用的会员等级' : '请选择要停用的会员等级');
      return;
    }
    try {
      await Promise.all(targetIds.map((levelId) => updateMemberLevel(tenantId, levelId, { status: nextStatus })));
      message.success(nextStatus === 'ACTIVE' ? '会员等级已批量启用' : '会员等级已批量停用');
      await queryClient.invalidateQueries({ queryKey: ['member-levels', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新会员等级状态失败');
    }
  };

  const toolbarLeftContent = useMemo(() => {
    const targetIds = selectedLevelIds.length > 0 ? selectedLevelIds : activeLevelId ? [activeLevelId] : [];
    const targetLevel = pageRows.find((item) => item.level_id === targetIds[0]) ?? activeLevel;
    return (
      <Space size={8} wrap>
        <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>
          新建等级
        </Button>
        <Button
          size="small"
          disabled={!canWrite || targetIds.length !== 1 || !targetLevel}
          onClick={() => targetLevel && openEditDrawer(targetLevel)}
        >
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
              title: `确认删除选中的 ${targetIds.length} 个会员等级吗？`,
              okText: '删除',
              cancelText: '取消',
              onOk: async () => handleDeleteLevels(targetIds),
            });
          }}
        >
          删除
        </Button>
      </Space>
    );
  }, [activeLevel, activeLevelId, canWrite, pageRows, selectedLevelIds]);

  const rightPanel = activeLevel ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'rules']}
      panels={[
        {
          key: 'detail',
          label: '等级详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="等级名称">{activeLevel.name}</Descriptions.Item>
              <Descriptions.Item label="等级排序">{activeLevel.level_rank}</Descriptions.Item>
              <Descriptions.Item label="折扣">{formatDiscount(activeLevel.discount_rate)}</Descriptions.Item>
              <Descriptions.Item label="状态">
                {activeLevel.status === 'ACTIVE' ? '启用' : '停用'}
              </Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'rules',
          label: '等级规则',
          children: (
            <Typography.Text type="secondary">
              {trimText(activeLevel.rules) ||
                '当前等级暂未配置额外规则，工作台仍保持统一的左筛选、主表、右详情和底部明细结构。'}
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="等级详情">
      <Typography.Text type="secondary">请选择一条会员等级记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeLevel
    ? [
        { key: 'name', field: '等级名称', value: activeLevel.name },
        { key: 'rank', field: '等级排序', value: String(activeLevel.level_rank) },
        { key: 'discount', field: '折扣', value: formatDiscount(activeLevel.discount_rate) },
      ]
    : [];

  const ruleRows: DirectoryBottomDetailRow[] = activeLevel
    ? [
        { key: 'rules', field: '等级规则', value: trimText(activeLevel.rules) || '-' },
        { key: 'status', field: '状态', value: activeLevel.status === 'ACTIVE' ? '启用' : '停用' },
        { key: 'search', field: '当前检索词', value: trimText(searchKeyword) || '-' },
      ]
    : [];

  const timelineRows: DirectoryBottomDetailRow[] = activeLevel
    ? [
        { key: 'created_at', field: '创建时间', value: formatDateTime(activeLevel.created_at) },
        { key: 'updated_at', field: '更新时间', value: formatDateTime(activeLevel.updated_at ?? activeLevel.created_at) },
        { key: 'selected', field: '当前勾选数', value: String(selectedLevelIds.length) },
      ]
    : [];

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="member-levels-workbench-page"
        className="order-review-page member-levels-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="member-levels-left-panel"
            dataTestId="member-levels-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as MemberLevelLeftFilterKey | undefined;
              if (!nextKey) return;
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems.map(({ color, value }) => ({ color, value })))}
            globalOrderTotal={searchScopedRows.length}
            statsScopeText="口径：当前会员等级查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">等级检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按名称 / 规则 / 状态缩小左侧结果"
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
        rightPanelSyncKey={activeLevel?.level_id ?? 'empty'}
        bottomPanel={
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
                  Boolean(activeLevel),
                  profileRows,
                  '暂无基础资料',
                  `member-level-profile-${activeLevel?.level_id ?? 'empty'}`,
                ),
              },
              {
                key: 'rule',
                label: `规则说明 (${ruleRows.length})`,
                children: renderDirectoryBottomTable(
                  Boolean(activeLevel),
                  ruleRows,
                  '暂无规则说明',
                  `member-level-rule-${activeLevel?.level_id ?? 'empty'}`,
                ),
              },
              {
                key: 'timeline',
                label: `时间轨迹 (${timelineRows.length})`,
                children: renderDirectoryBottomTable(
                  Boolean(activeLevel),
                  timelineRows,
                  '暂无时间轨迹',
                  `member-level-timeline-${activeLevel?.level_id ?? 'empty'}`,
                ),
              },
            ]}
          />
        }
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
            searchPlaceholder="搜索等级名称 / 规则 / 状态"
            onExport={() => message.info('会员等级目录导出能力待接入')}
            onPrint={() => message.info('会员等级目录打印能力待接入')}
            onHelp={() => message.info('会员等级目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<MemberLevel>
            rowKey={(record) => record.level_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="customers.member-levels.main-grid"
            dataTestId="member-levels-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={query.isLoading || query.isFetching}
            error={query.error}
            onRetry={() => void query.refetch()}
            errorTitle="加载会员等级失败"
            emptyText="暂无会员等级"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedLevelIds}
            activeRowKey={activeLevelId}
            onSelectionChange={(keys) => {
              setSelectedLevelIds(normalizeSelectionKeys(keys));
            }}
            onActiveRowChange={(record) => {
              setActiveLevelId(record.level_id);
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
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedLevelIds.length} 条`}</span>
            <span>{activeLevel ? `当前焦点：${activeLevel.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`member-levels-${displayRows.length}-${pageRows.length}-${selectedLevelIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingLevel ? '编辑会员等级' : '新建会员等级'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={560}
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
          <Form.Item name="name" label="等级名称" rules={[{ required: true, message: '请输入等级名称' }]}>
            <Input placeholder="等级名称" />
          </Form.Item>
          <Form.Item name="level_rank" label="等级排序">
            <InputNumber min={0} style={{ width: '100%' }} placeholder="等级排序" />
          </Form.Item>
          <Form.Item name="discount_rate" label="折扣">
            <InputNumber min={0} max={1} step={0.01} style={{ width: '100%' }} placeholder="例如 0.95" />
          </Form.Item>
          <Form.Item name="rules" label="规则">
            <Input.TextArea rows={4} placeholder="等级规则" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
