import {
  Button,
  Card,
  Col,
  Drawer,
  DatePicker,
  Form,
  Input,
  InputNumber,
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
import dayjs, { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  ShipmentPreprocessHit,
  ShipmentPreprocessHitListQuery,
  ShipmentPreprocessStrategy,
  ShipmentPreprocessStrategyListQuery
} from '@/api/warehouseStrategies';
import {
  createShipmentPreprocessStrategy,
  deleteShipmentPreprocessStrategy,
  listShipmentPreprocessHits,
  listShipmentPreprocessStrategies,
  updateShipmentPreprocessStrategy
} from '@/api/warehouseStrategies';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  enabled?: boolean;
};

type StrategyFormValues = {
  name: string;
  enabled: boolean;
  priority: number;
  effective_range?: [Dayjs, Dayjs];
  conditions_json?: string;
  rules_json?: string;
  auto_freeze: boolean;
  auto_waybill_reserve: boolean;
  sender_address_id?: string;
};

type HitFilterValues = {
  strategy_id?: string;
  shipment_id?: string;
  order_id?: string;
};

const enabledOptions = [
  { label: '启用', value: true },
  { label: '停用', value: false }
];

const defaultConditions = [];
const defaultRules = {};

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

export const ShipmentPreprocessStrategiesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState('strategies');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingStrategy, setEditingStrategy] = useState<ShipmentPreprocessStrategy | null>(
    null
  );
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<StrategyFormValues>();
  const [hitFilters, setHitFilters] = useState<HitFilterValues>({});
  const [hitPagination, setHitPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [hitForm] = Form.useForm<HitFilterValues>();

  const listQuery = useMemo<ShipmentPreprocessStrategyListQuery>(
    () => ({
      enabled: filters.enabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters.enabled, pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['shipment-preprocess-strategies', tenantId, listQuery],
    queryFn: () => listShipmentPreprocessStrategies(tenantId, listQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const hitQuery = useMemo<ShipmentPreprocessHitListQuery>(
    () => ({
      strategy_id: hitFilters.strategy_id?.trim() || undefined,
      shipment_id: hitFilters.shipment_id?.trim() || undefined,
      order_id: hitFilters.order_id?.trim() || undefined,
      offset: ((hitPagination.current ?? 1) - 1) * (hitPagination.pageSize ?? 20),
      limit: hitPagination.pageSize ?? 20
    }),
    [hitFilters, hitPagination.current, hitPagination.pageSize]
  );

  const hitQueryState = useQuery({
    queryKey: ['shipment-preprocess-hits', tenantId, hitQuery],
    queryFn: () => listShipmentPreprocessHits(tenantId, hitQuery),
    enabled: tenantReady && activeTab === 'hits',
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (strategyId: string) =>
      deleteShipmentPreprocessStrategy(tenantId, strategyId),
    onMutate: (strategyId) => {
      setDeletingId(strategyId);
    },
    onSuccess: async () => {
      message.success('策略已删除');
      await queryClient.invalidateQueries({
        queryKey: ['shipment-preprocess-strategies', tenantId]
      });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除策略失败');
    },
    onSettled: () => {
      setDeletingId(null);
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

  const handleHitSearch = (values: HitFilterValues) => {
    setHitFilters(values);
    setHitPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleHitReset = () => {
    hitForm.resetFields();
    setHitFilters({});
    setHitPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingStrategy(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      enabled: true,
      priority: 0,
      auto_freeze: false,
      auto_waybill_reserve: false,
      conditions_json: stringifyJson(defaultConditions, defaultConditions),
      rules_json: stringifyJson(defaultRules, defaultRules)
    });
    setDrawerOpen(true);
  };

  const openEditDrawer = (strategy: ShipmentPreprocessStrategy) => {
    setEditingStrategy(strategy);
    drawerForm.setFieldsValue({
      name: strategy.name,
      enabled: strategy.enabled,
      priority: strategy.priority,
      effective_range:
        strategy.effective_from && strategy.effective_to
          ? [dayjs(strategy.effective_from), dayjs(strategy.effective_to)]
          : undefined,
      conditions_json: stringifyJson(strategy.conditions_json, defaultConditions),
      rules_json: stringifyJson(strategy.rules_json, defaultRules),
      auto_freeze: strategy.auto_freeze,
      auto_waybill_reserve: strategy.auto_waybill_reserve,
      sender_address_id: strategy.sender_address_id ?? undefined
    });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const conditionsJson = parseJson(values.conditions_json, defaultConditions);
      const rulesJson = parseJson(values.rules_json, defaultRules);
      const payload: Partial<ShipmentPreprocessStrategy> = {
        name: values.name.trim(),
        enabled: values.enabled,
        priority: values.priority,
        effective_from: values.effective_range?.[0]?.toISOString(),
        effective_to: values.effective_range?.[1]?.toISOString(),
        conditions_json: conditionsJson,
        rules_json: rulesJson,
        auto_freeze: values.auto_freeze,
        auto_waybill_reserve: values.auto_waybill_reserve,
        sender_address_id: values.sender_address_id?.trim() || null
      };
      setSaving(true);
      if (editingStrategy) {
        await updateShipmentPreprocessStrategy(
          tenantId,
          editingStrategy.strategy_id,
          payload
        );
        message.success('策略已更新');
      } else {
        await createShipmentPreprocessStrategy(tenantId, payload);
        message.success('策略已创建');
      }
      setDrawerOpen(false);
      setEditingStrategy(null);
      await queryClient.invalidateQueries({
        queryKey: ['shipment-preprocess-strategies', tenantId]
      });
    } catch (err) {
      if (err instanceof SyntaxError) {
        message.error('条件或规则 JSON 格式错误');
      } else if (err instanceof Error) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const columns = useMemo<ColumnsType<ShipmentPreprocessStrategy>>(
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
        title: '优先级',
        dataIndex: 'priority'
      },
      {
        title: '生效开始',
        dataIndex: 'effective_from',
        render: (_, record) => formatDateTime(record.effective_from)
      },
      {
        title: '生效结束',
        dataIndex: 'effective_to',
        render: (_, record) => formatDateTime(record.effective_to)
      },
      {
        title: '自动冻结',
        dataIndex: 'auto_freeze',
        render: (_, record) =>
          record.auto_freeze ? <Tag color="green">开启</Tag> : <Tag>关闭</Tag>
      },
      {
        title: '自动预约单号',
        dataIndex: 'auto_waybill_reserve',
        render: (_, record) =>
          record.auto_waybill_reserve ? <Tag color="green">开启</Tag> : <Tag>关闭</Tag>
      },
      {
        title: '寄方地址',
        dataIndex: 'sender_address_id',
        render: (_, record) => record.sender_address_id ?? '-'
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

  const hitColumns = useMemo<ColumnsType<ShipmentPreprocessHit>>(
    () => [
      {
        title: '命中时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: '策略ID',
        dataIndex: 'strategy_id',
        render: (_, record) => record.strategy_id ?? '-'
      },
      {
        title: '发货单',
        dataIndex: 'shipment_id',
        render: (_, record) => record.shipment_id ?? '-'
      },
      {
        title: '订单',
        dataIndex: 'order_id',
        render: (_, record) => record.order_id ?? '-'
      },
      {
        title: '结果',
        dataIndex: 'success',
        render: (_, record) =>
          record.success ? <Tag color="green">成功</Tag> : <Tag color="red">失败</Tag>
      },
      {
        title: '命中原因',
        dataIndex: 'hit_reason',
        ellipsis: true,
        render: (_, record) => record.hit_reason ?? '-'
      },
      {
        title: '执行结果',
        dataIndex: 'result_json',
        ellipsis: true,
        render: (_, record) =>
          record.result_json ? JSON.stringify(record.result_json) : '-'
      }
    ],
    []
  );

  if (!tenantReady) {
    return (
      <ErrorState title="请先选择商家" error={new Error('未选择商家')} />
    );
  }

  const strategyToolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        发货预处理策略
      </Typography.Title>
      {canWrite ? (
        <Button type="primary" onClick={openCreateDrawer}>
          新建策略
        </Button>
      ) : null}
    </Space>
  );

  const hitToolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      发货预处理策略
    </Typography.Title>
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

  const hitHeader = (
    <Card>
      <Form form={hitForm} layout="vertical" onFinish={handleHitSearch}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="strategy_id" label="策略ID">
              <Input allowClear placeholder="策略ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="shipment_id" label="发货单ID">
              <Input allowClear placeholder="发货单ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="order_id" label="订单ID">
              <Input allowClear placeholder="订单ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button onClick={handleHitReset}>重置</Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
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
              <UiModule03MainTableSection<ShipmentPreprocessStrategy>
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
                columnSettingsKey="wms.shipment-preprocess-strategies.table.columns"
              />
            )
          },
          {
            key: 'hits',
            label: '命中日志',
            children: (
              <UiModule03MainTableSection<ShipmentPreprocessHit>
                toolbarLeft={hitToolbarLeft}
                header={hitHeader}
                rowKey="hit_id"
                columns={hitColumns}
                data={hitQueryState.data?.items ?? []}
                loading={hitQueryState.isLoading || hitQueryState.isFetching}
                error={hitQueryState.error}
                onRetry={() => hitQueryState.refetch()}
                errorTitle="命中日志加载失败"
                emptyText="暂无命中记录"
                total={hitQueryState.data?.total ?? 0}
                pagination={hitPagination}
                onPaginationChange={setHitPagination}
                columnSettingsKey="wms.shipment-preprocess-hits.table.columns"
              />
            )
          }
        ]}
      />
      <Drawer
        title={editingStrategy ? '编辑策略' : '新建策略'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={560}
        destroyOnHidden
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
            name="priority"
            label="优先级"
            rules={[{ required: true, message: '请输入优先级' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="effective_range" label="生效时间">
            <DatePicker.RangePicker showTime style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="sender_address_id" label="寄方地址ID">
            <Input placeholder="sender_address_id（可选）" />
          </Form.Item>
          <Form.Item name="auto_freeze" label="自动冻结" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="auto_waybill_reserve" label="自动预约单号" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="conditions_json" label="处理条件 (DSL)">
            <Input.TextArea rows={4} placeholder="[]" />
          </Form.Item>
          <Form.Item name="rules_json" label="预处理规则">
            <Input.TextArea rows={4} placeholder='{"warehouse_id":"..."}' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
