import {
  Button,
  Card,
  Col,
  Descriptions,
  Divider,
  Drawer,
  Form,
  Input,
  Row,
  Select,
  Space,
  Tag,
  Timeline,
  Typography,
  TablePaginationConfig,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { PlatformCode } from '@/features/platform';
import { platformLabels } from '@/features/platform';
import { useTenantId } from '@/features/tenant';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule02WorkbenchTemplate, UiModule03MainTableSection } from '@/components/ui-modules';
import { useShopProductSkus } from '@/api/hooks';
import type { ShopProductSku, ShopProductSkuListQuery } from '@/api/shopProducts';
import { listShops, type Shop } from '@/api/shops';

type DiffFilters = {
  platform?: PlatformCode;
  shop_ids?: string[];
  status?: 'ACTIVE' | 'INACTIVE' | 'ALL';
  match_status?: string;
  mapping_status?: string;
  match_source?: string;
  conflict_reason?: string;
  keyword?: string;
};

const statusOptions = [
  { label: '全部', value: 'ALL' },
  { label: '已激活', value: 'ACTIVE' },
  { label: '未激活', value: 'INACTIVE' },
];

const matchStatusOptions = [
  { label: '已匹配', value: 'MATCHED' },
  { label: '未匹配', value: 'UNMATCHED' },
  { label: '自动匹配', value: 'AUTO' },
  { label: '手动匹配', value: 'MANUAL' },
];

const mappingStatusOptions = [
  { label: '正常', value: 'ACTIVE' },
  { label: '冲突', value: 'CONFLICT' },
  { label: '未激活', value: 'INACTIVE' },
];

const matchSourceOptions = [
  { label: '自动', value: 'AUTO' },
  { label: '手动', value: 'MANUAL' },
];

const conflictReasonOptions = [
  { label: '无候选SKU', value: 'NO_CANDIDATE' },
  { label: '多个候选SKU', value: 'MULTI_CANDIDATE' },
  { label: 'SKU不可用', value: 'SKU_INACTIVE' },
];

const conflictReasonLabelMap: Record<string, string> = {
  NO_CANDIDATE: '无候选SKU',
  MULTI_CANDIDATE: '多个候选SKU',
  SKU_INACTIVE: 'SKU不可用',
};

const matchStatusLabelMap: Record<string, string> = {
  MATCHED: '已匹配',
  UNMATCHED: '未匹配',
  AUTO: '自动匹配',
  MANUAL: '手动匹配',
};

const mappingStatusLabelMap: Record<string, string> = {
  ACTIVE: '正常',
  CONFLICT: '冲突',
  INACTIVE: '未激活',
};

const matchSourceLabelMap: Record<string, string> = {
  AUTO: '自动',
  MANUAL: '手动',
};

const resolveMatchStatus = (record: ShopProductSku) => {
  if (record.mapping_status === 'CONFLICT') {
    return { label: '冲突', color: 'red' };
  }
  if (!record.mapping_id) {
    return { label: '未匹配', color: 'default' };
  }
  if (record.match_source === 'MANUAL') {
    return { label: '手动', color: 'blue' };
  }
  if (record.match_source === 'AUTO') {
    return { label: '自动', color: 'green' };
  }
  return { label: '已匹配', color: 'default' };
};

