import dayjs from 'dayjs';

import type {
  FrontendRouteDiagnosticReport,
  FrontendRouteDiagnosticsSummary,
  FrontendRouteDiagnosticsPathSummary,
} from '@/api/system';
import type {
  RouteDiagnosticEvent,
  RouteLoadingSnapshot,
  RoutePrefetchSkipReason,
  RoutePrefetchSource,
} from '@/app/navigation/routeLoadingDiagnostics';

type FrontendRouteTone = 'default' | 'processing' | 'success' | 'warning' | 'error';

export type FrontendRouteHealthSummary = {
  trackedRoutes: number;
  readyRoutes: number;
  loadingRoutes: number;
  timeoutRoutes: number;
  errorRoutes: number;
  slowRoutes: number;
  prefetchedRoutes: number;
};

export type FrontendRouteEfficiencySummary = {
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadSuccessRate: number;
  firstLoadAvgDurationMs?: number;
  totalReadyEvents: number;
  prefetchHitCount: number;
  prefetchHitRate: number;
  prefetchShortCircuitCount: number;
};

export type FrontendRouteBackendEfficiencySummary = {
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadSuccessRate: number;
  firstLoadSuccessText: string;
  firstLoadAvgDurationMs?: number;
  prefetchShortCircuitCount: number;
};

export type FrontendRouteHealthRow = {
  key: string;
  path: string;
  routeLabel: string;
  statusLabel: string;
  statusTone: FrontendRouteTone;
  durationText: string;
  firstOpenText: string;
  prefetchLabel: string;
  prefetchTone: FrontendRouteTone;
  prefetchDurationText: string;
  prefetchHitText: string;
  prefetchShortCircuitText: string;
  errorSummary?: string;
  updatedAt?: number;
  updatedAtText: string;
};

export type FrontendRouteEventRow = {
  key: string;
  path: string;
  title: string;
  detail?: string;
  tone: FrontendRouteTone;
  at: number;
  atText: string;
};

export type FrontendRouteTrendRow = {
  key: string;
  bucketStartAt: number;
  bucketEndAt: number;
  bucketLabel: string;
  totalEvents: number;
  timeoutEvents: number;
  errorEvents: number;
  prefetchHits: number;
};

export type FrontendRoutePathRankRow = {
  key: string;
  path: string;
  routeLabel: string;
  riskLabel: string;
  riskTone: FrontendRouteTone;
  totalEvents: number;
  timeoutEvents: number;
  errorEvents: number;
  slowEvents: number;
  lastSeenAt?: number;
  lastSeenAtText: string;
};

export type FrontendRouteVersionRow = {
  key: string;
  buildVersion: string;
  totalEvents: number;
  affectedRoutes: number;
  affectedSessions: number;
  timeoutEvents: number;
  errorEvents: number;
  prefetchHits: number;
  healthLabel: string;
  healthTone: FrontendRouteTone;
  lastSeenAt: number;
  lastSeenAtText: string;
};

const routePrefetchSourceLabels: Record<RoutePrefetchSource, string> = {
  hover: '悬停',
  open: '展开',
  search: '搜索',
  recommendation: '推荐',
  programmatic: '程序触发',
};

const routePrefetchReasonLabels: Record<RoutePrefetchSkipReason, string> = {
  'data-saver': '省流模式',
  'document-hidden': '页面不可见',
  'network-constrained': '弱网环境',
  'already-ready': '已就绪',
  inflight: '请求合并',
};

const shortCircuitPrefetchReasons = new Set<RoutePrefetchSkipReason>(['already-ready', 'inflight']);

const formatDurationText = (value?: number) =>
  typeof value === 'number' && Number.isFinite(value) ? `${Math.round(value)}ms` : '-';

const formatDateTimeText = (value?: number) =>
  typeof value === 'number' && Number.isFinite(value)
    ? dayjs(value).format('YYYY-MM-DD HH:mm:ss')
    : '-';

