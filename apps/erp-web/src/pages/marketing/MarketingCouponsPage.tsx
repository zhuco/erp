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
  Select,
  Space,
  Row,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { MarketingCoupon, MarketingCouponCreatePayload } from '@/api/marketingCoupons';
import {
  createMarketingCoupon,
  deleteMarketingCoupon,
  listMarketingCoupons,
  updateMarketingCoupon
} from '@/api/marketingCoupons';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  discount_type?: string;
};

type FormValues = {
  name: string;
  code_prefix?: string;
  discount_type: string;
  discount_value: number;
  max_usage?: number;
  per_user_limit?: number;
  status?: string;
  rules_json?: string;
  date_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const statusOptions = [
  { label: '草稿', value: 'DRAFT' },
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const discountTypeOptions = [
  { label: '满减', value: 'FULL_REDUCTION' },
  { label: '折扣', value: 'PERCENT' },
  { label: '固定金额', value: 'FIXED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatRange = (start?: string | null, end?: string | null) => {
  if (!start && !end) {
    return '-';
  }
  return `${start ? dayjs(start).format('YYYY-MM-DD') : '不限'} ~ ${
    end ? dayjs(end).format('YYYY-MM-DD') : '不限'
  }`;
};

const parseJson = (value?: string, fallback: unknown = {}) => {
  if (!value) {
    return fallback;
  }
  try {
    return JSON.parse(value);
  } catch (error) {
    throw new Error('JSON格式不正确');
  }
};

export const MarketingCouponsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('marketing_coupons.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<MarketingCoupon | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status || undefined,
      discount_type: filters.discount_type || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['marketing-coupons', tenantId, query],
    queryFn: () => listMarketingCoupons(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: MarketingCouponCreatePayload = {
        name: values.name.trim(),
        code_prefix: values.code_prefix?.trim(),
        discount_type: values.discount_type,
        discount_value: values.discount_value,
        max_usage: values.max_usage,
        per_user_limit: values.per_user_limit,
        status: values.status,
        rules_json: parseJson(values.rules_json, {}),
        start_at: values.date_range?.[0]?.toISOString(),
        end_at: values.date_range?.[1]?.toISOString()
      };
      if (editing) {
        return updateMarketingCoupon(tenantId, editing.coupon_id, payload);
      }
      return createMarketingCoupon(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '优惠券已更新' : '优惠券已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['marketing-coupons', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存优惠券失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (couponId: string) => deleteMarketingCoupon(tenantId, couponId),
    onSuccess: async () => {
      message.success('优惠券已删除');
      await queryClient.invalidateQueries({ queryKey: ['marketing-coupons', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除优惠券失败');
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

  const openCreate = () => {
    setEditing(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      discount_type: 'FULL_REDUCTION',
      status: 'DRAFT',
      rules_json: '{}'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: MarketingCoupon) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      name: record.name,
      code_prefix: record.code_prefix ?? undefined,
      discount_type: record.discount_type,
      discount_value: record.discount_value,
      max_usage: record.max_usage,
      per_user_limit: record.per_user_limit,
      status: record.status,
      rules_json: JSON.stringify(record.rules_json ?? {}, null, 2),
      date_range:
        record.start_at && record.end_at
          ? [dayjs(record.start_at), dayjs(record.end_at)]
          : undefined
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<MarketingCoupon>>(
    () => [
      {
        title: '优惠券名称',
        dataIndex: 'name'
      },
      {
        title: '类型',
        dataIndex: 'discount_type'
      },
      {
        title: '面值/折扣',
        dataIndex: 'discount_value'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'ACTIVE' ? <Tag color="green">启用</Tag> : value === 'DISABLED' ? (
            <Tag color="red">停用</Tag>
          ) : (
            <Tag color="default">草稿</Tag>
          )
      },
      {
        title: '使用次数',
        render: (_, record) => `${record.max_usage}/${record.per_user_limit}`
      },
      {
        title: '有效期',
        render: (_, record) => formatRange(record.start_at, record.end_at)
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => openEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该优惠券？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.coupon_id)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          优惠券
        </Typography.Title>
        <Button type="primary" onClick={openCreate} disabled={!canWrite}>
          新建优惠券
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="优惠券名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="discount_type" label="类型">
                <Select allowClear options={discountTypeOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
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
      <UiModule03MainTableSection<MarketingCoupon>
        header={header}
        rowKey="coupon_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isLoading || listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="优惠券加载失败"
        emptyText="暂无优惠券"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="marketing.coupons.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editing ? '编辑优惠券' : '新建优惠券'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={620}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saveMutation.isPending} onClick={() => saveMutation.mutate()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item name="name" label="优惠券名称" rules={[{ required: true, message: '请输入优惠券名称' }]}>
            <Input placeholder="例如：满100减10" />
          </Form.Item>
          <Form.Item name="code_prefix" label="券码前缀">
            <Input placeholder="例如：SALE2026" />
          </Form.Item>
          <Form.Item name="discount_type" label="优惠类型" rules={[{ required: true, message: '请选择优惠类型' }]}>
            <Select options={discountTypeOptions} />
          </Form.Item>
          <Form.Item name="discount_value" label="优惠值" rules={[{ required: true, message: '请输入优惠值' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="max_usage" label="总使用次数">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="per_user_limit" label="单人次数">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="date_range" label="有效期">
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="rules_json" label="规则JSON">
            <Input.TextArea rows={4} placeholder='例如：{"min_amount":100}' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
