import {
  Button,
  Card,
  Col,
  DatePicker,
  Drawer,
  Form,
  Input,
  InputNumber,
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
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  CustomerServiceTicket,
  CustomerServiceTicketCreatePayload
} from '@/api/customerServiceTickets';
import {
  createCustomerServiceTicket,
  deleteCustomerServiceTicket,
  listCustomerServiceTickets,
  updateCustomerServiceTicket
} from '@/api/customerServiceTickets';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  priority?: string;
  channel?: string;
  category?: string;
  handler?: string;
  updated_range?: [Dayjs, Dayjs];
};

type FormValues = {
  customer_name: string;
  customer_id?: string;
  contact_phone?: string;
  channel?: string;
  category?: string;
  priority?: string;
  status?: string;
  related_order_id?: string;
  satisfaction_score?: number;
  handler?: string;
  description?: string;
};

const statusOptions = [
  { label: '待处理', value: 'OPEN' },
  { label: '处理中', value: 'IN_PROGRESS' },
  { label: '已解决', value: 'RESOLVED' },
  { label: '已关闭', value: 'CLOSED' }
];

const statusColorMap: Record<string, string> = {
  OPEN: 'orange',
  IN_PROGRESS: 'blue',
  RESOLVED: 'green',
  CLOSED: 'default'
};

const priorityOptions = [
  { label: '高', value: 'HIGH' },
  { label: '中', value: 'MEDIUM' },
  { label: '低', value: 'LOW' }
];

const priorityColorMap: Record<string, string> = {
  HIGH: 'red',
  MEDIUM: 'gold',
  LOW: 'default'
};

const channelOptions = [
  { label: '电话', value: '电话' },
  { label: '在线', value: '在线' },
  { label: '门店', value: '门店' }
];

const categoryOptions = [
  { label: '咨询', value: '咨询' },
  { label: '投诉', value: '投诉' },
  { label: '售后', value: '售后' }
];

