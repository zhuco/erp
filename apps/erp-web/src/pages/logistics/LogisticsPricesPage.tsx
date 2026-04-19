import {
  Alert,
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
import type { LogisticsProduct } from '@/api/logisticsProducts';
import { listLogisticsProducts } from '@/api/logisticsProducts';
import type { LogisticsPrice, LogisticsPriceCreatePayload } from '@/api/logisticsPrices';
import {
  createLogisticsPrice,
  deleteLogisticsPrice,
  listLogisticsPrices,
  updateLogisticsPrice
} from '@/api/logisticsPrices';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  carrier_code?: string;
  product_id?: string;
  region_code?: string;
  weight_kg?: number;
};

type FormValues = {
  carrier_code: string;
  product_id?: string;
  region_code: string;
  weight_min_kg: number;
  weight_max_kg: number;
  price: number;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatPrice = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;

export const LogisticsPricesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('logistics_prices.write');
  const featureFlag = useFeatureFlag('directory.logistics_price');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingPrice, setEditingPrice] = useState<LogisticsPrice | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      carrier_code: filters.carrier_code?.trim() || undefined,
      product_id: filters.product_id?.trim() || undefined,
      region_code: filters.region_code?.trim() || undefined,
      weight_kg: filters.weight_kg ?? undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['logistics-prices', tenantId, query],
    queryFn: () => listLogisticsPrices(tenantId, query),
    enabled: featureFlag.enabled,
    placeholderData: (previousData) => previousData
  });

  const carriersQuery = useQuery({
    queryKey: ['carriers', tenantId, 'options'],
    queryFn: () => listCarriers(tenantId, { offset: 0, limit: 200 }),
    enabled: featureFlag.enabled
  });

  const productsQuery = useQuery({
    queryKey: ['logistics-products', tenantId, 'options'],
    queryFn: () =>
      listLogisticsProducts(tenantId, {
        offset: 0,
        limit: 500
      }),
    enabled: featureFlag.enabled
  });

  const carrierOptions = useMemo(
    () =>
      (carriersQuery.data?.items ?? []).map((carrier: Carrier) => ({
        label: `${carrier.name} (${carrier.carrier_code})`,
        value: carrier.carrier_code
      })),
    [carriersQuery.data?.items]
  );

  const productOptions = useMemo(
    () =>
      (productsQuery.data?.items ?? []).map((product: LogisticsProduct) => ({
        label: `${product.name} (${product.code})`,
        value: product.product_id
      })),
    [productsQuery.data?.items]
  );

  const deleteMutation = useMutation({
    mutationFn: (priceId: string) => deleteLogisticsPrice(tenantId, priceId),
    onMutate: (priceId) => setDeletingId(priceId),
    onSuccess: () => {
      message.success('物流价格已删除');
      queryClient.invalidateQueries({ queryKey: ['logistics-prices', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除物流价格失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<LogisticsPrice>>(
    () => [
      {
        title: '承运商',
        dataIndex: 'carrier_code'
      },
      {
        title: '物流产品',
        dataIndex: 'product_id',
        render: (_, record) => {
          const match = productsQuery.data?.items?.find(
            (item) => item.product_id === record.product_id
          );
          return match ? `${match.name} (${match.code})` : record.product_id ?? '-';
        }
      },
      {
        title: '区域编码',
        dataIndex: 'region_code'
      },
      {
        title: '重量区间(kg)',
        key: 'weight_range',
        render: (_, record) => `${record.weight_min_kg} - ${record.weight_max_kg}`
      },
      {
        title: '价格',
        dataIndex: 'price_cent',
        render: (value) => formatPrice(value)
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
                setEditingPrice(record);
                drawerForm.setFieldsValue({
                  carrier_code: record.carrier_code,
                  product_id: record.product_id ?? undefined,
                  region_code: record.region_code,
                  weight_min_kg: record.weight_min_kg,
                  weight_max_kg: record.weight_max_kg,
                  price: record.price_cent / 100
                });
                setDrawerOpen(true);
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该物流价格？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.price_id)}
              disabled={!canWrite}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deletingId === record.price_id}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, deletingId, drawerForm, productsQuery.data?.items]
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
    setEditingPrice(null);
    drawerForm.resetFields();
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: LogisticsPriceCreatePayload = {
        carrier_code: values.carrier_code,
        product_id: values.product_id || null,
        region_code: values.region_code.trim(),
        weight_min_kg: values.weight_min_kg,
        weight_max_kg: values.weight_max_kg,
        price_cent: Math.round(values.price * 100)
      };
      setSaving(true);
      if (editingPrice) {
        await updateLogisticsPrice(tenantId, editingPrice.price_id, payload);
        message.success('物流价格已更新');
      } else {
        await createLogisticsPrice(tenantId, payload);
        message.success('物流价格已创建');
      }
      setDrawerOpen(false);
      setEditingPrice(null);
      await queryClient.invalidateQueries({ queryKey: ['logistics-prices', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  if (!featureFlag.enabled) {
    return (
      <Alert
        message="物流价格功能已关闭"
        description={featureFlag.reason || '请在系统设置中启用该功能'}
        type="warning"
        showIcon
      />
    );
  }

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          物流价格
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
          新建物流价格
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="carrier_code" label="承运商">
                <Select allowClear options={carrierOptions} placeholder="选择承运商" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="product_id" label="物流产品">
                <Select allowClear options={productOptions} placeholder="选择物流产品" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="region_code" label="区域编码">
                <Input allowClear placeholder="例如：CN-310000" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="weight_kg" label="重量(kg)">
                <InputNumber min={0} style={{ width: '100%' }} />
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
      <UiModule03MainTableSection<LogisticsPrice>
        header={header}
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="物流价格加载失败"
        emptyText="暂无物流价格"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="logistics-prices.table.columns"
        rowKey="price_id"
        enablePanels={false}
      />
      <Drawer
        title={editingPrice ? '编辑物流价格' : '新建物流价格'}
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
            name="carrier_code"
            label="承运商"
            rules={[{ required: true, message: '请选择承运商' }]}
          >
            <Select options={carrierOptions} placeholder="选择承运商" />
          </Form.Item>
          <Form.Item name="product_id" label="物流产品">
            <Select options={productOptions} allowClear placeholder="选择物流产品" />
          </Form.Item>
          <Form.Item
            name="region_code"
            label="区域编码"
            rules={[{ required: true, message: '请输入区域编码' }]}
          >
            <Input placeholder="例如：CN-310000" />
          </Form.Item>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name="weight_min_kg"
                label="起始重量(kg)"
                rules={[{ required: true, message: '请输入起始重量' }]}
              >
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name="weight_max_kg"
                label="结束重量(kg)"
                rules={[{ required: true, message: '请输入结束重量' }]}
              >
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item
            name="price"
            label="价格(元)"
            rules={[{ required: true, message: '请输入价格' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
