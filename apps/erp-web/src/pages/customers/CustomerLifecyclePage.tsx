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
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { CustomerLifecycleTask, CustomerLifecycleTaskCreatePayload } from '@/api/customerLifecycle';
import {
  createCustomerLifecycleTask,
  deleteCustomerLifecycleTask,
  listCustomerLifecycleTasks,
  updateCustomerLifecycleTask
} from '@/api/customerLifecycle';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  stage?: string;
  status?: string;
};

type FormValues = {
  stage: string;
  strategy: string;
  trigger_rule?: string;
  owner?: string;
  status?: string;
};

const statusOptions = [
  { label: '鍚敤', value: 'ACTIVE' },
  { label: '鍋滅敤', value: 'INACTIVE' }
];

const stageOptions = [
  { label: '鏂板', value: 'NEW' },
  { label: '娲昏穬', value: 'ACTIVE' },
  { label: '娌夐粯', value: 'DORMANT' },
  { label: '娴佸け', value: 'CHURNED' }
];

const stageMap = new Map(stageOptions.map((option) => [option.value, option.label]));

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const CustomerLifecyclePage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('customers.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingTask, setEditingTask] = useState<CustomerLifecycleTask | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      stage: filters.stage || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['customer-lifecycle', tenantId, query],
    queryFn: () => listCustomerLifecycleTasks(tenantId, query),
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (taskId: string) => deleteCustomerLifecycleTask(tenantId, taskId),
    onMutate: (taskId) => {
      setDeletingId(taskId);
    },
    onSuccess: () => {
      message.success('生命周期任务已删除');
      queryClient.invalidateQueries({ queryKey: ['customer-lifecycle', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '鍒犻櫎鐢熷懡鍛ㄦ湡浠诲姟澶辫触');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<CustomerLifecycleTask>>(
    () => [
      {
        title: '闃舵',
        dataIndex: 'stage',
        render: (value: string) => stageMap.get(value) ?? value
      },
      {
        title: '瑙﹁揪绛栫暐',
        dataIndex: 'strategy'
      },
      {
        title: '瑙﹀彂瑙勫垯',
        dataIndex: 'trigger_rule',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '负责人',
        dataIndex: 'owner',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value?: string | null) => (
          <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value ?? '-'}</Tag>
        )
      },
      {
        title: '最近触达',
        dataIndex: 'last_triggered_at',
        render: (value?: string | null) => formatDateTime(value)
      },
      {
        title: '鏇存柊鏃堕棿',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '鎿嶄綔',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditingTask(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  stage: record.stage,
                  strategy: record.strategy,
                  trigger_rule: record.trigger_rule ?? undefined,
                  owner: record.owner ?? undefined,
                  status: record.status ?? undefined
                });
              }}
            >
              缂栬緫
            </Button>
            <Popconfirm
              title="确认删除生命周期任务？"
              okText="鍒犻櫎"
              cancelText="鍙栨秷"
              onConfirm={() => deleteMutation.mutate(record.task_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.task_id}
              >
                鍒犻櫎
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, deletingId, drawerForm]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingTask(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: CustomerLifecycleTaskCreatePayload = {
        stage: values.stage as CustomerLifecycleTaskCreatePayload['stage'],
        strategy: values.strategy.trim(),
        trigger_rule: values.trigger_rule?.trim() || undefined,
        owner: values.owner?.trim() || undefined,
        status: values.status
      };
      setSaving(true);
      if (editingTask) {
        await updateCustomerLifecycleTask(tenantId, editingTask.task_id, payload);
        message.success('生命周期任务已更新');
      } else {
        await createCustomerLifecycleTask(tenantId, payload);
        message.success('生命周期任务已创建');
      }
      setDrawerOpen(false);
      setEditingTask(null);
      await queryClient.invalidateQueries({ queryKey: ['customer-lifecycle', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }} data-testid="customer-lifecycle-title">
          鐢熷懡鍛ㄦ湡杩愯惀
        </Typography.Title>
        <Button
          type="primary"
          disabled={!canWrite}
          onClick={openCreateDrawer}
          data-testid="customer-lifecycle-create"
        >
          鏂板缓浠诲姟
        </Button>
      </Space>
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSearch}
          data-testid="customer-lifecycle-filter-form"
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="策略/负责人" data-testid="customer-lifecycle-filter-keyword" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="stage" label="闃舵">
                <Select allowClear options={stageOptions} placeholder="鍏ㄩ儴" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="鍏ㄩ儴" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit" data-testid="customer-lifecycle-filter-submit">
                  鏌ヨ
                </Button>
                <Button onClick={handleReset} data-testid="customer-lifecycle-filter-reset">重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<CustomerLifecycleTask>
        header={header}
        rowKey="task_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="鍔犺浇鐢熷懡鍛ㄦ湡浠诲姟澶辫触"
        emptyText="鏆傛棤鐢熷懡鍛ㄦ湡浠诲姟"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="customer.lifecycle.table.columns"
        enablePanels={false}
        dataTestId="customer-lifecycle-table"
      />
      <Drawer
        title={editingTask ? '缂栬緫鐢熷懡鍛ㄦ湡浠诲姟' : '鏂板缓鐢熷懡鍛ㄦ湡浠诲姟'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>鍙栨秷</Button>
            <Button type="primary" loading={saving} onClick={handleSave} data-testid="customer-lifecycle-form-save">
              淇濆瓨
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="stage"
            label="闃舵"
            rules={[{ required: true, message: '璇烽€夋嫨闃舵' }]}
          >
            <Select options={stageOptions} placeholder="选择阶段" data-testid="customer-lifecycle-form-stage" />
          </Form.Item>
          <Form.Item
            name="strategy"
            label="瑙﹁揪绛栫暐"
            rules={[{ required: true, message: '请输入触达策略' }]}
          >
            <Input placeholder="触达策略" data-testid="customer-lifecycle-form-strategy" />
          </Form.Item>
          <Form.Item name="trigger_rule" label="触发规则">
            <Input.TextArea rows={3} placeholder="例如：注册后24小时未下单" />
          </Form.Item>
          <Form.Item name="owner" label="负责人">
            <Input placeholder="负责人" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};


