import {
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Form,
  Row,
  Select,
  Space,
  Table,
  Tag,
  Typography
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { CaretDownOutlined, CaretUpOutlined, MinusOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import { getStatsInfraHealth, getStatsReport, type StatsType } from '@/api/stats';
import { EmptyState } from '@/components/EmptyState';
import { LoadingSkeleton } from '@/components/LoadingSkeleton';
import { ErrorState } from '@/components/QueryState';
import { isTenantSelected, useTenantId } from '@/features/tenant';
import {
  buildStatsDiff,
  formatDiffRate,
  formatDiffValue,
  formatStatsValue,
  type StatsDiffItem,
  type StatsMetricDefinition
} from '@/utils/statsDiff';

type StatsPlaceholderProps = {
  title: string;
  statsType: StatsType;
  dataSources: string[];
  metrics: StatsMetricDefinition[];
  pendingReasons: string[];
};

type StatsCompareMode = 'previous' | 'none';

type StatsFilterValues = {
  range: [Dayjs, Dayjs];
  dimension?: string;
  metrics?: string[];
  compare_mode?: StatsCompareMode;
};

const renderTags = (items: string[]) => (
  <Space size={[8, 8]} wrap>
    {items.map((item) => (
      <Tag key={item}>{item}</Tag>
    ))}
  </Space>
);

const dimensionOptions = [
  { label: '按渠道', value: 'channel' },
  { label: '按仓库', value: 'warehouse' },
  { label: '按品类', value: 'category' }
];

const compareModeOptions = [
  { label: '对比上期', value: 'previous' },
  { label: '不对比', value: 'none' }
];

const resolveCompareRange = (
  range: [Dayjs, Dayjs],
  compareMode: StatsCompareMode
): [Dayjs, Dayjs] | null => {
  if (compareMode === 'none') {
    return null;
  }
  const start = range[0].startOf('day');
  const end = range[1].endOf('day');
  const diffDays = end.startOf('day').diff(start.startOf('day'), 'day') + 1;
  return [start.subtract(diffDays, 'day'), end.subtract(diffDays, 'day')];
};

export const StatsPlaceholder = ({
  title,
  statsType,
  dataSources,
  metrics,
  pendingReasons
}: StatsPlaceholderProps) => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [form] = Form.useForm<StatsFilterValues>();

  const defaultRange = useMemo<[Dayjs, Dayjs]>(
    () => [dayjs().subtract(6, 'day').startOf('day'), dayjs().endOf('day')],
    []
  );
  const defaultMetricKeys = useMemo(() => metrics.map((metric) => metric.key), [metrics]);
  const [filters, setFilters] = useState(() => ({
    range: defaultRange,
    dimension: undefined as string | undefined,
    metricKeys: defaultMetricKeys,
    compareMode: 'previous' as StatsCompareMode
  }));

  const metricOptions = useMemo(
    () =>
      metrics.map((metric) => ({
        label: metric.label,
        value: metric.key
      })),
    [metrics]
  );

  const selectedMetrics = useMemo(() => {
    if (!filters.metricKeys.length) {
      return metrics;
    }
    return metrics.filter((metric) => filters.metricKeys.includes(metric.key));
  }, [filters.metricKeys, metrics]);

  const compareRange = useMemo(
    () => resolveCompareRange(filters.range, filters.compareMode),
    [filters.compareMode, filters.range]
  );

  const query = useMemo(
    () => ({
      start_at: filters.range[0].toISOString(),
      end_at: filters.range[1].toISOString(),
      compare_start_at: compareRange?.[0]?.toISOString(),
      compare_end_at: compareRange?.[1]?.toISOString(),
      dimension: filters.dimension,
      metrics: filters.metricKeys
    }),
    [compareRange, filters.dimension, filters.metricKeys, filters.range]
  );

  const statsQuery = useQuery({
    queryKey: ['stats', statsType, query],
    queryFn: () => getStatsReport(tenantId, statsType, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });
  const infraQuery = useQuery({
    queryKey: ['stats-infra-health', tenantId],
    queryFn: () => getStatsInfraHealth(tenantId),
    enabled: tenantReady
  });

  const diffRows = useMemo(() => {
    if (!statsQuery.data) {
      return [];
    }
    return buildStatsDiff(
      selectedMetrics,
      statsQuery.data.current ?? {},
      filters.compareMode === 'none' ? undefined : statsQuery.data.compare
    );
  }, [filters.compareMode, selectedMetrics, statsQuery.data]);

  const diffColumns = useMemo<ColumnsType<StatsDiffItem>>(
    () => [
      { title: '指标', dataIndex: 'label' },
      {
        title: '当前值',
        dataIndex: 'current',
        align: 'right',
        render: (_, record) => formatStatsValue(record.current, record.metric)
      },
      {
        title: '对比值',
        dataIndex: 'compare',
        align: 'right',
        render: (_, record) =>
          filters.compareMode === 'none'
            ? '-'
            : formatStatsValue(record.compare, record.metric)
      },
      {
        title: '相差',
        dataIndex: 'diff',
        align: 'right',
        render: (_, record) => {
          if (filters.compareMode === 'none') {
            return '-';
          }
          if (record.diff == null) {
            return '-';
          }
          if (record.diff === 0) {
            return (
              <Tag color="default" icon={<MinusOutlined />}>
                {formatDiffValue(record.diff, record.metric)}
              </Tag>
            );
          }
          const color = record.diff > 0 ? 'green' : 'red';
          const icon = record.diff > 0 ? <CaretUpOutlined /> : <CaretDownOutlined />;
          return (
            <Tag color={color} icon={icon}>
              {formatDiffValue(record.diff, record.metric)}
            </Tag>
          );
        }
      },
      {
        title: '变化率',
        dataIndex: 'diffRate',
        align: 'right',
        render: (_, record) => {
          if (filters.compareMode === 'none') {
            return '-';
          }
          if (record.diffRate == null) {
            return '-';
          }
          const color = record.diffRate > 0 ? 'green' : record.diffRate < 0 ? 'red' : 'default';
          const icon =
            record.diffRate > 0
              ? <CaretUpOutlined />
              : record.diffRate < 0
                ? <CaretDownOutlined />
                : <MinusOutlined />;
          return (
            <Tag color={color} icon={icon}>
              {formatDiffRate(record.diffRate)}
            </Tag>
          );
        }
      }
    ],
    [filters.compareMode]
  );

  const breakdownColumns = useMemo(() => {
    if (!filters.dimension) {
      return [] as ColumnsType<Record<string, unknown>>;
    }
    const dimensionLabel =
      dimensionOptions.find((option) => option.value === filters.dimension)?.label ??
      '维度';
    return [
      { title: dimensionLabel, dataIndex: 'dimension_label' },
      ...selectedMetrics.map((metric) => ({
        title: metric.label,
        dataIndex: metric.key,
        align: 'right' as const,
        render: (value: number) => formatStatsValue(value, metric)
      }))
    ];
  }, [filters.dimension, selectedMetrics]);

  const breakdownData = useMemo(
    () =>
      (statsQuery.data?.breakdown ?? []).map((item) => ({
        dimension_label: item.dimension_label ?? item.dimension,
        ...item.metrics
      })),
    [statsQuery.data?.breakdown]
  );

  const statsLoading = statsQuery.isLoading;

  const handleSearch = (values: StatsFilterValues) => {
    if (!values.range || values.range.length !== 2) {
      return;
    }
    setFilters({
      range: values.range,
      dimension: values.dimension?.trim() || undefined,
      metricKeys:
        values.metrics && values.metrics.length > 0 ? values.metrics : defaultMetricKeys,
      compareMode: values.compare_mode ?? 'previous'
    });
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({
      range: defaultRange,
      dimension: undefined,
      metricKeys: defaultMetricKeys,
      compareMode: 'previous'
    });
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Row align="middle" justify="space-between">
        <Typography.Title level={3} style={{ margin: 0 }}>
          {title}
        </Typography.Title>
        <Space>
          <Button disabled>导出</Button>
        </Space>
      </Row>
      <Card>
        <Space wrap data-testid="stats-infra-health">
          <Tag color="blue">分析引擎: {infraQuery.data?.analytics_backend ?? 'POSTGRES'}</Tag>
          <Tag color={infraQuery.data?.clickhouse_ready ? 'green' : 'default'}>
            ClickHouse: {infraQuery.data?.clickhouse_ready ? '已接入' : '未接入'}
          </Tag>
          <Tag color={infraQuery.data?.sso_ready ? 'green' : 'default'}>
            Keycloak SSO: {infraQuery.data?.sso_ready ? '已接入' : '未接入'}
          </Tag>
        </Space>
      </Card>
      <Card>
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            range: defaultRange,
            metrics: defaultMetricKeys,
            compare_mode: 'previous'
          }}
          onFinish={handleSearch}
        >
          <Row gutter={[16, 16]}>
            <Col xs={24} md={6}>
              <Form.Item name="range" label="统计周期">
                <DatePicker.RangePicker style={{ width: '100%' }} allowClear={false} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="dimension" label="统计维度">
                <Select allowClear placeholder="选择维度" options={dimensionOptions} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="metrics" label="核心指标">
                <Select
                  allowClear
                  mode="multiple"
                  placeholder="选择指标"
                  options={metricOptions}
                  maxTagCount="responsive"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="compare_mode" label="对比方式">
                <Select options={compareModeOptions} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
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
      <Card title="统计看板">
        {statsQuery.error ? (
          <ErrorState
            title="统计数据加载失败"
            error={statsQuery.error}
            onRetry={() => statsQuery.refetch()}
          />
        ) : statsLoading ? (
          <LoadingSkeleton rows={4} />
        ) : diffRows.length ? (
          <Table
            size="small"
            rowKey="key"
            columns={diffColumns}
            dataSource={diffRows}
            pagination={false}
          />
        ) : (
          <EmptyState description="暂无统计数据" />
        )}
      </Card>
      <Card title="维度明细">
        {filters.dimension ? (
          statsQuery.error ? (
            <ErrorState
              title="维度统计加载失败"
              error={statsQuery.error}
              onRetry={() => statsQuery.refetch()}
            />
          ) : statsLoading ? (
            <LoadingSkeleton rows={3} />
          ) : breakdownData.length ? (
            <Table
              size="small"
              rowKey="dimension_label"
              columns={breakdownColumns}
              dataSource={breakdownData}
              pagination={false}
            />
          ) : (
            <EmptyState description="暂无维度统计数据" />
          )
        ) : (
          <EmptyState description="选择统计维度后展示明细" />
        )}
      </Card>
      <Card title="接入说明">
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="数据来源">{renderTags(dataSources)}</Descriptions.Item>
          <Descriptions.Item label="建议指标">
            {renderTags(metrics.map((metric) => metric.label))}
          </Descriptions.Item>
          <Descriptions.Item label="未接入原因">{renderTags(pendingReasons)}</Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  );
};