const formatRateText = (numerator: number, denominator: number) => {
  if (!denominator) {
    return '-';
  }
  return `${Math.round((numerator / denominator) * 100)}% (${numerator}/${denominator})`;
};

const isSlowRoute = (snapshot: RouteLoadingSnapshot, slowThresholdMs: number) =>
  snapshot.status === 'timeout' ||
  (typeof snapshot.lastDurationMs === 'number' && snapshot.lastDurationMs >= slowThresholdMs);

const getHealthSeverityRank = (snapshot: RouteLoadingSnapshot, slowThresholdMs: number) => {
  if (snapshot.status === 'error') {
    return 0;
  }
  if (snapshot.status === 'timeout') {
    return 1;
  }
  if (snapshot.status === 'loading') {
    return 2;
  }
  if (isSlowRoute(snapshot, slowThresholdMs)) {
    return 3;
  }
  if (snapshot.status === 'ready') {
    return 4;
  }
  return 5;
};

const getHealthStatusLabel = (snapshot: RouteLoadingSnapshot, slowThresholdMs: number) => {
  if (snapshot.status === 'error') {
    return { label: '加载失败', tone: 'error' as const };
  }
  if (snapshot.status === 'timeout') {
    return { label: '加载超时', tone: 'warning' as const };
  }
  if (snapshot.status === 'loading') {
    return { label: '加载中', tone: 'processing' as const };
  }
  if (isSlowRoute(snapshot, slowThresholdMs)) {
    return { label: '加载偏慢', tone: 'warning' as const };
  }
  if (snapshot.status === 'ready') {
    return { label: '正常', tone: 'success' as const };
  }
  return { label: '未触发', tone: 'default' as const };
};

const getPrefetchSourceLabel = (source?: RoutePrefetchSource) =>
  source ? routePrefetchSourceLabels[source] : undefined;

const getPrefetchReasonLabel = (reason?: RoutePrefetchSkipReason) =>
  reason ? routePrefetchReasonLabels[reason] ?? reason : undefined;

const getFirstOpenText = (snapshot: RouteLoadingSnapshot) => {
  if (snapshot.firstTerminalStatus === 'ready') {
    return typeof snapshot.firstTerminalDurationMs === 'number'
      ? `首开 ${Math.round(snapshot.firstTerminalDurationMs)}ms`
      : '首开成功';
  }
  if (snapshot.firstTerminalStatus === 'timeout') {
    return '首开超时';
  }
  if (snapshot.firstTerminalStatus === 'error') {
    return '首开失败';
  }
  return '-';
};

const getPrefetchHitText = (snapshot: RouteLoadingSnapshot) =>
  formatRateText(snapshot.prefetchHitCount ?? 0, snapshot.readyCount ?? 0);

const getPrefetchShortCircuitText = (snapshot: RouteLoadingSnapshot) => {
  const count = snapshot.prefetchShortCircuitCount ?? 0;
  return count > 0 ? `${count} 次` : '-';
};

const getPrefetchLabel = (snapshot: RouteLoadingSnapshot) => {
  const sourceLabel = getPrefetchSourceLabel(snapshot.lastPrefetchSource);
  if (snapshot.lastPrefetchStatus === 'ready') {
    return {
      label: sourceLabel ? `已预热 / ${sourceLabel}` : '已预热',
      tone: 'success' as const,
    };
  }
  if (snapshot.lastPrefetchStatus === 'prefetching') {
    return {
      label: sourceLabel ? `预热中 / ${sourceLabel}` : '预热中',
      tone: 'processing' as const,
    };
  }
  if (snapshot.lastPrefetchStatus === 'error') {
    return {
      label: sourceLabel ? `预热失败 / ${sourceLabel}` : '预热失败',
      tone: 'error' as const,
    };
  }
  if (snapshot.lastPrefetchStatus === 'skipped') {
    const reasonLabel = getPrefetchReasonLabel(snapshot.lastPrefetchReason);
    if (snapshot.lastPrefetchReason && shortCircuitPrefetchReasons.has(snapshot.lastPrefetchReason)) {
      return {
        label: reasonLabel ? `已复用 / ${reasonLabel}` : '已复用',
        tone: 'success' as const,
      };
    }
    return {
      label: reasonLabel ? `已跳过 / ${reasonLabel}` : '已跳过',
      tone: 'default' as const,
    };
  }
  return {
    label: '未预热',
    tone: 'default' as const,
  };
};

