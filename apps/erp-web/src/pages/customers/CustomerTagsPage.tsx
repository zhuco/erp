import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  Popconfirm,
  Row,
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

import type { CustomerTag, CustomerTagCreatePayload } from '@/api/customerTags';
import {
  createCustomerTag,
  deleteCustomerTag,
  listCustomerTags,
  updateCustomerTag
} from '@/api/customerTags';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
};

type FormValues = {
  name: string;
  color?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const CustomerTagsPage = () => {
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
  const [editingTag, setEditingTag] = useState<CustomerTag | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['customer-tags', tenantId, query],
    queryFn: () => listCustomerTags(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (tagId: string) => deleteCustomerTag(tenantId, tagId),
    onMutate: (tagId) => {
      setDeletingId(tagId);
    },
    onSuccess: () => {
      message.success('客户标签已删除');
      queryClient.invalidateQueries({ queryKey: ['customer-tags', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除客户标签失败');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<CustomerTag>>(
    () => [
      {
        title: '标签',
        dataIndex: 'name'
      },
      {
        title: '颜色',
        dataIndex: 'color',
        render: (value?: string | null) =>
          value ? <Tag color={value}>{value}</Tag> : '-'
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
                setEditingTag(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  name: record.name,
                  color: record.color ?? undefined
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除客户标签？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.tag_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.tag_id}
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
    setEditingTag(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: CustomerTagCreatePayload = {
        name: values.name.trim(),
        color: values.color?.trim() || undefined
      };
      setSaving(true);
      if (editingTag) {
        await updateCustomerTag(tenantId, editingTag.tag_id, payload);
        message.success('客户标签已更新');
      } else {
        await createCustomerTag(tenantId, payload);
        message.success('客户标签已创建');
      }
      setDrawerOpen(false);
      setEditingTag(null);
      await queryClient.invalidateQueries({ queryKey: ['customer-tags', tenantId] });
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
          客户标签
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
          新建标签
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="标签名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={16} style={{ display: 'flex', alignItems: 'end' }}>
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

  return (
    <>
      <UiModule03MainTableSection<CustomerTag>
        header={header}
        rowKey="tag_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="加载客户标签失败"
        emptyText="暂无客户标签"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="customer.tags.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editingTag ? '编辑客户标签' : '新建客户标签'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={420}
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
            label="标签名称"
            rules={[{ required: true, message: '请输入标签名称' }]}
          >
            <Input placeholder="标签名称" />
          </Form.Item>
          <Form.Item name="color" label="颜色">
            <Input placeholder="颜色 / Hex" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
