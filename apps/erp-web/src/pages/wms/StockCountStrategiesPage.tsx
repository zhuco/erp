import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  Popconfirm,
  Row,
  Select,
  Space,
  Switch,
  TablePaginationConfig,
  Tag,
  Tabs,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  StockCountStrategy,
  StockCountStrategyListQuery,
  StockCountSuggestion,
  StockCountSuggestionListQuery
} from '@/api/warehouseStrategies';
import {
  applyStockCountSuggestion,
  createStockCountStrategy,
  deleteStockCountStrategy,
  listStockCountStrategies,
  listStockCountSuggestions,
  refreshStockCountSuggestions,
  updateStockCountStrategy
} from '@/api/warehouseStrategies';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  enabled?: boolean;
};

type StrategyFormValues = {
  name: string;
  enabled: boolean;
  strategy_type: string;
  config_json?: string;
};

type SuggestionFilterValues = {
  warehouse_id?: string;
  status?: string;
};

const enabledOptions = [
  { label: '启用', value: true },
  { label: '停用', value: false }
];

const strategyTypeOptions = [
  { label: '每周全仓盘点', value: 'WEEKLY_FULL' },
  { label: '每日活跃SKU盘点', value: 'DAILY_ACTIVE_SKU' },
  { label: '缺货/负库存盘点', value: 'NEGATIVE_STOCK' }
];

const suggestionStatusOptions = [
  { label: 'OPEN', value: 'OPEN' },
  { label: 'APPLIED', value: 'APPLIED' }
];