const getRouteActivityAt = (snapshot: RouteLoadingSnapshot) =>
  snapshot.lastReadyAt ??
  snapshot.lastStartedAt ??
  snapshot.lastPrefetchCompletedAt ??
  snapshot.lastPrefetchStartedAt;

const formatEventDetail = (event: RouteDiagnosticEvent) => {
  if (!event.detail) {
    return undefined;
  }
  if (event.type === 'prefetch-start') {
    return `来源：${getPrefetchSourceLabel(event.detail as RoutePrefetchSource) ?? event.detail}`;
  }
  if (event.type === 'prefetch-skipped') {
    return `原因：${getPrefetchReasonLabel(event.detail as RoutePrefetchSkipReason)}`;
  }
  return event.detail;
};

const getEventTitle = (eventType: RouteDiagnosticEvent['type']) => {
  switch (eventType) {
    case 'loading-start':
      return { title: '页面开始加载', tone: 'processing' as const };
    case 'loading-ready':
      return { title: '页面加载完成', tone: 'success' as const };
    case 'loading-timeout':
      return { title: '页面加载超时', tone: 'warning' as const };
    case 'loading-error':
      return { title: '页面加载失败', tone: 'error' as const };
    case 'prefetch-start':
      return { title: '页面开始预热', tone: 'processing' as const };
    case 'prefetch-ready':
      return { title: '页面预热完成', tone: 'success' as const };
    case 'prefetch-error':
      return { title: '页面预热失败', tone: 'error' as const };
    case 'prefetch-skipped':
      return { title: '页面跳过预热', tone: 'default' as const };
    default:
      return { title: '未知事件', tone: 'default' as const };
  }
};

const getReportAt = (report: FrontendRouteDiagnosticReport) =>
  report.created_at ? Date.parse(report.created_at) : report.event_at;

const getPathRiskDescriptor = (path: FrontendRouteDiagnosticsPathSummary) => {
  if (path.errorEvents > 0) {
    return { label: '异常优先', tone: 'error' as const };
  }
  if (path.timeoutEvents > 0) {
    return { label: '超时优先', tone: 'warning' as const };
  }
  if (path.slowEvents > 0) {
    return { label: '慢页关注', tone: 'warning' as const };
  }
  return { label: '稳定', tone: 'success' as const };
};

const getVersionDescriptor = (row: { timeoutEvents: number; errorEvents: number }) => {
  if (row.errorEvents > 0 || row.timeoutEvents > 0) {
    return { label: '需回归', tone: row.errorEvents > 0 ? ('error' as const) : ('warning' as const) };
  }
  return { label: '稳定', tone: 'success' as const };
};

const bucketLabelPattern = (bucketSpanMs: number) =>
  bucketSpanMs >= 24 * 60 * 60 * 1000 ? 'MM-DD' : 'MM-DD HH:mm';

export const buildFrontendRouteHealthSummary = (
  snapshots: RouteLoadingSnapshot[],
  slowThresholdMs: number,
): FrontendRouteHealthSummary => ({
  trackedRoutes: snapshots.length,
  readyRoutes: snapshots.filter((snapshot) => snapshot.status === 'ready').length,
  loadingRoutes: snapshots.filter((snapshot) => snapshot.status === 'loading').length,
  timeoutRoutes: snapshots.filter((snapshot) => snapshot.status === 'timeout').length,
  errorRoutes: snapshots.filter((snapshot) => snapshot.status === 'error').length,
  slowRoutes: snapshots.filter((snapshot) => isSlowRoute(snapshot, slowThresholdMs)).length,
  prefetchedRoutes: snapshots.filter(
    (snapshot) => (snapshot.prefetchReadyCount ?? 0) > 0 || snapshot.lastPrefetchStatus === 'ready',
  ).length,
});

