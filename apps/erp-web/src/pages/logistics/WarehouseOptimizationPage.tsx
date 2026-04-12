import {
  Button,
  Descriptions,
  Form,
  Input,
  Modal,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  LogisticsWarehouseStrategy,
  LogisticsWarehouseSuggestion
} from '@/api/logisticsOptimization';
import {
  applyLogisticsWarehouseSuggestion,
  createLogisticsWarehouseStrategy,
  listLogisticsWarehouseStrategies,
  listLogisticsWarehouseStrategyHistory,
  listLogisticsWarehouseSuggestions,
  updateLogisticsWarehouseStrategy
} from '@/api/logisticsOptimization';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type StrategyFormValues = {
  name: string;
  priority: LogisticsWarehouseStrategy['priority'];
  enabled: boolean;
};

const formatCurrency = (value: number) => `¥${(value / 100).toFixed(2)}`;

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const WarehouseOptimizationPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [strategyModalOpen, setStrategyModalOpen] = useState(false);
  const [historyModalOpen, setHistoryModalOpen] = useState(false);
  const [editingStrategy, setEditingStrategy] = useState<LogisticsWarehouseStrategy | null>(null);
  const [form] = Form.useForm<StrategyFormValues>();

  const strategiesQuery = useQuery({
    queryKey: ['logistics-warehouse-strategies', tenantId],
    queryFn: () =>
      listLogisticsWarehouseStrategies(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const suggestionsQuery = useQuery({
    queryKey: ['logistics-warehouse-suggestions', tenantId],
    queryFn: () =>
      listLogisticsWarehouseSuggestions(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const historyQuery = useQuery({
    queryKey: ['logistics-warehouse-strategy-history', tenantId, editingStrategy?.strategy_id],
    queryFn: () =>
      listLogisticsWarehouseStrategyHistory(tenantId, editingStrategy?.strategy_id ?? ''),
    enabled: historyModalOpen && Boolean(editingStrategy?.strategy_id)
  });

  const strategies = strategiesQuery.data?.items ?? [];
  const suggestions = suggestionsQuery.data?.items ?? [];

  const openStrategyModal = (record?: LogisticsWarehouseStrategy) => {
    setEditingStrategy(record ?? null);
    if (record) {
      form.setFieldsValue(record);
    } else {
      form.resetFields();
      form.setFieldsValue({
        name: '',
        priority: 'SPEED',
        enabled: true
      });
    }
    setStrategyModalOpen(true);
  };

  const saveStrategy = async () => {
    if (!canWrite) {
      message.warning('暂无权限');
      return;
    }
    try {
      const values = await form.validateFields();
      if (editingStrategy) {
        await updateLogisticsWarehouseStrategy(tenantId, editingStrategy.strategy_id, {
          name: values.name,
          priority: values.priority,
          enabled: values.enabled,
          updated_by: '物流主管',
          history_summary: `更新策略：${values.name}`
        });
      } else {
        await createLogisticsWarehouseStrategy(tenantId, {
          name: values.name,
          priority: values.priority,
          enabled: values.enabled,
          updated_by: '物流主管',
          history_summary: `新增策略：${values.name}`
        });
      }
      message.success('策略已保存');
      setStrategyModalOpen(false);
      setEditingStrategy(null);
      await queryClient.invalidateQueries({
        queryKey: ['logistics-warehouse-strategies', tenantId]
      });
      await queryClient.invalidateQueries({
        queryKey: ['logistics-warehouse-strategy-history', tenantId]
      });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  };

  const handleApply = async (record: LogisticsWarehouseSuggestion) => {
    if (!canWrite) {
      return;
    }
    try {
      await applyLogisticsWarehouseSuggestion(tenantId, record.suggestion_id);
      message.success(`已回写履约仓：${record.recommended_warehouse}`);
      await queryClient.invalidateQueries({
        queryKey: ['logistics-warehouse-suggestions', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '回写失败');
    }
  };

  const strategyColumns = useMemo<ColumnsType<LogisticsWarehouseStrategy>>(
    () => [
      { title: '策略名称', dataIndex: 'name' },
      {
        title: '优先级',
        dataIndex: 'priority',
        render: (value: LogisticsWarehouseStrategy['priority']) => (
          <Tag color={value === 'SPEED' ? 'blue' : 'purple'}>
            {value === 'SPEED' ? '时效优先' : '成本优先'}
          </Tag>
        )
      },
      {
        title: '启用',
        dataIndex: 'enabled',
        render: (value: boolean, record) => (
          <Switch
            size="small"
            checked={value}
            disabled={!canWrite}
            onChange={async (checked) => {
              try {
                await updateLogisticsWarehouseStrategy(tenantId, record.strategy_id, {
                  enabled: checked,
                  updated_by: '物流主管',
                  history_summary: checked ? '启用策略' : '停用策略'
                });
                await queryClient.invalidateQueries({
                  queryKey: ['logistics-warehouse-strategies', tenantId]
                });
              } catch (err) {
                message.error(err instanceof Error ? err.message : '更新失败');
              }
            }}
          />
        )
      },
      { title: '更新人', dataIndex: 'updated_by' },
      { title: '更新时间', dataIndex: 'updated_at', render: formatDateTime },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8}>
            <Button size="small" disabled={!canWrite} onClick={() => openStrategyModal(record)}>
              编辑
            </Button>
            <Button
              size="small"
              onClick={() => {
                setEditingStrategy(record);
                setHistoryModalOpen(true);
              }}
            >
              变更记录
            </Button>
          </Space>
        )
      }
    ],
    [canWrite, queryClient, tenantId]
  );

  const suggestionColumns = useMemo<ColumnsType<LogisticsWarehouseSuggestion>>(
    () => [
      { title: '订单号', dataIndex: 'order_id' },
      { title: '目的地', dataIndex: 'destination' },
      { title: '推荐仓库', dataIndex: 'recommended_warehouse' },
      { title: '预计时效(小时)', dataIndex: 'eta_hours' },
      {
        title: '预计成本',
        dataIndex: 'cost_cent',
        render: (value: number) => formatCurrency(value)
      },
      {
        title: '节省成本',
        dataIndex: 'savings_cent',
        render: (value: number) => formatCurrency(value)
      },
      {
        title: '策略',
        dataIndex: 'strategy_id',
        render: (value: string) =>
          strategies.find((item) => item.strategy_id === value)?.name ?? value
      },
      {
        title: '状态',
        dataIndex: 'applied',
        render: (value: boolean, record) =>
          value ? (
            <Tag color="green">已回写 {formatDateTime(record.applied_at)}</Tag>
          ) : (
            <Tag color="blue">待处理</Tag>
          )
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" disabled={!canWrite || record.applied} onClick={() => handleApply(record)}>
            回写订单
          </Button>
        )
      }
    ],
    [canWrite, strategies]
  );

  return (
    <>
      <UiModule07SettingsLayout title="仓配优化与成本控制">
        <UiModule07SettingsSection
          title="优化策略"
          actions={
            <Button type="primary" disabled={!canWrite} onClick={() => openStrategyModal()}>
              新增策略
            </Button>
          }
        >
          <Table
            rowKey="strategy_id"
            columns={strategyColumns}
            dataSource={strategies}
            pagination={false}
            loading={strategiesQuery.isLoading}
          />
        </UiModule07SettingsSection>
        <UiModule07SettingsSection title="优化建议">
          <Table
            rowKey="suggestion_id"
            columns={suggestionColumns}
            dataSource={suggestions}
            loading={suggestionsQuery.isLoading}
          />
        </UiModule07SettingsSection>
      </UiModule07SettingsLayout>
      <Modal
        title={editingStrategy ? '编辑策略' : '新增策略'}
        open={strategyModalOpen}
        onCancel={() => setStrategyModalOpen(false)}
        onOk={saveStrategy}
        okText="保存"
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label="策略名称"
            rules={[{ required: true, message: '请输入名称' }]}
          >
            <Input placeholder="例如：时效优先策略" />
          </Form.Item>
          <Form.Item
            name="priority"
            label="优先级"
            rules={[{ required: true, message: '请选择优先级' }]}
          >
            <Select
              options={[
                { label: '时效优先', value: 'SPEED' },
                { label: '成本优先', value: 'COST' }
              ]}
            />
          </Form.Item>
          <Form.Item name="enabled" label="启用状态" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="策略变更记录"
        open={historyModalOpen}
        onCancel={() => setHistoryModalOpen(false)}
        footer={[
          <Button key="close" onClick={() => setHistoryModalOpen(false)}>
            关闭
          </Button>
        ]}
      >
        {editingStrategy ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions size="small" bordered>
              <Descriptions.Item label="策略名称">{editingStrategy.name}</Descriptions.Item>
              <Descriptions.Item label="优先级">
                {editingStrategy.priority === 'SPEED' ? '时效优先' : '成本优先'}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(editingStrategy.updated_at)}
              </Descriptions.Item>
            </Descriptions>
            <Table
              rowKey="record_id"
              pagination={false}
              loading={historyQuery.isLoading}
              dataSource={historyQuery.data?.items ?? []}
              columns={[
                { title: '变更说明', dataIndex: 'summary' },
                { title: '变更人', dataIndex: 'updated_by' },
                { title: '时间', dataIndex: 'updated_at', render: formatDateTime }
              ]}
            />
          </Space>
        ) : null}
      </Modal>
    </>
  );
};
