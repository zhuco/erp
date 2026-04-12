import {
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Select,
  Space,
  Row,
  Table,
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
import type {
  MarketingPriceItem,
  MarketingPriceList,
  MarketingPriceListCreatePayload
} from '@/api/marketingPriceLists';
import {
  createMarketingPriceItem,
  createMarketingPriceList,
  deleteMarketingPriceItem,
  deleteMarketingPriceList,
  getMarketingPriceList,
  listMarketingPriceLists,
  updateMarketingPriceList
} from '@/api/marketingPriceLists';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  channel_id?: string;
  currency?: string;
};

type PriceListFormValues = {
  name: string;
  channel_id?: string;
  currency?: string;
  includes_tax?: boolean;
  status?: string;
  version?: number;
  effective_range?: [dayjs.Dayjs, dayjs.Dayjs];
  notes?: string;
};

type ItemFormValues = {
  sku_id: string;
  base_price_cent?: number;
  channel_price_cent?: number;
  member_price_cent?: number;
};

const statusOptions = [
  { label: '草稿', value: 'DRAFT' },
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatRange = (from?: string | null, to?: string | null) => {
  if (!from && !to) {
    return '-';
  }
  return `${from ? dayjs(from).format('YYYY-MM-DD') : '不限'} ~ ${
    to ? dayjs(to).format('YYYY-MM-DD') : '不限'
  }`;
};

export const MarketingPriceListsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('marketing_prices.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<PriceListFormValues>();
  const [itemForm] = Form.useForm<ItemFormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [editing, setEditing] = useState<MarketingPriceList | null>(null);
  const [activeListId, setActiveListId] = useState<string | null>(null);

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
      currency: filters.currency?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['marketing-price-lists', tenantId, query],
    queryFn: () => listMarketingPriceLists(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const detailQuery = useQuery({
    queryKey: ['marketing-price-lists', tenantId, activeListId],
    queryFn: () => getMarketingPriceList(tenantId, activeListId ?? ''),
    enabled: tenantReady && Boolean(activeListId) && detailOpen
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: MarketingPriceListCreatePayload = {
        name: values.name.trim(),
        channel_id: values.channel_id,
        currency: values.currency?.trim(),
        includes_tax: values.includes_tax,
        status: values.status,
        version: values.version,
        effective_from: values.effective_range?.[0]?.toISOString(),
        effective_to: values.effective_range?.[1]?.toISOString(),
        notes: values.notes?.trim()
      };
      if (editing) {
        return updateMarketingPriceList(tenantId, editing.price_list_id, payload);
      }
      return createMarketingPriceList(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '渠道价表已更新' : '渠道价表已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['marketing-price-lists', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存渠道价表失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (priceListId: string) => deleteMarketingPriceList(tenantId, priceListId),
    onSuccess: async () => {
      message.success('渠道价表已删除');
      await queryClient.invalidateQueries({ queryKey: ['marketing-price-lists', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除渠道价表失败');
    }
  });

  const createItemMutation = useMutation({
    mutationFn: async () => {
      if (!activeListId) {
        throw new Error('请选择价表');
      }
      const values = await itemForm.validateFields();
      return createMarketingPriceItem(tenantId, activeListId, values);
    },
    onSuccess: async () => {
      message.success('价表明细已保存');
      itemForm.resetFields();
      await queryClient.invalidateQueries({
        queryKey: ['marketing-price-lists', tenantId, activeListId]
      });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存价表明细失败');
    }
  });

  const deleteItemMutation = useMutation({
    mutationFn: (itemId: string) => deleteMarketingPriceItem(tenantId, itemId),
    onSuccess: async () => {
      message.success('价表明细已删除');
      await queryClient.invalidateQueries({
        queryKey: ['marketing-price-lists', tenantId, activeListId]
      });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除价表明细失败');
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
      currency: 'CNY',
      includes_tax: true,
      version: 1
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: MarketingPriceList) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      name: record.name,
      channel_id: record.channel_id ?? undefined,
      currency: record.currency,
      includes_tax: record.includes_tax,
      status: record.status,
      version: record.version,
      effective_range:
        record.effective_from && record.effective_to
          ? [dayjs(record.effective_from), dayjs(record.effective_to)]
          : undefined,
      notes: record.notes ?? undefined
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<MarketingPriceList>>(
    () => [
      {
        title: '价表名称',
        dataIndex: 'name'
      },
      {
        title: '渠道',
        dataIndex: 'channel_id',
        render: (value) => (value ? channelMap.get(value) ?? value : '通用')
      },
      {
        title: '币种',
        dataIndex: 'currency'
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
        title: '版本',
        dataIndex: 'version'
      },
      {
        title: '生效期',
        render: (_, record) => formatRange(record.effective_from, record.effective_to)
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
            <Button
              size="small"
              onClick={() => {
                setActiveListId(record.price_list_id);
                setDetailOpen(true);
              }}
            >
              明细
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => openEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该价表？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.price_list_id)}
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

  const detail = detailQuery.data?.price_list;
  const detailItems = detailQuery.data?.items ?? [];

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          渠道价表
        </Typography.Title>
        <Button type="primary" onClick={openCreate} disabled={!canWrite}>
          新建价表
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="价表名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="channel_id" label="渠道">
                <Select allowClear options={channelOptions} placeholder="全部渠道" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="全部状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="currency" label="币种">
                <Input allowClear placeholder="CNY" />
              </Form.Item>
            </Col>
            <Col xs={24} md={2} style={{ display: 'flex', alignItems: 'end' }}>
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
      <UiModule03MainTableSection<MarketingPriceList>
        header={header}
        rowKey="price_list_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isLoading || listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="渠道价表加载失败"
        emptyText="暂无渠道价表"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="marketing.price-lists.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editing ? '编辑价表' : '新建价表'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={560}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={createMutation.isPending} onClick={() => createMutation.mutate()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item name="name" label="价表名称" rules={[{ required: true, message: '请输入价表名称' }]}>
            <Input placeholder="例如：天猫会员价" />
          </Form.Item>
          <Form.Item name="channel_id" label="渠道">
            <Select allowClear options={channelOptions} placeholder="通用渠道" />
          </Form.Item>
          <Form.Item name="currency" label="币种" rules={[{ required: true, message: '请输入币种' }]}>
            <Input placeholder="CNY" />
          </Form.Item>
          <Form.Item name="includes_tax" label="含税">
            <Select
              options={[
                { label: '含税', value: true },
                { label: '未税', value: false }
              ]}
            />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="version" label="版本">
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="effective_range" label="生效期">
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="notes" label="备注">
            <Input.TextArea rows={3} placeholder="价表说明" />
          </Form.Item>
        </Form>
      </Drawer>
      <Drawer
        title="价表明细"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={900}
        destroyOnClose
      >
        {detail ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions column={2} bordered size="small">
              <Descriptions.Item label="价表名称">{detail.name}</Descriptions.Item>
              <Descriptions.Item label="渠道">
                {detail.channel_id ? channelMap.get(detail.channel_id) ?? detail.channel_id : '通用'}
              </Descriptions.Item>
              <Descriptions.Item label="币种">{detail.currency}</Descriptions.Item>
              <Descriptions.Item label="状态">{detail.status}</Descriptions.Item>
              <Descriptions.Item label="版本">{detail.version}</Descriptions.Item>
              <Descriptions.Item label="生效期">
                {formatRange(detail.effective_from, detail.effective_to)}
              </Descriptions.Item>
            </Descriptions>
            <Card size="small" title="新增/更新明细">
              <Form form={itemForm} layout="inline" onFinish={() => createItemMutation.mutate()}>
                <Form.Item
                  name="sku_id"
                  label="SKU ID"
                  rules={[{ required: true, message: '请输入SKU ID' }]}
                >
                  <Input placeholder="SKU ID" style={{ width: 180 }} />
                </Form.Item>
                <Form.Item name="base_price_cent" label="基础价(分)">
                  <InputNumber min={0} />
                </Form.Item>
                <Form.Item name="channel_price_cent" label="渠道价(分)">
                  <InputNumber min={0} />
                </Form.Item>
                <Form.Item name="member_price_cent" label="会员价(分)">
                  <InputNumber min={0} />
                </Form.Item>
                <Form.Item>
                  <Button type="primary" htmlType="submit" loading={createItemMutation.isPending} disabled={!canWrite}>
                    保存
                  </Button>
                </Form.Item>
              </Form>
            </Card>
            <Table<MarketingPriceItem>
              rowKey="item_id"
              columns={[
                { title: 'SKU', dataIndex: 'sku_id' },
                {
                  title: '基础价(分)',
                  dataIndex: 'base_price_cent',
                  render: (value) => value ?? '-'
                },
                {
                  title: '渠道价(分)',
                  dataIndex: 'channel_price_cent',
                  render: (value) => value ?? '-'
                },
                {
                  title: '会员价(分)',
                  dataIndex: 'member_price_cent',
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
                    <Popconfirm
                      title="确认删除该明细？"
                      okText="删除"
                      cancelText="取消"
                      onConfirm={() => deleteItemMutation.mutate(record.item_id)}
                      disabled={!canWrite}
                    >
                      <Button size="small" danger disabled={!canWrite}>
                        删除
                      </Button>
                    </Popconfirm>
                  )
                }
              ]}
              dataSource={detailItems}
              pagination={{ pageSize: 10 }}
            />
          </Space>
        ) : null}
      </Drawer>
    </>
  );
};
