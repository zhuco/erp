import { Children, isValidElement } from 'react';
import type { ComponentType, ReactNode } from 'react';

import { getCanonicalPath } from '@/app/navigation/routeAliases';
import {
  getRouteLoadingSnapshot,
  markRoutePrefetchError,
  markRoutePrefetchReady,
  markRoutePrefetchSkipped,
  markRoutePrefetchStart,
  type RoutePrefetchSource,
} from '@/app/navigation/routeLoadingDiagnostics';

type RoutePrefetchMetadata = {
  routeLabel: string;
  prefetch: () => Promise<unknown>;
};

type PrefetchableComponent = ComponentType<unknown> & {
  __routePrefetchMetadata__?: RoutePrefetchMetadata;
};

type RouteConfigLike = {
  path?: string;
  index?: boolean;
  element?: ReactNode;
  children?: RouteConfigLike[];
};

type PrefetchRouteOptions = {
  source?: RoutePrefetchSource;
};

type SchedulePrefetchOptions = PrefetchRouteOptions & {
  limit?: number;
  excludePath?: string;
};

const routePrefetchRegistry = new Map<string, RoutePrefetchMetadata>();
const routePrefetchInflight = new Map<string, Promise<boolean>>();
const requestIdle =
  typeof window !== 'undefined' && typeof window.requestIdleCallback === 'function'
    ? window.requestIdleCallback.bind(window)
    : null;
const cancelIdle =
  typeof window !== 'undefined' && typeof window.cancelIdleCallback === 'function'
    ? window.cancelIdleCallback.bind(window)
    : null;

