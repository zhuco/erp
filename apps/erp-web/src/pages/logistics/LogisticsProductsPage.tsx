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

import type { Carrier } from '@/api/carriers';
import { listCarriers } from '@/api/carriers';
import type {
  LogisticsProduct,
  LogisticsProductCreatePayload
} from '@/api/logisticsProducts';
import {
  createLogisticsProduct,
  deleteLogisticsProduct,
  listLogisticsProducts,
  updateLogisticsProduct
} from '@/api/logisticsProducts';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  carrier_code?: string;
  status?: string;
};

type FormValues = {
  carrier_code: string;
  code: string;
  name: string;
  status?: string;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const LogisticsProductsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('logistics_products.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingProduct, setEditingProduct] = useState<LogisticsProduct | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      carrier_code: filters.carrier_code?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['logistics-products', tenantId, query],
    queryFn: () => listLogisticsProducts(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const carriersQuery = useQuery({
    queryKey: ['carriers', tenantId, 'options'],
    queryFn: () => listCarriers(tenantId, { offset: 0, limit: 200 })
  });

  const carrierOptions = useMemo(
    () =>
      (carriersQuery.data?.items ?? []).map((carrier: Carrier) => ({
        label: `${carrier.name} (${carrier.carrier_code})`,
        value: carrier.carrier_code
      })),
    [carriersQuery.data?.items]
  );

  const deleteMutation = useMutation({
    mutationFn: (productId: string) => deleteLogisticsProduct(tenantId, productId),
    onMutate: (productId) => setDeletingId(productId),
    onSuccess: () => {
      message.success('物流产品已删除');
      queryClient.invalidateQueries({ queryKey: ['logistics-products', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除物流产品失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<LogisticsProduct>>(
    () => [
      {
        title: '承运商',
        dataIndex: 'carrier_code',
        render: (value) => value ?? '-'
      },
      {
        title: '产品编码',
        dataIndex: 'code'
      },
      {
        title: '产品名称',
        dataIndex: 'name'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) => (value === 'DISABLED' ? <Tag color="red">停用</Tag> : '启用')
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
                setEditingProduct(record);
                drawerForm.setFieldsValue({
                  carrier_code: record.carrier_code,
                  code: record.code,
                  name: record.name,
                  status: record.status
                });
                setDrawerOpen(true);
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该物流产品？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.product_id)}
              disabled={!canWrite}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deletingId === record.product_id}
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
    setEditingProduct(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ status: 'ACTIVE' });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: LogisticsProductCreatePayload = {
        carrier_code: values.carrier_code,
        code: values.code.trim(),
        name: values.name.trim(),
        status: values.status?.trim() || undefined
      };
      setSaving(true);
      if (editingProduct) {
        await updateLogisticsProduct(tenantId, editingProduct.product_id, payload);
        message.success('物流产品已更新');
      } else {
        await createLogisticsProduct(tenantId, payload);
        message.success('物流产品已创建');
      }
      setDrawerOpen(false);
      setEditingProduct(null);
      await queryClient.invalidateQueries({ queryKey: ['logistics-products', tenantId] });
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
          物流产品
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
          新建物流产品
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="产品名称/编码" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="carrier_code" label="承运商">
                <Select allowClear options={carrierOptions} placeholder="选择承运商" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="全部状态" />
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
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<LogisticsProduct>
        header={header}
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="物流产品加载失败"
        emptyText="暂无物流产品"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="logistics-products.table.columns"
        rowKey="product_id"
        enablePanels={false}
      />
      <Drawer
        title={editingProduct ? '编辑物流产品' : '新建物流产品'}
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
            name="carrier_code"
            label="承运商"
            rules={[{ required: true, message: '请选择承运商' }]}
          >
            <Select options={carrierOptions} placeholder="选择承运商" />
          </Form.Item>
          <Form.Item
            name="code"
            label="产品编码"
            rules={[{ required: true, message: '请输入产品编码' }]}
          >
            <Input placeholder="例如：SF-EXPRESS" />
          </Form.Item>
          <Form.Item
            name="name"
            label="产品名称"
            rules={[{ required: true, message: '请输入产品名称' }]}
          >
            <Input placeholder="例如：顺丰标快" />
          </Form.Item>
          <Form.Item name="status" label="状态" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
