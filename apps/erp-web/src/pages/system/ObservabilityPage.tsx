import { Button, Card, Col, Row, Space, Statistic, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type {
  DeadLetter,
  DeadLetterStatus,
  OutboxEvent,
  OutboxStatus
} from '@/api/failureQueue';
import { listDeadLetters, listOutboxEvents } from '@/api/failureQueue';
import { getStatsInfraHealth } from '@/api/stats';
import { fetchMetrics, type MetricsSnapshot } from '@/api/system';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const deadLetterStatusColors: Record<DeadLetterStatus, string> = {
  PENDING: 'orange',
  REPLAYED: 'green'
};

const outboxStatusColors: Record<OutboxStatus, string> = {
  PENDING: 'blue',
  FAILED: 'red',
  DISPATCHING: 'orange',
  SENT: 'green',
  DEAD: 'volcano'
};

const metricValue = (metrics: MetricsSnapshot | undefined, key: string) =>
  metrics?.[key] ?? 0;

const averageMetric = (
  metrics: MetricsSnapshot | undefined,
  sumKey: string,
  countKey: string
) => {
  const count = metricValue(metrics, countKey);
  if (!count) {
    return 0;
  }
  return Math.round(metricValue(metrics, sumKey) / count);
};

export const ObservabilityPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();

  const metricsQuery = useQuery({
    queryKey: ['metrics'],
    queryFn: fetchMetrics
  });
  const statsInfraQuery = useQuery({
    queryKey: ['stats-infra-health', tenantId],
    queryFn: () => getStatsInfraHealth(tenantId)
  });

  const deadLettersQuery = useQuery({
    queryKey: ['dead-letters', tenantId, 'observability'],
    queryFn: () => listDeadLetters(tenantId, { status: 'PENDING', limit: 5 })
  });

  const outboxEventsQuery = useQuery({
    queryKey: ['outbox-events', tenantId, 'observability'],
    queryFn: () => listOutboxEvents(tenantId, { status: 'FAILED', limit: 5 })
  });

  const metricGroups = useMemo(() => {
    const metrics = metricsQuery.data;
    return [
      {
        title: 'Webhook',
        items: [
          {
            label: '接收次数',
            value: metricValue(metrics, 'erp_webhook_inbox_received_total')
          },
          {
            label: '去重命中',
            value: metricValue(metrics, 'erp_webhook_inbox_dedup_hit_total')
          }
        ]
      },
      {
        title: 'Worker',
        items: [
          {
            label: '成功消费',
            value: metricValue(metrics, 'erp_worker_consume_success_total')
          },
          {
            label: '失败消费',
            value: metricValue(metrics, 'erp_worker_consume_failed_total')
          },
          {
            label: 'DLQ累计',
            value: metricValue(metrics, 'erp_worker_dlq_total')
          },
          {
            label: 'Outbox堆积',
            value: metricValue(metrics, 'erp_outbox_backlog')
          }
        ]
      },
      {
        title: '发货',
        items: [
          {
            label: '发货成功',
            value: metricValue(metrics, 'erp_shipping_success_total')
          },
          {
            label: '发货失败',
            value: metricValue(metrics, 'erp_shipping_failed_total')
          },
          {
            label: '推送成功',
            value: metricValue(metrics, 'erp_shipping_push_success_total')
          },
          {
            label: '推送失败',
            value: metricValue(metrics, 'erp_shipping_push_failed_total')
          },
          {
            label: '推送重试',
            value: metricValue(metrics, 'erp_shipping_push_retry_total')
          },
          {
            label: '推送耗时',
            value: averageMetric(
              metrics,
              'erp_shipping_push_latency_ms_sum',
              'erp_shipping_push_latency_ms_count'
            ),
            suffix: 'ms'
          }
        ]
      },
      {
        title: '库存',
        items: [
          {
            label: '调整次数',
            value: metricValue(metrics, 'erp_inventory_adjust_total')
          },
          {
            label: '预留次数',
            value: metricValue(metrics, 'erp_inventory_reserve_total')
          },
          {
            label: '扣减次数',
            value: metricValue(metrics, 'erp_inventory_consume_total')
          },
          {
            label: '释放次数',
            value: metricValue(metrics, 'erp_inventory_return_total')
          }
        ]
      },
      {
        title: '打印',
        items: [
          {
            label: '成功任务',
            value: metricValue(metrics, 'erp_print_jobs_success_total')
          },
          {
            label: '失败任务',
            value: metricValue(metrics, 'erp_print_jobs_failed_total')
          }
        ]
      }
    ];
  }, [metricsQuery.data]);

  const deadLetterColumns = useMemo<ColumnsType<DeadLetter>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={deadLetterStatusColors[record.status]}>{record.status}</Tag>
        )
      },
      {
        title: '事件',
        dataIndex: 'event_type'
      },
      {
        title: '错误',
        dataIndex: 'error',
        ellipsis: true,
        render: (_, record) => record.error || '-'
      },
      {
        title: '次数',
        dataIndex: 'attempts'
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      }
    ],
    []
  );

  const outboxColumns = useMemo<ColumnsType<OutboxEvent>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={outboxStatusColors[record.status]}>{record.status}</Tag>
        )
      },
      {
        title: '事件',
        dataIndex: 'event_type'
      },
      {
        title: '次数',
        dataIndex: 'attempts'
      },
      {
        title: '错误',
        dataIndex: 'last_error',
        ellipsis: true,
        render: (_, record) => record.last_error || '-'
      },
      {
        title: '最近尝试',
        dataIndex: 'last_attempt_at',
        render: (_, record) => formatDateTime(record.last_attempt_at)
      }
    ],
    []
  );

  const deadLetters = deadLettersQuery.data?.data ?? [];
  const outboxEvents = outboxEventsQuery.data?.data ?? [];
  const showDeadLetterError = deadLettersQuery.error && deadLetters.length === 0;
  const showOutboxError = outboxEventsQuery.error && outboxEvents.length === 0;

  const handleRefresh = () => {
    metricsQuery.refetch();
    statsInfraQuery.refetch();
    deadLettersQuery.refetch();
    outboxEventsQuery.refetch();
  };

  return (
    <UiModule07SettingsLayout title="可观测性面板">
      <UiModule07SettingsSection
        title="关键指标"
        actions={
          <Button onClick={handleRefresh}>刷新</Button>
        }
      >
        <Space wrap style={{ marginBottom: 16 }} data-testid="observability-stats-infra-health">
          <Tag color="blue">分析引擎: {statsInfraQuery.data?.analytics_backend ?? 'POSTGRES'}</Tag>
          <Tag color={statsInfraQuery.data?.clickhouse_ready ? 'green' : 'default'}>
            ClickHouse: {statsInfraQuery.data?.clickhouse_ready ? '已接入' : '未接入'}
          </Tag>
          <Tag color={statsInfraQuery.data?.sso_ready ? 'green' : 'default'}>
            Keycloak SSO: {statsInfraQuery.data?.sso_ready ? '已接入' : '未接入'}
          </Tag>
        </Space>
        {metricsQuery.error ? (
          <ErrorState
            title="指标加载失败"
            error={metricsQuery.error}
            onRetry={() => metricsQuery.refetch()}
          />
        ) : metricsQuery.isLoading && !metricsQuery.data ? (
          <LoadingState loading={metricsQuery.isLoading} onRetry={() => metricsQuery.refetch()} />
        ) : (
          <Row gutter={[16, 16]}>
            {metricGroups.map((group) => (
              <Col xs={24} md={12} xl={8} key={group.title}>
                <Card size="small" title={group.title}>
                  <Row gutter={[16, 12]}>
                    {group.items.map((item) => (
                      <Col xs={12} key={item.label}>
                        <Statistic
                          title={item.label}
                          value={item.value}
                          suffix={item.suffix}
                        />
                      </Col>
                    ))}
                  </Row>
                </Card>
              </Col>
            ))}
          </Row>
        )}
      </UiModule07SettingsSection>
      <UiModule07SettingsSection title="异常队列">
        <Row gutter={[16, 16]}>
          <Col xs={24} lg={12}>
            <Card
              title="待处理死信"
              extra={
                <Button type="link" onClick={() => navigate('/failure-queue')}>
                  查看全部
                </Button>
              }
            >
              {showDeadLetterError ? (
                <ErrorState
                  title="死信列表加载失败"
                  error={deadLettersQuery.error}
                  onRetry={() => deadLettersQuery.refetch()}
                />
              ) : (
                <Table<DeadLetter>
                  rowKey="dead_letter_id"
                  columns={deadLetterColumns}
                  dataSource={deadLetters}
                  loading={deadLettersQuery.isFetching}
                  pagination={false}
                  size="small"
                  locale={{ emptyText: '暂无待处理死信' }}
                />
              )}
            </Card>
          </Col>
          <Col xs={24} lg={12}>
            <Card
              title="Outbox异常事件"
              extra={
                <Button type="link" onClick={() => navigate('/failure-queue')}>
                  查看全部
                </Button>
              }
            >
              {showOutboxError ? (
                <ErrorState
                  title="Outbox列表加载失败"
                  error={outboxEventsQuery.error}
                  onRetry={() => outboxEventsQuery.refetch()}
                />
              ) : (
                <Table<OutboxEvent>
                  rowKey="outbox_id"
                  columns={outboxColumns}
                  dataSource={outboxEvents}
                  loading={outboxEventsQuery.isFetching}
                  pagination={false}
                  size="small"
                  locale={{ emptyText: '暂无Outbox异常事件' }}
                />
              )}
            </Card>
          </Col>
        </Row>
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};