export const buildFrontendRouteEfficiencySummary = (
  snapshots: RouteLoadingSnapshot[],
): FrontendRouteEfficiencySummary => {
  const measuredFirstLoads = snapshots.filter((snapshot) => Boolean(snapshot.firstTerminalStatus)).length;
  const successfulFirstLoads = snapshots.filter(
    (snapshot) => snapshot.firstTerminalStatus === 'ready',
  ).length;
  const firstLoadDurations = snapshots
    .filter((snapshot) => snapshot.firstTerminalStatus === 'ready')
    .map((snapshot) => snapshot.firstTerminalDurationMs)
    .filter((value): value is number => typeof value === 'number' && Number.isFinite(value));
  const totalReadyEvents = snapshots.reduce((sum, snapshot) => sum + (snapshot.readyCount ?? 0), 0);
  const prefetchHitCount = snapshots.reduce((sum, snapshot) => sum + (snapshot.prefetchHitCount ?? 0), 0);
  const prefetchShortCircuitCount = snapshots.reduce(
    (sum, snapshot) => sum + (snapshot.prefetchShortCircuitCount ?? 0),
    0,
  );

  return {
    measuredFirstLoads,
    successfulFirstLoads,
    firstLoadSuccessRate: measuredFirstLoads
      ? Math.round((successfulFirstLoads / measuredFirstLoads) * 100)
      : 0,
    firstLoadAvgDurationMs: firstLoadDurations.length
      ? Math.round(
          firstLoadDurations.reduce((sum, value) => sum + value, 0) / firstLoadDurations.length,
        )
      : undefined,
    totalReadyEvents,
    prefetchHitCount,
    prefetchHitRate: totalReadyEvents ? Math.round((prefetchHitCount / totalReadyEvents) * 100) : 0,
    prefetchShortCircuitCount,
  };
};

export const buildFrontendRouteBackendEfficiencySummary = (
  summary?: FrontendRouteDiagnosticsSummary,
): FrontendRouteBackendEfficiencySummary => {
  const measuredFirstLoads = summary?.measuredFirstLoads ?? 0;
  const successfulFirstLoads = summary?.successfulFirstLoads ?? 0;

  return {
    measuredFirstLoads,
    successfulFirstLoads,
    firstLoadSuccessRate: measuredFirstLoads
      ? Math.round((successfulFirstLoads / measuredFirstLoads) * 100)
      : 0,
    firstLoadSuccessText: formatRateText(successfulFirstLoads, measuredFirstLoads),
    firstLoadAvgDurationMs: summary?.firstLoadAvgDurationMs,
    prefetchShortCircuitCount: summary?.prefetchShortCircuitCount ?? 0,
  };
};

export const buildFrontendRouteHealthRows = (
  snapshots: RouteLoadingSnapshot[],
  slowThresholdMs: number,
): FrontendRouteHealthRow[] =>
  [...snapshots]
    .sort((left, right) => {
      const severityDiff =
        getHealthSeverityRank(left, slowThresholdMs) - getHealthSeverityRank(right, slowThresholdMs);
      if (severityDiff !== 0) {
        return severityDiff;
      }
      const updatedAtDiff = (getRouteActivityAt(right) ?? 0) - (getRouteActivityAt(left) ?? 0);
      if (updatedAtDiff !== 0) {
        return updatedAtDiff;
      }
      return left.path.localeCompare(right.path);
    })
    .map((snapshot) => {
      const status = getHealthStatusLabel(snapshot, slowThresholdMs);
      const prefetch = getPrefetchLabel(snapshot);
      const updatedAt = getRouteActivityAt(snapshot);
      return {
        key: snapshot.path,
        path: snapshot.path,
        routeLabel: snapshot.routeLabel ?? '-',
        statusLabel: status.label,
        statusTone: status.tone,
        durationText: formatDurationText(snapshot.lastDurationMs),
        firstOpenText: getFirstOpenText(snapshot),
        prefetchLabel: prefetch.label,
        prefetchTone: prefetch.tone,
        prefetchDurationText: formatDurationText(snapshot.lastPrefetchDurationMs),
        prefetchHitText: getPrefetchHitText(snapshot),
        prefetchShortCircuitText: getPrefetchShortCircuitText(snapshot),
        errorSummary: snapshot.lastError ?? snapshot.lastPrefetchError,
        updatedAt,
        updatedAtText: formatDateTimeText(updatedAt),
      };
    });

