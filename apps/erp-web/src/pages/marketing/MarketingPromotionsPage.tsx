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
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { MarketingPromotion, MarketingPromotionCreatePayload } from '@/api/marketingPromotions';
import {
  createMarketingPromotion,
  deleteMarketingPromotion,
  listMarketingPromotions,
  updateMarketingPromotion
} from '@/api/marketingPromotions';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  promo_type?: string;
};

type FormValues = {
  name: string;
  promo_type: string;
  status?: string;
  priority?: number;
  exclusive?: boolean;
  scope_type?: string;
  scope_json?: string;
  rule_json?: string;
  date_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const statusOptions = [
  { label: '草稿', value: 'DRAFT' },
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const promoTypeOptions = [
  { label: '满减', value: 'FULL_REDUCTION' },
  { label: '折扣', value: 'DISCOUNT' },
  { label: '第二件优惠', value: 'SECOND_ITEM' },
  { label: '自定义', value: 'CUSTOM' }
];

const scopeTypeOptions = [
  { label: 'SKU', value: 'SKU' },
  { label: '类目', value: 'CATEGORY' },
  { label: '品牌', value: 'BRAND' },
  { label: '渠道', value: 'CHANNEL' },
  { label: '全部', value: 'ALL' }
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

export const MarketingPromotionsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('marketing_promotions.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<MarketingPromotion | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status || undefined,
      promo_type: filters.promo_type || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['marketing-promotions', tenantId, query],
    queryFn: () => listMarketingPromotions(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: MarketingPromotionCreatePayload = {
        name: values.name.trim(),
        promo_type: values.promo_type,
        status: values.status,
        priority: values.priority,
        exclusive: values.exclusive,
        scope_type: values.scope_type,
        scope_json: parseJson(values.scope_json, {}),
        rule_json: parseJson(values.rule_json, {}),
        start_at: values.date_range?.[0]?.toISOString(),
        end_at: values.date_range?.[1]?.toISOString()
      };
      if (editing) {
        return updateMarketingPromotion(tenantId, editing.promotion_id, payload);
      }
      return createMarketingPromotion(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '促销活动已更新' : '促销活动已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['marketing-promotions', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存促销活动失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (promotionId: string) => deleteMarketingPromotion(tenantId, promotionId),
    onSuccess: async () => {
      message.success('促销活动已删除');
      await queryClient.invalidateQueries({ queryKey: ['marketing-promotions', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除促销活动失败');
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
      promo_type: 'FULL_REDUCTION',
      status: 'DRAFT',
      priority: 0,
      exclusive: false,
      scope_type: 'SKU',
      scope_json: '{}',
      rule_json: '{}'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: MarketingPromotion) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      name: record.name,
      promo_type: record.promo_type,
      status: record.status,
      priority: record.priority,
      exclusive: record.exclusive,
      scope_type: record.scope_type,
      scope_json: JSON.stringify(record.scope_json ?? {}, null, 2),
      rule_json: JSON.stringify(record.rule_json ?? {}, null, 2),
      date_range:
        record.start_at && record.end_at
          ? [dayjs(record.start_at), dayjs(record.end_at)]
          : undefined
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<MarketingPromotion>>(
    () => [
      {
        title: '活动名称',
        dataIndex: 'name'
      },
      {
        title: '类型',
        dataIndex: 'promo_type'
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
        title: '优先级',
        dataIndex: 'priority'
      },
      {
        title: '互斥',
        dataIndex: 'exclusive',
        render: (value) => (value ? '是' : '否')
      },
      {
        title: '生效期',
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
              title="确认删除该活动？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.promotion_id)}
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
          促销活动
        </Typography.Title>
        <Button type="primary" onClick={openCreate} disabled={!canWrite}>
          新建活动
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="活动名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="promo_type" label="类型">
                <Select allowClear options={promoTypeOptions} placeholder="全部" />
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
      <UiModule03MainTableSection<MarketingPromotion>
        header={header}
        rowKey="promotion_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isLoading || listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="促销活动加载失败"
        emptyText="暂无促销活动"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="marketing.promotions.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editing ? '编辑促销活动' : '新建促销活动'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={620}
        destroyOnHidden
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
          <Form.Item name="name" label="活动名称" rules={[{ required: true, message: '请输入活动名称' }]}>
            <Input placeholder="例如：双11满减" />
          </Form.Item>
          <Form.Item name="promo_type" label="活动类型" rules={[{ required: true, message: '请选择类型' }]}>
            <Select options={promoTypeOptions} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="priority" label="优先级">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="exclusive" label="互斥" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="scope_type" label="适用范围">
            <Select options={scopeTypeOptions} />
          </Form.Item>
          <Form.Item name="date_range" label="活动时间">
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="scope_json" label="范围JSON">
            <Input.TextArea rows={4} placeholder='例如：{"sku_ids":["xxx"]}' />
          </Form.Item>
          <Form.Item name="rule_json" label="规则JSON">
            <Input.TextArea rows={4} placeholder='例如：{"threshold":2,"discount":0.9}' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
