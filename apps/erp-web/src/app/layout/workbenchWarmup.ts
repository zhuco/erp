type WorkbenchWarmupLink = {
  path: string;
};

type WorkbenchWarmupGroup = {
  key: string;
  items: WorkbenchWarmupLink[];
};

export type WorkbenchWarmupModule = {
  key: string;
  defaultPath?: string;
  leafPaths: string[];
  groups: WorkbenchWarmupGroup[];
};

type BuildWorkbenchWarmupPathsOptions = {
  currentPath: string;
  activeModuleKey?: string;
  modules: WorkbenchWarmupModule[];
  recommendedPaths?: string[];
  moduleLimit?: number;
  limit?: number;
};

const normalizePath = (path: string) => {
  const [pathname] = path.split(/[?#]/, 1);
  const normalized = pathname.startsWith('/') ? pathname : `/${pathname}`;
  if (normalized.length > 1 && normalized.endsWith('/')) {
    return normalized.slice(0, -1);
  }
  return normalized || '/';
};

const appendUniquePaths = (
  target: string[],
  seen: Set<string>,
  paths: string[],
  limit: number,
) => {
  paths.forEach((path) => {
    const normalizedPath = normalizePath(path);
    if (seen.has(normalizedPath) || target.length >= limit) {
      return;
    }
    seen.add(normalizedPath);
    target.push(normalizedPath);
  });
};

export const buildWorkbenchWarmupPaths = ({
  currentPath,
  activeModuleKey,
  modules,
  recommendedPaths = [],
  moduleLimit = 3,
  limit = 6,
}: BuildWorkbenchWarmupPathsOptions) => {
  const normalizedCurrentPath = normalizePath(currentPath);
  const results: string[] = [];
  const seen = new Set<string>([normalizedCurrentPath]);
  const activeModule = activeModuleKey ? modules.find((module) => module.key === activeModuleKey) : undefined;

  if (activeModule && moduleLimit > 0 && results.length < limit) {
    const activeGroup = activeModule.groups.find((group) =>
      group.items.some((item) => normalizePath(item.path) === normalizedCurrentPath),
    );
    const defaultPath = activeModule.defaultPath ? normalizePath(activeModule.defaultPath) : undefined;
    const groupPaths = (activeGroup?.items ?? [])
      .map((item) => normalizePath(item.path))
      .filter((path) => path !== normalizedCurrentPath);

    appendUniquePaths(results, seen, groupPaths, Math.min(limit, moduleLimit));

    if (defaultPath && results.length < Math.min(limit, moduleLimit)) {
      appendUniquePaths(results, seen, [defaultPath], Math.min(limit, moduleLimit));
    }

    const modulePaths = activeModule.leafPaths
      .map((path) => normalizePath(path))
      .filter((path) => path !== normalizedCurrentPath);
    appendUniquePaths(results, seen, modulePaths, Math.min(limit, moduleLimit));
  }

  appendUniquePaths(results, seen, recommendedPaths, limit);
  return results.slice(0, limit);
};
