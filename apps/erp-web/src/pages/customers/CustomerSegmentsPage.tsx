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

import type { CustomerSegment, CustomerSegmentCreatePayload } from '@/api/customerSegments';
import {
  createCustomerSegment,
  deleteCustomerSegment,
  listCustomerSegments,
  updateCustomerSegment
} from '@/api/customerSegments';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  segment_type?: string;
};

type FormValues = {
  name: string;
  segment_type: string;
  rule_summary?: string;
  status?: string;
};

const statusOptions = [
  { label: '鍚敤', value: 'ACTIVE' },
  { label: '鍋滅敤', value: 'INACTIVE' }
];

const segmentTypeOptions = [
  { label: '动态分群', value: 'DYNAMIC' },
  { label: '静态分群', value: 'STATIC' }
];

const segmentTypeMap = new Map(
  segmentTypeOptions.map((option) => [option.value, option.label])
);

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const CustomerSegmentsPage = () => {
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
  const [editingSegment, setEditingSegment] = useState<CustomerSegment | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      segment_type: filters.segment_type || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['customer-segments', tenantId, query],
    queryFn: () => listCustomerSegments(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (segmentId: string) => deleteCustomerSegment(tenantId, segmentId),
    onMutate: (segmentId) => {
      setDeletingId(segmentId);
    },
    onSuccess: () => {
      message.success('客户分群已删除');
      queryClient.invalidateQueries({ queryKey: ['customer-segments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除客户分群失败');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<CustomerSegment>>(
    () => [
      {
        title: '分群鍚嶇О',
        dataIndex: 'name'
      },
      {
        title: '分群绫诲瀷',
        dataIndex: 'segment_type',
        render: (value: string) => segmentTypeMap.get(value) ?? value
      },
      {
        title: '规则姒傝',
        dataIndex: 'rule_summary',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '客户数',
        dataIndex: 'customer_count'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value?: string | null) => (
          <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value ?? '-'}</Tag>
        )
      },
      {
        title: '更新时间',
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
                setEditingSegment(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  name: record.name,
                  segment_type: record.segment_type,
                  rule_summary: record.rule_summary ?? undefined,
                  status: record.status ?? undefined
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除客户分群？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.segment_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.segment_id}
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
    setEditingSegment(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: CustomerSegmentCreatePayload = {
        name: values.name.trim(),
        segment_type: values.segment_type as CustomerSegmentCreatePayload['segment_type'],
        rule_summary: values.rule_summary?.trim() || undefined,
        status: values.status
      };
      setSaving(true);
      if (editingSegment) {
        await updateCustomerSegment(tenantId, editingSegment.segment_id, payload);
        message.success('客户分群已更新');
      } else {
        await createCustomerSegment(tenantId, payload);
        message.success('客户分群已创建');
      }
      setDrawerOpen(false);
      setEditingSegment(null);
      await queryClient.invalidateQueries({ queryKey: ['customer-segments', tenantId] });
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
        <Typography.Title level={3} style={{ margin: 0 }} data-testid="customer-segments-title">
          客户分群
        </Typography.Title>
        <Button
          type="primary"
          disabled={!canWrite}
          onClick={openCreateDrawer}
          data-testid="customer-segments-create"
        >
          新建分群
        </Button>
      </Space>
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSearch}
          data-testid="customer-segments-filter-form"
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="分群名称" data-testid="customer-segments-filter-keyword" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="segment_type" label="分群绫诲瀷">
                <Select allowClear options={segmentTypeOptions} placeholder="鍏ㄩ儴" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="鍏ㄩ儴" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit" data-testid="customer-segments-filter-submit">
                  鏌ヨ
                </Button>
                <Button onClick={handleReset} data-testid="customer-segments-filter-reset">重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<CustomerSegment>
        header={header}
        rowKey="segment_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="加载客户分群失败"
        emptyText="暂无客户分群"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="customer.segments.table.columns"
        enablePanels={false}
        dataTestId="customer-segments-table"
      />
      <Drawer
        title={editingSegment ? '编辑客户分群' : '鏂板缓客户分群'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave} data-testid="customer-segments-form-save">
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="name"
            label="分群鍚嶇О"
            rules={[{ required: true, message: '请输入分群名称' }]}
          >
            <Input placeholder="分群名称" data-testid="customer-segments-form-name" />
          </Form.Item>
          <Form.Item
            name="segment_type"
            label="分群绫诲瀷"
            rules={[{ required: true, message: '璇烽€夋嫨分群绫诲瀷' }]}
          >
            <Select options={segmentTypeOptions} placeholder="选择分群类型" data-testid="customer-segments-form-type" />
          </Form.Item>
          <Form.Item name="rule_summary" label="规则姒傝">
            <Input.TextArea rows={3} placeholder="濡傦細杩?0澶╂秷璐归噾棰?5000" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};

