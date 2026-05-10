import './channelDirectoryPage.css';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Drawer,
  Empty,
  Form,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tabs,
  Switch,
  Typography,
  message,
} from 'antd';
import type { MenuProps } from 'antd';
import type { Key, MouseEvent as ReactMouseEvent } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import {
  createChannel,
  deleteChannel,
  listChannels,
  updateChannel,
  type Channel,
  type ChannelStatus,
  type ChannelType,
} from '@/api/channels';
import { listWarehouses } from '@/api/warehouses';
import {
  WorkbenchPageTemplate,
  WorkbenchToolbar,
  useWorkbenchInteractionController,
  useWorkbenchSavedViewState,
  useWorkbenchSelectionController,
  useWorkbenchTablePagination,
  useWorkbenchViewStateController,
} from '@/components/page-templates';
import { ErpDataGridNext } from '@/components/ErpDataGridNext';
import { UiModule03ContextMenu, UiModule04BottomTable } from '@/components/ui-modules';
import { platformOptions, type PlatformCode } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';
import { useLeftFilterModule } from '@/pages/orders/shared/useLeftFilterModule';

import { ChannelDirectoryLeftPanel, ChannelDirectoryRightPanel } from './ChannelDirectoryPanels';
import { buildChannelDirectoryColumns } from './channelDirectoryColumns';
import {
  buildChannelDirectoryLeftCheckedKeys,
  buildChannelDirectoryLeftNodes,
  buildChannelDirectoryPieBackground,
  buildChannelDirectoryGridFilterModel,
  createDefaultChannelDirectoryColumnFilters,
  createDefaultChannelDirectoryLeftFilters,
  extractChannelDirectoryGridFilterModel,
  filterChannelsByColumnFilters,
  filterChannelsByKeyword,
  filterChannelsByLeftFilters,
  hasChannelDirectoryLeftManualFilter,
  reduceChannelDirectoryLeftFiltersByNode,
  resolveChannelAuthorizationEnabled,
  resolveChannelRowKey,
  summarizeChannels,
  type ChannelDirectoryColumnFilters,
  type ChannelDirectoryLeftFilters,
} from './channelDirectoryModel';

type ChannelDirectoryFormValues = {
  name: string;
  channel_type: ChannelType;
  platform?: PlatformCode;
  contact_name?: string;
  contact_phone?: string;
  contact_address?: string;
  department?: string;
  owner?: string;
  commission_formula?: string;
  default_warehouse_id?: string;
  allow_manual_order?: boolean;
  allow_manual_after_sales?: boolean;
  status: ChannelStatus;
};

type ChannelDirectorySavedViewState = {
  searchInput: string;
  searchKeyword: string;
  leftFilters: ChannelDirectoryLeftFilters;
  columnFilters: ChannelDirectoryColumnFilters;
};

const pageSizeOptions = ['20', '50', '100', '200'];

const normalizeNullableText = (value?: string | null) => {
  const nextValue = String(value ?? '').trim();
  return nextValue || null;
};

type ChannelDetailRow = {
  key: string;
  field: string;
  value: string;
};

