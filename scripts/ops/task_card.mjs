import { spawnSync } from 'node:child_process';

const KEY_MAP = {
  name: 'name',
  goal: 'goal',
  background: 'background',
  owner: 'owner',
  branch: 'branch',
  worktree: 'worktree',
  allowed_files: 'allowedFiles',
  allowedFiles: 'allowedFiles',
  disallowed_files: 'disallowedFiles',
  disallowedFiles: 'disallowedFiles',
  dependencies: 'dependencies',
  dod: 'dod',
  test_commands: 'testCommands',
  testCommands: 'testCommands',
  delivery: 'delivery',
  retry_policy: 'retryPolicy',
  retryPolicy: 'retryPolicy',
  fallback_strategy: 'fallbackStrategy',
  fallbackStrategy: 'fallbackStrategy',
};

function ensureArray(value) {
  if (value === undefined || value === null) {
    return [];
  }
  if (Array.isArray(value)) {
    return value;
  }
  return [value];
}

function sanitizeScalar(value) {
  if (value === undefined || value === null) {
    return '';
  }
  return String(value).replace(/[\r\n]+/g, ' ').trim();
}

function listLines(key, items) {
  if (!items.length) {
    return [`${key}: []`];
  }
  const lines = [`${key}:`];
  for (const item of items) {
    lines.push(`  - ${sanitizeScalar(item)}`);
  }
  return lines;
}

function normalizePath(filePath) {
  return String(filePath).replace(/\\/g, '/').replace(/^\.\/+/, '');
}

function escapeRegex(pattern) {
  return pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
}

function globToRegExp(pattern) {
  const normalized = normalizePath(pattern);
  const withGlobstar = normalized.replace(/\*\*/g, '::GLOBSTAR::');
  const withStar = withGlobstar.replace(/\*/g, '::STAR::');
  const escaped = escapeRegex(withStar);
  const restoredStar = escaped.replace(/::STAR::/g, '[^/]*');
  const finalPattern = restoredStar.replace(/::GLOBSTAR::/g, '.*');
  return new RegExp(`^${finalPattern}$`);
}

function matchesAnyPattern(filePath, patterns) {
  const normalizedFile = normalizePath(filePath);
  return ensureArray(patterns)
    .filter(Boolean)
    .some((pattern) => globToRegExp(pattern).test(normalizedFile));
}

