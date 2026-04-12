import {
  Button,
  Card,
  Col,
  Form,
  Input,
  Row,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { ProductChannelAvailabilityListQuery } from '@/api/productChannelAvailability';
import {
  listProductChannelAvailability,
  updateProductChannelAvailability,
} from '@/api/productChannelAvailability';
import { listChannels } from '@/api/channels';
import { listSkus } from '@/api/skus';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type AvailabilityStatus = 'ON' | 'OFF';

type AvailabilityRecord = {
  id: string;
  sku_id: string;
  sku_code: string;
  sku_name: string;
  channel_id?: string | null;
  channel_name: string;
  warehouse_id?: string | null;
  warehouse_name: string;
  status: AvailabilityStatus;
  available: number;
  rule: string;
  on_shelf_at: string;
};

type FilterValues = {
  keyword?: string;
  channel?: string;
  warehouse?: string;
  status?: AvailabilityStatus;
};

type ChannelRule = {
  channel: string;
  stock_threshold: number;
  price_guard: string;
  auto_unlist: boolean;
};

const statusLabels: Record<AvailabilityStatus, string> = {
  ON: '可售',
  OFF: '禁售',
};

const statusColors: Record<AvailabilityStatus, string> = {
  ON: 'green',
  OFF: 'red',
};

const channelRules: ChannelRule[] = [
  {
    channel: '天猫旗舰店',
    stock_threshold: 20,
    price_guard: '售价≥99元',
    auto_unlist: true,
  },
  {
    channel: '京东自营',
    stock_threshold: 15,
    price_guard: '促销价需审批',
    auto_unlist: true,
  },
  {
    channel: '线下门店',
    stock_threshold: 5,
    price_guard: '门店可手动改价',
    auto_unlist: false,
  },
];

export const ProductChannelAvailabilityPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });

  const listQuery = useMemo<ProductChannelAvailabilityListQuery>(
    () => ({
      channel_id: filters.channel,
      warehouse_id: filters.warehouse,
      status: filters.status,
      offset: 0,
      limit: 200,
    }),
    [filters.channel, filters.status, filters.warehouse],
  );

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['product-channel-availability', tenantId, listQuery],
    queryFn: () => listProductChannelAvailability(tenantId, listQuery),
    enabled: Boolean(tenantId),
  });

  const channelQuery = useQuery({
    queryKey: ['channels', tenantId],
    queryFn: () => listChannels(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const skuQuery = useQuery({
    queryKey: ['skus', tenantId],
    queryFn: () => listSkus(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const skuMap = useMemo(
    () => new Map((skuQuery.data?.data ?? []).map((sku) => [sku.sku_id, sku])),
    [skuQuery.data?.data],
  );

  const channelMap = useMemo(
    () => new Map((channelQuery.data?.items ?? []).map((channel) => [channel.channel_id, channel])),
    [channelQuery.data?.items],
  );

  const warehouseMap = useMemo(
    () =>
      new Map(
        (warehouseQuery.data?.items ?? []).map((warehouse) => [warehouse.warehouse_id, warehouse]),
      ),
    [warehouseQuery.data?.items],
  );

  const channelOptions = useMemo(
    () =>
      (channelQuery.data?.items ?? []).map((channel) => ({
        value: channel.channel_id,
        label: channel.name,
      })),
    [channelQuery.data?.items],
  );

  const warehouseOptions = useMemo(
    () =>
      (warehouseQuery.data?.items ?? []).map((warehouse) => ({
        value: warehouse.warehouse_id,
        label: warehouse.name,
      })),
    [warehouseQuery.data?.items],
  );

  const records = useMemo<AvailabilityRecord[]>(
    () =>
      (data?.items ?? []).map((item) => {
        const sku = skuMap.get(item.sku_id);
        const channel = item.channel_id ? channelMap.get(item.channel_id) : undefined;
        const warehouse = item.warehouse_id ? warehouseMap.get(item.warehouse_id) : undefined;
        return {
          id: item.availability_id,
          sku_id: item.sku_id,
          sku_code: item.sku_code ?? sku?.sku_code ?? item.sku_id,
          sku_name: item.sku_name ?? sku?.name ?? '',
          channel_id: item.channel_id ?? null,
          channel_name: channel?.name ?? item.channel_platform ?? item.channel_id ?? '',
          warehouse_id: item.warehouse_id ?? null,
          warehouse_name: item.warehouse_name ?? warehouse?.name ?? '',
          status: item.status as AvailabilityStatus,
          available: item.available_qty ?? 0,
          rule: item.rule ?? '',
          on_shelf_at: item.on_shelf_at ?? item.updated_at ?? item.created_at ?? '',
        };
      }),
    [channelMap, data?.items, skuMap, warehouseMap],
  );

  const filteredData = useMemo(() => {
    const keyword = filters.keyword?.trim();
    return records.filter((item) => {
      if (filters.channel && item.channel_id !== filters.channel) {
        return false;
      }
      if (filters.warehouse && item.warehouse_id !== filters.warehouse) {
        return false;
      }
      if (filters.status && item.status !== filters.status) {
        return false;
      }
      if (keyword) {
        const target = `${item.sku_code}${item.sku_name}`.toLowerCase();
        return target.includes(keyword.toLowerCase());
      }
      return true;
    });
  }, [filters.channel, filters.keyword, filters.status, filters.warehouse, records]);

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const toggleStatus = async (record: AvailabilityRecord) => {
    if (!canWrite) {
      message.warning('暂无可售配置权限');
      return;
    }
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    const nextStatus: AvailabilityStatus = record.status === 'ON' ? 'OFF' : 'ON';
    try {
      await updateProductChannelAvailability(tenantId, record.id, { status: nextStatus });
      message.success(`已${nextStatus === 'ON' ? '上架' : '下架'}商品`);
      await queryClient.invalidateQueries({
        queryKey: ['product-channel-availability', tenantId],
      });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '更新可售状态失败');
    }
  };

  const columns = useMemo<ColumnsType<AvailabilityRecord>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: '商品名称', dataIndex: 'sku_name' },
      { title: '渠道', dataIndex: 'channel_name' },
      { title: '仓库', dataIndex: 'warehouse_name' },
      { title: '可用库存', dataIndex: 'available' },
      { title: '上架规则', dataIndex: 'rule' },
      {
        title: '上架时间',
        dataIndex: 'on_shelf_at',
        render: (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm'),
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: AvailabilityStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        ),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => toggleStatus(record)}>
              {record.status === 'ON' ? '下架' : '上架'}
            </Button>
            <Button size="small" onClick={() => message.info('已打开渠道规则配置')}>
              配置规则
            </Button>
          </Space>
        ),
      },
    ],
    [canWrite],
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }} data-testid="product-channel-title">
        商品渠道可售与上架
      </Typography.Title>
      <Card title="渠道上架规则">
        <Table
          rowKey="channel"
          size="small"
          pagination={false}
          dataSource={channelRules}
          data-testid="product-channel-rules-table"
          columns={[
            { title: '渠道', dataIndex: 'channel' },
            { title: '安全库存', dataIndex: 'stock_threshold' },
            { title: '价格约束', dataIndex: 'price_guard' },
            {
              title: '自动下架',
              dataIndex: 'auto_unlist',
              render: (value: boolean) => (value ? '是' : '否'),
            },
          ]}
        />
      </Card>
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSearch}
          data-testid="product-channel-filter-form"
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="SKU/商品名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="channel" label="渠道">
                <Select allowClear options={channelOptions} placeholder="全部渠道" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse" label="仓库">
                <Select allowClear options={warehouseOptions} placeholder="全部仓库" />
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
      <Card title="可售清单">
        <UiModule03MainTableSection<AvailabilityRecord>
          dataTestId="product-channel-table"
          rowKey="id"
          columns={columns}
          data={filteredData}
          loading={isLoading || isFetching}
          onRetry={() => refetch()}
          errorTitle="渠道可售加载失败"
          emptyText="暂无可售记录"
          total={filteredData.length}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="product.channel.availability.table.columns"
          enablePanels={false}
        />
      </Card>
    </Space>
  );
};
