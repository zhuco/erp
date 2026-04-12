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

import type { MarketingGiftPolicy, MarketingGiftPolicyCreatePayload } from '@/api/marketingGifts';
import {
  createMarketingGiftPolicy,
  deleteMarketingGiftPolicy,
  listMarketingGiftPolicies,
  updateMarketingGiftPolicy
} from '@/api/marketingGifts';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
};

type FormValues = {
  name: string;
  status?: string;
  strategy?: string;
  probability?: number;
  scope_json?: string;
  gift_pool?: string;
  date_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const statusOptions = [
  { label: '草稿', value: 'DRAFT' },
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const strategyOptions = [
  { label: '随机', value: 'RANDOM' },
  { label: '固定', value: 'FIXED' },
  { label: '替换', value: 'REPLACEMENT' }
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

export const MarketingGiftPoliciesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('marketing_gifts.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<MarketingGiftPolicy | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['marketing-gifts', tenantId, query],
    queryFn: () => listMarketingGiftPolicies(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: MarketingGiftPolicyCreatePayload = {
        name: values.name.trim(),
        status: values.status,
        strategy: values.strategy,
        probability: values.probability,
        scope_json: parseJson(values.scope_json, {}),
        gift_pool: parseJson(values.gift_pool, []),
        start_at: values.date_range?.[0]?.toISOString(),
        end_at: values.date_range?.[1]?.toISOString()
      };
      if (editing) {
        return updateMarketingGiftPolicy(tenantId, editing.policy_id, payload);
      }
      return createMarketingGiftPolicy(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '赠品策略已更新' : '赠品策略已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['marketing-gifts', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存赠品策略失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (policyId: string) => deleteMarketingGiftPolicy(tenantId, policyId),
    onSuccess: async () => {
      message.success('赠品策略已删除');
      await queryClient.invalidateQueries({ queryKey: ['marketing-gifts', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除赠品策略失败');
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
      status: 'DRAFT',
      strategy: 'RANDOM',
      probability: 1,
      scope_json: '{}',
      gift_pool: '[]'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: MarketingGiftPolicy) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      name: record.name,
      status: record.status,
      strategy: record.strategy,
      probability: record.probability ?? undefined,
      scope_json: JSON.stringify(record.scope_json ?? {}, null, 2),
      gift_pool: JSON.stringify(record.gift_pool ?? [], null, 2),
      date_range:
        record.start_at && record.end_at
          ? [dayjs(record.start_at), dayjs(record.end_at)]
          : undefined
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<MarketingGiftPolicy>>(
    () => [
      {
        title: '策略名称',
        dataIndex: 'name'
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
        title: '策略类型',
        dataIndex: 'strategy'
      },
      {
        title: '概率',
        dataIndex: 'probability',
        render: (value) => (value != null ? value : '-')
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
              title="确认删除该策略？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.policy_id)}
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
          赠品策略
        </Typography.Title>
        <Button type="primary" onClick={openCreate} disabled={!canWrite}>
          新建策略
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="策略名称" />
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
      <UiModule03MainTableSection<MarketingGiftPolicy>
        header={header}
        rowKey="policy_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isLoading || listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="赠品策略加载失败"
        emptyText="暂无赠品策略"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="marketing.gifts.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editing ? '编辑赠品策略' : '新建赠品策略'}
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
          <Form.Item name="name" label="策略名称" rules={[{ required: true, message: '请输入策略名称' }]}>
            <Input placeholder="例如：随机赠品池" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="strategy" label="策略类型">
            <Select options={strategyOptions} />
          </Form.Item>
          <Form.Item name="probability" label="触发概率">
            <InputNumber min={0} max={1} step={0.01} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="date_range" label="生效期">
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="scope_json" label="范围JSON">
            <Input.TextArea rows={4} placeholder='例如：{"sku_ids":["xxx"]}' />
          </Form.Item>
          <Form.Item name="gift_pool" label="赠品池JSON">
            <Input.TextArea rows={4} placeholder='例如：[{"sku_id":"xxx","qty":1}]' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
