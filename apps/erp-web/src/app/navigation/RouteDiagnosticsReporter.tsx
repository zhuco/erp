import { useEffect, useMemo, useRef } from 'react';

import { reportFrontendRouteDiagnostics } from '@/api/frontendRouteDiagnostics';
import { useTenantId } from '@/features/tenant';
import { buildVersion } from '@/utils/env';

import {
  type RouteDiagnosticEvent,
  type RouteLoadingSnapshot,
  subscribeRouteDiagnosticEvents,
} from './routeLoadingDiagnostics';
import {
  buildRouteDiagnosticReport,
  isRouteDiagnosticEventReportable,
} from './routeDiagnosticsReporting';

const ROUTE_DIAGNOSTIC_FLUSH_DELAY_MS = 1200;
const ROUTE_DIAGNOSTIC_BATCH_SIZE = 20;
const ROUTE_DIAGNOSTIC_SESSION_STORAGE_KEY = 'erp.frontend-route-diagnostics.session-id';

const generateRouteDiagnosticSessionId = () => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `route-session-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
};

const getRouteDiagnosticSessionId = () => {
  if (typeof window === 'undefined') {
    return 'server-session';
  }
  const existing = window.sessionStorage.getItem(ROUTE_DIAGNOSTIC_SESSION_STORAGE_KEY);
  if (existing) {
    return existing;
  }
  const next = generateRouteDiagnosticSessionId();
  window.sessionStorage.setItem(ROUTE_DIAGNOSTIC_SESSION_STORAGE_KEY, next);
  return next;
};

export const RouteDiagnosticsReporter = () => {
  const tenantId = useTenantId();
  const queueRef = useRef<ReturnType<typeof buildRouteDiagnosticReport>[]>([]);
  const flushTimerRef = useRef<number | null>(null);
  const isFlushingRef = useRef(false);
  const tenantIdRef = useRef(tenantId);
  const sessionId = useMemo(() => getRouteDiagnosticSessionId(), []);

  useEffect(() => {
    tenantIdRef.current = tenantId;
  }, [tenantId]);

  useEffect(() => {
    const clearFlushTimer = () => {
      if (flushTimerRef.current === null) {
        return;
      }
      window.clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    };

    const flushReports = async () => {
      const currentTenantId = tenantIdRef.current;
      if (!currentTenantId || isFlushingRef.current || queueRef.current.length === 0) {
        return;
      }

      clearFlushTimer();
      isFlushingRef.current = true;
      const payload = queueRef.current.splice(0, ROUTE_DIAGNOSTIC_BATCH_SIZE);
      try {
        await reportFrontendRouteDiagnostics(currentTenantId, {
          events: payload,
        });
      } catch {
        queueRef.current.unshift(...payload);
      } finally {
        isFlushingRef.current = false;
        if (queueRef.current.length > 0) {
          flushTimerRef.current = window.setTimeout(() => {
            void flushReports();
          }, ROUTE_DIAGNOSTIC_FLUSH_DELAY_MS);
        }
      }
    };

    const scheduleFlush = () => {
      clearFlushTimer();
      flushTimerRef.current = window.setTimeout(() => {
        void flushReports();
      }, ROUTE_DIAGNOSTIC_FLUSH_DELAY_MS);
    };

    const enqueueReport = (event: RouteDiagnosticEvent, snapshot: RouteLoadingSnapshot) => {
      if (!tenantIdRef.current || !isRouteDiagnosticEventReportable(event)) {
        return;
      }
      queueRef.current.push(
        buildRouteDiagnosticReport(event, snapshot, {
          sessionId,
          currentUrl: window.location.href,
          buildVersion,
          userAgent: window.navigator.userAgent,
        }),
      );
      if (queueRef.current.length >= ROUTE_DIAGNOSTIC_BATCH_SIZE) {
        void flushReports();
        return;
      }
      scheduleFlush();
    };

    const flushOnPageHide = () => {
      void flushReports();
    };

    const flushOnVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        void flushReports();
      }
    };

    const unsubscribe = subscribeRouteDiagnosticEvents(enqueueReport);
    window.addEventListener('pagehide', flushOnPageHide);
    window.addEventListener('beforeunload', flushOnPageHide);
    document.addEventListener('visibilitychange', flushOnVisibilityChange);

    return () => {
      unsubscribe();
      window.removeEventListener('pagehide', flushOnPageHide);
      window.removeEventListener('beforeunload', flushOnPageHide);
      document.removeEventListener('visibilitychange', flushOnVisibilityChange);
      clearFlushTimer();
      void flushReports();
    };
  }, [sessionId]);

  return null;
};
