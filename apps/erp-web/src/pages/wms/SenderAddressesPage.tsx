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
  Switch,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  SenderAddress,
  SenderAddressCreatePayload,
  SenderAddressListQuery,
  SenderAddressUpdatePayload
} from '@/api/senderAddresses';
import {
  createSenderAddress,
  deleteSenderAddress,
  listSenderAddresses,
  updateSenderAddress
} from '@/api/senderAddresses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
};

type FormValues = {
  name: string;
  phone: string;
  province: string;
  city: string;
  district: string;
  address: string;
  is_default?: boolean;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const SenderAddressesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingSender, setEditingSender] = useState<SenderAddress | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo<SenderAddressListQuery>(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sender-addresses', tenantId, query],
    queryFn: () => listSenderAddresses(tenantId, query),
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (senderAddressId: string) => deleteSenderAddress(tenantId, senderAddressId),
    onMutate: (senderAddressId) => setDeletingId(senderAddressId),
    onSuccess: () => {
      message.success('寄方地址已删除');
      queryClient.invalidateQueries({ queryKey: ['sender-addresses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除寄方地址失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<SenderAddress>>(
    () => [
      { title: '姓名', dataIndex: 'name' },
      { title: '电话', dataIndex: 'phone' },
      {
        title: '地址',
        render: (_, record) =>
          `${record.province}${record.city}${record.district}${record.address}`
      },
      {
        title: '默认',
        dataIndex: 'is_default',
        render: (value) => (value ? '是' : '否')
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
                setEditingSender(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  name: record.name,
                  phone: record.phone,
                  province: record.province,
                  city: record.city,
                  district: record.district,
                  address: record.address,
                  is_default: record.is_default
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该寄方地址？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.sender_address_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.sender_address_id}
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
    filterForm.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingSender(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ is_default: false });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: SenderAddressCreatePayload = {
        name: values.name.trim(),
        phone: values.phone.trim(),
        province: values.province.trim(),
        city: values.city.trim(),
        district: values.district.trim(),
        address: values.address.trim(),
        is_default: values.is_default ?? false
      };
      setSaving(true);
      if (editingSender) {
        const updatePayload: SenderAddressUpdatePayload = payload;
        await updateSenderAddress(tenantId, editingSender.sender_address_id, updatePayload);
        message.success('寄方地址已更新');
      } else {
        await createSenderAddress(tenantId, payload);
        message.success('寄方地址已创建');
      }
      setDrawerOpen(false);
      setEditingSender(null);
      await queryClient.invalidateQueries({ queryKey: ['sender-addresses', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        寄方目录
      </Typography.Title>
      <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
        新建寄方
      </Button>
    </Space>
  );

  const header = (
    <Card>
      <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={8}>
            <Form.Item name="keyword" label="关键词">
              <Input allowClear placeholder="姓名/电话/地址" />
            </Form.Item>
          </Col>
          <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
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

  return (
    <>
      <UiModule03MainTableSection<SenderAddress>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="sender_address_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="寄方地址加载失败"
        emptyText="暂无寄方地址"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sender-addresses.table.columns"
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
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item name="name" label="姓名" rules={[{ required: true, message: '请输入姓名' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="phone" label="电话" rules={[{ required: true, message: '请输入电话' }]}>
            <Input />
          </Form.Item>
          <Form.Item
            name="province"
            label="省份"
            rules={[{ required: true, message: '请输入省份' }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="city" label="城市" rules={[{ required: true, message: '请输入城市' }]}>
            <Input />
          </Form.Item>
          <Form.Item
            name="district"
            label="区县"
            rules={[{ required: true, message: '请输入区县' }]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="address"
            label="详细地址"
            rules={[{ required: true, message: '请输入详细地址' }]}
          >
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
