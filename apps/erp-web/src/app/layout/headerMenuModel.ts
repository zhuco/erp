import type { AppMenuItem } from './menu';
import { flattenMenuItems } from './menu';

export type HeaderModuleLinkItem = {
  key: string;
  label: string;
  path: string;
  testId?: string;
};

export type HeaderModuleGroupItem = {
  key: string;
  label: string;
  items: HeaderModuleLinkItem[];
};

export type HeaderModuleItem = {
  key: string;
  label: string;
  groups: HeaderModuleGroupItem[];
  defaultPath?: string;
  leafPaths: string[];
};

const isLeafItem = (item: AppMenuItem) => item.path && (!item.children || item.children.length === 0);

const toPathTestId = (path: string, prefix: string) => {
  const normalized = path
    .replace(/^\/+/, '')
    .replace(/[^a-z0-9]+/gi, '-')
    .replace(/^-+|-+$/g, '');
  return `${prefix}-${normalized || 'home'}`;
};

export const collectLeafPaths = (item: AppMenuItem): string[] => {
  if (isLeafItem(item)) {
    return [item.path as string];
  }
  const children = (item.children as AppMenuItem[] | undefined) ?? [];
  return children.flatMap((child) => collectLeafPaths(child));
};

const buildHeaderLinks = (items: AppMenuItem[], groupKey: string): HeaderModuleLinkItem[] => {
  return flattenMenuItems(items).map((item, index) => ({
    key: `${groupKey}-${index}-${item.path}`,
    label: item.name,
    path: item.path,
    testId: toPathTestId(item.path, 'module-link'),
  }));
};

export const buildHeaderGroups = (items: AppMenuItem[], moduleKey: string): HeaderModuleGroupItem[] => {
  const directLinks: AppMenuItem[] = [];
  const groups: HeaderModuleGroupItem[] = [];

  items.forEach((item, index) => {
    const children = (item.children as AppMenuItem[] | undefined) ?? [];
    if (!item.path && children.length > 0) {
      const links = buildHeaderLinks(children, `${moduleKey}-group-${index}`);
      if (links.length > 0) {
        groups.push({
          key: `${moduleKey}-group-${index}`,
          label: item.name ?? `分组${index + 1}`,
          items: links,
        });
      }
      return;
    }

    if (item.path) {
      directLinks.push(item);
    }
  });

  if (directLinks.length > 0) {
    groups.unshift({
      key: `${moduleKey}-common`,
      label: '常用功能',
      items: buildHeaderLinks(directLinks, `${moduleKey}-common`),
    });
  }

  return groups;
};

const resolveModuleKey = (item: AppMenuItem, leafPaths: string[]) => {
  if (item.path) {
    return item.path.replace(/^\/+/, '') || 'home';
  }
  const firstLeafPath = leafPaths[0];
  if (!firstLeafPath) {
    return (item.name ?? 'module').toLowerCase();
  }
  const firstSegment = firstLeafPath.split('/').filter(Boolean)[0];
  return firstSegment || (item.name ?? 'module').toLowerCase();
};

export const buildHeaderModules = (menuRoutes: AppMenuItem[]): HeaderModuleItem[] => {
  return menuRoutes
    .filter((item) => item.path || collectLeafPaths(item).length > 0)
    .map((item) => {
      const leafPaths = item.path ? [item.path] : collectLeafPaths(item);
      const defaultPath = item.path ?? leafPaths[0];
      const uniqueLeafPaths = Array.from(new Set(leafPaths));
      const key = resolveModuleKey(item, uniqueLeafPaths);
      const label = item.name ?? defaultPath ?? key;
      const groups = buildHeaderGroups((item.children as AppMenuItem[] | undefined) ?? [], key);

      return {
        key,
        label,
        groups,
        defaultPath,
        leafPaths: defaultPath
          ? [defaultPath, ...uniqueLeafPaths.filter((path) => path !== defaultPath)]
          : uniqueLeafPaths,
      };
    });
};

export const isPathActive = (currentPath: string, targetPath: string) => {
  if (currentPath === targetPath) {
    return true;
  }
  return currentPath.startsWith(`${targetPath}/`);
};

export const resolveActiveHeaderModuleKey = (
  modules: HeaderModuleItem[],
  currentPath: string,
) => {
  const match = modules.find((module) =>
    module.leafPaths.some((path) => isPathActive(currentPath, path)),
  );
  return match?.key;
};
