import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  Popconfirm,
  Select,
  Space,
  Row,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Channel } from '@/api/channels';
import { listChannels } from '@/api/channels';
import type { MarketingPostmortem, MarketingPostmortemCreatePayload } from '@/api/marketingPostmortems';
import {
  createMarketingPostmortem,
  deleteMarketingPostmortem,
  listMarketingPostmortems,
  updateMarketingPostmortem
} from '@/api/marketingPostmortems';
import { listMarketingPromotions } from '@/api/marketingPromotions';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  promotion_id?: string;
  channel_id?: string;
};

type FormValues = {
  name: string;
  promotion_id?: string;
  channel_id?: string;
  summary?: string;
  metrics_json?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

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

export const MarketingPostmortemsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('marketing_postmortems.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<MarketingPostmortem | null>(null);

  const channelsQuery = useQuery({
    queryKey: ['channels', tenantId],
    queryFn: () => listChannels(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const promotionsQuery = useQuery({
    queryKey: ['marketing-promotions', tenantId, 'options'],
    queryFn: () => listMarketingPromotions(tenantId, { offset: 0, limit: 200 }),
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

  const promotionOptions = useMemo(
    () =>
      promotionsQuery.data?.items.map((promotion) => ({
        label: promotion.name,
        value: promotion.promotion_id
      })) ?? [],
    [promotionsQuery.data?.items]
  );

  const channelMap = useMemo(
    () =>
      new Map(
        channelsQuery.data?.items.map((channel) => [channel.channel_id, channel.name]) ?? []
      ),
    [channelsQuery.data?.items]
  );

  const promotionMap = useMemo(
    () =>
      new Map(
        promotionsQuery.data?.items.map((promotion) => [promotion.promotion_id, promotion.name]) ??
          []
      ),
    [promotionsQuery.data?.items]
  );

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      promotion_id: filters.promotion_id || undefined,
      channel_id: filters.channel_id || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['marketing-postmortems', tenantId, query],
    queryFn: () => listMarketingPostmortems(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: MarketingPostmortemCreatePayload = {
        name: values.name.trim(),
        promotion_id: values.promotion_id,
        channel_id: values.channel_id,
        summary: values.summary?.trim(),
        metrics_json: parseJson(values.metrics_json, {})
      };
      if (editing) {
        return updateMarketingPostmortem(tenantId, editing.postmortem_id, payload);
      }
      return createMarketingPostmortem(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '活动复盘已更新' : '活动复盘已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['marketing-postmortems', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存活动复盘失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (postmortemId: string) => deleteMarketingPostmortem(tenantId, postmortemId),
    onSuccess: async () => {
      message.success('活动复盘已删除');
      await queryClient.invalidateQueries({ queryKey: ['marketing-postmortems', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除活动复盘失败');
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
      metrics_json: '{}'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: MarketingPostmortem) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      name: record.name,
      promotion_id: record.promotion_id ?? undefined,
      channel_id: record.channel_id ?? undefined,
      summary: record.summary ?? undefined,
      metrics_json: JSON.stringify(record.metrics_json ?? {}, null, 2)
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<MarketingPostmortem>>(
    () => [
      {
        title: '复盘名称',
        dataIndex: 'name'
      },
      {
        title: '关联活动',
        dataIndex: 'promotion_id',
        render: (value) => (value ? promotionMap.get(value) ?? value : '-')
      },
      {
        title: '渠道',
        dataIndex: 'channel_id',
        render: (value) => (value ? channelMap.get(value) ?? value : '通用')
      },
      {
        title: '摘要',
        dataIndex: 'summary',
        render: (value) => value ?? '-'
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
              title="确认删除该复盘？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.postmortem_id)}
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
    [canWrite, channelMap, deleteMutation, promotionMap]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          活动复盘
        </Typography.Title>
        <Button type="primary" onClick={openCreate} disabled={!canWrite}>
          新建复盘
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="复盘名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="promotion_id" label="活动">
                <Select allowClear options={promotionOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="channel_id" label="渠道">
                <Select allowClear options={channelOptions} placeholder="全部" />
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
      <UiModule03MainTableSection<MarketingPostmortem>
        header={header}
        rowKey="postmortem_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isLoading || listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="活动复盘加载失败"
        emptyText="暂无活动复盘"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="marketing.postmortems.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editing ? '编辑活动复盘' : '新建活动复盘'}
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
          <Form.Item name="name" label="复盘名称" rules={[{ required: true, message: '请输入复盘名称' }]}>
            <Input placeholder="例如：双11活动复盘" />
          </Form.Item>
          <Form.Item name="promotion_id" label="关联活动">
            <Select allowClear options={promotionOptions} placeholder="选择活动" />
          </Form.Item>
          <Form.Item name="channel_id" label="渠道">
            <Select allowClear options={channelOptions} placeholder="通用" />
          </Form.Item>
          <Form.Item name="summary" label="摘要">
            <Input.TextArea rows={3} />
          </Form.Item>
          <Form.Item name="metrics_json" label="指标JSON">
            <Input.TextArea rows={4} placeholder='例如：{"sales":10000,"profit":2000}' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