export const buildFrontendRouteEventRows = (
  events: RouteDiagnosticEvent[],
): FrontendRouteEventRow[] =>
  events.map((event, index) => {
    const descriptor = getEventTitle(event.type);
    return {
      key: String(index),
      path: event.path,
      title: descriptor.title,
      detail: formatEventDetail(event),
      tone: descriptor.tone,
      at: event.at,
      atText: formatDateTimeText(event.at),
    };
  });

export const buildFrontendRouteTrendRows = (
  reports: FrontendRouteDiagnosticReport[],
  windowHours: number,
  bucketCount = 6,
): FrontendRouteTrendRow[] => {
  const safeBucketCount = Math.max(1, Math.round(bucketCount));
  const safeWindowHours = Math.max(1, Math.round(windowHours));
  const bucketSpanMs = Math.max(
    60 * 60 * 1000,
    Math.ceil((safeWindowHours * 60 * 60 * 1000) / safeBucketCount),
  );
  const latestAt = reports.reduce((max, report) => Math.max(max, getReportAt(report)), 0);
  const alignedEnd = latestAt
    ? Math.ceil(latestAt / bucketSpanMs) * bucketSpanMs
    : Math.ceil(Date.now() / bucketSpanMs) * bucketSpanMs;
  const windowStart = alignedEnd - bucketSpanMs * safeBucketCount;
  const buckets = Array.from({ length: safeBucketCount }, (_, index) => {
    const bucketStartAt = windowStart + bucketSpanMs * index;
    return {
      key: String(bucketStartAt),
      bucketStartAt,
      bucketEndAt: bucketStartAt + bucketSpanMs,
      bucketLabel: dayjs(bucketStartAt).format(bucketLabelPattern(bucketSpanMs)),
      totalEvents: 0,
      timeoutEvents: 0,
      errorEvents: 0,
      prefetchHits: 0,
    };
  });

  reports.forEach((report) => {
    const reportAt = getReportAt(report);
    if (!Number.isFinite(reportAt) || reportAt < windowStart || reportAt >= alignedEnd) {
      return;
    }
    const bucketIndex = Math.min(
      safeBucketCount - 1,
      Math.max(0, Math.floor((reportAt - windowStart) / bucketSpanMs)),
    );
    const bucket = buckets[bucketIndex];
    if (!bucket) {
      return;
    }
    bucket.totalEvents += 1;
    if (report.status === 'timeout') {
      bucket.timeoutEvents += 1;
    }
    if (report.status === 'error') {
      bucket.errorEvents += 1;
    }
    if (report.prefetch_status === 'ready') {
      bucket.prefetchHits += 1;
    }
  });

  return buckets;
};