function uniqueSorted(items) {
  return [...new Set(items.map(normalizePath).filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

function buildFrontMatter(metadata) {
  const delivery = sanitizeScalar(metadata.delivery) || 'Markdown + artifacts';
  const lines = ['---'];
  lines.push(`name: ${sanitizeScalar(metadata.name)}`);
  lines.push(`goal: ${sanitizeScalar(metadata.goal)}`);
  lines.push(`background: ${sanitizeScalar(metadata.background)}`);
  lines.push(`owner: ${sanitizeScalar(metadata.owner)}`);
  lines.push(`branch: ${sanitizeScalar(metadata.branch)}`);
  lines.push(`worktree: ${sanitizeScalar(metadata.worktree)}`);
  lines.push(...listLines('allowed_files', ensureArray(metadata.allowedFiles)));
  lines.push(...listLines('disallowed_files', ensureArray(metadata.disallowedFiles)));
  lines.push(...listLines('dependencies', ensureArray(metadata.dependencies)));
  lines.push(...listLines('dod', ensureArray(metadata.dod)));
  lines.push(...listLines('test_commands', ensureArray(metadata.testCommands)));
  lines.push(`retry_policy: ${sanitizeScalar(metadata.retryPolicy) || 'transient-auto-retry(max=2,delay=8s)'}`);
  lines.push(`fallback_strategy: ${sanitizeScalar(metadata.fallbackStrategy) || 'switch-to-local-execution'}`);
  lines.push(`delivery: ${delivery}`);
  lines.push('---');
  return lines.join('\n');
}

function bulletList(items, emptyPlaceholder, formatter = (item) => `- ${item}`) {
  const list = ensureArray(items).map(formatter);
  return list.length ? list.join('\n') : emptyPlaceholder;
}

function buildBodySections(metadata) {
  const sections = [];
  const goal = sanitizeScalar(metadata.goal);
  if (goal) {
    sections.push(`## Goal\n${goal}`);
  }
  const background = sanitizeScalar(metadata.background);
  if (background) {
    sections.push(`## Background\n${background}`);
  }
  sections.push(`## DoD\n${bulletList(metadata.dod, '- TBD')}`);
  sections.push(`## Dependencies\n${bulletList(metadata.dependencies, '- None')}`);
  sections.push(
    `## Test Commands\n${bulletList(metadata.testCommands, '- TBD', (item) => `- \`${item}\``)}`,
  );
  sections.push(`## Retry Policy\n${sanitizeScalar(metadata.retryPolicy) || 'transient-auto-retry(max=2,delay=8s)'}`);
  sections.push(`## Fallback Strategy\n${sanitizeScalar(metadata.fallbackStrategy) || 'switch-to-local-execution'}`);
  const delivery = sanitizeScalar(metadata.delivery) || 'Markdown + artifacts';
  sections.push(`## Delivery\n${delivery}`);
  return sections.join('\n\n');
}

function createTaskCard(metadata) {
  const front = buildFrontMatter(metadata);
  const body = buildBodySections(metadata);
  return `${front}\n\n${body}\n`;
}

function parseTaskCard(cardText) {
  const match = cardText.match(/^---\s*\r?\n([\s\S]*?)\r?\n---/);
  if (!match) {
    throw new Error('task card front matter is missing');
  }
  const raw = match[1];
  const result = {};
  let currentListKey = null;
  for (const line of raw.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    const listItem = trimmed.match(/^\-\s*(.*)$/);
    if (listItem && currentListKey) {
      result[currentListKey].push(listItem[1]);
      continue;
    }
    const keyMatch = trimmed.match(/^([a-zA-Z0-9_]+):\s*(.*)$/);
    if (!keyMatch) {
      currentListKey = null;
      continue;
    }
    const rawKey = keyMatch[1];
    const normalizedKey = KEY_MAP[rawKey] ?? rawKey;
    const value = keyMatch[2];
    if (value === '[]') {
      result[normalizedKey] = [];
      currentListKey = null;
    } else if (!value) {
      result[normalizedKey] = [];
      currentListKey = normalizedKey;
    } else {
      currentListKey = null;
      result[normalizedKey] = value;
    }
  }
  return result;
}

function validateTaskFiles(taskCard, files) {
  const allowedFiles = ensureArray(taskCard.allowedFiles);
  const disallowedFiles = ensureArray(taskCard.disallowedFiles);
  const normalizedFiles = uniqueSorted(files);

  const blockedFiles = normalizedFiles.filter((filePath) => matchesAnyPattern(filePath, disallowedFiles));
  const outOfScopeFiles = normalizedFiles.filter((filePath) => {
    if (!allowedFiles.length) {
      return true;
    }
    return !matchesAnyPattern(filePath, allowedFiles);
  });

  return {
    ok: blockedFiles.length === 0 && outOfScopeFiles.length === 0,
    checkedFiles: normalizedFiles,
    blockedFiles,
    outOfScopeFiles,
  };
}

function runGit(repoRoot, args) {
  const result = spawnSync('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || `git ${args.join(' ')} failed`);
  }
  return result.stdout;
}

function readGitChangedFiles({ repoRoot, mode = 'working' }) {
  const selectedMode = mode || 'working';
  if (!['working', 'staged', 'all'].includes(selectedMode)) {
    throw new Error(`unsupported git diff mode: ${selectedMode}`);
  }

  const modifiedWorkingFiles = selectedMode === 'staged'
    ? []
    : runGit(repoRoot, ['diff', '--name-only']).split(/\r?\n/).filter(Boolean);
  const untrackedWorkingFiles = selectedMode === 'staged'
    ? []
    : runGit(repoRoot, ['ls-files', '--others', '--exclude-standard']).split(/\r?\n/).filter(Boolean);
  const stagedFiles = selectedMode === 'working'
    ? []
    : runGit(repoRoot, ['diff', '--name-only', '--cached']).split(/\r?\n/).filter(Boolean);

  return uniqueSorted([...modifiedWorkingFiles, ...untrackedWorkingFiles, ...stagedFiles]);
}

export {
  createTaskCard,
  parseTaskCard,
  readGitChangedFiles,
  validateTaskFiles,
};

