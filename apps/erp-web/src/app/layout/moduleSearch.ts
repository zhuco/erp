import type { AppMenuItem } from './menu';
import { normalizeVisibleText } from './visibleText';

export type FunctionSearchEntry = {
  path: string;
  name: string;
  moduleName: string;
  groupName?: string;
  aliases?: string[];
};

export type FunctionRecommendation = FunctionSearchEntry & {
  source: 'recent' | 'shortcut';
};

export type FunctionSearchGroup<TEntry> = {
  key: string;
  label: string;
  items: TEntry[];
};

const normalizeText = (value: string) => normalizeVisibleText(value).trim().toLowerCase();

const functionSearchAliases: Record<string, string[]> = {
  '/orders/review': ['订单审核', '单审'],
};

const isSubsequenceMatch = (keyword: string, candidate: string) => {
  if (!keyword) {
    return false;
  }
  let keywordIndex = 0;
  for (const char of candidate) {
    if (char === keyword[keywordIndex]) {
      keywordIndex += 1;
      if (keywordIndex === keyword.length) {
        return true;
      }
    }
  }
  return false;
};

const scoreText = (
  keyword: string,
  candidate: string,
  weights: {
    exact: number;
    prefix: number;
    includes: number;
    fuzzy: number;
  },
) => {
  if (!candidate) {
    return 0;
  }
  if (candidate === keyword) {
    return weights.exact;
  }
  if (candidate.startsWith(keyword)) {
    return weights.prefix;
  }
  if (candidate.includes(keyword)) {
    return weights.includes;
  }
  if (isSubsequenceMatch(keyword, candidate)) {
    return weights.fuzzy;
  }
  return 0;
};

export const buildFunctionSearchEntries = (menuItems: AppMenuItem[]): FunctionSearchEntry[] => {
  const entries: FunctionSearchEntry[] = [];

  const visit = (item: AppMenuItem, moduleName: string, groupName?: string) => {
    const children = item.children ?? [];

    if (item.path && children.length === 0) {
      entries.push({
        path: item.path,
        name: normalizeVisibleText(item.name ?? item.path),
        moduleName: normalizeVisibleText(moduleName),
        groupName: groupName ? normalizeVisibleText(groupName) : undefined,
        aliases: functionSearchAliases[item.path] ?? [],
      });
      return;
    }

    const nextGroupName =
      item.path || item.name === moduleName ? groupName : normalizeVisibleText(item.name ?? groupName ?? '');
    children.forEach((child) => visit(child, moduleName, nextGroupName));
  };

  menuItems.forEach((moduleItem) => {
    const moduleName = moduleItem.name ?? moduleItem.path ?? '未命名模块';
    if (moduleItem.path && (!moduleItem.children || moduleItem.children.length === 0)) {
      entries.push({
        path: moduleItem.path,
        name: normalizeVisibleText(moduleName),
        moduleName: normalizeVisibleText(moduleName),
        aliases: functionSearchAliases[moduleItem.path] ?? [],
      });
      return;
    }
    moduleItem.children?.forEach((child) => visit(child, moduleName));
  });

  return entries;
};

export const searchFunctionEntries = (
  entries: FunctionSearchEntry[],
  keyword: string,
  limit = 20,
) => {
  const normalizedKeyword = normalizeText(keyword);
  if (!normalizedKeyword) {
    return entries.slice(0, limit);
  }

  return entries
    .map((entry) => {
      const normalizedName = normalizeText(entry.name);
      const normalizedModuleName = normalizeText(entry.moduleName);
      const normalizedGroupName = normalizeText(entry.groupName ?? '');
      const normalizedPath = normalizeText(entry.path);
      const aliasScore = Math.max(
        ...((entry.aliases ?? []).map((alias) =>
          scoreText(normalizedKeyword, normalizeText(alias), {
            exact: 138,
            prefix: 128,
            includes: 118,
            fuzzy: 112,
          }),
        )),
        0,
      );

      const score = Math.max(
        scoreText(normalizedKeyword, normalizedName, {
          exact: 140,
          prefix: 130,
          includes: 120,
          fuzzy: 110,
        }),
        scoreText(normalizedKeyword, normalizedGroupName, {
          exact: 100,
          prefix: 95,
          includes: 90,
          fuzzy: 80,
        }),
        scoreText(normalizedKeyword, normalizedModuleName, {
          exact: 75,
          prefix: 70,
          includes: 65,
          fuzzy: 55,
        }),
        scoreText(normalizedKeyword, normalizedPath, {
          exact: 60,
          prefix: 55,
          includes: 50,
          fuzzy: 40,
        }),
        aliasScore,
      );

      return { entry, score };
    })
    .filter((item) => item.score > 0)
    .sort((left, right) => {
      if (right.score !== left.score) {
        return right.score - left.score;
      }
      return left.entry.name.localeCompare(right.entry.name, 'zh-CN');
    })
    .slice(0, limit)
    .map((item) => item.entry);
};

export const buildFunctionRecommendations = (
  entries: FunctionSearchEntry[],
  options: {
    recentPaths?: string[];
    shortcutPaths?: string[];
    limit?: number;
  },
): FunctionRecommendation[] => {
  const entryMap = new Map(entries.map((entry) => [entry.path, entry]));
  const results: FunctionRecommendation[] = [];
  const seen = new Set<string>();
  const limit = options.limit ?? 8;

  const append = (paths: string[] | undefined, source: FunctionRecommendation['source']) => {
    paths?.forEach((path) => {
      if (seen.has(path) || results.length >= limit) {
        return;
      }
      const matchedEntry = entryMap.get(path);
      if (!matchedEntry) {
        return;
      }
      seen.add(path);
      results.push({
        ...matchedEntry,
        source,
      });
    });
  };

  append(options.recentPaths, 'recent');
  append(options.shortcutPaths, 'shortcut');

  return results;
};

const groupItems = <TEntry>(
  items: TEntry[],
  resolveKey: (item: TEntry) => string,
  resolveLabel: (item: TEntry) => string,
): FunctionSearchGroup<TEntry>[] => {
  const groups = new Map<string, FunctionSearchGroup<TEntry>>();

  items.forEach((item) => {
    const key = resolveKey(item);
    const currentGroup = groups.get(key);
    if (currentGroup) {
      currentGroup.items.push(item);
      return;
    }
    groups.set(key, {
      key,
      label: resolveLabel(item),
      items: [item],
    });
  });

  return Array.from(groups.values());
};

export const groupFunctionEntriesByModule = (
  entries: FunctionSearchEntry[],
): FunctionSearchGroup<FunctionSearchEntry>[] => {
  return groupItems(
    entries,
    (entry) => `module:${entry.moduleName}`,
    (entry) => normalizeVisibleText(entry.moduleName),
  );
};

export const groupFunctionRecommendationsBySource = (
  recommendations: FunctionRecommendation[],
): FunctionSearchGroup<FunctionRecommendation>[] => {
  return groupItems(
    recommendations,
    (recommendation) => `source:${recommendation.source}`,
    (recommendation) =>
      recommendation.source === 'recent'
        ? '最近使用'
        : '常用功能',
  );
};

export const mergeRecentFunctionPaths = (currentPaths: string[], nextPath: string, limit = 6) => {
  return [nextPath, ...currentPaths.filter((path) => path !== nextPath)].slice(0, limit);
};