const formatTimestamp = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const ProductDiffPage = () => {
  const tenantId = useTenantId();
  const [filters, setFilters] = useState<DiffFilters>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [drawerRecord, setDrawerRecord] = useState<ShopProductSku | null>(null);
  const [form] = Form.useForm<DiffFilters>();

  const shopQuery = useQuery({
    queryKey: ['shops', tenantId, 'product-diff'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const platformOptions = useMemo(() => {
    const platforms = Array.from(new Set((shopQuery.data?.items ?? []).map((shop) => shop.platform)));
    return platforms.map((platform) => ({
      value: platform,
      label: platformLabels[platform] ?? platform,
    }));
  }, [shopQuery.data?.items]);

  const shopOptions = useMemo(
    () =>
      (shopQuery.data?.items ?? []).map((shop) => ({
        value: shop.shop_id,
        label: `${shop.name} (${shop.shop_id})`,
      })),
    [shopQuery.data?.items],
  );

  const query = useMemo<ShopProductSkuListQuery>(() => {
    return {
      platform: filters.platform,
      shop_ids: filters.shop_ids,
      status: filters.status && filters.status !== 'ALL' ? filters.status : undefined,
      match_status: filters.match_status,
      mapping_status: filters.mapping_status,
      match_source: filters.match_source,
      conflict_reason: filters.conflict_reason,
      keyword: filters.keyword?.trim(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20,
    };
  }, [filters, pagination]);

  const { data, isLoading, isFetching, error, refetch } = useShopProductSkus({
    query,
    enabled: Boolean(tenantId),
  });

  const shopMap = useMemo(() => {
    const map = new Map<string, Shop>();
    (shopQuery.data?.items ?? []).forEach((shop) => map.set(shop.shop_id, shop));
    return map;
  }, [shopQuery.data?.items]);

  const records = data?.items ?? [];

  const conflictSummary = useMemo(() => {
    const counter = new Map<string, number>();
    records.forEach((record) => {
      if (record.conflict_reason) {
        counter.set(record.conflict_reason, (counter.get(record.conflict_reason) ?? 0) + 1);
      }
    });
    return Array.from(counter.entries()).sort((a, b) => b[1] - a[1]);
  }, [records]);

  const matchSourceSummary = useMemo(() => {
    const counter = new Map<string, number>();
    records.forEach((record) => {
      const source = record.match_source ?? 'UNKNOWN';
      counter.set(source, (counter.get(source) ?? 0) + 1);
    });
    return Array.from(counter.entries())
      .filter(([source]) => source !== 'UNKNOWN')
      .sort((a, b) => b[1] - a[1]);
  }, [records]);

  const columns = useMemo<ColumnsType<ShopProductSku>>(
    () => [
      {
        title: '平台',
        dataIndex: 'platform',
        render: (_, record) => <PlatformTag platform={record.platform} />,
      },
      {
        title: '店铺',
        dataIndex: 'shop_id',
        render: (value: string) => shopMap.get(value)?.name ?? value,
      },
      { title: '商品名称', dataIndex: 'product_title' },
      { title: '平台 SKU', dataIndex: 'platform_sku_id' },
      {
        title: '匹配来源',
        dataIndex: 'match_source',
        render: (value: string | null | undefined) =>
          value ? <Tag>{matchSourceLabelMap[value] ?? value}</Tag> : <Tag>未知</Tag>,
      },
      {
        title: '冲突理由',
        dataIndex: 'conflict_reason',
        render: (value: string | null | undefined) => (value ? conflictReasonLabelMap[value] ?? value : '-'),
      },
      {
        title: '映射状态',
        dataIndex: 'mapping_status',
        render: (value: string | null | undefined) => (value ? <Tag>{mappingStatusLabelMap[value] ?? value}</Tag> : '-'),
      },
      {
        title: '匹配状态',
        dataIndex: 'match_status',
        render: (_, record) => {
          const state = resolveMatchStatus(record);
          return <Tag color={state.color}>{state.label}</Tag>;
        },
      },
      {
        title: 'ERP SKU',
        dataIndex: 'mapped_sku_code',
        render: (value: string | null | undefined, record) =>
          value ? value + (record.mapped_sku_name ? ' ' + record.mapped_sku_name : '') : '-',
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value: string | null | undefined) => formatTimestamp(value),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button type="link" size="small" onClick={() => setDrawerRecord(record)}>
            查看溯源
          </Button>
        ),
      },
    ],
    [shopMap],
  );

  const handleSearch = (values: DiffFilters) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const timelineItems = useMemo(() => {
    if (!drawerRecord) {
      return [];
    }
    const shopName = shopMap.get(drawerRecord.shop_id)?.name ?? drawerRecord.shop_id;
    return [
      {
        children: (
          <Space direction="vertical" size={2}>
            <Typography.Text strong>平台同步</Typography.Text>
            <Typography.Text type="secondary">
              {drawerRecord.platform_sku_id} / {shopName}
            </Typography.Text>
            <Typography.Text type="secondary">
              最后更新时间 {formatTimestamp(drawerRecord.platform_updated_at ?? drawerRecord.updated_at)}
            </Typography.Text>
          </Space>
        ),
      },
      {
        children: (
          <Space direction="vertical" size={2}>
            <Typography.Text strong>匹配判断</Typography.Text>
            <Typography.Text>
              来源: {matchSourceLabelMap[drawerRecord.match_source ?? ''] ?? '未明确'}
            </Typography.Text>
            <Typography.Text>
              状态: {resolveMatchStatus(drawerRecord).label} / 映射状态{' '}
              {mappingStatusLabelMap[drawerRecord.mapping_status ?? ''] ?? '未知'}
            </Typography.Text>
          </Space>
        ),
        color: drawerRecord.mapping_status === 'CONFLICT' ? 'red' : 'green',
      },
      {
        children: (
          <Space direction="vertical" size={2}>
            <Typography.Text strong>冲突分析</Typography.Text>
            <Typography.Text type="secondary">
              {drawerRecord.conflict_reason
                ? conflictReasonLabelMap[drawerRecord.conflict_reason] ?? drawerRecord.conflict_reason
                : '未检测到'}
            </Typography.Text>
          </Space>
        ),
        color: drawerRecord.conflict_reason ? 'red' : 'gray',
      },
    ];
  }, [drawerRecord, shopMap]);

  return (
    <>
      <UiModule02WorkbenchTemplate
        header={
          <Typography.Title level={3} style={{ margin: 0 }} data-testid="product-diff-title">
            产品差异分析
          </Typography.Title>
        }
        tableContent={
          <>
            <Card>
              <Form
                form={form}
                layout="vertical"
                onFinish={handleSearch}
                data-testid="product-diff-filter-form"
              >
                <Row gutter={16}>
                  <Col xs={24} sm={12} md={8}>
                    <Form.Item name="platform" label="平台">
                      <Select allowClear options={platformOptions} placeholder="选择平台" />
                    </Form.Item>
                  </Col>
                  <Col xs={24} sm={12} md={8}>
                    <Form.Item name="shop_ids" label="店铺">
                      <Select mode="multiple" allowClear placeholder="筛选店铺" options={shopOptions} />
                    </Form.Item>
                  </Col>
                  <Col xs={24} sm={12} md={8}>
                    <Form.Item name="status" label="SKU 状态">
                      <Select options={statusOptions} />
                    </Form.Item>
                  </Col>
                  <Col xs={24} sm={12} md={8}>
                    <Form.Item name="match_status" label="匹配状态">
                      <Select allowClear options={matchStatusOptions} />
                    </Form.Item>
                  </Col>
                  <Col xs={24} sm={12} md={8}>
                    <Form.Item name="mapping_status" label="映射状态">
                      <Select allowClear options={mappingStatusOptions} />
                    </Form.Item>
                  </Col>
                  <Col xs={24} sm={12} md={8}>
                    <Form.Item name="match_source" label="匹配来源">
                      <Select allowClear options={matchSourceOptions} />
                    </Form.Item>
                  </Col>
                  <Col xs={24} sm={12} md={8}>
                    <Form.Item name="conflict_reason" label="冲突原因">
                      <Select allowClear options={conflictReasonOptions} />
                    </Form.Item>
                  </Col>
                  <Col xs={24} md={8}>
                    <Form.Item name="keyword" label="关键词">
                      <Input placeholder="名称 / 平台 SKU" allowClear />
                    </Form.Item>
                  </Col>
                  <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
                    <Space>
                      <Button type="primary" htmlType="submit">
                        筛选
                      </Button>
                      <Button onClick={handleReset}>重置</Button>
                    </Space>
                  </Col>
                </Row>
              </Form>
            </Card>
            <Card style={{ marginTop: 16 }}>
              <Space wrap data-testid="product-diff-reason-pills">
                {conflictSummary.length ? (
                  conflictSummary.map(([reason, count]) => (
                    <Tag key={reason} color="orange">
                      {conflictReasonLabelMap[reason] ?? reason} · {count}
                    </Tag>
                  ))
                ) : (
                  <Typography.Text type="secondary">当前无冲突记录</Typography.Text>
                )}
                {matchSourceSummary.map(([source, count]) => (
                  <Tag key={source} color="blue">
                    {matchSourceLabelMap[source] ?? source} · {count}
                  </Tag>
                ))}
              </Space>
            </Card>
            <Card title="差异概览" style={{ marginTop: 16 }}>
              <UiModule03MainTableSection<ShopProductSku>
                dataTestId="product-diff-table"
                rowKey="sku_record_id"
                columns={columns}
                data={records}
                loading={isLoading || isFetching}
                error={error}
                onRetry={() => refetch()}
                emptyText="暂无差异记录"
                total={data?.total ?? 0}
                pagination={pagination}
                onPaginationChange={setPagination}
                columnSettingsKey="product.diff.table.columns"
                enablePanels={false}
              />
            </Card>
          </>
        }
      />

      <Drawer title="映射追踪" open={Boolean(drawerRecord)} onClose={() => setDrawerRecord(null)} width={520}>
        {drawerRecord ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions column={1} size="small">
              <Descriptions.Item label="平台">{drawerRecord.platform}</Descriptions.Item>
              <Descriptions.Item label="店铺">
                {shopMap.get(drawerRecord.shop_id)?.name ?? drawerRecord.shop_id}
              </Descriptions.Item>
              <Descriptions.Item label="平台 SKU">{drawerRecord.platform_sku_id}</Descriptions.Item>
              <Descriptions.Item label="ERP SKU">
                {drawerRecord.mapped_sku_code ?? '-'} {drawerRecord.mapped_sku_name ?? ''}
              </Descriptions.Item>
              <Descriptions.Item label="映射状态">
                {mappingStatusLabelMap[drawerRecord.mapping_status ?? ''] ?? '未知'}
              </Descriptions.Item>
              <Descriptions.Item label="匹配来源">
                {matchSourceLabelMap[drawerRecord.match_source ?? ''] ?? '未明确'}
              </Descriptions.Item>
              <Descriptions.Item label="冲突原因">
                {drawerRecord.conflict_reason
                  ? conflictReasonLabelMap[drawerRecord.conflict_reason] ?? drawerRecord.conflict_reason
                  : '未检测到'}
              </Descriptions.Item>
            </Descriptions>
            <Divider style={{ margin: 0 }} />
            <Typography.Text strong>流向概览</Typography.Text>
            <Timeline items={timelineItems} />
          </Space>
        ) : null}
      </Drawer>
    </>
  );
};