export const ChannelDirectoryPage = () => {
  const tenantId = useTenantId();
  const userId = useSessionStore((state) => state.userId);
  const canWrite = useCan('channels.write');
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [editorForm] = Form.useForm<ChannelDirectoryFormValues>();

  const savedViewKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    return `channels.directory.saved-view.v1.${tenantId}.${String(userId ?? '').trim() || 'anonymous'}`;
  }, [tenantId, userId]);

  const paginationStorageKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    return `channels.directory.pagination.v1.${tenantId}.${String(userId ?? '').trim() || 'anonymous'}`;
  }, [tenantId, userId]);

  const stateCacheKey = useMemo(() => {
    if (!tenantId) {
      return '';
    }
    return `channels.directory.workbench-state.v1.${tenantId}.${String(userId ?? '').trim() || 'anonymous'}`;
  }, [tenantId, userId]);

  const { pagination, resetPagination, handlePaginationChange } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: paginationStorageKey,
  });

  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftFilters, setLeftFilters] = useState<ChannelDirectoryLeftFilters>(
    createDefaultChannelDirectoryLeftFilters,
  );
  const [columnFilters, setColumnFilters] = useState<ChannelDirectoryColumnFilters>(
    createDefaultChannelDirectoryColumnFilters,
  );
  const gridFilterModel = useMemo(
    () => buildChannelDirectoryGridFilterModel(columnFilters),
    [columnFilters],
  );
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);

  const channelsQuery = useQuery({
    queryKey: ['channels', tenantId, 'directory'],
    queryFn: () =>
      listChannels(tenantId, {
        offset: 0,
        limit: 500,
      }),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
  });

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'channel-directory'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const sourceRows = channelsQuery.data?.items ?? [];
  const searchScopedRows = useMemo(
    () => filterChannelsByKeyword(sourceRows, searchKeyword),
    [searchKeyword, sourceRows],
  );

  const summary = useMemo(() => summarizeChannels(searchScopedRows), [searchScopedRows]);
  const pieBackground = useMemo(
    () => buildChannelDirectoryPieBackground(summary),
    [summary],
  );

  const {
    filteredRows: leftFilteredRows,
    leftPanel,
  } = useLeftFilterModule<Channel, ChannelDirectoryLeftFilters>({
    rows: searchScopedRows,
    filters: leftFilters,
    onFiltersChange: (action) =>
      setLeftFilters((previous) => (typeof action === 'function' ? action(previous) : action)),
    initialExpandedKeys: ['group:overview', 'group:type', 'group:capability'],
    emptyRowLabel: '没有可用筛选项',
    hasManualFilter: hasChannelDirectoryLeftManualFilter,
    filterRows: filterChannelsByLeftFilters,
    buildCheckedKeys: buildChannelDirectoryLeftCheckedKeys,
    buildLeftNodes: buildChannelDirectoryLeftNodes,
    reduceFiltersByNode: reduceChannelDirectoryLeftFiltersByNode,
    onBeforeLeafFilterApply: resetPagination,
    renderPanel: ({ leftTab, onLeftTabChange, leftRowsForRender, onLeftRowClick, hasManualFilterActive }) => (
      <ChannelDirectoryLeftPanel
        leftTab={leftTab}
        onLeftTabChange={onLeftTabChange}
        leftRowsForRender={leftRowsForRender}
        onLeftRowClick={onLeftRowClick}
        hasManualFilterActive={hasManualFilterActive}
        filters={leftFilters}
        summaryItems={[
          { key: 'total', label: '全部渠道', value: summary.total, color: '#2f6fed' },
          { key: 'online', label: '线上', value: summary.online, color: '#2f6fed' },
          { key: 'offline', label: '线下', value: summary.offline, color: '#0f9d58' },
          { key: 'active', label: '启用', value: summary.active, color: '#1f8b4c' },
        ]}
        pieBackground={pieBackground}
        totalCount={summary.total}
        isFetching={channelsQuery.isFetching}
        onPatchFilters={(patch) => {
          resetPagination();
          setLeftFilters((previous) => ({ ...previous, ...patch }));
        }}
        onResetFilters={() => {
          resetPagination();
          setLeftFilters(createDefaultChannelDirectoryLeftFilters());
        }}
      />
    ),
  });

  const displayRows = useMemo(
    () => filterChannelsByColumnFilters(leftFilteredRows, columnFilters),
    [columnFilters, leftFilteredRows],
  );

  const currentPage = Number(pagination.current ?? 1);
  const currentPageSize = Number(pagination.pageSize ?? 20);
  const pagedRows = useMemo(() => {
    const start = (currentPage - 1) * currentPageSize;
    return displayRows.slice(start, start + currentPageSize);
  }, [currentPage, currentPageSize, displayRows]);

  const {
    activeRow,
    activeRowKey,
    focusedRowKey,
    selectedRowKeys,
    selectionState,
    setActiveRowKey,
    setFocusedRowKey,
    setSelectedRowKeys,
    setSelectionAnchorKey,
  } = useWorkbenchSelectionController<Channel>({
    rows: pagedRows,
    loading: channelsQuery.isFetching,
    resolveRowKey: resolveChannelRowKey,
  });

  const { hydrateViewState, serializeViewState } = useWorkbenchViewStateController<
    ChannelDirectorySavedViewState
  >({
    extraState: {
      searchInput,
      searchKeyword,
      leftFilters,
      columnFilters,
    },
    extraStateKeys: ['searchInput', 'searchKeyword', 'leftFilters', 'columnFilters'],
    selectionState,
    persistSelectionState: false,
    onHydrateExtraState: (state) => {
      if (typeof state.searchInput === 'string') {
        setSearchInput(state.searchInput);
      }
      if (typeof state.searchKeyword === 'string') {
        setSearchKeyword(state.searchKeyword);
      }
      if (state.leftFilters && typeof state.leftFilters === 'object') {
        setLeftFilters(state.leftFilters);
      }
      if (state.columnFilters && typeof state.columnFilters === 'object') {
        setColumnFilters(state.columnFilters);
      }
    },
    onHydrateSelectionState: (state) => {
      if (Object.prototype.hasOwnProperty.call(state, 'selectedRowKeys')) {
        setSelectedRowKeys((state.selectedRowKeys ?? []) as string[]);
      }
      if (Object.prototype.hasOwnProperty.call(state, 'focusedRowKey')) {
        setFocusedRowKey((state.focusedRowKey as string | null) ?? null);
      }
      if (Object.prototype.hasOwnProperty.call(state, 'activeRowKey')) {
        setActiveRowKey((state.activeRowKey as string | null) ?? null);
      }
      if (Object.prototype.hasOwnProperty.call(state, 'selectionAnchorKey')) {
        setSelectionAnchorKey((state.selectionAnchorKey as string | null) ?? null);
      }
    },
  });

  useWorkbenchSavedViewState<ChannelDirectorySavedViewState>({
    storageKey: savedViewKey,
    hydrate: hydrateViewState,
    serialize: serializeViewState,
  });

  const warehouseMap = useMemo(
    () =>
      new Map(
        (warehousesQuery.data?.items ?? []).map((warehouse) => [warehouse.warehouse_id, warehouse.name]),
      ),
    [warehousesQuery.data?.items],
  );

  const warehouseOptions = useMemo(
    () =>
      (warehousesQuery.data?.items ?? []).map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id,
      })),
    [warehousesQuery.data?.items],
  );

  const columns = useMemo(
    () => buildChannelDirectoryColumns(warehouseMap, warehouseOptions),
    [warehouseMap, warehouseOptions],
  );

  const selectedChannelMap = useMemo(
    () => new Map(pagedRows.map((channel) => [channel.channel_id, channel])),
    [pagedRows],
  );

  const resolveActionChannels = useCallback(() => {
    if (selectedRowKeys.length > 0) {
      return selectedRowKeys
        .map((rowKey) => selectedChannelMap.get(String(rowKey)))
        .filter((channel): channel is Channel => Boolean(channel));
    }
    return activeRow ? [activeRow] : [];
  }, [activeRow, selectedChannelMap, selectedRowKeys]);

  const saveMutation = useMutation({
    mutationFn: async (values: ChannelDirectoryFormValues) => {
      if (!tenantId) {
        throw new Error('请先选择租户');
      }
      const payload = {
        name: values.name.trim(),
        channel_type: values.channel_type,
        platform: values.channel_type === 'OFFLINE' ? null : values.platform ?? null,
        contact_name: normalizeNullableText(values.contact_name),
        contact_phone: normalizeNullableText(values.contact_phone),
        contact_address: normalizeNullableText(values.contact_address),
        department: normalizeNullableText(values.department),
        owner: normalizeNullableText(values.owner),
        commission_formula: normalizeNullableText(values.commission_formula),
        default_warehouse_id: normalizeNullableText(values.default_warehouse_id),
        allow_manual_order: Boolean(values.allow_manual_order),
        allow_manual_after_sales: Boolean(values.allow_manual_after_sales),
        status: values.status,
      } as const;

      if (editingChannel) {
        return updateChannel(tenantId, editingChannel.channel_id, payload);
      }
      return createChannel(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingChannel ? '渠道已更新' : '渠道已创建');
      setEditorOpen(false);
      setEditingChannel(null);
      await queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存渠道失败');
    },
  });

  const handleSearch = useCallback(() => {
    setSearchKeyword(searchInput.trim());
    resetPagination();
  }, [resetPagination, searchInput]);

  const openCreateDrawer = useCallback(() => {
    setEditingChannel(null);
    editorForm.resetFields();
    editorForm.setFieldsValue({
      channel_type: 'ONLINE',
      status: 'ACTIVE',
      allow_manual_order: true,
      allow_manual_after_sales: false,
    });
    setEditorOpen(true);
  }, [editorForm]);

  const openEditDrawer = useCallback(
    (channel: Channel) => {
      setEditingChannel(channel);
      editorForm.setFieldsValue({
        name: channel.name,
        channel_type: channel.channel_type ?? 'ONLINE',
        platform: channel.platform ?? undefined,
        contact_name: channel.contact_name ?? undefined,
        contact_phone: channel.contact_phone ?? undefined,
        contact_address: channel.contact_address ?? undefined,
        department: channel.department ?? undefined,
        owner: channel.owner ?? undefined,
        commission_formula: channel.commission_formula ?? undefined,
        default_warehouse_id: channel.default_warehouse_id ?? undefined,
        allow_manual_order: channel.allow_manual_order,
        allow_manual_after_sales: channel.allow_manual_after_sales,
        status: channel.status,
      });
      setEditorOpen(true);
    },
    [editorForm],
  );

  const handleSave = useCallback(async () => {
    try {
      const values = await editorForm.validateFields();
      await saveMutation.mutateAsync(values);
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    }
  }, [editorForm, saveMutation]);

  const handleDeleteSelected = useCallback(() => {
    const targetChannels = resolveActionChannels();
    if (targetChannels.length === 0) {
      message.warning('请先选择要删除的渠道');
      return;
    }

    Modal.confirm({
      title: `确认删除选中的 ${targetChannels.length} 个渠道吗？`,
      okText: '删除',
      cancelText: '取消',
      onOk: async () => {
        await Promise.all(
          targetChannels.map((channel) => deleteChannel(tenantId, channel.channel_id)),
        );
        message.success(`已删除 ${targetChannels.length} 个渠道`);
        setSelectedRowKeys([]);
        setSelectionAnchorKey(null);
        await queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
      },
    });
  }, [queryClient, resolveActionChannels, setSelectedRowKeys, setSelectionAnchorKey, tenantId]);

  const handleDisableSelected = useCallback(() => {
    const targetChannels = resolveActionChannels().filter((channel) => channel.status !== 'DISABLED');
    if (targetChannels.length === 0) {
      message.warning('当前没有可停用的渠道');
      return;
    }

    Modal.confirm({
      title: `确认停用选中的 ${targetChannels.length} 个渠道吗？`,
      okText: '停用',
      cancelText: '取消',
      onOk: async () => {
        await Promise.all(
          targetChannels.map((channel) =>
            updateChannel(tenantId, channel.channel_id, { status: 'DISABLED' }),
          ),
        );
        message.success(`已停用 ${targetChannels.length} 个渠道`);
        await queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
      },
    });
  }, [queryClient, resolveActionChannels, tenantId]);

  const handleOpenServicePurchase = useCallback(() => {
    if (!activeRow) {
      message.warning('请先选择渠道');
      return;
    }
    navigate(`/channels/service-purchase?channel_id=${activeRow.channel_id}`);
  }, [activeRow, navigate]);

  const handleOpenAuthorize = useCallback(() => {
    if (!activeRow) {
      message.warning('请先选择渠道');
      return;
    }
    if (!resolveChannelAuthorizationEnabled(activeRow)) {
      message.info('线下渠道无需授权');
      return;
    }
    navigate(`/channels/authorize?channel_id=${activeRow.channel_id}`);
  }, [activeRow, navigate]);

  const { closeContextMenu, contextMenuState, openContextMenu } =
    useWorkbenchInteractionController<Channel>();

  const handleContextMenu = useCallback(
    (channel: Channel, event: MouseEvent | ReactMouseEvent) => {
      const rowKey = resolveChannelRowKey(channel);
      setActiveRowKey(rowKey);
      setFocusedRowKey(rowKey);
      if (rowKey && !selectedRowKeys.some((item) => String(item) === rowKey)) {
        setSelectedRowKeys([rowKey]);
        setSelectionAnchorKey(rowKey);
      }
      openContextMenu(channel, 'nativeEvent' in event ? event.nativeEvent : event);
    },
    [
      openContextMenu,
      selectedRowKeys,
      setActiveRowKey,
      setFocusedRowKey,
      setSelectedRowKeys,
      setSelectionAnchorKey,
    ],
  );

  const contextMenuConfig = useMemo<MenuProps>(
    () => ({
      items: [
        { key: 'edit', label: '修改', disabled: !canWrite || !contextMenuState.row },
        { key: 'delete', label: '删除', disabled: !canWrite || !contextMenuState.row },
        { key: 'disable', label: '停用', disabled: !canWrite || !contextMenuState.row },
        { type: 'divider' },
        { key: 'service-purchase', label: '去订购', disabled: !contextMenuState.row },
        {
          key: 'authorize',
          label: '去授权',
          disabled:
            !contextMenuState.row ||
            !resolveChannelAuthorizationEnabled(contextMenuState.row),
        },
      ],
      onClick: ({ key }) => {
        const targetChannel = contextMenuState.row;
        if (!targetChannel) {
          return;
        }
        if (key === 'edit') {
          openEditDrawer(targetChannel);
        }
        if (key === 'delete') {
          Modal.confirm({
            title: `确认删除渠道 ${targetChannel.name} 吗？`,
            okText: '删除',
            cancelText: '取消',
            onOk: async () => {
              await deleteChannel(tenantId, targetChannel.channel_id);
              message.success('渠道已删除');
              await queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
            },
          });
        }
        if (key === 'disable') {
          Modal.confirm({
            title: `确认停用渠道 ${targetChannel.name} 吗？`,
            okText: '停用',
            cancelText: '取消',
            onOk: async () => {
              await updateChannel(tenantId, targetChannel.channel_id, { status: 'DISABLED' });
              message.success('渠道已停用');
              await queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
            },
          });
        }
        if (key === 'service-purchase') {
          navigate(`/channels/service-purchase?channel_id=${targetChannel.channel_id}`);
        }
        if (key === 'authorize') {
          navigate(`/channels/authorize?channel_id=${targetChannel.channel_id}`);
        }
        closeContextMenu();
      },
    }),
    [
      canWrite,
      closeContextMenu,
      contextMenuState.row,
      navigate,
      openEditDrawer,
      queryClient,
      tenantId,
    ],
  );

  const editorChannelType = Form.useWatch('channel_type', editorForm);

  useEffect(() => {
    if (editorChannelType === 'OFFLINE') {
      editorForm.setFieldValue('platform', undefined);
    }
  }, [editorChannelType, editorForm]);

  const toolbar = (
    <WorkbenchToolbar
      leftContent={
        <Space size={8} wrap>
          <Typography.Text strong className="channel-directory-title">
            渠道目录
          </Typography.Text>
          <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>
            新建渠道
          </Button>
          <Button
            size="small"
            disabled={!canWrite || resolveActionChannels().length !== 1}
            onClick={() => {
              const targetChannel = resolveActionChannels()[0];
              if (targetChannel) {
                openEditDrawer(targetChannel);
              }
            }}
          >
            修改
          </Button>
          <Button size="small" disabled={!canWrite} onClick={handleDeleteSelected}>
            删除
          </Button>
          <Button size="small" disabled={!canWrite} onClick={handleDisableSelected}>
            停用
          </Button>
        </Space>
      }
      searchValue={searchInput}
      onSearchChange={setSearchInput}
      onSearch={handleSearch}
      searchPlaceholder="搜索渠道名称 / 联系人 / 电话 / 地址 / 负责人"
      onExport={() => message.info('渠道目录导出能力待接入')}
      onPrint={() => message.info('渠道目录打印能力待接入')}
      onHelp={() => message.info('渠道目录帮助文档待接入')}
    />
  );

  const rightPanel = useMemo(
    () => (
      <ChannelDirectoryRightPanel
        activeChannel={activeRow}
        warehouseMap={warehouseMap}
        onOpenServicePurchase={handleOpenServicePurchase}
        onOpenAuthorize={handleOpenAuthorize}
      />
    ),
    [activeRow, handleOpenAuthorize, handleOpenServicePurchase, warehouseMap],
  );

  const rightPanelSyncKey = useMemo(
    () =>
      JSON.stringify({
        activeChannelId: activeRow?.channel_id ?? '',
        activeChannelStatus: activeRow?.status ?? '',
        activeChannelType: activeRow?.channel_type ?? '',
        activeChannelUpdatedAt: activeRow?.updated_at ?? '',
      }),
    [activeRow?.channel_id, activeRow?.channel_type, activeRow?.status, activeRow?.updated_at],
  );

  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const bottomDetailColumns = useMemo(
    () => [
      {
        title: '字段',
        dataIndex: 'field',
        key: 'field',
        width: 180,
      },
      {
        title: '值',
        dataIndex: 'value',
        key: 'value',
        width: 320,
      },
    ],
    [],
  );

  const channelProfileRows = useMemo<ChannelDetailRow[]>(() => {
    if (!activeRow) {
      return [];
    }
    return [
      { key: 'contact_name', field: '联系人', value: activeRow.contact_name || '-' },
      { key: 'contact_phone', field: '联系电话', value: activeRow.contact_phone || '-' },
      { key: 'contact_address', field: '联系地址', value: activeRow.contact_address || '-' },
      { key: 'department', field: '归属部门', value: activeRow.department || '-' },
      { key: 'owner', field: '负责人', value: activeRow.owner || '-' },
      {
        key: 'default_warehouse_id',
        field: '默认发货仓库',
        value: activeRow.default_warehouse_id
          ? warehouseMap.get(activeRow.default_warehouse_id) ?? activeRow.default_warehouse_id
          : '-',
      },
    ];
  }, [activeRow, warehouseMap]);

  const channelCapabilityRows = useMemo<ChannelDetailRow[]>(() => {
    if (!activeRow) {
      return [];
    }
    return [
      { key: 'channel_id', field: '渠道编号', value: activeRow.channel_id },
      {
        key: 'channel_type',
        field: '渠道类型',
        value: activeRow.channel_type === 'OFFLINE' ? '线下' : '线上',
      },
      {
        key: 'platform',
        field: '所属平台',
        value: activeRow.platform ? String(activeRow.platform) : '线下渠道',
      },
      {
        key: 'manual_order',
        field: '允许手工销售单',
        value: activeRow.allow_manual_order ? '允许' : '关闭',
      },
      {
        key: 'manual_after_sales',
        field: '允许手工售后单',
        value: activeRow.allow_manual_after_sales ? '允许' : '关闭',
      },
      {
        key: 'authorize',
        field: '授权能力',
        value: resolveChannelAuthorizationEnabled(activeRow) ? '需要授权' : '无需授权',
      },
      {
        key: 'commission_formula',
        field: '平台抽佣公式',
        value: activeRow.commission_formula || '-',
      },
      {
        key: 'status',
        field: '当前状态',
        value: activeRow.status === 'ACTIVE' ? '启用' : '停用',
      },
    ];
  }, [activeRow]);

  const channelTimelineRows = useMemo<ChannelDetailRow[]>(() => {
    if (!activeRow) {
      return [];
    }
    return [
      {
        key: 'created_at',
        field: '创建时间',
        value: String(activeRow.created_at ?? '-'),
      },
      {
        key: 'updated_at',
        field: '更新时间',
        value: String(activeRow.updated_at ?? activeRow.created_at ?? '-'),
      },
      {
        key: 'service_entry',
        field: '服务入口',
        value: '/channels/service-purchase',
      },
      {
        key: 'authorize_entry',
        field: '授权入口',
        value: resolveChannelAuthorizationEnabled(activeRow) ? '/channels/authorize' : '线下渠道无需授权',
      },
    ];
  }, [activeRow]);

  const renderBottomTable = useCallback(
    (rows: ChannelDetailRow[], emptyDescription: string, syncKey: string) =>
      activeRow ? (
        <UiModule04BottomTable syncKey={syncKey}>
          <Table<ChannelDetailRow>
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
        <Empty description="请选择渠道查看底部明细" />
      ),
    [activeRow, bottomDetailColumns],
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
            label: `基础资料 (${channelProfileRows.length})`,
            children: renderBottomTable(
              channelProfileRows,
              '暂无基础资料',
              `channel-profile-${activeRow?.channel_id ?? 'empty'}-${channelProfileRows.length}`,
            ),
          },
          {
            key: 'capability',
            label: `业务能力 (${channelCapabilityRows.length})`,
            children: renderBottomTable(
              channelCapabilityRows,
              '暂无业务能力信息',
              `channel-capability-${activeRow?.channel_id ?? 'empty'}-${channelCapabilityRows.length}`,
            ),
          },
          {
            key: 'timeline',
            label: `维护轨迹 (${channelTimelineRows.length})`,
            children: renderBottomTable(
              channelTimelineRows,
              '暂无维护轨迹',
              `channel-timeline-${activeRow?.channel_id ?? 'empty'}-${channelTimelineRows.length}`,
            ),
          },
        ]}
      />
    ),
    [
      activeBottomTab,
      activeRow?.channel_id,
      channelCapabilityRows,
      channelProfileRows,
      channelTimelineRows,
      renderBottomTable,
    ],
  );

  return (
    <WorkbenchPageTemplate
      pageTestId="channel-directory-workbench-page"
      className="order-review-page channel-directory-page"
      leftPanel={leftPanel}
      leftPanelVisible={true}
      toolbar={toolbar}
      tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
      tableContent={
        <div className="channel-directory-table-shell">
          <ErpDataGridNext<Channel>
            rows={pagedRows}
            columns={columns}
            rowKey={(channel) => channel.channel_id}
            gridPreset="industrial-review-compact"
            columnSettingsKey="channels.directory.main-grid"
            dataTestId="channel-directory-grid"
            enableRowModifierSelection
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            showColumnSettings
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setColumnFilters(extractChannelDirectoryGridFilterModel(model));
            }}
            selectedRowKeys={selectedRowKeys}
            activeRowKey={activeRowKey}
            onSelectionChange={(keys: Key[]) => {
              const normalized = Array.from(
                new Set(
                  keys
                    .map((key) => String(key ?? '').trim())
                    .filter(Boolean),
                ),
              );
              setSelectedRowKeys(normalized);
              setSelectionAnchorKey((currentAnchor) => {
                if (!normalized.length) {
                  return null;
                }
                if (currentAnchor && normalized.includes(currentAnchor)) {
                  return currentAnchor;
                }
                return normalized[0] ?? null;
              });
            }}
            onActiveRowChange={(channel) => {
              const rowKey = resolveChannelRowKey(channel);
              setActiveRowKey(rowKey);
              setFocusedRowKey(rowKey);
            }}
            onRowDoubleClick={(channel) => {
              if (!canWrite) {
                return;
              }
              openEditDrawer(channel);
            }}
            onRowContextMenu={(channel, event) => handleContextMenu(channel, event)}
            emptyText="暂无渠道数据"
            loading={channelsQuery.isFetching}
            error={channelsQuery.error}
            errorTitle="渠道目录加载失败"
            onRetry={() => {
              void channelsQuery.refetch();
            }}
          />
        </div>
      }
      footerSummary={
        <>
          <span>{`总数 ${displayRows.length} 条，本页 ${pagedRows.length} 条，已选 ${selectedRowKeys.length} 条`}</span>
          <span>{activeRow ? `当前焦点：${activeRow.name}` : '当前焦点：-'}</span>
        </>
      }
      footerSyncKey={`channel-directory-${displayRows.length}-${currentPage}-${currentPageSize}`}
      total={displayRows.length}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
      stateCacheKey={stateCacheKey}
      showFooterScrollbarRow={true}
      rightPanel={rightPanel}
      rightPanelVisible={true}
      rightPanelSyncKey={rightPanelSyncKey}
      bottomPanel={bottomPanel}
      bottomPanelVisible={true}
      overlays={
        <>
          <UiModule03ContextMenu
            open={contextMenuState.open}
            position={contextMenuState.position}
            menu={contextMenuConfig}
            onClose={closeContextMenu}
          />
          <Drawer
            title={editingChannel ? '修改渠道' : '新建渠道'}
            open={editorOpen}
            onClose={() => setEditorOpen(false)}
            width={560}
            destroyOnHidden
            footer={
              <Space>
                <Button onClick={() => setEditorOpen(false)}>取消</Button>
                <Button type="primary" loading={saveMutation.isPending} onClick={() => void handleSave()}>
                  保存
                </Button>
              </Space>
            }
          >
            <Form form={editorForm} layout="vertical">
              <Form.Item
                name="name"
                label="渠道名称"
                rules={[{ required: true, message: '请输入渠道名称' }]}
              >
                <Input placeholder="例如：淘宝旗舰渠道" />
              </Form.Item>
              <Form.Item
                name="channel_type"
                label="渠道类型"
                rules={[{ required: true, message: '请选择渠道类型' }]}
              >
                <Select
                  options={[
                    { label: '线上', value: 'ONLINE' },
                    { label: '线下', value: 'OFFLINE' },
                  ]}
                />
              </Form.Item>
              <Form.Item
                name="platform"
                label="平台"
                rules={[
                  ({ getFieldValue }) => ({
                    validator(_, value) {
                      if (getFieldValue('channel_type') === 'OFFLINE' || value) {
                        return Promise.resolve();
                      }
                      return Promise.reject(new Error('线上渠道必须选择平台'));
                    },
                  }),
                ]}
              >
                <Select
                  allowClear
                  disabled={editorChannelType === 'OFFLINE'}
                  options={platformOptions}
                  placeholder={editorChannelType === 'OFFLINE' ? '线下渠道无需平台' : '请选择平台'}
                />
              </Form.Item>
              <Form.Item name="contact_name" label="联系人">
                <Input placeholder="联系人姓名" />
              </Form.Item>
              <Form.Item name="contact_phone" label="电话">
                <Input placeholder="联系电话" />
              </Form.Item>
              <Form.Item name="contact_address" label="地址">
                <Input.TextArea rows={3} placeholder="联系地址" />
              </Form.Item>
              <Form.Item name="department" label="归属部门">
                <Input placeholder="例如：电商事业部" />
              </Form.Item>
              <Form.Item name="owner" label="负责人">
                <Input placeholder="负责人姓名" />
              </Form.Item>
              <Form.Item name="commission_formula" label="平台抽佣公式">
                <Input placeholder="例如：销售额 * 0.05" />
              </Form.Item>
              <Form.Item name="default_warehouse_id" label="默认发货仓库">
                <Select allowClear options={warehouseOptions} placeholder="请选择仓库" />
              </Form.Item>
              <Form.Item name="status" label="状态" initialValue="ACTIVE">
                <Select
                  options={[
                    { label: '启用', value: 'ACTIVE' },
                    { label: '停用', value: 'DISABLED' },
                  ]}
                />
              </Form.Item>
              <Form.Item name="allow_manual_order" label="允许手工建销售单" valuePropName="checked">
                <Switch />
              </Form.Item>
              <Form.Item
                name="allow_manual_after_sales"
                label="允许手工建售后单"
                valuePropName="checked"
              >
                <Switch />
              </Form.Item>
            </Form>
          </Drawer>
        </>
      }
    />
  );
};
