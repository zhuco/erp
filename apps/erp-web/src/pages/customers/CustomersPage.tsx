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
import { useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Customer, CustomerCreatePayload } from '@/api/customers';
import { createCustomer, deleteCustomer, listCustomers, updateCustomer } from '@/api/customers';
import { listCustomerCategories } from '@/api/customerCategories';
import { listCustomerTags } from '@/api/customerTags';
import { listMemberLevels } from '@/api/memberLevels';
import { UiModule03MainTableSection, UiModule03TableFooter } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  category_id?: string;
  member_level_id?: string;
  tag_id?: string;
};

type FormValues = {
  name: string;
  phone?: string;
  address?: string;
  note?: string;
  status?: string;
  category_id?: string;
  member_level_id?: string;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'INACTIVE' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const CustomersPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const canWrite = useCan('customers.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const tableContainerRef = useRef<HTMLDivElement | null>(null);
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingCustomer, setEditingCustomer] = useState<Customer | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      category_id: filters.category_id || undefined,
      member_level_id: filters.member_level_id || undefined,
      tag_id: filters.tag_id || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['customers', tenantId, query],
    queryFn: () => listCustomers(tenantId, query),
    keepPreviousData: true
  });

  const { data: categoryData } = useQuery({
    queryKey: ['customer-categories', tenantId],
    queryFn: () =>
      listCustomerCategories(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const { data: levelData } = useQuery({
    queryKey: ['member-levels', tenantId],
    queryFn: () =>
      listMemberLevels(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const { data: tagData } = useQuery({
    queryKey: ['customer-tags', tenantId],
    queryFn: () =>
      listCustomerTags(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const categoryOptions = useMemo(
    () =>
      (categoryData?.items ?? []).map((item) => ({
        label: item.name,
        value: item.category_id
      })),
    [categoryData]
  );

  const levelOptions = useMemo(
    () =>
      (levelData?.items ?? []).map((item) => ({
        label: item.name,
        value: item.level_id
      })),
    [levelData]
  );

  const tagOptions = useMemo(
    () =>
      (tagData?.items ?? []).map((item) => ({
        label: item.name,
        value: item.tag_id
      })),
    [tagData]
  );

  const deleteMutation = useMutation({
    mutationFn: (customerId: string) => deleteCustomer(tenantId, customerId),
    onMutate: (customerId) => {
      setDeletingId(customerId);
    },
    onSuccess: () => {
      message.success('客户已删除');
      queryClient.invalidateQueries({ queryKey: ['customers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除客户失败');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<Customer>>(
    () => [
      {
        title: '客户',
        dataIndex: 'name',
        render: (_, record) => (
          <Button type="link" onClick={() => navigate(`/customers/${record.customer_id}`)}>
            {record.name}
          </Button>
        )
      },
      {
        title: '电话',
        dataIndex: 'phone'
      },
      {
        title: '分类',
        dataIndex: 'category_name'
      },
      {
        title: '会员等级',
        dataIndex: 'member_level_name'
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
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              onClick={() => navigate(`/customers/${record.customer_id}`)}
            >
              详情
            </Button>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditingCustomer(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  name: record.name,
                  phone: record.phone ?? undefined,
                  address: record.address ?? undefined,
                  note: record.note ?? undefined,
                  status: record.status ?? undefined,
                  category_id: record.category_id ?? undefined,
                  member_level_id: record.member_level_id ?? undefined
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除客户？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.customer_id)}
            >
              <Button
                size="small"
                danger
                loading={deleteMutation.isPending && deletingId === record.customer_id}
                disabled={!canWrite}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, deletingId, drawerForm, navigate]
  );

  const totalCount = data?.total ?? 0;
  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${totalCount}`,
    [columns.length, totalCount]
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
    setEditingCustomer(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: CustomerCreatePayload = {
        name: values.name.trim(),
        phone: values.phone?.trim() || undefined,
        address: values.address?.trim() || undefined,
        note: values.note?.trim() || undefined,
        status: values.status,
        category_id: values.category_id,
        member_level_id: values.member_level_id
      };
      setSaving(true);
      if (editingCustomer) {
        await updateCustomer(tenantId, editingCustomer.customer_id, payload);
        message.success('客户已更新');
      } else {
        await createCustomer(tenantId, payload);
        message.success('客户已创建');
      }
      setDrawerOpen(false);
      setEditingCustomer(null);
      await queryClient.invalidateQueries({ queryKey: ['customers', tenantId] });
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
          客户档案
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
          新建客户
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="客户姓名/电话" />
              </Form.Item>
            </Col>
            <Col xs={24} md={5}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={5}>
              <Form.Item name="category_id" label="分类">
                <Select allowClear options={categoryOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={5}>
              <Form.Item name="member_level_id" label="会员等级">
                <Select allowClear options={levelOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={3}>
              <Form.Item name="tag_id" label="标签">
                <Select allowClear options={tagOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={24} style={{ display: 'flex', alignItems: 'end' }}>
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
    </Space>
  );

  const tableFooter = (
    <UiModule03TableFooter
      tableContainerRef={tableContainerRef}
      syncKey={tableFooterSyncKey}
      summary={`共 ${totalCount} 条`}
      total={totalCount}
      pagination={pagination}
      onPaginationChange={setPagination}
    />
  );

  return (
    <>
      <UiModule03MainTableSection<Customer>
        header={header}
        tableContainerRef={tableContainerRef}
        rowKey="customer_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="加载客户失败"
        emptyText="暂无客户"
        total={totalCount}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="customers.table.columns"
        enablePanels={false}
        dataTestId="customers-data-grid"
        tableFooter={tableFooter}
      />
      <Drawer
        title={editingCustomer ? '编辑客户' : '新建客户'}
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
            label="客户姓名"
            rules={[{ required: true, message: '请输入客户姓名' }]}
          >
            <Input placeholder="客户姓名" />
          </Form.Item>
          <Form.Item name="phone" label="电话">
            <Input placeholder="手机号" />
          </Form.Item>
          <Form.Item name="address" label="地址">
            <Input placeholder="收货地址" />
          </Form.Item>
          <Form.Item name="note" label="备注">
            <Input.TextArea rows={3} placeholder="客户备注" />
          </Form.Item>
          <Form.Item name="category_id" label="客户分类">
            <Select allowClear options={categoryOptions} placeholder="选择分类" />
          </Form.Item>
          <Form.Item name="member_level_id" label="会员等级">
            <Select allowClear options={levelOptions} placeholder="选择等级" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
