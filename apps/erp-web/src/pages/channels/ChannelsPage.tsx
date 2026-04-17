import './channelsPage.css';

import {
  Button,
  Card,
  Col,
  DatePicker,
  Drawer,
  Form,
  Input,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message,
} from 'antd';
import type { MenuProps, TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent, type Key } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocation, useNavigate } from 'react-router-dom';

import type { Channel, ChannelCreatePayload } from '@/api/channels';
import { createChannel, deleteChannel, listChannels, updateChannel } from '@/api/channels';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule03ContextMenu, UiModule03MainTableSection } from '@/components/ui-modules';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { routerBasePath, routerMode } from '@/utils/env';

import {
  ChannelsWorkbenchLeftPanel,
  ChannelsWorkbenchOverview,
  ChannelsWorkbenchRightPanel,
} from './ChannelWorkbenchPanels';
import {
  buildChannelActiveFilterSummary,
  buildChannelAuthCallbackUrl,
  buildChannelListQuery,
  buildChannelQuickFilters,
  filterChannels,
  parseChannelAuthCallbackSearch,
  resolveChannelQuickFilterState,
  summarizeChannels,
  type ChannelQuickFilterId,
  type ChannelStatusFilter,
  type ChannelWorkbenchColumnFilters,
} from './channelWorkbenchModel';

type FormValues = {
  platform: PlatformCode;
  name: string;
  department?: string;
  owner?: string;
  allow_manual_order: boolean;
  allow_manual_after_sales: boolean;
  default_warehouse_id?: string;
  commission_formula?: string;
};

type PlatformPickerMode = 'create' | 'copy';

type ChannelColumnFilters = Omit<ChannelWorkbenchColumnFilters, 'updatedRange'> & {
  updatedRange?: [Dayjs, Dayjs] | null;
};

const statusMeta = {
  ACTIVE: { label: '启用中', color: 'success' },
  DISABLED: { label: '已停用', color: 'default' },
} as const;

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const createEmptyColumnFilters = (): ChannelColumnFilters => ({
  updatedRange: null,
});

const isPlatformCode = (value: string | null): value is PlatformCode =>
  Boolean(value && platformOptions.some((option) => option.value === value));

