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

import type { Channel } from '@/api/channels';
import { listChannels } from '@/api/channels';
import type { MarketingCost, MarketingCostCreatePayload } from '@/api/marketingCosts';
import {
  createMarketingCost,
  deleteMarketingCost,
  listMarketingCosts,
  updateMarketingCost
} from '@/api/marketingCosts';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  channel_id?: string;
};

type FormValues = {
  name: string;
  channel_id?: string;
  amount_cent: number;
  currency?: string;
  allocation_rule?: string;
  period_range?: [dayjs.Dayjs, dayjs.Dayjs];
  status?: string;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
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

export const MarketingCostsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('marketing_costs.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<MarketingCost | null>(null);

  const channelsQuery = useQuery({
    queryKey: ['channels', tenantId],
    queryFn: () => listChannels(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const channelOptions = useMemo(
    () =>
      channelsQuery.data?.items.map((channel: Channel) => ({
        label: channel.name,
        value: channel.channel_id
      })) ?? [],
    [channelsQuery.data?.items]
  );

  const channelMap = useMemo(
    () =>
      new Map(
        channelsQuery.data?.items.map((channel) => [channel.channel_id, channel.name]) ?? []
      ),
    [channelsQuery.data?.items]
  );

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status || undefined,
      channel_id: filters.channel_id || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['marketing-costs', tenantId, query],
    queryFn: () => listMarketingCosts(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: MarketingCostCreatePayload = {
        name: values.name.trim(),
        channel_id: values.channel_id,
        amount_cent: values.amount_cent,
        currency: values.currency?.trim(),
        allocation_rule: parseJson(values.allocation_rule, {}),
        period_start: values.period_range?.[0]?.format('YYYY-MM-DD'),
        period_end: values.period_range?.[1]?.format('YYYY-MM-DD'),
        status: values.status
      };
      if (editing) {
        return updateMarketingCost(tenantId, editing.cost_id, payload);
      }
      return createMarketingCost(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '促销费用已更新' : '促销费用已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['marketing-costs', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存促销费用失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (costId: string) => deleteMarketingCost(tenantId, costId),
    onSuccess: async () => {
      message.success('促销费用已删除');
      await queryClient.invalidateQueries({ queryKey: ['marketing-costs', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除促销费用失败');
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
      status: 'ACTIVE',
      currency: 'CNY',
      allocation_rule: '{}'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: MarketingCost) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      name: record.name,
      channel_id: record.channel_id ?? undefined,
      amount_cent: record.amount_cent,
      currency: record.currency,
      allocation_rule: JSON.stringify(record.allocation_rule ?? {}, null, 2),
      period_range:
        record.period_start && record.period_end
          ? [dayjs(record.period_start), dayjs(record.period_end)]
          : undefined,
      status: record.status
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<MarketingCost>>(
    () => [
      {
        title: '费用名称',
        dataIndex: 'name'
      },
      {
        title: '渠道',
        dataIndex: 'channel_id',
        render: (value) => (value ? channelMap.get(value) ?? value : '通用')
      },
      {
        title: '金额(分)',
        dataIndex: 'amount_cent'
      },
      {
        title: '币种',
        dataIndex: 'currency'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'ACTIVE' ? <Tag color="green">启用</Tag> : <Tag color="red">停用</Tag>
      },
      {
        title: '周期',
        render: (_, record) => formatRange(record.period_start, record.period_end)
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
              title="确认删除该费用？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.cost_id)}
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
    [canWrite, channelMap, deleteMutation]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          促销费用
        </Typography.Title>
        <Button type="primary" onClick={openCreate} disabled={!canWrite}>
          新建费用
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="费用名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="channel_id" label="渠道">
                <Select allowClear options={channelOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
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
      <UiModule03MainTableSection<MarketingCost>
        header={header}
        rowKey="cost_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isLoading || listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="促销费用加载失败"
        emptyText="暂无促销费用"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="marketing.costs.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editing ? '编辑促销费用' : '新建促销费用'}
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
          <Form.Item name="name" label="费用名称" rules={[{ required: true, message: '请输入费用名称' }]}>
            <Input placeholder="例如：平台广告费" />
          </Form.Item>
          <Form.Item name="channel_id" label="渠道">
            <Select allowClear options={channelOptions} placeholder="通用" />
          </Form.Item>
          <Form.Item name="amount_cent" label="金额(分)" rules={[{ required: true, message: '请输入金额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="currency" label="币种">
            <Input placeholder="CNY" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="period_range" label="费用周期">
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="allocation_rule" label="分摊规则JSON">
            <Input.TextArea rows={4} placeholder='例如：{"method":"by_amount"}' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
