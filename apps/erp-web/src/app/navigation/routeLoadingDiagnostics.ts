import { useSyncExternalStore } from 'react';

export const ROUTE_LOADING_TIMEOUT_MS = 8000;
const ROUTE_DIAGNOSTIC_EVENT_LIMIT = 80;
const enableRouteDiagnosticsLog = import.meta.env.DEV && import.meta.env.MODE !== 'test';

type RouteLoadingStatus = 'idle' | 'loading' | 'ready' | 'timeout' | 'error';
type RoutePrefetchStatus = 'prefetching' | 'ready' | 'error' | 'skipped';
export type RoutePrefetchSource = 'hover' | 'open' | 'search' | 'recommendation' | 'programmatic';
export type RoutePrefetchSkipReason =
  | 'data-saver'
  | 'document-hidden'
  | 'network-constrained'
  | 'already-ready'
  | 'inflight';

export type RouteDiagnosticEvent = {
  path: string;
  type:
    | 'loading-start'
    | 'loading-ready'
    | 'loading-timeout'
    | 'loading-error'
    | 'prefetch-start'
    | 'prefetch-ready'
    | 'prefetch-error'
    | 'prefetch-skipped';
  at: number;
  detail?: string;
};

export type RouteLoadingSnapshot = {
  path: string;
  status: RouteLoadingStatus;
  routeLabel?: string;
  loadCount?: number;
  readyCount?: number;
  timeoutCount?: number;
  errorCount?: number;
  firstTerminalStatus?: Extract<RouteLoadingStatus, 'ready' | 'timeout' | 'error'>;
  firstTerminalAt?: number;
  firstTerminalDurationMs?: number;
  lastStartedAt?: number;
  lastReadyAt?: number;
  lastDurationMs?: number;
  lastError?: string;
  lastPrefetchSource?: RoutePrefetchSource;
  lastPrefetchStatus?: RoutePrefetchStatus;
  lastPrefetchStartedAt?: number;
  lastPrefetchCompletedAt?: number;
  lastPrefetchDurationMs?: number;
  lastPrefetchReason?: RoutePrefetchSkipReason;
  lastPrefetchError?: string;
  prefetchReadyCount?: number;
  prefetchSkipCount?: number;
  prefetchHitCount?: number;
  prefetchShortCircuitCount?: number;
  pendingPrefetchHit?: boolean;
};

const routeSnapshots = new Map<string, RouteLoadingSnapshot>();
const routeEmptySnapshots = new Map<string, RouteLoadingSnapshot>();
const routeLoadingTimeoutHandles = new Map<string, ReturnType<typeof setTimeout>>();
const routeDiagnosticEvents: RouteDiagnosticEvent[] = [];
const subscribers = new Set<() => void>();
const routeDiagnosticEventSubscribers = new Set<
  (event: RouteDiagnosticEvent, snapshot: RouteLoadingSnapshot) => void
>();

const createDefaultSnapshot = (path: string): RouteLoadingSnapshot => ({
  path,
  status: 'idle',
});

