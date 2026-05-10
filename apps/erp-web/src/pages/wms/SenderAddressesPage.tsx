import { Button, Card, Descriptions, Drawer, Form, Input, Modal, Space, Switch, Tabs, Typography, message } from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type {
  SenderAddress,
  SenderAddressCreatePayload,
  SenderAddressListQuery,
  SenderAddressUpdatePayload,
} from '@/api/senderAddresses';
import {
  createSenderAddress,
  deleteSenderAddress,
  listSenderAddresses,
  updateSenderAddress,
} from '@/api/senderAddresses';
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

type SenderAddressFormValues = {
  name: string;
  phone: string;
  province: string;
  city: string;
  district: string;
  address: string;
  is_default?: boolean;
};

type SenderAddressesGridFilters = {
  name: string;
  phone: string;
  address: string;
};

type SenderAddressLeftFilterKey = 'view:all' | 'default:yes' | 'default:no' | 'region:华东' | 'region:华南';

const pageSizeOptions = ['20', '50', '100'];

const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

const createDefaultGridFilters = (): SenderAddressesGridFilters => ({
  name: '',
  phone: '',
  address: '',
});

const buildGridFilterModel = (filters: SenderAddressesGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  if (trimText(filters.phone)) {
    model.phone = { filterType: 'text', type: 'contains', filter: trimText(filters.phone) };
  }
  if (trimText(filters.address)) {
    model.address = { filterType: 'text', type: 'contains', filter: trimText(filters.address) };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): SenderAddressesGridFilters => ({
  name:
    model?.name && typeof model.name === 'object' && 'filter' in model.name
      ? trimText(String(model.name.filter ?? ''))
      : '',
  phone:
    model?.phone && typeof model.phone === 'object' && 'filter' in model.phone
      ? trimText(String(model.phone.filter ?? ''))
      : '',
  address:
    model?.address && typeof model.address === 'object' && 'filter' in model.address
      ? trimText(String(model.address.filter ?? ''))
      : '',
});

const resolveRegionTag = (row: SenderAddress) => {
  if (row.province.includes('上海') || row.province.includes('江苏') || row.province.includes('浙江')) {
    return '华东';
  }
  if (row.province.includes('广东')) {
    return '华南';
  }
  return '其他';
};

const formatFullAddress = (row: SenderAddress) =>
  `${row.province}${row.city}${row.district}${row.address}`;

export const SenderAddressesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<SenderAddressFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<SenderAddressLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<SenderAddressesGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingSender, setEditingSender] = useState<SenderAddress | null>(null);
  const [activeSenderId, setActiveSenderId] = useState<string | null>(null);
  const [selectedSenderIds, setSelectedSenderIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantReady ? `wms.sender-addresses.pagination.${tenantId}` : undefined,
  });

  const queryPayload = useMemo<SenderAddressListQuery>(
    () => ({
      keyword: trimText(searchKeyword) || undefined,
      offset: 0,
      limit: 500,
    }),
    [searchKeyword],
  );

  const query = useQuery({
    queryKey: ['sender-addresses', tenantId, queryPayload],
    queryFn: () => listSenderAddresses(tenantId, queryPayload),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData,
  });

  const saveMutation = useMutation({
    mutationFn: async (values: SenderAddressFormValues) => {
      const payload: SenderAddressCreatePayload = {
        name: values.name.trim(),
        phone: values.phone.trim(),
        province: values.province.trim(),
        city: values.city.trim(),
        district: values.district.trim(),
        address: values.address.trim(),
        is_default: values.is_default ?? false,
      };
      if (editingSender) {
        const updatePayload: SenderAddressUpdatePayload = payload;
        return updateSenderAddress(tenantId, editingSender.sender_address_id, updatePayload);
      }
      return createSenderAddress(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingSender ? '寄方地址已更新' : '寄方地址已创建');
      setDrawerOpen(false);
      setEditingSender(null);
      await queryClient.invalidateQueries({ queryKey: ['sender-addresses', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存寄方地址失败');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (senderIds: string[]) => {
      await Promise.all(senderIds.map((senderId) => deleteSenderAddress(tenantId, senderId)));
    },
    onSuccess: async (_data, senderIds) => {
      message.success(`已删除 ${senderIds.length} 个寄方地址`);
      setSelectedSenderIds([]);
      await queryClient.invalidateQueries({ queryKey: ['sender-addresses', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '删除寄方地址失败');
    },
  });

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);
  const sourceRows = useMemo(() => query.data?.items ?? [], [query.data]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部寄方', value: sourceRows.length, color: '#2f6fed' },
      {
        key: 'default',
        label: '默认寄方',
        value: sourceRows.filter((row) => row.is_default).length,
        color: '#0f9d58',
      },
      {
        key: 'east',
        label: '华东',
        value: sourceRows.filter((row) => resolveRegionTag(row) === '华东').length,
        color: '#d48806',
      },
    ],
    [sourceRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部寄方',
        count: sourceRows.length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'default:yes',
        label: '默认寄方',
        count: sourceRows.filter((row) => row.is_default).length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'default:yes',
      },
      {
        key: 'default:no',
        label: '普通寄方',
        count: sourceRows.filter((row) => !row.is_default).length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'default:no',
      },
      {
        key: 'region:华东',
        label: '华东寄方',
        count: sourceRows.filter((row) => resolveRegionTag(row) === '华东').length,
        nodeType: 'shop',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'region:华东',
      },
      {
        key: 'region:华南',
        label: '华南寄方',
        count: sourceRows.filter((row) => resolveRegionTag(row) === '华南').length,
        nodeType: 'shop',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'region:华南',
      },
    ],
    [leftFilterKey, sourceRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return sourceRows.filter((row) => {
      if (leftFilterKey === 'default:yes' && !row.is_default) return false;
      if (leftFilterKey === 'default:no' && row.is_default) return false;
      if (leftFilterKey === 'region:华东' && resolveRegionTag(row) !== '华东') return false;
      if (leftFilterKey === 'region:华南' && resolveRegionTag(row) !== '华南') return false;
      if (!keyword) {
        return true;
      }
      return [row.name, row.phone, formatFullAddress(row)]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword));
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
        if (
          trimText(gridFilters.phone) &&
          !row.phone.toLowerCase().includes(trimText(gridFilters.phone).toLowerCase())
        ) {
          return false;
        }
        if (
          trimText(gridFilters.address) &&
          !formatFullAddress(row).toLowerCase().includes(trimText(gridFilters.address).toLowerCase())
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
    setActiveSenderId((previous) =>
      previous && pageRows.some((row) => row.sender_address_id === previous)
        ? previous
        : pageRows[0]?.sender_address_id ?? null,
    );
    setSelectedSenderIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.sender_address_id === item)),
    );
  }, [pageRows]);

  const activeSender = useMemo(
    () => pageRows.find((row) => row.sender_address_id === activeSenderId) ?? null,
    [activeSenderId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<SenderAddress>[]>(
    () => [
      {
        key: 'name',
        field: 'name',
        title: '寄方名称',
        width: 150,
        minWidth: 130,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'phone',
        field: 'phone',
        title: '联系电话',
        width: 150,
        minWidth: 130,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'address',
        field: 'address',
        title: '详细地址',
        width: 360,
        minWidth: 260,
        renderCell: (_value, record) => formatFullAddress(record),
      },
      {
        key: 'is_default',
        field: 'is_default',
        title: '默认寄方',
        width: 110,
        minWidth: 100,
        renderCell: (value) => (value ? '是' : '否'),
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
    setEditingSender(null);
    form.resetFields();
    form.setFieldsValue({ is_default: false });
    setDrawerOpen(true);
  }, [form]);

  const openEditDrawer = useCallback(
    (sender: SenderAddress) => {
      setEditingSender(sender);
      form.setFieldsValue({
        name: sender.name,
        phone: sender.phone,
        province: sender.province,
        city: sender.city,
        district: sender.district,
        address: sender.address,
        is_default: sender.is_default,
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
    async (senderIds: string[]) => {
      if (!senderIds.length) {
        message.warning('请先选择要删除的寄方地址');
        return;
      }
      await deleteMutation.mutateAsync(senderIds);
    },
    [deleteMutation],
  );

  const targetIds = selectedSenderIds.length > 0 ? selectedSenderIds : activeSenderId ? [activeSenderId] : [];
  const targetSender = pageRows.find((row) => row.sender_address_id === targetIds[0]) ?? activeSender;

  const rightPanel = activeSender ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'ops']}
      panels={[
        {
          key: 'detail',
          label: '寄方详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="寄方名称">{activeSender.name}</Descriptions.Item>
              <Descriptions.Item label="联系电话">{activeSender.phone}</Descriptions.Item>
              <Descriptions.Item label="详细地址">{formatFullAddress(activeSender)}</Descriptions.Item>
              <Descriptions.Item label="默认寄方">{activeSender.is_default ? '是' : '否'}</Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'ops',
          label: '操作约束',
          children: (
            <Typography.Text type="secondary">
              寄方目录已纳入统一工业工作台结构，左侧筛选聚焦默认寄方与区域分布，主表与右侧详情维持订单审核同级密度。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="寄方详情">
      <Typography.Text type="secondary">请选择一条寄方记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeSender
    ? [
        { key: 'name', field: '寄方名称', value: activeSender.name },
        { key: 'phone', field: '联系电话', value: activeSender.phone },
        { key: 'default', field: '默认寄方', value: activeSender.is_default ? '是' : '否' },
      ]
    : [];

  const addressRows: DirectoryBottomDetailRow[] = activeSender
    ? [
        { key: 'province', field: '省份', value: activeSender.province },
        { key: 'city', field: '城市', value: activeSender.city },
        { key: 'district', field: '区县', value: activeSender.district },
        { key: 'address', field: '详细地址', value: activeSender.address },
      ]
    : [];

  const opsRows: DirectoryBottomDetailRow[] = activeSender
    ? [
        { key: 'selected', field: '当前勾选数', value: String(selectedSenderIds.length) },
        { key: 'search', field: '当前搜索关键字', value: trimText(searchKeyword) || '-' },
        { key: 'edit', field: '编辑能力', value: canWrite ? '可编辑' : '只读' },
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
            Boolean(activeSender),
            profileRows,
            '暂无基础资料',
            `sender-profile-${activeSender?.sender_address_id ?? 'empty'}`,
          ),
        },
        {
          key: 'address',
          label: `地址明细 (${addressRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeSender),
            addressRows,
            '暂无地址明细',
            `sender-address-${activeSender?.sender_address_id ?? 'empty'}`,
          ),
        },
        {
          key: 'ops',
          label: `操作上下文 (${opsRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activeSender),
            opsRows,
            '暂无操作上下文',
            `sender-ops-${activeSender?.sender_address_id ?? 'empty'}`,
          ),
        },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="sender-addresses-workbench-page"
        className="order-review-page sender-addresses-directory-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="sender-addresses-left-panel"
            dataTestId="sender-addresses-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as SenderAddressLeftFilterKey | undefined;
              if (!nextKey) {
                return;
              }
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems)}
            globalOrderTotal={sourceRows.length}
            statsScopeText="口径：当前寄方目录查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">寄方检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按姓名 / 电话 / 地址缩小左侧结果"
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
        rightPanelSyncKey={activeSender?.sender_address_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={
              <Space size={8} wrap>
                <Button type="primary" size="small" disabled={!tenantReady || !canWrite} onClick={openCreateDrawer}>
                  新建寄方
                </Button>
                <Button
                  size="small"
                  disabled={!tenantReady || !canWrite || targetIds.length !== 1 || !targetSender}
                  onClick={() => targetSender && openEditDrawer(targetSender)}
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
                      title: `确认删除选中的 ${targetIds.length} 个寄方地址吗？`,
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
            searchPlaceholder="搜索寄方名称 / 电话 / 地址"
            onExport={() => message.info('寄方目录导出能力待接入')}
            onPrint={() => message.info('寄方目录打印能力待接入')}
            onHelp={() => message.info('寄方目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<SenderAddress>
            rowKey={(record) => record.sender_address_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="wms.sender-addresses.main-grid"
            dataTestId="sender-addresses-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={query.isLoading || query.isFetching}
            error={query.error}
            onRetry={() => {
              void query.refetch();
            }}
            errorTitle="加载寄方地址失败"
            emptyText="暂无寄方地址"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              resetPagination();
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedSenderIds}
            activeRowKey={activeSenderId}
            onSelectionChange={(keys) => {
              setSelectedSenderIds(normalizeSelectionKeys(keys));
            }}
            onActiveRowChange={(record) => {
              setActiveSenderId(record.sender_address_id);
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
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedSenderIds.length} 条`}</span>
            <span>{activeSender ? `当前焦点：${activeSender.name}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`sender-addresses-${displayRows.length}-${pageRows.length}-${selectedSenderIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingSender ? '编辑寄方' : '新建寄方'}
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
          <Form.Item name="name" label="姓名" rules={[{ required: true, message: '请输入姓名' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="phone" label="电话" rules={[{ required: true, message: '请输入电话' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="province" label="省份" rules={[{ required: true, message: '请输入省份' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="city" label="城市" rules={[{ required: true, message: '请输入城市' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="district" label="区县" rules={[{ required: true, message: '请输入区县' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="address" label="详细地址" rules={[{ required: true, message: '请输入详细地址' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="is_default" label="默认寄方" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