const statusLabelMap = new Map(statusOptions.map((option) => [option.value, option.label]));
const priorityLabelMap = new Map(priorityOptions.map((option) => [option.value, option.label]));

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const CustomerServiceTicketsPage = () => {
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
  const [editingTicket, setEditingTicket] = useState<CustomerServiceTicket | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      priority: filters.priority?.trim() || undefined,
      channel: filters.channel?.trim() || undefined,
      category: filters.category?.trim() || undefined,
      handler: filters.handler?.trim() || undefined,
      updated_from: filters.updated_range?.[0]?.toISOString(),
      updated_to: filters.updated_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['customer-service-tickets', tenantId, query],
    queryFn: () => listCustomerServiceTickets(tenantId, query),
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (ticketId: string) => deleteCustomerServiceTicket(tenantId, ticketId),
    onMutate: (ticketId) => {
      setDeletingId(ticketId);
    },
    onSuccess: () => {
      message.success('客服工单已删除');
      queryClient.invalidateQueries({ queryKey: ['customer-service-tickets', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除客服工单失败');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<CustomerServiceTicket>>(
    () => [
      {
        title: '工单号',
        dataIndex: 'ticket_id'
      },
      {
        title: '客户',
        dataIndex: 'customer_name'
      },
      {
        title: '渠道',
        dataIndex: 'channel',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '联系方式',
        dataIndex: 'contact_phone',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '类型',
        dataIndex: 'category',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '优先级',
        dataIndex: 'priority',
        render: (value?: string | null) => {
          if (!value) {
            return '-';
          }
          return (
            <Tag color={priorityColorMap[value] ?? 'default'}>
              {priorityLabelMap.get(value) ?? value}
            </Tag>
          );
        }
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value?: string | null) => {
          if (!value) {
            return '-';
          }
          return (
            <Tag color={statusColorMap[value] ?? 'default'}>
              {statusLabelMap.get(value) ?? value}
            </Tag>
          );
        }
      },
      {
        title: '满意度',
        dataIndex: 'satisfaction_score',
        render: (value?: number | null) => (value == null ? '-' : `${value}分`)
      },
      {
        title: '关联订单',
        dataIndex: 'related_order_id',
        render: (value?: string | null) =>
          value ? <Link to={`/orders/${value}`}>{value}</Link> : '-'
      },
      {
        title: '负责人',
        dataIndex: 'handler',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditingTicket(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  customer_name: record.customer_name,
                  customer_id: record.customer_id,
                  contact_phone: record.contact_phone ?? undefined,
                  channel: record.channel ?? undefined,
                  category: record.category ?? undefined,
                  priority: record.priority ?? undefined,
                  status: record.status ?? undefined,
                  related_order_id: record.related_order_id ?? undefined,
                  satisfaction_score: record.satisfaction_score ?? undefined,
                  handler: record.handler ?? undefined,
                  description: record.description ?? undefined
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除客服工单？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.ticket_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.ticket_id}
              >
                删除
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
    setEditingTicket(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: CustomerServiceTicketCreatePayload = {
        customer_id: values.customer_id?.trim() || undefined,
        customer_name: values.customer_name.trim(),
        contact_phone: values.contact_phone?.trim() || undefined,
        channel: values.channel,
        category: values.category,
        priority: values.priority as CustomerServiceTicketCreatePayload['priority'],
        status: values.status as CustomerServiceTicketCreatePayload['status'],
        related_order_id: values.related_order_id?.trim() || undefined,
        satisfaction_score: values.satisfaction_score ?? null,
        handler: values.handler?.trim() || undefined,
        description: values.description?.trim() || undefined
      };
      setSaving(true);
      if (editingTicket) {
        await updateCustomerServiceTicket(tenantId, editingTicket.ticket_id, payload);
        message.success('客服工单已更新');
      } else {
        await createCustomerServiceTicket(tenantId, payload);
        message.success('客服工单已创建');
      }
      setDrawerOpen(false);
      setEditingTicket(null);
      await queryClient.invalidateQueries({
        queryKey: ['customer-service-tickets', tenantId]
      });
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
        <Typography.Title level={3} style={{ margin: 0 }}>
          <span data-testid="customer-service-title">客服工单</span>
        </Typography.Title>
        <Button
          type="primary"
          disabled={!canWrite}
          onClick={openCreateDrawer}
          data-testid="customer-service-create"
        >
          新建工单
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input
                  allowClear
                  placeholder="客户/工单号/订单号"
                  data-testid="customer-service-filter-keyword"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="channel" label="渠道">
                <Select
                  allowClear
                  options={channelOptions}
                  placeholder="全部"
                  data-testid="customer-service-filter-channel"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="category" label="类型">
                <Select
                  allowClear
                  options={categoryOptions}
                  placeholder="全部"
                  data-testid="customer-service-filter-category"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="priority" label="优先级">
                <Select
                  allowClear
                  options={priorityOptions}
                  placeholder="全部"
                  data-testid="customer-service-filter-priority"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select
                  allowClear
                  options={statusOptions}
                  placeholder="全部"
                  data-testid="customer-service-filter-status"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="handler" label="负责人">
                <Input
                  allowClear
                  placeholder="输入负责人"
                  data-testid="customer-service-filter-handler"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="updated_range" label="更新时间">
                <DatePicker.RangePicker
                  allowClear
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['开始时间', '结束时间']}
                  data-testid="customer-service-filter-updated"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button
                  type="primary"
                  htmlType="submit"
                  data-testid="customer-service-filter-submit"
                >
                  查询
                </Button>
                <Button
                  onClick={handleReset}
                  data-testid="customer-service-filter-reset"
                >
                  重置
                </Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <div data-testid="customer-service-table">
        <UiModule03MainTableSection<CustomerServiceTicket>
          header={header}
          rowKey="ticket_id"
          columns={columns}
          data={data?.items ?? []}
          loading={isLoading || isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="加载客服工单失败"
          emptyText="暂无客服工单"
          total={data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="customer.serviceTickets.table.columns"
          enablePanels={false}
        />
      </div>
      <Drawer
        title={editingTicket ? '编辑客服工单' : '新建客服工单'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button
              type="primary"
              loading={saving}
              onClick={handleSave}
              data-testid="customer-service-form-save"
            >
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="customer_name"
            label="客户名称"
            rules={[{ required: true, message: '请输入客户名称' }]}
          >
            <Input placeholder="客户名称" data-testid="customer-service-form-name" />
          </Form.Item>
          <Form.Item name="customer_id" label="客户ID">
            <Input placeholder="客户ID（可选）" data-testid="customer-service-form-id" />
          </Form.Item>
          <Form.Item name="contact_phone" label="联系方式">
            <Input placeholder="联系电话" data-testid="customer-service-form-phone" />
          </Form.Item>
          <Form.Item name="channel" label="渠道">
            <Select
              allowClear
              options={channelOptions}
              placeholder="选择渠道"
              data-testid="customer-service-form-channel"
            />
          </Form.Item>
          <Form.Item name="category" label="类型">
            <Select
              allowClear
              options={categoryOptions}
              placeholder="选择类型"
              data-testid="customer-service-form-category"
            />
          </Form.Item>
          <Form.Item name="priority" label="优先级">
            <Select
              allowClear
              options={priorityOptions}
              placeholder="选择优先级"
              data-testid="customer-service-form-priority"
            />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select
              allowClear
              options={statusOptions}
              placeholder="选择状态"
              data-testid="customer-service-form-status"
            />
          </Form.Item>
          <Form.Item name="related_order_id" label="关联订单">
            <Input placeholder="订单号" data-testid="customer-service-form-order" />
          </Form.Item>
          <Form.Item name="satisfaction_score" label="满意度">
            <InputNumber
              min={1}
              max={5}
              style={{ width: '100%' }}
              placeholder="1-5 分"
              data-testid="customer-service-form-score"
            />
          </Form.Item>
          <Form.Item name="handler" label="负责人">
            <Input placeholder="负责人" data-testid="customer-service-form-handler" />
          </Form.Item>
          <Form.Item name="description" label="问题描述">
            <Input.TextArea
              rows={3}
              placeholder="问题描述"
              data-testid="customer-service-form-description"
            />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
