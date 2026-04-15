import {
  Button,
  Card,
  Col,
  Input,
  Row,
  Select,
  Space,
  Statistic,
  Table,
  Tag,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { DeadLetter, DeadLetterStatus, OutboxEvent, OutboxStatus } from '@/api/failureQueue';
import { listDeadLetters, listOutboxEvents } from '@/api/failureQueue';
import type { MetricsSnapshot } from '@/api/system';
import { fetchMetrics } from '@/api/system';
import type {
  FrontendRouteDiagnosticEventType,
  FrontendRouteDiagnosticReport,
  FrontendRouteDiagnosticStatus,
  FrontendRouteDiagnosticsPathSummary,
} from '@/api/frontendRouteDiagnostics';
import { getFrontendRouteDiagnostics } from '@/api/frontendRouteDiagnostics';
import { getStatsInfraHealth } from '@/api/stats';
import {
  ROUTE_LOADING_TIMEOUT_MS,
  getAllRouteLoadingSnapshots,
  getRouteDiagnosticEvents,
  subscribeRouteLoadingDiagnostics,
} from '@/app/navigation/routeLoadingDiagnostics';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

import type {
  FrontendRouteBackendEfficiencySummary,
  FrontendRouteEventRow,
  FrontendRouteEfficiencySummary,
  FrontendRouteHealthRow,
  FrontendRoutePathRankRow,
  FrontendRouteTrendRow,
  FrontendRouteVersionRow,
} from './frontendRouteObservability';
import {
  buildFrontendRouteBackendEfficiencySummary,
  buildFrontendRouteEventRows,
  buildFrontendRouteEfficiencySummary,
  buildFrontendRouteHealthRows,
  buildFrontendRouteHealthSummary,
  buildFrontendRoutePathRankRows,
  buildFrontendRouteTrendRows,
  buildFrontendRouteVersionRows,
} from './frontendRouteObservability';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const FRONTEND_ROUTE_BACKEND_AUTO_REFRESH_DELAY_MS = 1500;

const formatRateText = (numerator: number, denominator: number) => {
  if (!denominator) {
    return '-';
  }
  return `${Math.round((numerator / denominator) * 100)}% (${numerator}/${denominator})`;
};

const formatDurationMs = (value?: number | null) =>
  typeof value === 'number' && Number.isFinite(value) ? `${Math.round(value)}ms` : '-';

const deadLetterStatusColors: Record<DeadLetterStatus, string> = {
  PENDING: 'orange',
  REPLAYED: 'green',
};

const outboxStatusColors: Record<OutboxStatus, string> = {
  PENDING: 'blue',
  FAILED: 'red',
  DISPATCHING: 'orange',
  SENT: 'green',
  DEAD: 'volcano',
};

const routeToneColors = {
  default: 'default',
  processing: 'blue',
  success: 'green',
  warning: 'gold',
  error: 'red',
} as const;

const frontendRouteDiagnosticStatusOptions: Array<{
  label: string;
  value: FrontendRouteDiagnosticStatus | 'all';
}> = [
  { label: '全部状态', value: 'all' },
  { label: '正常', value: 'ready' },
  { label: '超时', value: 'timeout' },
  { label: '异常', value: 'error' },
];

const frontendRouteDiagnosticEventTypeOptions: Array<{
  label: string;
  value: FrontendRouteDiagnosticEventType | 'all';
}> = [
  { label: '全部事件', value: 'all' },
  { label: '加载完成', value: 'loading-ready' },
  { label: '加载超时', value: 'loading-timeout' },
  { label: '加载失败', value: 'loading-error' },
  { label: '预热完成', value: 'prefetch-ready' },
  { label: '预热失败', value: 'prefetch-error' },
  { label: '预热跳过', value: 'prefetch-skipped' },
];

const frontendRouteDiagnosticWindowOptions = [
  { label: '近 1 小时', value: 1 },
  { label: '近 6 小时', value: 6 },
  { label: '近 24 小时', value: 24 },
  { label: '近 72 小时', value: 72 },
];

const metricValue = (metrics: MetricsSnapshot | undefined, key: string) => metrics?.[key] ?? 0;

const averageMetric = (metrics: MetricsSnapshot | undefined, sumKey: string, countKey: string) => {
  const count = metricValue(metrics, countKey);
  if (!count) {
    return 0;
  }
  return Math.round(metricValue(metrics, sumKey) / count);
};

const percentageMetric = (numerator: number, denominator: number) => {
  if (!denominator) {
    return 0;
  }
  return Math.round((numerator / denominator) * 1000) / 10;
};

const getBackendPathHealthDescriptor = (row: FrontendRouteDiagnosticsPathSummary) => {
  if (row.errorEvents > 0) {
    return { label: '异常', tone: 'error' as const };
  }
  if (row.timeoutEvents > 0) {
    return { label: '超时', tone: 'warning' as const };
  }
  if (row.slowEvents > 0) {
    return { label: '偏慢', tone: 'warning' as const };
  }
  if (row.readyEvents > 0) {
    return { label: '正常', tone: 'success' as const };
  }
  return { label: '待观察', tone: 'default' as const };
};

const getBackendEventDescriptor = (event: FrontendRouteDiagnosticReport) => {
  switch (event.event_type) {
    case 'loading-ready':
      return { label: '加载完成', tone: 'success' as const };
    case 'loading-timeout':
      return { label: '加载超时', tone: 'warning' as const };
    case 'loading-error':
      return { label: '加载失败', tone: 'error' as const };
    case 'prefetch-ready':
      return { label: '预热完成', tone: 'success' as const };
    case 'prefetch-error':
      return { label: '预热失败', tone: 'error' as const };
    case 'prefetch-skipped':
      return { label: '预热跳过', tone: 'default' as const };
    default:
      return { label: event.event_type, tone: 'default' as const };
  }
};

export const ObservabilityPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const [routeSnapshots, setRouteSnapshots] = useState(() => getAllRouteLoadingSnapshots());
  const [routeEvents, setRouteEvents] = useState(() => getRouteDiagnosticEvents());
  const [backendAutoRefreshKey, setBackendAutoRefreshKey] = useState('');
  const [backendPathKeyword, setBackendPathKeyword] = useState('');
  const [backendStatus, setBackendStatus] = useState<FrontendRouteDiagnosticStatus | 'all'>('all');
  const [backendEventType, setBackendEventType] = useState<
    FrontendRouteDiagnosticEventType | 'all'
  >('all');
  const [backendWindowHours, setBackendWindowHours] = useState(24);

  useEffect(() => {
    const syncDiagnostics = () => {
      setRouteSnapshots(getAllRouteLoadingSnapshots());
      setRouteEvents(getRouteDiagnosticEvents());
    };

    syncDiagnostics();
    return subscribeRouteLoadingDiagnostics(syncDiagnostics);
  }, []);

  const metricsQuery = useQuery({
    queryKey: ['metrics'],
    queryFn: fetchMetrics,
  });
  const statsInfraQuery = useQuery({
    queryKey: ['stats-infra-health', tenantId],
    queryFn: () => getStatsInfraHealth(tenantId),
    enabled: Boolean(tenantId),
  });

  const deadLettersQuery = useQuery({
    queryKey: ['dead-letters', tenantId, 'observability'],
    queryFn: () => listDeadLetters(tenantId, { status: 'PENDING', limit: 5 }),
    enabled: Boolean(tenantId),
  });

  const outboxEventsQuery = useQuery({
    queryKey: ['outbox-events', tenantId, 'observability'],
    queryFn: () => listOutboxEvents(tenantId, { status: 'FAILED', limit: 5 }),
    enabled: Boolean(tenantId),
  });
  const frontendRouteBackendQuery = useQuery({
    queryKey: [
      'frontend-route-diagnostics',
      tenantId,
      backendPathKeyword,
      backendStatus,
      backendEventType,
      backendWindowHours,
    ],
    queryFn: () =>
      getFrontendRouteDiagnostics(tenantId, {
        limit: 120,
        pathKeyword: backendPathKeyword,
        status: backendStatus,
        eventType: backendEventType,
        windowHours: backendWindowHours,
      }),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData,
    retry: false,
  });

  const routeHealthSummary = useMemo(
    () => buildFrontendRouteHealthSummary(routeSnapshots, ROUTE_LOADING_TIMEOUT_MS),
    [routeSnapshots],
  );
  const routeEfficiencySummary = useMemo<FrontendRouteEfficiencySummary>(
    () => buildFrontendRouteEfficiencySummary(routeSnapshots),
    [routeSnapshots],
  );
  const frontendRouteBackendEfficiencySummary = useMemo<FrontendRouteBackendEfficiencySummary>(
    () => buildFrontendRouteBackendEfficiencySummary(frontendRouteBackendQuery.data?.summary),
    [frontendRouteBackendQuery.data?.summary],
  );
  const routeHealthRows = useMemo<FrontendRouteHealthRow[]>(
    () => buildFrontendRouteHealthRows(routeSnapshots, ROUTE_LOADING_TIMEOUT_MS),
    [routeSnapshots],
  );
  const routeEventRows = useMemo<FrontendRouteEventRow[]>(
    () => buildFrontendRouteEventRows(routeEvents),
    [routeEvents],
  );
  const frontendRouteTrendRows = useMemo<FrontendRouteTrendRow[]>(
    () => buildFrontendRouteTrendRows(frontendRouteBackendQuery.data?.events ?? [], backendWindowHours, 6),
    [backendWindowHours, frontendRouteBackendQuery.data?.events],
  );
  const frontendRoutePathRankRows = useMemo<FrontendRoutePathRankRow[]>(
    () => buildFrontendRoutePathRankRows(frontendRouteBackendQuery.data?.paths ?? []),
    [frontendRouteBackendQuery.data?.paths],
  );
  const frontendRouteVersionRows = useMemo<FrontendRouteVersionRow[]>(
    () => buildFrontendRouteVersionRows(frontendRouteBackendQuery.data?.events ?? []),
    [frontendRouteBackendQuery.data?.events],
  );

  const metricGroups = useMemo(() => {
    const metrics = metricsQuery.data;
    const afterSalesSlaOverdue = metricValue(metrics, 'erp_after_sales_sla_overdue_total');
    const afterSalesSlaEvaluated = metricValue(metrics, 'erp_after_sales_sla_evaluated_total');
    const afterSalesSlaOverdueRate = percentageMetric(afterSalesSlaOverdue, afterSalesSlaEvaluated);
    return [
      {
        title: 'Webhook',
        items: [
          {
            label: '接收次数',
            value: metricValue(metrics, 'erp_webhook_inbox_received_total'),
          },
          {
            label: '去重命中',
            value: metricValue(metrics, 'erp_webhook_inbox_dedup_hit_total'),
          },
        ],
      },
      {
        title: '任务消费',
        items: [
          {
            label: '成功消费',
            value: metricValue(metrics, 'erp_worker_consume_success_total'),
          },
          {
            label: '失败消费',
            value: metricValue(metrics, 'erp_worker_consume_failed_total'),
          },
          {
            label: 'DLQ 累计',
            value: metricValue(metrics, 'erp_worker_dlq_total'),
          },
          {
            label: 'Outbox 堆积',
            value: metricValue(metrics, 'erp_outbox_backlog'),
          },
        ],
      },
      {
        title: '发货链路',
        items: [
          {
            label: '发货成功',
            value: metricValue(metrics, 'erp_shipping_success_total'),
          },
          {
            label: '发货失败',
            value: metricValue(metrics, 'erp_shipping_failed_total'),
          },
          {
            label: '推送成功',
            value: metricValue(metrics, 'erp_shipping_push_success_total'),
          },
          {
            label: '推送失败',
            value: metricValue(metrics, 'erp_shipping_push_failed_total'),
          },
          {
            label: '推送重试',
            value: metricValue(metrics, 'erp_shipping_push_retry_total'),
          },
          {
            label: '推送耗时',
            value: averageMetric(
              metrics,
              'erp_shipping_push_latency_ms_sum',
              'erp_shipping_push_latency_ms_count',
            ),
            suffix: 'ms',
          },
        ],
      },
      {
        title: '库存链路',
        items: [
          {
            label: '调整次数',
            value: metricValue(metrics, 'erp_inventory_adjust_total'),
          },
          {
            label: '预留次数',
            value: metricValue(metrics, 'erp_inventory_reserve_total'),
          },
          {
            label: '扣减次数',
            value: metricValue(metrics, 'erp_inventory_consume_total'),
          },
          {
            label: '释放次数',
            value: metricValue(metrics, 'erp_inventory_return_total'),
          },
        ],
      },
      {
        title: '售后链路',
        items: [
          {
            label: '同步成功',
            value: metricValue(metrics, 'erp_after_sales_sync_success_total'),
          },
          {
            label: '补偿执行',
            value: metricValue(metrics, 'erp_after_sales_compensation_total'),
          },
          {
            label: 'SLA超时',
            value: afterSalesSlaOverdue,
          },
          {
            label: 'SLA评估总量',
            value: afterSalesSlaEvaluated,
          },
          {
            label: 'SLA超时率',
            value: afterSalesSlaOverdueRate,
            suffix: '%',
          },
          {
            label: '闭环耗时',
            value: averageMetric(
              metrics,
              'erp_after_sales_cycle_time_ms_sum',
              'erp_after_sales_cycle_time_ms_count',
            ),
            suffix: 'ms',
          },
        ],
      },
      {
        title: '打印链路',
        items: [
          {
            label: '成功任务',
            value: metricValue(metrics, 'erp_print_jobs_success_total'),
          },
          {
            label: '失败任务',
            value: metricValue(metrics, 'erp_print_jobs_failed_total'),
          },
        ],
      },
    ];
  }, [metricsQuery.data]);

  const routeHealthColumns = useMemo<ColumnsType<FrontendRouteHealthRow>>(
    () => [
      {
        title: '路由',
        dataIndex: 'path',
        width: 240,
        render: (_, record) => <Typography.Text code>{record.path}</Typography.Text>,
      },
      {
        title: '页面',
        dataIndex: 'routeLabel',
        width: 180,
        ellipsis: true,
      },
      {
        title: '状态',
        dataIndex: 'statusLabel',
        width: 120,
        render: (_, record) => (
          <Tag color={routeToneColors[record.statusTone]}>{record.statusLabel}</Tag>
        ),
      },
      {
        title: '加载耗时',
        dataIndex: 'durationText',
        width: 110,
      },
      {
        title: '首开画像',
        dataIndex: 'firstOpenText',
        width: 120,
      },
      {
        title: '预热状态',
        dataIndex: 'prefetchLabel',
        width: 160,
        render: (_, record) => (
          <Tag color={routeToneColors[record.prefetchTone]}>{record.prefetchLabel}</Tag>
        ),
      },
      {
        title: '预热耗时',
        dataIndex: 'prefetchDurationText',
        width: 110,
      },
      {
        title: '预热命中',
        dataIndex: 'prefetchHitText',
        width: 120,
      },
      {
        title: '短路复用',
        dataIndex: 'prefetchShortCircuitText',
        width: 100,
      },
      {
        title: '最近活动',
        dataIndex: 'updatedAtText',
        width: 180,
      },
      {
        title: '异常摘要',
        dataIndex: 'errorSummary',
        ellipsis: true,
        render: (_, record) => record.errorSummary || '-',
      },
    ],
    [],
  );

  const routeEventColumns = useMemo<ColumnsType<FrontendRouteEventRow>>(
    () => [
      {
        title: '时间',
        dataIndex: 'atText',
        width: 180,
      },
      {
        title: '路由',
        dataIndex: 'path',
        width: 240,
        render: (_, record) => <Typography.Text code>{record.path}</Typography.Text>,
      },
      {
        title: '事件',
        dataIndex: 'title',
        width: 160,
        render: (_, record) => <Tag color={routeToneColors[record.tone]}>{record.title}</Tag>,
      },
      {
        title: '明细',
        dataIndex: 'detail',
        ellipsis: true,
        render: (_, record) => record.detail || '-',
      },
    ],
    [],
  );

  const frontendRouteBackendPathColumns = useMemo<ColumnsType<FrontendRouteDiagnosticsPathSummary>>(
    () => [
      {
        title: '页面路由',
        dataIndex: 'path',
        width: 280,
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <Typography.Text code>{record.path}</Typography.Text>
            <Typography.Text type="secondary">{record.routeLabel ?? '-'}</Typography.Text>
          </Space>
        ),
      },
      {
        title: '健康度',
        key: 'health',
        width: 100,
        render: (_, record) => {
          const descriptor = getBackendPathHealthDescriptor(record);
          return <Tag color={routeToneColors[descriptor.tone]}>{descriptor.label}</Tag>;
        },
      },
      {
        title: '事件数',
        dataIndex: 'totalEvents',
        width: 90,
      },
      {
        title: '异常',
        dataIndex: 'errorEvents',
        width: 80,
      },
      {
        title: '超时',
        dataIndex: 'timeoutEvents',
        width: 80,
      },
      {
        title: '慢页',
        dataIndex: 'slowEvents',
        width: 80,
      },
      {
        title: '平均耗时',
        key: 'avgDurationMs',
        width: 100,
        render: (_, record) => formatDurationMs(record.avgDurationMs),
      },
      {
        title: '最大耗时',
        key: 'maxDurationMs',
        width: 100,
        render: (_, record) => formatDurationMs(record.maxDurationMs),
      },
      {
        title: '首开成功',
        key: 'firstLoadSuccess',
        width: 120,
        render: (_, record) =>
          formatRateText(record.successfulFirstLoads, record.measuredFirstLoads),
      },
      {
        title: '首开均耗时',
        key: 'firstLoadAvgDurationMs',
        width: 110,
        render: (_, record) => formatDurationMs(record.firstLoadAvgDurationMs),
      },
      {
        title: '短路复用',
        key: 'prefetchShortCircuitCount',
        width: 100,
        render: (_, record) =>
          record.prefetchShortCircuitCount > 0 ? `${record.prefetchShortCircuitCount} 次` : '-',
      },
      {
        title: '最近出现',
        dataIndex: 'lastSeenAt',
        width: 160,
        render: (_, record) => formatDateTime(record.lastSeenAt),
      },
    ],
    [],
  );

  const frontendRouteBackendEventColumns = useMemo<ColumnsType<FrontendRouteDiagnosticReport>>(
    () => [
      {
        title: '时间',
        dataIndex: 'created_at',
        width: 160,
        render: (_, record) =>
          formatDateTime(record.created_at ?? new Date(record.event_at).toISOString()),
      },
      {
        title: '路由',
        dataIndex: 'path',
        width: 260,
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <Typography.Text code>{record.path}</Typography.Text>
            <Typography.Text type="secondary">{record.route_label ?? '-'}</Typography.Text>
          </Space>
        ),
      },
      {
        title: '事件',
        key: 'event_type',
        width: 110,
        render: (_, record) => {
          const descriptor = getBackendEventDescriptor(record);
          return <Tag color={routeToneColors[descriptor.tone]}>{descriptor.label}</Tag>;
        },
      },
      {
        title: '状态',
        dataIndex: 'status',
        width: 90,
        render: (_, record) => {
          const tone =
            record.status === 'error'
              ? 'error'
              : record.status === 'timeout'
                ? 'warning'
                : record.status === 'ready'
                  ? 'success'
                  : 'default';
          const label =
            record.status === 'ready'
              ? '正常'
              : record.status === 'timeout'
                ? '超时'
                : record.status === 'error'
                  ? '异常'
                  : record.status;
          return <Tag color={routeToneColors[tone]}>{label}</Tag>;
        },
      },
      {
        title: '耗时',
        key: 'duration_ms',
        width: 90,
        render: (_, record) =>
          typeof record.duration_ms === 'number' ? `${record.duration_ms}ms` : '-',
      },
      {
        title: '预热',
        key: 'prefetch_status',
        width: 120,
        render: (_, record) => {
          if (!record.prefetch_status) {
            return '-';
          }
          const label =
            record.prefetch_status === 'ready'
              ? '命中预热'
              : record.prefetch_status === 'error'
                ? '预热失败'
                : record.prefetch_status === 'skipped'
                  ? '跳过预热'
                  : '预热中';
          return label;
        },
      },
      {
        title: '异常摘要',
        dataIndex: 'error_summary',
        ellipsis: true,
        render: (_, record) => record.error_summary || record.detail || '-',
      },
    ],
    [],
  );

  const frontendRouteTrendColumns = useMemo<ColumnsType<FrontendRouteTrendRow>>(
    () => [
      {
        title: '时间窗',
        dataIndex: 'bucketLabel',
        width: 150,
      },
      {
        title: '事件数',
        dataIndex: 'totalEvents',
        width: 90,
      },
      {
        title: '超时',
        dataIndex: 'timeoutEvents',
        width: 80,
      },
      {
        title: '异常',
        dataIndex: 'errorEvents',
        width: 80,
      },
      {
        title: '预热命中',
        dataIndex: 'prefetchHits',
        width: 100,
      },
    ],
    [],
  );

  const frontendRoutePathRankColumns = useMemo<ColumnsType<FrontendRoutePathRankRow>>(
    () => [
      {
        title: '页面',
        dataIndex: 'path',
        width: 240,
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <Typography.Text code>{record.path}</Typography.Text>
            <Typography.Text type="secondary">{record.routeLabel}</Typography.Text>
          </Space>
        ),
      },
      {
        title: '风险',
        dataIndex: 'riskLabel',
        width: 100,
        render: (_, record) => <Tag color={routeToneColors[record.riskTone]}>{record.riskLabel}</Tag>,
      },
      {
        title: '异常',
        dataIndex: 'errorEvents',
        width: 70,
      },
      {
        title: '超时',
        dataIndex: 'timeoutEvents',
        width: 70,
      },
      {
        title: '慢页',
        dataIndex: 'slowEvents',
        width: 70,
      },
      {
        title: '最近出现',
        dataIndex: 'lastSeenAtText',
        width: 160,
      },
    ],
    [],
  );

  const frontendRouteVersionColumns = useMemo<ColumnsType<FrontendRouteVersionRow>>(
    () => [
      {
        title: '版本',
        dataIndex: 'buildVersion',
        width: 180,
        render: (_, record) => <Typography.Text code>{record.buildVersion}</Typography.Text>,
      },
      {
        title: '健康',
        dataIndex: 'healthLabel',
        width: 90,
        render: (_, record) => (
          <Tag color={routeToneColors[record.healthTone]}>{record.healthLabel}</Tag>
        ),
      },
      {
        title: '事件',
        dataIndex: 'totalEvents',
        width: 70,
      },
      {
        title: '页面',
        dataIndex: 'affectedRoutes',
        width: 70,
      },
      {
        title: '会话',
        dataIndex: 'affectedSessions',
        width: 70,
      },
      {
        title: '最近出现',
        dataIndex: 'lastSeenAtText',
        width: 160,
      },
    ],
    [],
  );

  const deadLetterColumns = useMemo<ColumnsType<DeadLetter>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={deadLetterStatusColors[record.status]}>{record.status}</Tag>
        ),
      },
      {
        title: '事件',
        dataIndex: 'event_type',
      },
      {
        title: '错误',
        dataIndex: 'error',
        ellipsis: true,
        render: (_, record) => record.error || '-',
      },
      {
        title: '次数',
        dataIndex: 'attempts',
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at),
      },
    ],
    [],
  );

  const outboxColumns = useMemo<ColumnsType<OutboxEvent>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => <Tag color={outboxStatusColors[record.status]}>{record.status}</Tag>,
      },
      {
        title: '事件',
        dataIndex: 'event_type',
      },
      {
        title: '次数',
        dataIndex: 'attempts',
      },
      {
        title: '错误',
        dataIndex: 'last_error',
        ellipsis: true,
        render: (_, record) => record.last_error || '-',
      },
      {
        title: '最近尝试',
        dataIndex: 'last_attempt_at',
        render: (_, record) => formatDateTime(record.last_attempt_at),
      },
    ],
    [],
  );

  const deadLetters = deadLettersQuery.data?.data ?? [];
  const outboxEvents = outboxEventsQuery.data?.data ?? [];
  const frontendRouteBackendSummary = frontendRouteBackendQuery.data?.summary;
  const frontendRouteBackendPaths = frontendRouteBackendQuery.data?.paths ?? [];
  const frontendRouteBackendEvents = frontendRouteBackendQuery.data?.events ?? [];
  const showDeadLetterError = deadLettersQuery.error && deadLetters.length === 0;
  const showOutboxError = outboxEventsQuery.error && outboxEvents.length === 0;
  const showFrontendRouteBackendError =
    frontendRouteBackendQuery.error &&
    frontendRouteBackendPaths.length === 0 &&
    frontendRouteBackendEvents.length === 0;
  useEffect(() => {
    const hasLocalRouteSignals = routeEvents.length > 0 || routeSnapshots.length > 0;
    const hasBackendTelemetry =
      frontendRouteBackendPaths.length > 0 || frontendRouteBackendEvents.length > 0;
    const nextAutoRefreshKey = [
      tenantId ?? '',
      backendPathKeyword.trim(),
      backendStatus,
      backendEventType,
      String(backendWindowHours),
      String(routeEvents.length),
      String(routeSnapshots.length),
    ].join('|');

    if (!tenantId || frontendRouteBackendQuery.isFetching || !hasLocalRouteSignals || hasBackendTelemetry) {
      if (backendAutoRefreshKey) {
        setBackendAutoRefreshKey('');
      }
      return;
    }

    if (backendAutoRefreshKey === nextAutoRefreshKey) {
      return;
    }

    setBackendAutoRefreshKey(nextAutoRefreshKey);
    const timer = window.setTimeout(() => {
      void frontendRouteBackendQuery.refetch();
    }, FRONTEND_ROUTE_BACKEND_AUTO_REFRESH_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [
    backendAutoRefreshKey,
    backendEventType,
    backendPathKeyword,
    backendStatus,
    backendWindowHours,
    frontendRouteBackendEvents.length,
    frontendRouteBackendPaths.length,
    frontendRouteBackendQuery,
    routeEvents.length,
    routeSnapshots.length,
    tenantId,
  ]);

  const handleRefresh = () => {
    metricsQuery.refetch();
    statsInfraQuery.refetch();
    deadLettersQuery.refetch();
    outboxEventsQuery.refetch();
    frontendRouteBackendQuery.refetch();
    setRouteSnapshots(getAllRouteLoadingSnapshots());
    setRouteEvents(getRouteDiagnosticEvents());
  };

  return (
    <UiModule07SettingsLayout title="系统可观测性">
      <UiModule07SettingsSection
        title="关键指标"
        description="汇总后端链路健康、基础设施接入状态与核心操作指标。"
        actions={<Button onClick={handleRefresh}>刷新</Button>}
      >
        <Space wrap style={{ marginBottom: 16 }} data-testid="observability-stats-infra-health">
          <Tag color="blue">分析引擎：{statsInfraQuery.data?.analytics_backend ?? 'POSTGRES'}</Tag>
          <Tag color={statsInfraQuery.data?.clickhouse_ready ? 'green' : 'default'}>
            ClickHouse：{statsInfraQuery.data?.clickhouse_ready ? '已接入' : '未接入'}
          </Tag>
          <Tag color={statsInfraQuery.data?.sso_ready ? 'green' : 'default'}>
            Keycloak SSO：{statsInfraQuery.data?.sso_ready ? '已接入' : '未接入'}
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
                        <Statistic title={item.label} value={item.value} suffix={item.suffix} />
                      </Col>
                    ))}
                  </Row>
                </Card>
              </Col>
            ))}
          </Row>
        )}
      </UiModule07SettingsSection>

      <UiModule07SettingsSection
        title="前端路由"
        description="聚焦当前浏览器窗口的首开懒加载、路由超时与预热命中情况，问题路由会自动排在前面。"
      >
        <Space wrap style={{ marginBottom: 8 }}>
          <Tag color="blue">仅当前浏览器窗口会话</Tag>
          <Tag color="gold">加载超时阈值：{ROUTE_LOADING_TIMEOUT_MS}ms</Tag>
          <Typography.Text type="secondary">
            这组数据不依赖后端上报，适合复盘“首次打开页面卡住”这类前端链路问题。
          </Typography.Text>
        </Space>
        <Row
          gutter={[16, 16]}
          style={{ marginBottom: 8 }}
          data-testid="observability-frontend-route-summary"
        >
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="已跟踪路由" value={routeHealthSummary.trackedRoutes} />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="正常路由" value={routeHealthSummary.readyRoutes} />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="加载中" value={routeHealthSummary.loadingRoutes} />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="超时路由" value={routeHealthSummary.timeoutRoutes} />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="异常路由" value={routeHealthSummary.errorRoutes} />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="慢路由" value={routeHealthSummary.slowRoutes} />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="命中预热" value={routeHealthSummary.prefetchedRoutes} />
            </Card>
          </Col>
        </Row>
        <Row
          gutter={[16, 16]}
          style={{ marginBottom: 8 }}
          data-testid="observability-frontend-route-efficiency"
        >
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic title="已测首开" value={routeEfficiencySummary.measuredFirstLoads} />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic
                title="首开成功率"
                value={routeEfficiencySummary.firstLoadSuccessRate}
                suffix="%"
              />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic
                title="首开均耗时"
                value={routeEfficiencySummary.firstLoadAvgDurationMs ?? '-'}
                suffix={routeEfficiencySummary.firstLoadAvgDurationMs ? 'ms' : undefined}
              />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic
                title="预热命中率"
                value={routeEfficiencySummary.prefetchHitRate}
                suffix="%"
              />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic
                title="预热命中次数"
                value={routeEfficiencySummary.prefetchHitCount}
              />
            </Card>
          </Col>
          <Col xs={12} md={8} xl={4}>
            <Card size="small">
              <Statistic
                title="重复预热短路"
                value={routeEfficiencySummary.prefetchShortCircuitCount}
              />
            </Card>
          </Col>
        </Row>
        <Row gutter={[16, 16]}>
          <Col xs={24} xl={14}>
            <Card title="路由健康" size="small">
              <Table<FrontendRouteHealthRow>
                rowKey="key"
                columns={routeHealthColumns}
                dataSource={routeHealthRows}
                pagination={false}
                size="small"
                scroll={{ x: 1340 }}
                locale={{ emptyText: '当前窗口还没有产生路由诊断数据' }}
              />
            </Card>
          </Col>
          <Col xs={24} xl={10}>
            <Card title="最近事件" size="small">
              <Table<FrontendRouteEventRow>
                rowKey="key"
                columns={routeEventColumns}
                dataSource={routeEventRows.slice(0, 12)}
                pagination={false}
                size="small"
                scroll={{ x: 720 }}
                locale={{ emptyText: '当前窗口还没有路由事件' }}
              />
            </Card>
          </Col>
        </Row>
      </UiModule07SettingsSection>

      <UiModule07SettingsSection
        title="前端留痕"
        description="查看已写入后端观测体系的前端路由历史，可按页面、状态和事件类型聚合分析跨页面首开问题。"
        actions={
          <Space wrap>
            <Input
              allowClear
              value={backendPathKeyword}
              onChange={(event) => setBackendPathKeyword(event.target.value)}
              placeholder="筛选路由关键字"
              style={{ width: 180 }}
            />
            <Select
              value={backendStatus}
              onChange={setBackendStatus}
              options={frontendRouteDiagnosticStatusOptions}
              style={{ width: 120 }}
            />
            <Select
              value={backendEventType}
              onChange={setBackendEventType}
              options={frontendRouteDiagnosticEventTypeOptions}
              style={{ width: 140 }}
            />
            <Select
              value={backendWindowHours}
              onChange={setBackendWindowHours}
              options={frontendRouteDiagnosticWindowOptions}
              style={{ width: 120 }}
            />
          </Space>
        }
      >
        <Space wrap style={{ marginBottom: 8 }}>
          <Tag color="blue">系统级历史观测</Tag>
          <Tag color="green">支持按页面聚合</Tag>
          <Typography.Text type="secondary">
            这组数据来自前端终态事件上报，适合复盘“哪个页面首次打开慢、超时多、异常多”。
          </Typography.Text>
        </Space>
        {showFrontendRouteBackendError ? (
          <ErrorState
            title="前端留痕加载失败"
            error={frontendRouteBackendQuery.error}
            onRetry={() => frontendRouteBackendQuery.refetch()}
          />
        ) : frontendRouteBackendQuery.isLoading && !frontendRouteBackendQuery.data ? (
          <LoadingState
            loading={frontendRouteBackendQuery.isLoading}
            onRetry={() => frontendRouteBackendQuery.refetch()}
          />
        ) : (
          <>
            <Row
              gutter={[16, 16]}
              style={{ marginBottom: 8 }}
              data-testid="observability-frontend-route-backend-summary"
            >
              <Col xs={12} md={8} xl={4}>
                <Card size="small">
                  <Statistic
                    title="历史事件"
                    value={frontendRouteBackendSummary?.totalEvents ?? 0}
                  />
                </Card>
              </Col>
              <Col xs={12} md={8} xl={4}>
                <Card size="small">
                  <Statistic
                    title="影响页面"
                    value={frontendRouteBackendSummary?.uniqueRoutes ?? 0}
                  />
                </Card>
              </Col>
              <Col xs={12} md={8} xl={4}>
                <Card size="small">
                  <Statistic
                    title="影响会话"
                    value={frontendRouteBackendSummary?.affectedSessions ?? 0}
                  />
                </Card>
              </Col>
              <Col xs={12} md={8} xl={4}>
                <Card size="small">
                  <Statistic
                    title="异常事件"
                    value={frontendRouteBackendSummary?.errorEvents ?? 0}
                  />
                </Card>
              </Col>
              <Col xs={12} md={8} xl={4}>
                <Card size="small">
                  <Statistic
                    title="超时事件"
                    value={frontendRouteBackendSummary?.timeoutEvents ?? 0}
                  />
                </Card>
              </Col>
              <Col xs={12} md={8} xl={4}>
                <Card size="small">
                  <Statistic
                    title="预热命中"
                    value={frontendRouteBackendSummary?.prefetchHits ?? 0}
                  />
                </Card>
              </Col>
            </Row>
            <Row
              gutter={[16, 16]}
              style={{ marginBottom: 8 }}
              data-testid="observability-frontend-route-backend-efficiency"
            >
              <Col xs={12} md={12} xl={6}>
                <Card size="small">
                  <Statistic
                    title="历史首开样本"
                    value={frontendRouteBackendEfficiencySummary.measuredFirstLoads}
                  />
                </Card>
              </Col>
              <Col xs={12} md={12} xl={6}>
                <Card size="small">
                  <Statistic
                    title="首开成功率"
                    value={frontendRouteBackendEfficiencySummary.firstLoadSuccessRate}
                    suffix="%"
                  />
                </Card>
              </Col>
              <Col xs={12} md={12} xl={6}>
                <Card size="small">
                  <Statistic
                    title="历史首开均耗时"
                    value={frontendRouteBackendEfficiencySummary.firstLoadAvgDurationMs ?? '-'}
                    suffix={
                      frontendRouteBackendEfficiencySummary.firstLoadAvgDurationMs ? 'ms' : undefined
                    }
                  />
                </Card>
              </Col>
              <Col xs={12} md={12} xl={6}>
                <Card size="small">
                  <Statistic
                    title="预热短路复用"
                    value={frontendRouteBackendEfficiencySummary.prefetchShortCircuitCount}
                  />
                </Card>
              </Col>
            </Row>
            <Row gutter={[16, 16]} style={{ marginBottom: 8 }}>
              <Col xs={24} xl={10}>
                <Card title="趋势回放" size="small">
                  <div data-testid="observability-frontend-route-backend-trend">
                    <Table<FrontendRouteTrendRow>
                      rowKey="key"
                      columns={frontendRouteTrendColumns}
                      dataSource={frontendRouteTrendRows}
                      pagination={false}
                      size="small"
                      loading={frontendRouteBackendQuery.isFetching}
                      scroll={{ x: 520 }}
                      locale={{ emptyText: '当前时间窗暂无趋势样本' }}
                    />
                  </div>
                </Card>
              </Col>
              <Col xs={24} xl={7}>
                <Card title="异常排行" size="small">
                  <Table<FrontendRoutePathRankRow>
                    rowKey="key"
                    columns={frontendRoutePathRankColumns}
                    dataSource={frontendRoutePathRankRows.slice(0, 6)}
                    pagination={false}
                    size="small"
                    loading={frontendRouteBackendQuery.isFetching}
                    scroll={{ x: 700 }}
                    locale={{ emptyText: '当前筛选条件下暂无异常排行' }}
                  />
                </Card>
              </Col>
              <Col xs={24} xl={7}>
                <Card title="版本画像" size="small">
                  <Table<FrontendRouteVersionRow>
                    rowKey="key"
                    columns={frontendRouteVersionColumns}
                    dataSource={frontendRouteVersionRows.slice(0, 6)}
                    pagination={false}
                    size="small"
                    loading={frontendRouteBackendQuery.isFetching}
                    scroll={{ x: 640 }}
                    locale={{ emptyText: '当前筛选条件下暂无版本画像' }}
                  />
                </Card>
              </Col>
            </Row>
            <Row gutter={[16, 16]}>
              <Col xs={24} xl={15}>
                <Card title="页面聚合" size="small">
                  <div data-testid="observability-frontend-route-backend-table">
                    <Table<FrontendRouteDiagnosticsPathSummary>
                      rowKey="path"
                      columns={frontendRouteBackendPathColumns}
                      dataSource={frontendRouteBackendPaths}
                      pagination={false}
                      size="small"
                      loading={frontendRouteBackendQuery.isFetching}
                      scroll={{ x: 1460 }}
                      locale={{ emptyText: '当前筛选条件下暂无前端留痕' }}
                    />
                  </div>
                </Card>
              </Col>
              <Col xs={24} xl={9}>
                <Card title="最近留痕事件" size="small">
                  <Table<FrontendRouteDiagnosticReport>
                    rowKey={(record) =>
                      `${record.session_id}-${record.path}-${record.event_type}-${record.event_at}`
                    }
                    columns={frontendRouteBackendEventColumns}
                    dataSource={frontendRouteBackendEvents}
                    pagination={false}
                    size="small"
                    loading={frontendRouteBackendQuery.isFetching}
                    scroll={{ x: 920 }}
                    locale={{ emptyText: '当前筛选条件下暂无留痕事件' }}
                  />
                </Card>
              </Col>
            </Row>
          </>
        )}
      </UiModule07SettingsSection>

      <UiModule07SettingsSection
        title="异常队列"
        description="集中查看待处理死信与失败 Outbox 事件，便于排查异步链路阻塞。"
      >
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
              title="Outbox 异常事件"
              extra={
                <Button type="link" onClick={() => navigate('/failure-queue')}>
                  查看全部
                </Button>
              }
            >
              {showOutboxError ? (
                <ErrorState
                  title="Outbox 列表加载失败"
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
                  locale={{ emptyText: '暂无 Outbox 异常事件' }}
                />
              )}
            </Card>
          </Col>
        </Row>
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};