const normalizePath = (path: string) => {
  const canonicalPath = getCanonicalPath(path.split(/[?#]/, 1)[0] || '/');
  const normalized = canonicalPath.startsWith('/') ? canonicalPath : `/${canonicalPath}`;
  if (normalized.length > 1 && normalized.endsWith('/')) {
    return normalized.slice(0, -1);
  }
  return normalized || '/';
};

const isStaticRoutePath = (path: string) => !path.includes(':') && !path.includes('*');

const joinRoutePath = (basePath: string, childPath?: string, isIndex?: boolean) => {
  if (isIndex || !childPath) {
    return basePath;
  }
  if (childPath.startsWith('/')) {
    return normalizePath(childPath);
  }
  const normalizedBase = basePath === '/' ? '' : basePath;
  return normalizePath(`${normalizedBase}/${childPath}`);
};

const getConnection = () => {
  if (typeof navigator === 'undefined') {
    return undefined;
  }
  return (navigator as Navigator & {
    connection?: {
      saveData?: boolean;
      effectiveType?: string;
    };
  }).connection;
};

const shouldSkipPrefetch = () => {
  const connection = getConnection();
  if (connection?.saveData) {
    return 'network-constrained';
  }
  if (connection?.effectiveType === 'slow-2g' || connection?.effectiveType === '2g') {
    return 'network-constrained';
  }
  if (typeof document !== 'undefined' && document.visibilityState === 'hidden') {
    return 'document-hidden';
  }
  return null;
};

const findPrefetchMetadata = (node: ReactNode): RoutePrefetchMetadata | undefined => {
  if (!node) {
    return undefined;
  }
  if (Array.isArray(node)) {
    for (const child of node) {
      const matched = findPrefetchMetadata(child);
      if (matched) {
        return matched;
      }
    }
    return undefined;
  }
  if (!isValidElement(node)) {
    return undefined;
  }

  const typedElement = node.type as PrefetchableComponent;
  if (typedElement.__routePrefetchMetadata__) {
    return typedElement.__routePrefetchMetadata__;
  }

  const children = Children.toArray(node.props?.children as ReactNode);
  for (const child of children) {
    const matched = findPrefetchMetadata(child);
    if (matched) {
      return matched;
    }
  }
  return undefined;
};

export const attachRoutePrefetch = <T extends ComponentType<unknown>>(
  component: T,
  metadata: RoutePrefetchMetadata,
): T => {
  (component as PrefetchableComponent).__routePrefetchMetadata__ = metadata;
  return component;
};

export const buildRoutePrefetchRegistry = (routes: RouteConfigLike[]) => {
  const nextRegistry = new Map<string, RoutePrefetchMetadata>();

  const visit = (items: RouteConfigLike[], parentPath: string) => {
    items.forEach((item) => {
      const resolvedPath = joinRoutePath(parentPath, item.path, item.index);
      if (item.path && isStaticRoutePath(resolvedPath)) {
        const metadata = findPrefetchMetadata(item.element);
        if (metadata) {
          nextRegistry.set(resolvedPath, metadata);
        }
      }
      if (item.children?.length) {
        visit(item.children, resolvedPath);
      }
    });
  };

  visit(routes, '/');
  return nextRegistry;
};

export const installRoutePrefetchRegistry = (routes: RouteConfigLike[]) => {
  routePrefetchRegistry.clear();
  const nextRegistry = buildRoutePrefetchRegistry(routes);
  nextRegistry.forEach((metadata, path) => {
    routePrefetchRegistry.set(path, metadata);
  });
};

export const clearRoutePrefetchRegistry = () => {
  routePrefetchRegistry.clear();
  routePrefetchInflight.clear();
};

export const prefetchRoutePath = async (
  path: string,
  options: PrefetchRouteOptions = {},
): Promise<boolean> => {
  const normalizedPath = normalizePath(path);
  const metadata = routePrefetchRegistry.get(normalizedPath);
  if (!metadata) {
    return false;
  }

  const snapshot = getRouteLoadingSnapshot(normalizedPath);
  if (snapshot.status === 'ready' || snapshot.lastPrefetchStatus === 'ready') {
    markRoutePrefetchSkipped(normalizedPath, 'already-ready');
    return true;
  }

  const skipReason = shouldSkipPrefetch();
  if (skipReason) {
    markRoutePrefetchSkipped(normalizedPath, skipReason);
    return false;
  }

  const inflight = routePrefetchInflight.get(normalizedPath);
  if (inflight) {
    markRoutePrefetchSkipped(normalizedPath, 'inflight');
    return inflight;
  }

  const source = options.source ?? 'programmatic';
  markRoutePrefetchStart(normalizedPath, source);

  const task = Promise.resolve(metadata.prefetch())
    .then(() => {
      markRoutePrefetchReady(normalizedPath);
      return true;
    })
    .catch((error) => {
      markRoutePrefetchError(normalizedPath, error);
      return false;
    })
    .finally(() => {
      routePrefetchInflight.delete(normalizedPath);
    });

  routePrefetchInflight.set(normalizedPath, task);
  return task;
};

export const prefetchRoutePaths = async (
  paths: string[],
  options: SchedulePrefetchOptions = {},
): Promise<boolean[]> => {
  const limit = options.limit ?? 3;
  const excludedPath = options.excludePath ? normalizePath(options.excludePath) : null;
  const candidates = Array.from(
    new Set(
      paths
        .map((path) => normalizePath(path))
        .filter((path) => path !== excludedPath && routePrefetchRegistry.has(path)),
    ),
  ).slice(0, limit);

  return Promise.all(candidates.map((path) => prefetchRoutePath(path, options)));
};

export const scheduleRoutePrefetchPaths = (
  paths: string[],
  options: SchedulePrefetchOptions = {},
) => {
  if (paths.length === 0) {
    return () => undefined;
  }

  if (requestIdle) {
    const idleHandle = requestIdle(() => {
      void prefetchRoutePaths(paths, options);
    }, { timeout: 400 });
    return () => {
      if (cancelIdle) {
        cancelIdle(idleHandle);
      }
    };
  }

  const timeoutHandle = window.setTimeout(() => {
    void prefetchRoutePaths(paths, options);
  }, 120);
  return () => {
    window.clearTimeout(timeoutHandle);
  };
};

export const getInstalledRoutePrefetchPaths = () => Array.from(routePrefetchRegistry.keys());
