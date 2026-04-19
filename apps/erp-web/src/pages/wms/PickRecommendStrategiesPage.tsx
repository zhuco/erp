import {
  Button,
  Card,
  Col,
  Drawer,
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
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  PickRecommendHit,
  PickRecommendHitListQuery,
  PickRecommendStrategy,
  PickRecommendStrategyListQuery
} from '@/api/warehouseStrategies';
import {
  createPickRecommendStrategy,
  deletePickRecommendStrategy,
  listPickRecommendHits,
  listPickRecommendStrategies,
  updatePickRecommendStrategy
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
  method: string;
  prefer_location_priority: boolean;
  prefer_pick_zone: boolean;
  prefer_sufficient: boolean;
};

type HitFilterValues = {
  strategy_id?: string;
  pick_task_id?: string;
};

const enabledOptions = [
  { label: '启用', value: true },
  { label: '停用', value: false }
];

const methodOptions = [
  { label: 'FIFO', value: 'FIFO' },
  { label: 'FEFO', value: 'FEFO' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const PickRecommendStrategiesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('wms.write');
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState('strategies');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingStrategy, setEditingStrategy] = useState<PickRecommendStrategy | null>(null);
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

  const listQuery = useMemo<PickRecommendStrategyListQuery>(
    () => ({
      enabled: filters.enabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters.enabled, pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['pick-recommend-strategies', tenantId, listQuery],
    queryFn: () => listPickRecommendStrategies(tenantId, listQuery),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const hitQuery = useMemo<PickRecommendHitListQuery>(
    () => ({
      strategy_id: hitFilters.strategy_id?.trim() || undefined,
      pick_task_id: hitFilters.pick_task_id?.trim() || undefined,
      offset: ((hitPagination.current ?? 1) - 1) * (hitPagination.pageSize ?? 20),
      limit: hitPagination.pageSize ?? 20
    }),
    [hitFilters, hitPagination.current, hitPagination.pageSize]
  );

  const hitQueryState = useQuery({
    queryKey: ['pick-recommend-hits', tenantId, hitQuery],
    queryFn: () => listPickRecommendHits(tenantId, hitQuery),
    enabled: tenantReady && activeTab === 'hits',
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (strategyId: string) => deletePickRecommendStrategy(tenantId, strategyId),
    onMutate: (strategyId) => {
      setDeletingId(strategyId);
    },
    onSuccess: async () => {
      message.success('策略已删除');
      await queryClient.invalidateQueries({
        queryKey: ['pick-recommend-strategies', tenantId]
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
      method: 'FIFO',
      prefer_location_priority: false,
      prefer_pick_zone: false,
      prefer_sufficient: false
    });
    setDrawerOpen(true);
  };

  const openEditDrawer = (strategy: PickRecommendStrategy) => {
    setEditingStrategy(strategy);
    drawerForm.setFieldsValue({
      name: strategy.name,
      enabled: strategy.enabled,
      priority: strategy.priority,
      method: strategy.method,
      prefer_location_priority: strategy.prefer_location_priority,
      prefer_pick_zone: strategy.prefer_pick_zone,
      prefer_sufficient: strategy.prefer_sufficient
    });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: Partial<PickRecommendStrategy> = {
        name: values.name.trim(),
        enabled: values.enabled,
        priority: values.priority,
        method: values.method,
        prefer_location_priority: values.prefer_location_priority,
        prefer_pick_zone: values.prefer_pick_zone,
        prefer_sufficient: values.prefer_sufficient
      };
      setSaving(true);
      if (editingStrategy) {
        await updatePickRecommendStrategy(tenantId, editingStrategy.strategy_id, payload);
        message.success('策略已更新');
      } else {
        await createPickRecommendStrategy(tenantId, payload);
        message.success('策略已创建');
      }
      setDrawerOpen(false);
      setEditingStrategy(null);
      await queryClient.invalidateQueries({
        queryKey: ['pick-recommend-strategies', tenantId]
      });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const columns = useMemo<ColumnsType<PickRecommendStrategy>>(
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
        title: '方法',
        dataIndex: 'method'
      },
      {
        title: '货位优先',
        dataIndex: 'prefer_location_priority',
        render: (_, record) =>
          record.prefer_location_priority ? <Tag color="green">是</Tag> : <Tag>否</Tag>
      },
      {
        title: '拣货区优先',
        dataIndex: 'prefer_pick_zone',
        render: (_, record) =>
          record.prefer_pick_zone ? <Tag color="green">是</Tag> : <Tag>否</Tag>
      },
      {
        title: '库存充足优先',
        dataIndex: 'prefer_sufficient',
        render: (_, record) =>
          record.prefer_sufficient ? <Tag color="green">是</Tag> : <Tag>否</Tag>
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

  const hitColumns = useMemo<ColumnsType<PickRecommendHit>>(
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
        title: '拣货任务',
        dataIndex: 'pick_task_id',
        render: (_, record) => record.pick_task_id ?? '-'
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (_, record) => record.sku_id ?? '-'
      },
      {
        title: '货位',
        dataIndex: 'from_location_id',
        render: (_, record) => record.from_location_id ?? '-'
      },
      {
        title: '原因',
        dataIndex: 'reason',
        ellipsis: true,
        render: (_, record) => record.reason ?? '-'
      },
      {
        title: '结果',
        dataIndex: 'success',
        render: (_, record) =>
          record.success ? <Tag color="green">成功</Tag> : <Tag color="red">失败</Tag>
      }
    ],
    []
  );

  if (!tenantReady) {
    return <ErrorState title="请先选择商家" error={new Error('未选择商家')} />;
  }

  const strategyToolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        拣货推荐策略
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
      拣货推荐策略
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
            <Form.Item name="pick_task_id" label="拣货任务ID">
              <Input allowClear placeholder="拣货任务ID" />
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
              <UiModule03MainTableSection<PickRecommendStrategy>
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
                columnSettingsKey="wms.pick-recommend-strategies.table.columns"
              />
            )
          },
          {
            key: 'hits',
            label: '命中日志',
            children: (
              <UiModule03MainTableSection<PickRecommendHit>
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
                columnSettingsKey="wms.pick-recommend-hits.table.columns"
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
          <Form.Item name="method" label="拣货方法" rules={[{ required: true }]}>
            <Select options={methodOptions} />
          </Form.Item>
          <Form.Item
            name="prefer_location_priority"
            label="货位优先"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
          <Form.Item name="prefer_pick_zone" label="拣货区优先" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="prefer_sufficient" label="库存充足优先" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