const statusColors: Record<string, string> = {
  OPEN: 'green',
  APPLIED: 'blue'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const stringifyJson = (value: unknown, fallback: unknown) =>
  JSON.stringify(value ?? fallback, null, 2);

const parseJson = (value: string | undefined, fallback: unknown) => {
  if (!value || !value.trim()) {
    return fallback;
  }
  return JSON.parse(value);
};

const getStrategyTypeLabel = (value?: string | null) =>
  strategyTypeOptions.find((option) => option.value === value)?.label ?? value ?? '-';

export const StockCountStrategiesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('inventory.write');
  const stockSuggestFlag = useFeatureFlag('flows.stock_count_suggest');
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState('strategies');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingStrategy, setEditingStrategy] = useState<StockCountStrategy | null>(
    null
  );
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<StrategyFormValues>();
  const [suggestionFilters, setSuggestionFilters] = useState<SuggestionFilterValues>({});
  const [suggestionPagination, setSuggestionPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [suggestionForm] = Form.useForm<SuggestionFilterValues>();
  const [applyingId, setApplyingId] = useState<string | null>(null);

  const listQuery = useMemo<StockCountStrategyListQuery>(
    () => ({
      enabled: filters.enabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters.enabled, pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['stock-count-strategies', tenantId, listQuery],
    queryFn: () => listStockCountStrategies(tenantId, listQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'stock-count-suggestions'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const suggestionQuery = useMemo<StockCountSuggestionListQuery>(
    () => ({
      warehouse_id: suggestionFilters.warehouse_id,
      status: suggestionFilters.status,
      offset: ((suggestionPagination.current ?? 1) - 1) * (suggestionPagination.pageSize ?? 20),
      limit: suggestionPagination.pageSize ?? 20
    }),
    [suggestionFilters, suggestionPagination.current, suggestionPagination.pageSize]
  );

  const suggestionQueryState = useQuery({
    queryKey: ['stock-count-suggestions', tenantId, suggestionQuery],
    queryFn: () => listStockCountSuggestions(tenantId, suggestionQuery),
    enabled: tenantReady && stockSuggestFlag.enabled && activeTab === 'suggestions',
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (strategyId: string) => deleteStockCountStrategy(tenantId, strategyId),
    onMutate: (strategyId) => {
      setDeletingId(strategyId);
    },
    onSuccess: async () => {
      message.success('策略已删除');
      await queryClient.invalidateQueries({
        queryKey: ['stock-count-strategies', tenantId]
      });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除策略失败');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const refreshMutation = useMutation({
    mutationFn: (warehouseId?: string) =>
      refreshStockCountSuggestions(tenantId, {
        warehouse_id: warehouseId
      }),
    onSuccess: async (data) => {
      message.success(`已生成${data.created}条建议`);
      await queryClient.invalidateQueries({
        queryKey: ['stock-count-suggestions', tenantId]
      });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '刷新建议失败');
    }
  });

  const applyMutation = useMutation({
    mutationFn: (suggestionId: string) => applyStockCountSuggestion(tenantId, suggestionId),
    onMutate: (suggestionId) => {
      setApplyingId(suggestionId);
    },
    onSuccess: async (data) => {
      message.success(`盘点单已生成：${data.count_id}`);
      await queryClient.invalidateQueries({
        queryKey: ['stock-count-suggestions', tenantId]
      });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '生成盘点单失败');
    },
    onSettled: () => {
      setApplyingId(null);
    }
  });

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleSuggestionSearch = (values: SuggestionFilterValues) => {
    setSuggestionFilters(values);
    setSuggestionPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleSuggestionReset = () => {
    suggestionForm.resetFields();
    setSuggestionFilters({});
    setSuggestionPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingStrategy(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      enabled: true,
      strategy_type: 'WEEKLY_FULL',
      config_json: stringifyJson({}, {})
    });
    setDrawerOpen(true);
  };

  const openEditDrawer = (strategy: StockCountStrategy) => {
    setEditingStrategy(strategy);
    drawerForm.setFieldsValue({
      name: strategy.name,
      enabled: strategy.enabled,
      strategy_type: strategy.strategy_type,
      config_json: stringifyJson(strategy.config_json, {})
    });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const configJson = parseJson(values.config_json, {});
      const payload: Partial<StockCountStrategy> = {
        name: values.name.trim(),
        enabled: values.enabled,
        strategy_type: values.strategy_type,
        config_json: configJson
      };
      setSaving(true);
      if (editingStrategy) {
        await updateStockCountStrategy(tenantId, editingStrategy.strategy_id, payload);
        message.success('策略已更新');
      } else {
        await createStockCountStrategy(tenantId, payload);
        message.success('策略已创建');
      }
      setDrawerOpen(false);
      setEditingStrategy(null);
      await queryClient.invalidateQueries({
        queryKey: ['stock-count-strategies', tenantId]
      });
    } catch (err) {
      if (err instanceof SyntaxError) {
        message.error('配置 JSON 格式错误');
      } else if (err instanceof Error) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const columns = useMemo<ColumnsType<StockCountStrategy>>(
    () => [
      {
        title: '策略名称',
        dataIndex: 'name'
      },
      {
        title: '启用',
        dataIndex: 'enabled',
        render: (_, record) =>
          record.enabled ? <Tag color="green">启用</Tag> : <Tag>停用</Tag>
      },
      {
        title: '策略类型',
        dataIndex: 'strategy_type',
        render: (_, record) => getStrategyTypeLabel(record.strategy_type)
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) =>
          canWrite ? (
            <Space>
              <Button size="small" onClick={() => openEditDrawer(record)}>
                编辑
              </Button>
              <Popconfirm
                title="确认删除该策略？"
                okText="删除"
                cancelText="取消"
                onConfirm={() => deleteMutation.mutate(record.strategy_id)}
              >
                <Button
                  size="small"
                  danger
                  loading={deleteMutation.isPending && deletingId === record.strategy_id}
                >
                  删除
                </Button>
              </Popconfirm>
            </Space>
          ) : (
            <span>-</span>
          )
      }
    ],
    [canWrite, deleteMutation, deletingId]
  );

  const suggestionColumns = useMemo<ColumnsType<StockCountSuggestion>>(
    () => [
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (_, record) =>
          warehouseOptions.find((option) => option.value === record.warehouse_id)?.label ??
          record.warehouse_id
      },
      {
        title: '策略',
        dataIndex: 'strategy_name',
        render: (_, record) => record.strategy_name ?? '-'
      },
      {
        title: '原因',
        dataIndex: 'reason',
        ellipsis: true,
        render: (_, record) => record.reason ?? '-'
      },
      {
        title: 'SKU数量',
        dataIndex: 'item_count'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={statusColors[record.status] ?? 'default'}>{record.status}</Tag>
        )
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) =>
          canWrite ? (
            <Button
              size="small"
              type="primary"
              disabled={record.status !== 'OPEN'}
              loading={applyMutation.isPending && applyingId === record.suggestion_id}
              onClick={() => applyMutation.mutate(record.suggestion_id)}
            >
              生成盘点单
            </Button>
          ) : (
            <span>-</span>
          )
      }
    ],
    [applyMutation, applyingId, canWrite, warehouseOptions]
  );

  if (!tenantReady) {
    return <ErrorState title="请先选择商家" error={new Error('未选择商家')} />;
  }

  const strategyToolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        盘点策略
      </Typography.Title>
      {canWrite ? (
        <Button type="primary" onClick={openCreateDrawer}>
          新建策略
        </Button>
      ) : null}
    </Space>
  );

  const suggestionToolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        盘点策略
      </Typography.Title>
      <Button
        type="dashed"
        disabled={!stockSuggestFlag.enabled || !canWrite}
        loading={refreshMutation.isPending}
        onClick={() => refreshMutation.mutate(suggestionForm.getFieldValue('warehouse_id'))}
      >
        刷新建议
      </Button>
    </Space>
  );

  const strategyHeader = (
    <Card>
      <Form form={form} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="enabled" label="启用状态">
              <Select allowClear options={enabledOptions} placeholder="选择状态" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button onClick={handleReset}>重置</Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  );

  const suggestionHeader = (
    <Space direction="vertical" size={8}>
      {!stockSuggestFlag.enabled ? (
        <Typography.Text type="warning">
          {stockSuggestFlag.reason ?? '盘点建议已关闭'}
        </Typography.Text>
      ) : null}
      <Card>
        <Form form={suggestionForm} layout="vertical" onFinish={handleSuggestionSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库">
                <Select
                  allowClear
                  options={warehouseOptions}
                  placeholder="选择仓库"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={suggestionStatusOptions} placeholder="选择状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleSuggestionReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: 'strategies',
            label: '策略配置',
            children: (
              <UiModule03MainTableSection<StockCountStrategy>
                toolbarLeft={strategyToolbarLeft}
                header={strategyHeader}
                rowKey="strategy_id"
                columns={columns}
                data={listQueryState.data?.items ?? []}
                loading={listQueryState.isLoading || listQueryState.isFetching}
                error={listQueryState.error}
                onRetry={() => listQueryState.refetch()}
                errorTitle="策略加载失败"
                emptyText="暂无策略"
                total={listQueryState.data?.total ?? 0}
                pagination={pagination}
                onPaginationChange={setPagination}
                columnSettingsKey="wms.stock-count-strategies.table.columns"
              />
            )
          },
          {
            key: 'suggestions',
            label: '盘点建议',
            children: (
              <UiModule03MainTableSection<StockCountSuggestion>
                toolbarLeft={suggestionToolbarLeft}
                header={suggestionHeader}
                rowKey="suggestion_id"
                columns={suggestionColumns}
                data={suggestionQueryState.data?.items ?? []}
                loading={suggestionQueryState.isLoading || suggestionQueryState.isFetching}
                error={suggestionQueryState.error}
                onRetry={() => suggestionQueryState.refetch()}
                errorTitle="建议加载失败"
                emptyText="暂无盘点建议"
                total={suggestionQueryState.data?.total ?? 0}
                pagination={suggestionPagination}
                onPaginationChange={setSuggestionPagination}
                columnSettingsKey="wms.stock-count-suggestions.table.columns"
              />
            )
          }
        ]}
      />
      <Drawer
        title={editingStrategy ? '编辑策略' : '新建策略'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="name"
            label="策略名称"
            rules={[{ required: true, message: '请输入策略名称' }]}
          >
            <Input placeholder="策略名称" />
          </Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item
            name="strategy_type"
            label="策略类型"
            rules={[{ required: true, message: '请选择策略类型' }]}
          >
            <Select options={strategyTypeOptions} />
          </Form.Item>
          <Form.Item name="config_json" label="策略配置 (JSON)">
            <Input.TextArea rows={4} placeholder='{"weekday":1}' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
