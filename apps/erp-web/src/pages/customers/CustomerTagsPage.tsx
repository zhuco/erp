import {
  Button,
  Descriptions,
  Drawer,
  Form,
  Input,
  Modal,
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

import type { CustomerTag, CustomerTagCreatePayload } from '@/api/customerTags';
import {
  createCustomerTag,
  deleteCustomerTag,
  listCustomerTags,
  updateCustomerTag,
} from '@/api/customerTags';
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
import { IndustrialCard } from '@/components/industrial';

type CustomerTagFormValues = {
  name: string;
  color?: string;
};

type CustomerTagGridFilters = {
  name: string;
  color: string;
};

type CustomerTagLeftFilterKey = 'view:all' | 'color:yes' | 'color:no' | 'color:warm' | 'color:cold';

const pageSizeOptions = ['20', '50', '100', '200'];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const createDefaultGridFilters = (): CustomerTagGridFilters => ({
  name: '',
  color: '',
});

const buildGridFilterModel = (filters: CustomerTagGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  if (trimText(filters.color)) {
    model.color = { filterType: 'text', type: 'contains', filter: trimText(filters.color) };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): CustomerTagGridFilters => ({
  name:
    model?.name && typeof model.name === 'object' && 'filter' in model.name
      ? trimText(String(model.name.filter ?? ''))
      : '',
  color:
    model?.color && typeof model.color === 'object' && 'filter' in model.color
      ? trimText(String(model.color.filter ?? ''))
      : '',
});

const isWarmColor = (color?: string | null) => {
  const normalized = trimText(color).toLowerCase();
  return ['red', 'orange', 'gold', 'yellow', 'pink', '#ff', '#fa', '#fd', '#fe'].some((token) =>
    normalized.includes(token),
  );
};

const isColdColor = (color?: string | null) => {
  const normalized = trimText(color).toLowerCase();
  return ['blue', 'cyan', 'green', 'teal', 'purple', '#0f', '#1f', '#2f', '#3f'].some((token) =>
    normalized.includes(token),
  );
};

export const CustomerTagsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('customers.write');
  const queryClient = useQueryClient();
  const [drawerForm] = Form.useForm<CustomerTagFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<CustomerTagLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<CustomerTagGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingTag, setEditingTag] = useState<CustomerTag | null>(null);
  const [activeTagId, setActiveTagId] = useState<string | null>(null);
  const [selectedTagIds, setSelectedTagIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');
  const [saving, setSaving] = useState(false);

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantId ? `customers.tags.pagination.v2.${tenantId}` : undefined,
  });

  const query = useQuery({
    queryKey: ['customer-tags', tenantId, searchKeyword],
    queryFn: () =>
      listCustomerTags(tenantId, { keyword: searchKeyword || undefined, offset: 0, limit: 500 }),
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
      [row.name, row.color ?? '']
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword)),
    );
  }, [query.data, searchKeyword]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部标签', value: searchScopedRows.length, color: '#2f6fed' },
      {
        key: 'colored',
        label: '已配颜色',
        value: searchScopedRows.filter((row) => trimText(row.color)).length,
        color: '#0f9d58',
      },
      {
        key: 'warm',
        label: '暖色标签',
        value: searchScopedRows.filter((row) => isWarmColor(row.color)).length,
        color: '#d48806',
      },
    ],
    [searchScopedRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部标签',
        count: searchScopedRows.length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'color:yes',
        label: '已配颜色',
        count: searchScopedRows.filter((row) => trimText(row.color)).length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'color:yes',
      },
      {
        key: 'color:no',
        label: '待补颜色',
        count: searchScopedRows.filter((row) => !trimText(row.color)).length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'color:no',
      },
      {
        key: 'color:warm',
        label: '暖色标签',
        count: searchScopedRows.filter((row) => isWarmColor(row.color)).length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'color:warm',
      },
      {
        key: 'color:cold',
        label: '冷色标签',
        count: searchScopedRows.filter((row) => isColdColor(row.color)).length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'color:cold',
      },
    ],
    [leftFilterKey, searchScopedRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return searchScopedRows.filter((row) => {
      if (leftFilterKey === 'color:yes' && !trimText(row.color)) return false;
      if (leftFilterKey === 'color:no' && trimText(row.color)) return false;
      if (leftFilterKey === 'color:warm' && !isWarmColor(row.color)) return false;
      if (leftFilterKey === 'color:cold' && !isColdColor(row.color)) return false;
      if (!keyword) return true;
      return [row.name, row.color ?? '']
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
        if (trimText(gridFilters.color) && !trimText(row.color).toLowerCase().includes(trimText(gridFilters.color).toLowerCase())) {
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
    setActiveTagId((previous) =>
      previous && pageRows.some((item) => item.tag_id === previous) ? previous : pageRows[0]?.tag_id ?? null,
    );
    setSelectedTagIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.tag_id === item)),
    );
  }, [pageRows]);

  const activeTag = useMemo(
    () => pageRows.find((item) => item.tag_id === activeTagId) ?? null,
    [activeTagId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<CustomerTag>[]>(
    () => [
      {
        key: 'name',
        field: 'name',
        title: '标签名称',
        width: 220,
        minWidth: 180,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'color',
        field: 'color',
        title: '颜色',
        width: 180,
        minWidth: 140,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
        renderCell: (value) => {
          const color = trimText(String(value ?? ''));
          return color ? <Tag color={color}>{color}</Tag> : '-';
        },
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
    setEditingTag(null);
    drawerForm.resetFields();
    setDrawerOpen(true);
  };

  const openEditDrawer = (tag: CustomerTag) => {
    setEditingTag(tag);
    drawerForm.setFieldsValue({
      name: tag.name,
      color: tag.color ?? undefined,
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
      const payload: CustomerTagCreatePayload = {
        name: values.name.trim(),
        color: trimText(values.color) || undefined,
      };
      setSaving(true);
      if (editingTag) {
        await updateCustomerTag(tenantId, editingTag.tag_id, payload);
        message.success('客户标签已更新');
      } else {
        await createCustomerTag(tenantId, payload);
        message.success('客户标签已创建');
      }
      setDrawerOpen(false);
      setEditingTag(null);
      await queryClient.invalidateQueries({ queryKey: ['customer-tags', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteTags = async (tagIds: string[]) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (!tagIds.length) {
      message.warning('请选择要删除的客户标签');
      return;
    }
    try {
      await Promise.all(tagIds.map((tagId) => deleteCustomerTag(tenantId, tagId)));
      message.success(`已删除 ${tagIds.length} 个客户标签`);
      setSelectedTagIds([]);
      await queryClient.invalidateQueries({ queryKey: ['customer-tags', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除客户标签失败');
    }
  };

  const toolbarLeftContent = useMemo(() => {
    const targetIds = selectedTagIds.length > 0 ? selectedTagIds : activeTagId ? [activeTagId] : [];
    const targetTag = pageRows.find((item) => item.tag_id === targetIds[0]) ?? activeTag;
    return (
      <Space size={8} wrap>
        <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>
          新建标签
        </Button>
        <Button
          size="small"
          disabled={!canWrite || targetIds.length !== 1 || !targetTag}
          onClick={() => targetTag && openEditDrawer(targetTag)}
        >
          编辑
        </Button>
        <Button
          size="small"
          danger
          disabled={!canWrite || targetIds.length === 0}
          onClick={() => {
            Modal.confirm({
              title: `确认删除选中的 ${targetIds.length} 个客户标签吗？`,
              okText: '删除',
              cancelText: '取消',
              onOk: async () => handleDeleteTags(targetIds),
            });
          }}
        >
          删除
        </Button>
      </Space>
    );
  }, [activeTag, activeTagId, canWrite, pageRows, selectedTagIds]);

  const rightPanel = activeTag ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'rule']}
      panels={[
        {
          key: 'detail',
          label: '标签详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="标签名称">{activeTag.name}</Descriptions.Item>
              <Descriptions.Item label="颜色">
                {trimText(activeTag.color) ? (
                  <Tag color={activeTag.color ?? undefined}>{activeTag.color}</Tag>
                ) : (
                  '-'
                )}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(activeTag.updated_at ?? activeTag.created_at)}
              </Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'rule',
          label: '工作台约束',
          children: (
            <Typography.Text type="secondary">
              客户标签目录延续订单审核的工业工作台结构，左侧用标签色系做快速筛选，中间维持紧凑表格，右侧与底部分别承载标签详情和运维上下文。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <IndustrialCard size="small" title="标签详情">
      <Typography.Text type="secondary">请选择一条客户标签记录查看详情。</Typography.Text>
    </IndustrialCard>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeTag
    ? [
        { key: 'name', field: '标签名称', value: activeTag.name },
        { key: 'color', field: '颜色', value: trimText(activeTag.color) || '-' },
        { key: 'search', field: '当前检索词', value: trimText(searchKeyword) || '-' },
      ]
    : [];

  const paletteRows: DirectoryBottomDetailRow[] = activeTag
    ? [
        { key: 'warm', field: '暖色标签', value: isWarmColor(activeTag.color) ? '是' : '否' },
        { key: 'cold', field: '冷色标签', value: isColdColor(activeTag.color) ? '是' : '否' },
        { key: 'left-filter', field: '左侧筛选', value: leftFilterKey },
      ]
    : [];

  const timelineRows: DirectoryBottomDetailRow[] = activeTag
    ? [
        { key: 'created_at', field: '创建时间', value: formatDateTime(activeTag.created_at) },
        { key: 'updated_at', field: '更新时间', value: formatDateTime(activeTag.updated_at ?? activeTag.created_at) },
        { key: 'selected', field: '当前勾选数', value: String(selectedTagIds.length) },
      ]
    : [];

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="customer-tags-workbench-page"
        className="order-review-page customer-tags-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="customer-tags-left-panel"
            dataTestId="customer-tags-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as CustomerTagLeftFilterKey | undefined;
              if (!nextKey) return;
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems.map(({ color, value }) => ({ color, value })))}
            globalOrderTotal={searchScopedRows.length}
            statsScopeText="口径：当前客户标签查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">标签检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按名称 / 颜色缩小左侧结果"
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
        rightPanelSyncKey={activeTag?.tag_id ?? 'empty'}
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
                  Boolean(activeTag),
                  profileRows,
                  '暂无基础资料',
                  `customer-tag-profile-${activeTag?.tag_id ?? 'empty'}`,
                ),
              },
              {
                key: 'palette',
                label: `色盘上下文 (${paletteRows.length})`,
                children: renderDirectoryBottomTable(
                  Boolean(activeTag),
                  paletteRows,
                  '暂无色盘上下文',
                  `customer-tag-palette-${activeTag?.tag_id ?? 'empty'}`,
                ),
              },
              {
                key: 'timeline',
                label: `时间轨迹 (${timelineRows.length})`,
                children: renderDirectoryBottomTable(
                  Boolean(activeTag),
                  timelineRows,
                  '暂无时间轨迹',
                  `customer-tag-timeline-${activeTag?.tag_id ?? 'empty'}`,
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
            searchPlaceholder="搜索标签名称 / 颜色"
            onExport={() => message.info('客户标签目录导出能力待接入')}
            onPrint={() => message.info('客户标签目录打印能力待接入')}
            onHelp={() => message.info('客户标签目录帮助文档待接入')}
          />
        }
        tableContent={
          <ErpDataGridNext<CustomerTag>
            rowKey={(record) => record.tag_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="customers.tags.main-grid"
            dataTestId="customer-tags-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={query.isLoading || query.isFetching}
            error={query.error}
            onRetry={() => void query.refetch()}
            errorTitle="加载客户标签失败"
            emptyText="暂无客户标签"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedTagIds}
            activeRowKey={activeTagId}
            onSelectionChange={(keys) => {
              setSelectedTagIds(normalizeSelectionKeys(keys));
            }}
            onActiveRowChange={(record) => {
              setActiveTagId(record.tag_id);
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
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedTagIds.length} 条`}</span>
            <span>{activeTag ? `当前焦点：${activeTag.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`customer-tags-${displayRows.length}-${pageRows.length}-${selectedTagIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingTag ? '编辑客户标签' : '新建客户标签'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={460}
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
          <Form.Item name="name" label="标签名称" rules={[{ required: true, message: '请输入标签名称' }]}>
            <Input placeholder="标签名称" />
          </Form.Item>
          <Form.Item name="color" label="颜色">
            <Input placeholder="颜色值，例如 gold / #1677ff" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