export const buildFrontendRoutePathRankRows = (
  paths: FrontendRouteDiagnosticsPathSummary[],
): FrontendRoutePathRankRow[] =>
  [...paths]
    .sort((left, right) => {
      const leftDescriptor = getPathRiskDescriptor(left);
      const rightDescriptor = getPathRiskDescriptor(right);
      const toneRank = { error: 0, warning: 1, success: 2, processing: 3, default: 4 } as const;
      const severityDiff = toneRank[leftDescriptor.tone] - toneRank[rightDescriptor.tone];
      if (severityDiff !== 0) {
        return severityDiff;
      }
      if (right.errorEvents !== left.errorEvents) {
        return right.errorEvents - left.errorEvents;
      }
      if (right.timeoutEvents !== left.timeoutEvents) {
        return right.timeoutEvents - left.timeoutEvents;
      }
      if (right.slowEvents !== left.slowEvents) {
        return right.slowEvents - left.slowEvents;
      }
      return right.totalEvents - left.totalEvents;
    })
    .map((path) => {
      const descriptor = getPathRiskDescriptor(path);
      const lastSeenAt = Date.parse(path.lastSeenAt);
      return {
        key: path.path,
        path: path.path,
        routeLabel: path.routeLabel ?? '-',
        riskLabel: descriptor.label,
        riskTone: descriptor.tone,
        totalEvents: path.totalEvents,
        timeoutEvents: path.timeoutEvents,
        errorEvents: path.errorEvents,
        slowEvents: path.slowEvents,
        lastSeenAt: Number.isNaN(lastSeenAt) ? undefined : lastSeenAt,
        lastSeenAtText: formatDateTimeText(Number.isNaN(lastSeenAt) ? undefined : lastSeenAt),
      };
    });

type FrontendRouteVersionAccumulator = {
  key: string;
  buildVersion: string;
  totalEvents: number;
  affectedRoutes: Set<string>;
  affectedSessions: Set<string>;
  timeoutEvents: number;
  errorEvents: number;
  prefetchHits: number;
  lastSeenAt: number;
};

export const buildFrontendRouteVersionRows = (
  reports: FrontendRouteDiagnosticReport[],
): FrontendRouteVersionRow[] =>
  Array.from(
    reports.reduce((map, report) => {
      const buildVersion = report.build_version.trim() || '未知版本';
      const current =
        map.get(buildVersion) ??
        ({
          key: buildVersion,
          buildVersion,
          totalEvents: 0,
          affectedRoutes: new Set<string>(),
          affectedSessions: new Set<string>(),
          timeoutEvents: 0,
          errorEvents: 0,
          prefetchHits: 0,
          lastSeenAt: 0,
        } satisfies FrontendRouteVersionAccumulator);
      current.totalEvents += 1;
      current.affectedRoutes.add(report.path);
      current.affectedSessions.add(report.session_id);
      if (report.status === 'timeout') {
        current.timeoutEvents += 1;
      }
      if (report.status === 'error') {
        current.errorEvents += 1;
      }
      if (report.prefetch_status === 'ready') {
        current.prefetchHits += 1;
      }
      current.lastSeenAt = Math.max(current.lastSeenAt, getReportAt(report));
      map.set(buildVersion, current);
      return map;
    }, new Map<string, FrontendRouteVersionAccumulator>()),
  )
    .map(([, item]) => {
      const descriptor = getVersionDescriptor(item);
      return {
        key: item.key,
        buildVersion: item.buildVersion,
        totalEvents: item.totalEvents,
        affectedRoutes: item.affectedRoutes.size,
        affectedSessions: item.affectedSessions.size,
        timeoutEvents: item.timeoutEvents,
        errorEvents: item.errorEvents,
        prefetchHits: item.prefetchHits,
        healthLabel: descriptor.label,
        healthTone: descriptor.tone,
        lastSeenAt: item.lastSeenAt,
        lastSeenAtText: formatDateTimeText(item.lastSeenAt),
      };
    })
    .sort((left, right) => {
      const toneRank = { error: 0, warning: 1, success: 2, processing: 3, default: 4 } as const;
      const severityDiff = toneRank[left.healthTone] - toneRank[right.healthTone];
      if (severityDiff !== 0) {
        return severityDiff;
      }
      return right.lastSeenAt - left.lastSeenAt;
    });

