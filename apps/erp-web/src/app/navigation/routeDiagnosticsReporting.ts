import type {
  RouteDiagnosticEvent,
  RouteLoadingSnapshot,
} from '@/app/navigation/routeLoadingDiagnostics';

export type RouteDiagnosticReportContext = {
  sessionId: string;
  currentUrl: string;
  buildVersion: string;
  userAgent: string;
};

export type RouteDiagnosticReport = {
  session_id: string;
  path: string;
  route_label?: string;
  event_type: RouteDiagnosticEvent['type'];
  status: RouteLoadingSnapshot['status'];
  duration_ms?: number;
  prefetch_status?: RouteLoadingSnapshot['lastPrefetchStatus'];
  prefetch_source?: RouteLoadingSnapshot['lastPrefetchSource'];
  prefetch_duration_ms?: number;
  error_summary?: string;
  detail?: string;
  event_at: number;
  page_url: string;
  build_version: string;
  user_agent: string;
};

const REPORTABLE_EVENT_TYPES: RouteDiagnosticEvent['type'][] = [
  'loading-ready',
  'loading-timeout',
  'loading-error',
  'prefetch-ready',
  'prefetch-error',
  'prefetch-skipped',
];

export const isRouteDiagnosticEventReportable = (event: RouteDiagnosticEvent) =>
  REPORTABLE_EVENT_TYPES.includes(event.type);

export const buildRouteDiagnosticReport = (
  event: RouteDiagnosticEvent,
  snapshot: RouteLoadingSnapshot,
  context: RouteDiagnosticReportContext,
): RouteDiagnosticReport => ({
  session_id: context.sessionId,
  path: snapshot.path,
  route_label: snapshot.routeLabel,
  event_type: event.type,
  status: snapshot.status,
  duration_ms: snapshot.lastDurationMs,
  prefetch_status: snapshot.lastPrefetchStatus,
  prefetch_source: snapshot.lastPrefetchSource,
  prefetch_duration_ms: snapshot.lastPrefetchDurationMs,
  error_summary: snapshot.lastError ?? snapshot.lastPrefetchError,
  detail: event.detail,
  event_at: event.at,
  page_url: context.currentUrl,
  build_version: context.buildVersion,
  user_agent: context.userAgent,
});