export const ChannelsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('channels.write');
  const queryClient = useQueryClient();
  const location = useLocation();
  const navigate = useNavigate();
  const {
    setLeftPanel,
    setRightPanel,
    setBottomPanel,
    setLeftPanelVisible,
    setRightPanelVisible,
    setBottomPanelVisible,
    clearPanels,
  } = useWorkbenchPanels();
  const [drawerForm] = Form.useForm<FormValues>();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [platformPickerOpen, setPlatformPickerOpen] = useState(false);
  const [platformPickerMode, setPlatformPickerMode] = useState<PlatformPickerMode>('create');
  const [selectedPlatform, setSelectedPlatform] = useState<PlatformCode | null>(null);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [copySource, setCopySource] = useState<Channel | null>(null);
  const [saving, setSaving] = useState(false);
  const [statusFilter, setStatusFilter] = useState<ChannelStatusFilter>('ACTIVE');
  const [activeQuickFilterId, setActiveQuickFilterId] = useState<ChannelQuickFilterId | null>('ACTIVE');
  const [searchValue, setSearchValue] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [columnFilters, setColumnFilters] = useState<ChannelColumnFilters>(() =>
    createEmptyColumnFilters(),
  );
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; channel: Channel } | null>(
    null,
  );
  const lastSelectedIndexRef = useRef<number | null>(null);
  const createFromQueryHandledRef = useRef(false);

  const resetSelection = useCallback(() => {
    setSelectedRowKeys([]);
    setSelectedChannel(null);
    setContextMenu(null);
    lastSelectedIndexRef.current = null;
  }, []);

  const resetToFirstPage = useCallback(() => {
    setPagination((prev) => ({ ...prev, current: 1 }));
  }, []);

  const normalizedColumnFilters = useMemo<ChannelWorkbenchColumnFilters>(
    () => ({
      ...columnFilters,
      updatedRange:
        columnFilters.updatedRange && columnFilters.updatedRange.length === 2
          ? {
              start: columnFilters.updatedRange[0]?.format('YYYY-MM-DD'),
              end: columnFilters.updatedRange[1]?.format('YYYY-MM-DD'),
            }
          : null,
    }),
    [columnFilters],
  );

  const query = useMemo(
    () => buildChannelListQuery(searchKeyword, statusFilter),
    [searchKeyword, statusFilter],
  );

  const channelsQuery = useQuery({
    queryKey: ['channels', tenantId, query],
    queryFn: () => listChannels(tenantId, query),
    enabled: Boolean(tenantId),
    keepPreviousData: true,
  });

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId],
    queryFn: () =>
      listWarehouses(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200,
      }),
    enabled: Boolean(tenantId),
    keepPreviousData: true,
  });

  const warehouseMap = useMemo(() => {
    const map = new Map<string, string>();
    warehousesQuery.data?.items.forEach((warehouse) => {
      map.set(warehouse.warehouse_id, warehouse.name);
    });
    return map;
  }, [warehousesQuery.data?.items]);

  const warehouseOptions = useMemo(
    () =>
      warehousesQuery.data?.items.map((warehouse) => ({
        value: warehouse.warehouse_id,
        label: `${warehouse.name} (${warehouse.code})`,
      })) ?? [],
    [warehousesQuery.data?.items],
  );

  const listItems = channelsQuery.data?.items ?? [];
  const filteredItems = useMemo(
    () =>
      filterChannels(listItems, {
        searchKeyword,
        statusFilter,
        columnFilters: normalizedColumnFilters,
      }),
    [listItems, normalizedColumnFilters, searchKeyword, statusFilter],
  );

  const pagedItems = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    const start = (current - 1) * pageSize;
    return filteredItems.slice(start, start + pageSize);
  }, [filteredItems, pagination.current, pagination.pageSize]);

  const quickFilters = useMemo(() => buildChannelQuickFilters(listItems), [listItems]);
  const workbenchSummary = useMemo(() => summarizeChannels(listItems), [listItems]);
  const callbackState = useMemo(
    () => parseChannelAuthCallbackSearch(location.search),
    [location.search],
  );

  const selectedChannelCallbackUrl = useMemo(() => {
    if (!selectedChannel || typeof window === 'undefined') {
      return null;
    }
    return buildChannelAuthCallbackUrl({
      origin: window.location.origin,
      routerMode,
      routerBasePath,
      channelId: selectedChannel.channel_id,
      platform: selectedChannel.platform,
    });
  }, [selectedChannel]);

  const activeFilterSummary = useMemo(
    () =>
      buildChannelActiveFilterSummary({
        activeQuickFilterId,
        quickFilters,
        searchKeyword,
        statusFilter,
        columnFilters: normalizedColumnFilters,
        warehouseNameResolver: (warehouseId) => warehouseMap.get(warehouseId) ?? warehouseId,
      }),
    [activeQuickFilterId, normalizedColumnFilters, quickFilters, searchKeyword, statusFilter, warehouseMap],
  );

  const syncSelection = useCallback(
    (keys: Key[], rows?: Channel[], active?: Channel | null) => {
      const normalizedKeys = Array.from(new Set(keys.map((key) => String(key))));
      setSelectedRowKeys(normalizedKeys);
      if (!normalizedKeys.length) {
        setSelectedChannel(null);
        lastSelectedIndexRef.current = null;
        return;
      }
      if (active) {
        setSelectedChannel(active);
        return;
      }
      if (rows?.length) {
        setSelectedChannel(rows[0] ?? null);
        return;
      }
      setSelectedChannel((previous) => {
        if (previous && normalizedKeys.includes(previous.channel_id)) {
          return previous;
        }
        return pagedItems.find((item) => normalizedKeys.includes(item.channel_id)) ?? null;
      });
    },
    [pagedItems],
  );

  useEffect(() => {
    if (!selectedRowKeys.length) {
      return;
    }
    setSelectedRowKeys((previous) =>
      previous.filter((key) => filteredItems.some((item) => item.channel_id === key)),
    );
  }, [filteredItems, selectedRowKeys.length]);

  useEffect(() => {
    if (!selectedChannel) {
      return;
    }
    if (!filteredItems.some((item) => item.channel_id === selectedChannel.channel_id)) {
      setSelectedChannel(null);
    }
  }, [filteredItems, selectedChannel]);

  useEffect(() => {
    if (!callbackState?.channelId) {
      return;
    }
    setActiveQuickFilterId('all');
    setStatusFilter('all');
    setSearchValue('');
    setSearchKeyword('');
    setColumnFilters(createEmptyColumnFilters());
    resetToFirstPage();
  }, [callbackState?.channelId, resetToFirstPage]);

  useEffect(() => {
    if (!callbackState?.channelId) {
      return;
    }
    const callbackChannel = listItems.find((item) => item.channel_id === callbackState.channelId);
    if (callbackChannel) {
      syncSelection([callbackChannel.channel_id], [callbackChannel], callbackChannel);
    }
  }, [callbackState?.channelId, listItems, syncSelection]);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    if (params.get('open_create') !== '1') {
      createFromQueryHandledRef.current = false;
      return;
    }
    if (createFromQueryHandledRef.current || !canWrite) {
      return;
    }

    createFromQueryHandledRef.current = true;
    const platform = params.get('platform');
    if (isPlatformCode(platform)) {
      drawerForm.resetFields();
      drawerForm.setFieldsValue({
        platform,
        name: '',
        department: undefined,
        owner: undefined,
        allow_manual_order: false,
        allow_manual_after_sales: false,
        default_warehouse_id: undefined,
        commission_formula: undefined,
      });
      setSelectedPlatform(platform);
      setPlatformPickerOpen(false);
      setDrawerOpen(true);
    } else {
      setPlatformPickerMode('create');
      setSelectedPlatform(null);
      setPlatformPickerOpen(true);
    }

    params.delete('open_create');
    params.delete('platform');
    const nextSearch = params.toString();
    navigate(
      {
        pathname: location.pathname,
        search: nextSearch ? `?${nextSearch}` : '',
      },
      { replace: true },
    );
  }, [canWrite, drawerForm, location.pathname, location.search, navigate]);

  const handleSearch = useCallback(
    (value: string) => {
      setSearchValue(value);
      setSearchKeyword(value.trim());
      resetSelection();
      resetToFirstPage();
    },
    [resetSelection, resetToFirstPage],
  );

  const handleSearchChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const value = event.target.value;
      setSearchValue(value);
      if (!value.trim()) {
        setSearchKeyword('');
        resetSelection();
        resetToFirstPage();
      }
    },
    [resetSelection, resetToFirstPage],
  );

  const updateColumnFilters = useCallback(
    (patch: Partial<ChannelColumnFilters>) => {
      setColumnFilters((previous) => ({ ...previous, ...patch }));
      setActiveQuickFilterId(null);
      resetSelection();
      resetToFirstPage();
    },
    [resetSelection, resetToFirstPage],
  );

  const handleQuickFilterSelect = useCallback(
    (filterId: ChannelQuickFilterId) => {
      const next = resolveChannelQuickFilterState(filterId);
      setActiveQuickFilterId(filterId);
      setStatusFilter(next.statusFilter);
      setColumnFilters((previous) => ({ ...previous, ...next.columnFilters }));
      resetSelection();
      resetToFirstPage();
    },
    [resetSelection, resetToFirstPage],
  );

  const handleCopyCallbackUrl = useCallback(async () => {
    if (!selectedChannelCallbackUrl) {
      message.warning('请先选择渠道');
      return;
    }
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(selectedChannelCallbackUrl);
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = selectedChannelCallbackUrl;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
      }
      message.success('回调地址已复制');
    } catch {
      message.error('复制回调地址失败');
    }
  }, [selectedChannelCallbackUrl]);

  const handleOpenShops = useCallback(() => {
    const params = new URLSearchParams();
    if (selectedChannel) {
      params.set('platform', selectedChannel.platform);
    }
    navigate(`/channels/shops${params.toString() ? `?${params.toString()}` : ''}`);
  }, [navigate, selectedChannel]);

  const handleOpenSyncJobs = useCallback(() => {
    navigate('/channels/sync-jobs');
  }, [navigate]);

  const workbenchLeftPanel = useMemo(
    () => (
      <ChannelsWorkbenchLeftPanel
        quickFilters={quickFilters}
        activeQuickFilterId={activeQuickFilterId}
        activeFilterSummary={activeFilterSummary}
        selectedChannel={selectedChannel}
        onSelectQuickFilter={handleQuickFilterSelect}
      />
    ),
    [
      activeFilterSummary,
      activeQuickFilterId,
      handleQuickFilterSelect,
      quickFilters,
      selectedChannel,
    ],
  );

  const workbenchRightPanel = useMemo(
    () => (
      <ChannelsWorkbenchRightPanel
        selectedChannel={selectedChannel}
        warehouseMap={warehouseMap}
        callbackUrl={selectedChannelCallbackUrl}
        callbackState={callbackState}
        onCopyCallbackUrl={handleCopyCallbackUrl}
        onOpenShops={handleOpenShops}
        onOpenSyncJobs={handleOpenSyncJobs}
      />
    ),
    [
      callbackState,
      handleCopyCallbackUrl,
      handleOpenShops,
      handleOpenSyncJobs,
      selectedChannel,
      selectedChannelCallbackUrl,
      warehouseMap,
    ],
  );

  useEffect(() => {
    setLeftPanel(workbenchLeftPanel);
    setRightPanel(workbenchRightPanel);
    setBottomPanel(undefined);
    setLeftPanelVisible(true);
    setRightPanelVisible(true);
    setBottomPanelVisible(false);
    return () => clearPanels();
  }, [
    clearPanels,
    setBottomPanel,
    setBottomPanelVisible,
    setLeftPanel,
    setLeftPanelVisible,
    setRightPanel,
    setRightPanelVisible,
    workbenchLeftPanel,
    workbenchRightPanel,
  ]);

  const deleteMutation = useMutation({
    mutationFn: (channelId: string) => deleteChannel(tenantId, channelId),
    onSuccess: () => {
      message.success('渠道已删除');
      resetSelection();
      void queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '删除渠道失败');
    },
  });

  const statusMutation = useMutation({
    mutationFn: ({ channelId, status }: { channelId: string; status: 'ACTIVE' | 'DISABLED' }) =>
      updateChannel(tenantId, channelId, { status }),
    onSuccess: (_, variables) => {
      message.success(variables.status === 'DISABLED' ? '渠道已停用' : '渠道已启用');
      void queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '更新渠道状态失败');
    },
  });

  const handleOpenPlatformPicker = useCallback((mode: PlatformPickerMode) => {
    setPlatformPickerMode(mode);
    setSelectedPlatform(null);
    setPlatformPickerOpen(true);
  }, []);

  const openCreate = useCallback(() => {
    setEditingChannel(null);
    setCopySource(null);
    handleOpenPlatformPicker('create');
  }, [handleOpenPlatformPicker]);

  const openCopy = useCallback(() => {
    if (!selectedChannel) {
      message.warning('请先选择渠道');
      return;
    }
    setEditingChannel(null);
    setCopySource(selectedChannel);
    handleOpenPlatformPicker('copy');
  }, [handleOpenPlatformPicker, selectedChannel]);

  const openEditWithChannel = useCallback(
    (channel: Channel) => {
      setCopySource(null);
      setEditingChannel(channel);
      drawerForm.setFieldsValue({
        platform: channel.platform,
        name: channel.name,
        department: channel.department ?? undefined,
        owner: channel.owner ?? undefined,
        allow_manual_order: channel.allow_manual_order,
        allow_manual_after_sales: channel.allow_manual_after_sales,
        default_warehouse_id: channel.default_warehouse_id ?? undefined,
        commission_formula: channel.commission_formula ?? undefined,
      });
      setDrawerOpen(true);
    },
    [drawerForm],
  );

  const openEdit = useCallback(() => {
    if (!selectedChannel) {
      message.warning('请先选择渠道');
      return;
    }
    openEditWithChannel(selectedChannel);
  }, [openEditWithChannel, selectedChannel]);

  const handlePlatformConfirm = useCallback(() => {
    if (!selectedPlatform) {
      message.warning('请选择平台');
      return;
    }
    const base = platformPickerMode === 'copy' ? copySource : null;
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      platform: selectedPlatform,
      name: base ? `${base.name}-复制` : '',
      department: base?.department ?? undefined,
      owner: base?.owner ?? undefined,
      allow_manual_order: base?.allow_manual_order ?? false,
      allow_manual_after_sales: base?.allow_manual_after_sales ?? false,
      default_warehouse_id: base?.default_warehouse_id ?? undefined,
      commission_formula: base?.commission_formula ?? undefined,
    });
    setPlatformPickerOpen(false);
    setDrawerOpen(true);
  }, [copySource, drawerForm, platformPickerMode, selectedPlatform]);

  const handleSave = useCallback(async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: ChannelCreatePayload = {
        platform: values.platform,
        name: values.name.trim(),
        department: values.department?.trim() || undefined,
        owner: values.owner?.trim() || undefined,
        allow_manual_order: values.allow_manual_order,
        allow_manual_after_sales: values.allow_manual_after_sales,
        default_warehouse_id: values.default_warehouse_id,
        commission_formula: values.commission_formula?.trim() || undefined,
      };
      setSaving(true);
      if (editingChannel) {
        await updateChannel(tenantId, editingChannel.channel_id, payload);
        message.success('渠道已更新');
      } else {
        await createChannel(tenantId, payload);
        message.success('渠道已创建');
      }
      setDrawerOpen(false);
      setEditingChannel(null);
      setCopySource(null);
      await queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  }, [drawerForm, editingChannel, queryClient, tenantId]);

  const headerContent = useMemo(
    () => <ChannelsWorkbenchOverview summary={workbenchSummary} callbackState={callbackState} />,
    [callbackState, workbenchSummary],
  );

  const columns = useMemo<ColumnsType<Channel>>(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    return [
      {
        title: '序号',
        key: 'index',
        width: 72,
        align: 'center',
        render: (_value, _record, index) => (current - 1) * pageSize + index + 1,
      },
      { title: '渠道名称', dataIndex: 'name', key: 'name' },
      {
        title: '平台',
        dataIndex: 'platform',
        key: 'platform',
        render: (_value, record) => <PlatformTag platform={record.platform} />,
      },
      { title: '负责部门', dataIndex: 'department', key: 'department', render: (value) => value ?? '-' },
      { title: '负责人', dataIndex: 'owner', key: 'owner', render: (value) => value ?? '-' },
      {
        title: '默认仓',
        dataIndex: 'default_warehouse_id',
        key: 'default_warehouse_id',
        render: (value: string | null | undefined) => (value ? warehouseMap.get(value) ?? value : '-'),
      },
      {
        title: '手工建单',
        dataIndex: 'allow_manual_order',
        key: 'allow_manual_order',
        render: (value: boolean) => (value ? <Tag color="green">允许</Tag> : <Tag>不允许</Tag>),
      },
      {
        title: '手工售后',
        dataIndex: 'allow_manual_after_sales',
        key: 'allow_manual_after_sales',
        render: (value: boolean) => (value ? <Tag color="green">允许</Tag> : <Tag>不允许</Tag>),
      },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        render: (value: 'ACTIVE' | 'DISABLED') => (
          <Tag color={statusMeta[value].color}>{statusMeta[value].label}</Tag>
        ),
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        key: 'updated_at',
        render: (_value, record) => formatDateTime(record.updated_at ?? record.created_at),
      },
    ];
  }, [pagination.current, pagination.pageSize, warehouseMap]);

  const queryCells = useMemo(
    () => ({
      name: (
        <Input
          size="small"
          allowClear
          placeholder="渠道名称"
          value={searchValue}
          onChange={handleSearchChange}
          onPressEnter={(event) => handleSearch((event.target as HTMLInputElement).value ?? '')}
        />
      ),
      platform: (
        <Select
          size="small"
          allowClear
          placeholder="平台"
          options={platformOptions}
          value={columnFilters.platform}
          onChange={(value) => updateColumnFilters({ platform: value ?? undefined })}
        />
      ),
      department: (
        <Input
          size="small"
          allowClear
          placeholder="负责部门"
          value={columnFilters.department}
          onChange={(event) => updateColumnFilters({ department: event.target.value || undefined })}
        />
      ),
      owner: (
        <Input
          size="small"
          allowClear
          placeholder="负责人"
          value={columnFilters.owner}
          onChange={(event) => updateColumnFilters({ owner: event.target.value || undefined })}
        />
      ),
      default_warehouse_id: (
        <Select
          size="small"
          allowClear
          placeholder="默认仓"
          options={warehouseOptions}
          loading={warehousesQuery.isFetching}
          value={columnFilters.warehouseId}
          onChange={(value) => updateColumnFilters({ warehouseId: value ?? undefined })}
        />
      ),
      allow_manual_order: (
        <Select
          size="small"
          allowClear
          placeholder="手工建单"
          options={[
            { label: '允许', value: 'allow' },
            { label: '不允许', value: 'deny' },
          ]}
          value={columnFilters.allowManualOrder}
          onChange={(value) =>
            updateColumnFilters({
              allowManualOrder:
                (value as ChannelWorkbenchColumnFilters['allowManualOrder']) ?? undefined,
            })
          }
        />
      ),
      allow_manual_after_sales: (
        <Select
          size="small"
          allowClear
          placeholder="手工售后"
          options={[
            { label: '允许', value: 'allow' },
            { label: '不允许', value: 'deny' },
          ]}
          value={columnFilters.allowManualAfterSales}
          onChange={(value) =>
            updateColumnFilters({
              allowManualAfterSales:
                (value as ChannelWorkbenchColumnFilters['allowManualAfterSales']) ?? undefined,
            })
          }
        />
      ),
      status: (
        <Select
          size="small"
          allowClear
          placeholder="状态"
          options={[
            { label: '启用中', value: 'ACTIVE' },
            { label: '已停用', value: 'DISABLED' },
          ]}
          value={statusFilter === 'all' ? undefined : statusFilter}
          onChange={(value) => {
            setActiveQuickFilterId(null);
            setStatusFilter((value as ChannelStatusFilter) ?? 'all');
            resetSelection();
            resetToFirstPage();
          }}
        />
      ),
      updated_at: (
        <DatePicker.RangePicker
          size="small"
          allowClear
          value={columnFilters.updatedRange ?? undefined}
          onChange={(range) => updateColumnFilters({ updatedRange: (range as [Dayjs, Dayjs] | null) ?? null })}
        />
      ),
    }),
    [
      columnFilters.allowManualAfterSales,
      columnFilters.allowManualOrder,
      columnFilters.department,
      columnFilters.owner,
      columnFilters.platform,
      columnFilters.updatedRange,
      columnFilters.warehouseId,
      handleSearch,
      handleSearchChange,
      resetSelection,
      resetToFirstPage,
      searchValue,
      statusFilter,
      updateColumnFilters,
      warehouseOptions,
      warehousesQuery.isFetching,
    ],
  );

  const contextMenuItems: MenuProps['items'] = [
    {
      key: 'toggle',
      label: contextMenu?.channel.status === 'DISABLED' ? '启用渠道' : '停用渠道',
      disabled: !canWrite,
    },
  ];

  const toolbarLeft = (
    <Space size={8}>
      <Button type="primary" onClick={openCreate} disabled={!canWrite} data-testid="channels-create">
        新增
      </Button>
      <Button onClick={openCopy} disabled={!canWrite || !selectedChannel} data-testid="channels-copy">
        复制
      </Button>
      <Button onClick={openEdit} disabled={!canWrite || !selectedChannel} data-testid="channels-edit">
        修改
      </Button>
      <Popconfirm
        title="确认删除当前渠道？"
        okText="删除"
        cancelText="取消"
        onConfirm={() => selectedChannel && deleteMutation.mutate(selectedChannel.channel_id)}
        disabled={!selectedChannel || !canWrite}
      >
        <Button danger disabled={!selectedChannel || !canWrite} data-testid="channels-delete">
          删除
        </Button>
      </Popconfirm>
      <Button icon={<ReloadOutlined />} onClick={() => void channelsQuery.refetch()}>
        刷新
      </Button>
    </Space>
  );

  const toolbarCenter = (
    <Input.Search
      allowClear
      value={searchValue}
      placeholder="搜索渠道名称 / 负责人 / 部门"
      onSearch={handleSearch}
      onChange={handleSearchChange}
      style={{ width: 280 }}
      data-testid="channels-search"
    />
  );

  const toolbarRight = (
    <Space size={8} wrap>
      <Button onClick={handleOpenShops} data-testid="channels-open-shops">
        店铺接入
      </Button>
      <Button onClick={handleOpenSyncJobs}>同步任务</Button>
      <Button onClick={() => message.info('帮助文档入口待接入')}>帮助</Button>
    </Space>
  );

  const footerSummary = (
    <span>
      总计 {filteredItems.length} 条，本页 {pagedItems.length} 条，已选 {selectedRowKeys.length} 条
    </span>
  );

  return (
    <div className="channels-page" data-testid="channels-page">
      <UiModule03MainTableSection<Channel>
        className="channels-workbench"
        dataTestId="channels-workbench-grid"
        tableClassName="channels-table"
        toolbarLeft={toolbarLeft}
        toolbarCenter={toolbarCenter}
        toolbarRight={toolbarRight}
        header={headerContent}
        columns={columns}
        data={pagedItems}
        loading={channelsQuery.isLoading || channelsQuery.isFetching}
        error={channelsQuery.error}
        onRetry={() => void channelsQuery.refetch()}
        errorTitle="渠道列表加载失败"
        emptyText="暂无渠道"
        total={filteredItems.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        rowKey="channel_id"
        columnSettingsKey="channels.table.columns"
        queryCells={queryCells}
        footerSummary={footerSummary}
        leftPanelWidth={320}
        contextMenuSelectsRow={false}
        rowSelection={{ selectedRowKeys, onChange: (keys, rows) => syncSelection(keys, rows as Channel[]) }}
        rowClassName={(record) =>
          record.channel_id === selectedChannel?.channel_id ? 'ant-table-row-selected' : ''
        }
        onRow={(record, index) => ({
          onClick: (event) => {
            const mouseEvent = event as MouseEvent;
            const currentIndex = index ?? pagedItems.findIndex((item) => item.channel_id === record.channel_id);
            if (currentIndex < 0) {
              syncSelection([record.channel_id], [record], record);
              return;
            }
            if (mouseEvent.shiftKey && lastSelectedIndexRef.current != null) {
              const start = Math.min(lastSelectedIndexRef.current, currentIndex);
              const end = Math.max(lastSelectedIndexRef.current, currentIndex);
              syncSelection(pagedItems.slice(start, end + 1).map((item) => item.channel_id), undefined, record);
            } else if (mouseEvent.ctrlKey || mouseEvent.metaKey) {
              const keySet = new Set(selectedRowKeys);
              if (keySet.has(record.channel_id)) {
                keySet.delete(record.channel_id);
              } else {
                keySet.add(record.channel_id);
              }
              syncSelection(Array.from(keySet), undefined, record);
            } else {
              syncSelection([record.channel_id], [record], record);
            }
            lastSelectedIndexRef.current = currentIndex;
          },
          onDoubleClick: () => {
            if (!canWrite) {
              message.warning('当前没有编辑权限');
              return;
            }
            setSelectedChannel(record);
            openEditWithChannel(record);
          },
          onContextMenu: (event) => {
            event.preventDefault();
            const currentIndex = index ?? pagedItems.findIndex((item) => item.channel_id === record.channel_id);
            if (currentIndex >= 0) {
              lastSelectedIndexRef.current = currentIndex;
            }
            syncSelection([record.channel_id], [record], record);
            setContextMenu({ x: event.clientX, y: event.clientY, channel: record });
          },
        })}
      />

      <UiModule03ContextMenu
        open={Boolean(contextMenu)}
        position={contextMenu}
        menu={{
          items: contextMenuItems,
          onClick: () => {
            if (!contextMenu || !canWrite) {
              return;
            }
            const nextStatus = contextMenu.channel.status === 'DISABLED' ? 'ACTIVE' : 'DISABLED';
            statusMutation.mutate({ channelId: contextMenu.channel.channel_id, status: nextStatus });
            setContextMenu(null);
          },
        }}
        onClose={() => setContextMenu(null)}
      />

      <Modal title="选择平台" open={platformPickerOpen} onCancel={() => setPlatformPickerOpen(false)} onOk={handlePlatformConfirm} okText="确定" cancelText="取消">
        <Row gutter={[12, 12]}>
          {platformOptions.map((platform) => (
            <Col span={12} key={platform.value}>
              <Card
                hoverable
                onClick={() => setSelectedPlatform(platform.value as PlatformCode)}
                style={{ borderColor: selectedPlatform === platform.value ? 'var(--ant-color-primary)' : undefined }}
              >
                <Space direction="vertical" size={4}>
                  <PlatformTag platform={platform.value as PlatformCode} />
                  <Typography.Text>{platform.label}</Typography.Text>
                </Space>
              </Card>
            </Col>
          ))}
        </Row>
      </Modal>

      <Drawer
        title={editingChannel ? '修改渠道' : copySource ? '复制渠道' : '新增渠道'}
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
        <Form form={drawerForm} layout="vertical" initialValues={{ allow_manual_order: false, allow_manual_after_sales: false }}>
          <Form.Item name="platform" label="平台" rules={[{ required: true, message: '请选择平台' }]}>
            <Select options={platformOptions} disabled />
          </Form.Item>
          <Form.Item name="name" label="渠道名称" rules={[{ required: true, message: '请输入渠道名称' }]}>
            <Input placeholder="例如：淘宝旗舰店渠道" />
          </Form.Item>
          <Form.Item name="department" label="负责部门">
            <Input placeholder="例如：电商事业部" />
          </Form.Item>
          <Form.Item name="owner" label="负责人">
            <Input placeholder="例如：张三" />
          </Form.Item>
          <Form.Item name="allow_manual_order" label="允许手工建单" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="allow_manual_after_sales" label="允许手工新建售后单" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="default_warehouse_id" label="默认发货仓">
            <Select allowClear placeholder="选择默认发货仓" options={warehouseOptions} loading={warehousesQuery.isLoading} />
          </Form.Item>
          <Form.Item name="commission_formula" label="平台佣金公式">
            <Input.TextArea rows={3} placeholder="例如：sales_amount * 0.05" />
          </Form.Item>
        </Form>
      </Drawer>
    </div>
  );
};