const normalizePath = (path: string) => {
  if (!path) {
    return '/';
  }
  const [pathname] = path.split(/[?#]/, 1);
  const normalized = pathname.startsWith('/') ? pathname : `/${pathname}`;
  if (normalized.length > 1 && normalized.endsWith('/')) {
    return normalized.slice(0, -1);
  }
  return normalized || '/';
};

const emitChange = () => {
  subscribers.forEach((listener) => listener());
};

const shortCircuitPrefetchReasons = new Set<RoutePrefetchSkipReason>([
  'already-ready',
  'inflight',
]);

const pushEvent = (event: RouteDiagnosticEvent) => {
  routeDiagnosticEvents.unshift(event);
  if (routeDiagnosticEvents.length > ROUTE_DIAGNOSTIC_EVENT_LIMIT) {
    routeDiagnosticEvents.length = ROUTE_DIAGNOSTIC_EVENT_LIMIT;
  }
  const snapshot = getRouteLoadingSnapshot(event.path);
  routeDiagnosticEventSubscribers.forEach((listener) => listener(event, snapshot));
  emitChange();
};

const writeSnapshot = (
  path: string,
  updater: (current: RouteLoadingSnapshot) => RouteLoadingSnapshot,
) => {
  const normalizedPath = normalizePath(path);
  const current = routeSnapshots.get(normalizedPath) ?? createDefaultSnapshot(normalizedPath);
  routeSnapshots.set(normalizedPath, updater(current));
  routeEmptySnapshots.delete(normalizedPath);
  emitChange();
};

const clearRouteLoadingTimeout = (path: string) => {
  const normalizedPath = normalizePath(path);
  const handle = routeLoadingTimeoutHandles.get(normalizedPath);
  if (!handle) {
    return;
  }
  clearTimeout(handle);
  routeLoadingTimeoutHandles.delete(normalizedPath);
};

const summarizeError = (error: unknown) => {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  return '未知错误';
};

const logRouteDiagnostic = (message: string) => {
  if (!enableRouteDiagnosticsLog) {
    return;
  }
  console.info(`[route] ${message}`);
};

export const clearRouteLoadingDiagnostics = () => {
  Array.from(routeLoadingTimeoutHandles.keys()).forEach((path) => clearRouteLoadingTimeout(path));
  routeSnapshots.clear();
  routeEmptySnapshots.clear();
  routeDiagnosticEvents.length = 0;
  emitChange();
};

export const markRouteLoadingStart = (path: string) => {
  const normalizedPath = normalizePath(path);
  const startedAt = Date.now();

  clearRouteLoadingTimeout(normalizedPath);
  writeSnapshot(normalizedPath, (current) => ({
    ...current,
    path: normalizedPath,
    status: 'loading',
    loadCount: (current.loadCount ?? 0) + 1,
    lastStartedAt: startedAt,
    lastError: undefined,
    pendingPrefetchHit:
      current.lastPrefetchStatus === 'ready' &&
      typeof current.lastPrefetchCompletedAt === 'number' &&
      current.lastPrefetchCompletedAt > (current.lastReadyAt ?? 0),
  }));
  pushEvent({ path: normalizedPath, type: 'loading-start', at: startedAt });
  logRouteDiagnostic(`开始加载 ${normalizedPath}`);

  const timeoutHandle = setTimeout(() => {
    const timeoutAt = Date.now();
    writeSnapshot(normalizedPath, (current) => {
      if (current.status !== 'loading') {
        return current;
      }
      const firstDurationMs = current.lastStartedAt
        ? Math.max(0, timeoutAt - current.lastStartedAt)
        : ROUTE_LOADING_TIMEOUT_MS;
      return {
        ...current,
        status: 'timeout',
        timeoutCount: (current.timeoutCount ?? 0) + 1,
        firstTerminalStatus: current.firstTerminalStatus ?? 'timeout',
        firstTerminalAt: current.firstTerminalAt ?? timeoutAt,
        firstTerminalDurationMs: current.firstTerminalDurationMs ?? firstDurationMs,
        pendingPrefetchHit: false,
      };
    });
    pushEvent({ path: normalizedPath, type: 'loading-timeout', at: timeoutAt });
    logRouteDiagnostic(`加载超时 ${normalizedPath}`);
  }, ROUTE_LOADING_TIMEOUT_MS);
  routeLoadingTimeoutHandles.set(normalizedPath, timeoutHandle);
};

export const markRouteLoadingReady = (
  path: string,
  options: {
    routeLabel?: string;
  } = {},
) => {
  const normalizedPath = normalizePath(path);
  const readyAt = Date.now();

  clearRouteLoadingTimeout(normalizedPath);
  writeSnapshot(normalizedPath, (current) => {
    const lastDurationMs = current.lastStartedAt ? Math.max(0, readyAt - current.lastStartedAt) : 0;
    return {
      ...current,
      path: normalizedPath,
      status: 'ready',
      readyCount: (current.readyCount ?? 0) + 1,
      routeLabel: options.routeLabel ?? current.routeLabel,
      lastReadyAt: readyAt,
      lastDurationMs,
      firstTerminalStatus: current.firstTerminalStatus ?? 'ready',
      firstTerminalAt: current.firstTerminalAt ?? readyAt,
      firstTerminalDurationMs: current.firstTerminalDurationMs ?? lastDurationMs,
      lastError: undefined,
      prefetchHitCount: (current.prefetchHitCount ?? 0) + (current.pendingPrefetchHit ? 1 : 0),
      pendingPrefetchHit: false,
    };
  });
  pushEvent({
    path: normalizedPath,
    type: 'loading-ready',
    at: readyAt,
    detail: options.routeLabel,
  });
  logRouteDiagnostic(
    `加载完成 ${normalizedPath}${options.routeLabel ? ` label=${options.routeLabel}` : ''}`,
  );
};

export const markRouteLoadingError = (path: string, error: unknown) => {
  const normalizedPath = normalizePath(path);
  const summary = summarizeError(error);
  const errorAt = Date.now();

  clearRouteLoadingTimeout(normalizedPath);
  writeSnapshot(normalizedPath, (current) => ({
    ...current,
    path: normalizedPath,
    status: 'error',
    errorCount: (current.errorCount ?? 0) + 1,
    firstTerminalStatus: current.firstTerminalStatus ?? 'error',
    firstTerminalAt: current.firstTerminalAt ?? errorAt,
    firstTerminalDurationMs:
      current.firstTerminalDurationMs ??
      (current.lastStartedAt ? Math.max(0, errorAt - current.lastStartedAt) : undefined),
    lastError: summary,
    pendingPrefetchHit: false,
  }));
  pushEvent({ path: normalizedPath, type: 'loading-error', at: errorAt, detail: summary });
  logRouteDiagnostic(`加载失败 ${normalizedPath} error="${summary}"`);
};

export const markRoutePrefetchStart = (path: string, source: RoutePrefetchSource) => {
  const normalizedPath = normalizePath(path);
  const startedAt = Date.now();

  writeSnapshot(normalizedPath, (current) => ({
    ...current,
    path: normalizedPath,
    lastPrefetchSource: source,
    lastPrefetchStatus: 'prefetching',
    lastPrefetchStartedAt: startedAt,
    lastPrefetchReason: undefined,
    lastPrefetchError: undefined,
  }));
  pushEvent({ path: normalizedPath, type: 'prefetch-start', at: startedAt, detail: source });
  logRouteDiagnostic(`开始预热 ${normalizedPath} source=${source}`);
};

export const markRoutePrefetchReady = (path: string) => {
  const normalizedPath = normalizePath(path);
  const completedAt = Date.now();

  writeSnapshot(normalizedPath, (current) => ({
    ...current,
    path: normalizedPath,
    lastPrefetchStatus: 'ready',
    lastPrefetchCompletedAt: completedAt,
    lastPrefetchDurationMs: current.lastPrefetchStartedAt
      ? Math.max(0, completedAt - current.lastPrefetchStartedAt)
      : 0,
    prefetchReadyCount: (current.prefetchReadyCount ?? 0) + 1,
  }));
  pushEvent({ path: normalizedPath, type: 'prefetch-ready', at: completedAt });
  logRouteDiagnostic(`预热完成 ${normalizedPath}`);
};

export const markRoutePrefetchError = (path: string, error: unknown) => {
  const normalizedPath = normalizePath(path);
  const summary = summarizeError(error);

  writeSnapshot(normalizedPath, (current) => ({
    ...current,
    path: normalizedPath,
    lastPrefetchStatus: 'error',
    lastPrefetchCompletedAt: Date.now(),
    lastPrefetchError: summary,
  }));
  pushEvent({ path: normalizedPath, type: 'prefetch-error', at: Date.now(), detail: summary });
  logRouteDiagnostic(`预热失败 ${normalizedPath} error="${summary}"`);
};

export const markRoutePrefetchSkipped = (path: string, reason: RoutePrefetchSkipReason) => {
  const normalizedPath = normalizePath(path);
  const completedAt = Date.now();
  const isShortCircuit = shortCircuitPrefetchReasons.has(reason);

  writeSnapshot(normalizedPath, (current) => ({
    ...current,
    path: normalizedPath,
    lastPrefetchStatus: 'skipped',
    lastPrefetchCompletedAt: completedAt,
    lastPrefetchReason: reason,
    prefetchSkipCount: (current.prefetchSkipCount ?? 0) + 1,
    prefetchShortCircuitCount:
      (current.prefetchShortCircuitCount ?? 0) + (isShortCircuit ? 1 : 0),
  }));
  pushEvent({ path: normalizedPath, type: 'prefetch-skipped', at: completedAt, detail: reason });
  logRouteDiagnostic(`跳过预热 ${normalizedPath} reason=${reason}`);
};

export const getRouteLoadingSnapshot = (path: string): RouteLoadingSnapshot => {
  const normalizedPath = normalizePath(path);
  const existingSnapshot = routeSnapshots.get(normalizedPath);
  if (existingSnapshot) {
    return existingSnapshot;
  }
  const emptySnapshot = routeEmptySnapshots.get(normalizedPath);
  if (emptySnapshot) {
    return emptySnapshot;
  }
  const snapshot = createDefaultSnapshot(normalizedPath);
  routeEmptySnapshots.set(normalizedPath, snapshot);
  return snapshot;
};

export const getAllRouteLoadingSnapshots = () =>
  Array.from(routeSnapshots.values()).sort((left, right) => left.path.localeCompare(right.path));

export const getRouteDiagnosticEvents = () => [...routeDiagnosticEvents];

export const subscribeRouteLoadingDiagnostics = (listener: () => void) => {
  subscribers.add(listener);
  return () => {
    subscribers.delete(listener);
  };
};

export const subscribeRouteDiagnosticEvents = (
  listener: (event: RouteDiagnosticEvent, snapshot: RouteLoadingSnapshot) => void,
) => {
  routeDiagnosticEventSubscribers.add(listener);
  return () => {
    routeDiagnosticEventSubscribers.delete(listener);
  };
};

export const useRouteLoadingSnapshot = (path: string) =>
  useSyncExternalStore(
    subscribeRouteLoadingDiagnostics,
    () => getRouteLoadingSnapshot(path),
    () => getRouteLoadingSnapshot(path),
  );
