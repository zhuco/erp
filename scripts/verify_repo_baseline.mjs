import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const requiredFiles = [
  '.editorconfig',
  '.gitattributes',
  '.gitignore',
  'rustfmt.toml',
  'clippy.toml',
];

const requiredScripts = [
  'check:utf8',
  'lint',
  'typecheck',
  'test',
  'build',
  'accept:web',
  'accept:backend',
  'accept:all',
  'test:backend:release-gate',
  'test:e2e:release-gate',
  'release:gate',
  'release:gate:full',
  'repo:verify:openapi',
];

const workflowDirRelativePath = '.github/workflows';
const frontendGateWorkflowFiles = ['ci.yml', 'web-acceptance.yml'];
const requiredCiWorkflowCommands = [
  { type: 'install', label: 'pnpm install' },
  { type: 'script', script: 'repo:verify', label: 'pnpm run repo:verify（或 pnpm repo:verify）' },
  { type: 'script', script: 'accept:web', label: 'pnpm run accept:web（或 pnpm accept:web）' },
  {
    type: 'script',
    script: 'test:e2e:release-gate',
    label: 'pnpm --dir apps/erp-web test:e2e:release-gate',
  },
];
const forbiddenCiInlineScripts = new Set(['lint', 'typecheck', 'build', 'test:e2e']);
const pnpmDirectScriptCommands = new Set([
  'lint',
  'typecheck',
  'build',
  'test:e2e',
  'repo:verify',
  'accept:web',
]);
const parserRegressionRequiredCommands = requiredCiWorkflowCommands.filter(
  (requirement) => !(requirement.type === 'script' && requirement.script === 'test:e2e:release-gate'),
);

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..');
const deletedDocsPathPatterns = [
  /docs[\\/](?:dev|spec|ops|web|reports|superpowers|ErpDataGrid)(?:[\\/]|$)/,
  /docs[\\/]list\.md\b/,
];
const baselineScanExtensions = new Set([
  '.md',
  '.ts',
  '.tsx',
  '.rs',
  '.js',
  '.mjs',
  '.cjs',
  '.sh',
  '.sql',
  '.json',
  '.yaml',
  '.yml',
]);
const baselineScanExcludedDirs = new Set([
  '.git',
  'node_modules',
  'target',
  'dist',
  'build',
  'vendor',
  'coverage',
  '.worktrees',
  'tmp',
]);
const allowedDuplicateMigrationFiles = new Set([
  '0060_stock_count_diff_reason.sql',
  '0060_wms_pack_checks.sql',
  '0073_marketing_management.sql',
  '0073_procurement_management.sql',
  '0074_after_sales_service_ops.sql',
  '0074_logistics_fulfillment.sql',
  '0074_product_master_data.sql',
  '0079_ai1_platform_integration.sql',
  '0079_global_status_fields.sql',
  '0079_oms_order_statuses.sql',
  '0095_multi_merchant_org_baseline.sql',
  '0095_payment_reconcile_query_indexes.sql',
]);
const releaseGateRequiredStages = [
  { label: '登录', patterns: ['loginAndSelectTenant'] },
  { label: '订单', patterns: ['/orders/review', '/sales/order-query'] },
  { label: '库存', patterns: ['/inventory'] },
  { label: 'WMS', patterns: ['/wms/'] },
  { label: '发货', patterns: ['/wms/ship', 'ship-title', 'ship-table'] },
  { label: '售后', patterns: ['/after-sales'] },
  { label: '财务', patterns: ['/finance'] },
  { label: '报表', patterns: ['/stats', '/reports'] },
];

function fail(message) {
  console.error(message);
  process.exit(1);
}

function readTextFile(relativePath) {
  const filePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(filePath)) {
    fail(`缺少文件: ${relativePath}`);
  }
  return fs.readFileSync(filePath, 'utf8');
}

function listFilesRecursively(startDir, shouldInclude) {
  if (!fs.existsSync(startDir)) {
    return [];
  }
  const result = [];
  const entries = fs.readdirSync(startDir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(startDir, entry.name);
    if (entry.isDirectory()) {
      if (!baselineScanExcludedDirs.has(entry.name)) {
        result.push(...listFilesRecursively(fullPath, shouldInclude));
      }
      continue;
    }
    if (entry.isFile() && shouldInclude(fullPath)) {
      result.push(fullPath);
    }
  }
  return result;
}

function toRepoRelativePath(filePath) {
  return path.relative(repoRoot, filePath).replaceAll(path.sep, '/');
}

function verifyDeletedDocsPathsAreNotReferenced() {
  const files = listFilesRecursively(repoRoot, (filePath) => {
    const relativePath = toRepoRelativePath(filePath);
    if (relativePath === 'docs/ROADMAP.md') {
      return false;
    }
    return baselineScanExtensions.has(path.extname(filePath));
  });
  const hits = [];
  for (const filePath of files) {
    const relativePath = toRepoRelativePath(filePath);
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split(/\r?\n/);
    lines.forEach((line, index) => {
      if (deletedDocsPathPatterns.some((pattern) => pattern.test(line))) {
        hits.push(`${relativePath}:${index + 1}`);
      }
    });
  }
  if (hits.length > 0) {
    fail(`发现已删除历史文档路径引用，请改用当前 docs 核心文档入口: ${hits.join(', ')}`);
  }
}

function verifyMigrationVersionPrefixes() {
  const migrationsDir = path.join(repoRoot, 'migrations');
  if (!fs.existsSync(migrationsDir)) {
    fail('缺少 migrations 目录');
  }
  const versionToFiles = new Map();
  for (const fileName of fs.readdirSync(migrationsDir).filter((name) => name.endsWith('.sql'))) {
    const match = fileName.match(/^(\d+)_/);
    if (!match) {
      fail(`迁移文件名缺少数字前缀: ${fileName}`);
    }
    const version = match[1];
    const existing = versionToFiles.get(version) ?? [];
    existing.push(fileName);
    versionToFiles.set(version, existing);
  }
  const unexpectedDuplicateVersions = [...versionToFiles.entries()]
    .filter(([, files]) => files.length > 1)
    .map(([version, files]) => [
      version,
      files.filter((fileName) => !allowedDuplicateMigrationFiles.has(fileName)),
    ])
    .filter(([, unexpectedFiles]) => unexpectedFiles.length > 0)
    .map(([version, unexpectedFiles]) => `${version}: ${unexpectedFiles.join(', ')}`);
  if (unexpectedDuplicateVersions.length > 0) {
    fail(`迁移编号前缀出现新的重复，请先合并或重新编号: ${unexpectedDuplicateVersions.join(' | ')}`);
  }
}

function verifyReleaseGateP0Stages() {
  const releaseGateFiles = [
    'apps/erp-web/e2e/release-gate-main-chain.spec.ts',
    'apps/erp-web/e2e/release-gate-cross-chain.spec.ts',
  ];
  const combinedContent = releaseGateFiles.map(readTextFile).join('\n');
  const missingStages = releaseGateRequiredStages
    .filter(({ patterns }) => !patterns.some((pattern) => combinedContent.includes(pattern)))
    .map(({ label }) => label);
  if (missingStages.length > 0) {
    fail(`P0 Release Gate 缺少关键阶段覆盖: ${missingStages.join('、')}`);
  }
}

function normalizeFrontendPath(pathValue) {
  if (!pathValue || pathValue === '*') {
    return pathValue;
  }
  return pathValue.startsWith('/') ? pathValue : `/${pathValue}`;
}

function extractSingleQuotedPropertyValues(content, propertyName) {
  const values = [];
  const pattern = new RegExp(`${propertyName}:\\s*'([^']+)'`, 'g');
  let match = pattern.exec(content);
  while (match) {
    values.push(match[1]);
    match = pattern.exec(content);
  }
  return values;
}

function verifyFrontendRouteRegistryReachability() {
  const registryContent = readTextFile('apps/erp-web/src/app/routeRegistry.ts');
  const routerContent = readTextFile('apps/erp-web/src/app/router.tsx');
  const registryPaths = extractSingleQuotedPropertyValues(registryContent, 'path')
    .map(normalizeFrontendPath)
    .filter((routePath) => routePath && routePath !== '/login');
  const routerPaths = new Set(
    extractSingleQuotedPropertyValues(routerContent, 'path').map(normalizeFrontendPath),
  );
  const missingPaths = registryPaths.filter((routePath) => !routerPaths.has(routePath));
  if (missingPaths.length > 0) {
    fail(`前端路由注册表存在未挂载到 router 的路径: ${missingPaths.join(', ')}`);
  }
}

function listWorkflowFiles() {
  const workflowDir = path.join(repoRoot, workflowDirRelativePath);
  if (!fs.existsSync(workflowDir)) {
    fail(`缺少 workflow 目录: ${workflowDirRelativePath}`);
  }
  return fs
    .readdirSync(workflowDir)
    .filter((name) => fs.statSync(path.join(workflowDir, name)).isFile())
    .filter((name) => /\.ya?ml$/i.test(name));
}

function getLineIndent(line) {
  const match = line.match(/^\s*/);
  return match ? match[0].length : 0;
}

function stripWrappingQuotes(value) {
  const trimmed = value.trim();
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"'))
    || (trimmed.startsWith('\'') && trimmed.endsWith('\''))
  ) {
    return trimmed.slice(1, -1).trim();
  }
  return trimmed;
}

function normalizeYamlScalar(rawValue) {
  const trimmed = rawValue.trim();
  if (!trimmed) {
    return '';
  }
  const commentIndex = trimmed.search(/\s+#/);
  const withoutComment = commentIndex === -1 ? trimmed : trimmed.slice(0, commentIndex).trim();
  return stripWrappingQuotes(withoutComment);
}

function parseWorkflowStepChunk(stepLines, stepIndent) {
  const step = {
    line: stepLines[0].index + 1,
    uses: '',
    ifCondition: '',
    runLines: [],
  };

  let multilineKey = '';
  let multilineIndent = 0;

  function parseStepKeyValue(lineContent, lineIndent) {
    const keyValueMatch = lineContent.match(/^([A-Za-z_][A-Za-z0-9_-]*):\s*(.*)$/);
    if (!keyValueMatch) {
      return;
    }

    const key = keyValueMatch[1];
    const rawValue = keyValueMatch[2] ?? '';
    const trimmedRawValue = rawValue.trim();
    const isMultilineValue = /^[>|][+-]?$/.test(trimmedRawValue);

    if (isMultilineValue) {
      multilineKey = key;
      multilineIndent = lineIndent;
      return;
    }

    if (key === 'run') {
      const normalizedRunValue = stripWrappingQuotes(trimmedRawValue);
      if (normalizedRunValue) {
        step.runLines.push(normalizedRunValue);
      }
      return;
    }

    if (key === 'if') {
      step.ifCondition = normalizeYamlScalar(rawValue);
      return;
    }

    if (key === 'uses') {
      step.uses = normalizeYamlScalar(rawValue);
    }
  }

  const firstLineMatch = stepLines[0].text.match(/^\s*-\s*(.*)$/);
  const firstLineContent = firstLineMatch ? firstLineMatch[1].trim() : '';
  if (firstLineContent) {
    parseStepKeyValue(firstLineContent, stepIndent + 2);
  }

  for (let i = 1; i < stepLines.length; i += 1) {
    const { text } = stepLines[i];
    const trimmedLine = text.trim();
    const lineIndent = getLineIndent(text);

    if (multilineKey) {
      if (trimmedLine !== '' && lineIndent <= multilineIndent) {
        multilineKey = '';
      } else {
        if (multilineKey === 'run' && trimmedLine && !trimmedLine.startsWith('#')) {
          step.runLines.push(trimmedLine);
        }
        continue;
      }
    }

    if (trimmedLine === '' || trimmedLine.startsWith('#')) {
      continue;
    }

    // 仅接受 step 的直接子级键，避免误解析 with/env 等嵌套对象中的同名键。
    if (lineIndent !== stepIndent + 2) {
      continue;
    }

    const lineContent = text.slice(Math.min(stepIndent + 2, text.length)).trim();
    parseStepKeyValue(lineContent, lineIndent);
  }

  return step;
}

function parseWorkflowJobChunk(jobLines, jobIndent, jobName) {
  const job = {
    name: jobName,
    line: jobLines[0].index + 1,
    ifCondition: '',
    steps: [],
  };

  for (let i = 1; i < jobLines.length; i += 1) {
    const { text } = jobLines[i];
    const trimmedLine = text.trim();
    const lineIndent = getLineIndent(text);

    if (trimmedLine === '' || trimmedLine.startsWith('#')) {
      continue;
    }

    if (lineIndent !== jobIndent + 2) {
      continue;
    }

    const lineContent = text.slice(Math.min(jobIndent + 2, text.length)).trim();
    const keyValueMatch = lineContent.match(/^([A-Za-z_][A-Za-z0-9_-]*):\s*(.*)$/);
    if (!keyValueMatch) {
      continue;
    }

    const key = keyValueMatch[1];
    const rawValue = keyValueMatch[2] ?? '';
    if (key === 'if') {
      job.ifCondition = normalizeYamlScalar(rawValue);
      continue;
    }

    if (key !== 'steps') {
      continue;
    }

    const stepsIndent = lineIndent;
    let j = i + 1;
    while (j < jobLines.length) {
      const currentLine = jobLines[j].text;
      const currentTrimmed = currentLine.trim();
      const currentIndent = getLineIndent(currentLine);

      if (currentTrimmed !== '' && currentIndent <= stepsIndent) {
        break;
      }

      const stepStartMatch = currentLine.match(/^(\s*)-\s*(.*)$/);
      if (stepStartMatch && currentIndent > stepsIndent) {
        const stepIndent = stepStartMatch[1].length;
        const stepLines = [{ text: currentLine, index: jobLines[j].index }];
        j += 1;

        while (j < jobLines.length) {
          const nextLine = jobLines[j].text;
          const nextTrimmed = nextLine.trim();
          const nextIndent = getLineIndent(nextLine);
          const nextStepStartMatch = nextLine.match(/^(\s*)-\s*(.*)$/);

          if (nextTrimmed !== '' && nextIndent <= stepsIndent) {
            break;
          }
          if (nextStepStartMatch && nextIndent === stepIndent) {
            break;
          }

          stepLines.push({ text: nextLine, index: jobLines[j].index });
          j += 1;
        }

        job.steps.push(parseWorkflowStepChunk(stepLines, stepIndent));
        continue;
      }

      j += 1;
    }

    i = j - 1;
  }

  return job;
}

function parseWorkflowJobs(workflowContent) {
  const lines = workflowContent.split(/\r?\n/);
  const jobs = [];

  for (let i = 0; i < lines.length; i += 1) {
    const jobsMatch = lines[i].match(/^(\s*)jobs:\s*(?:#.*)?$/);
    if (!jobsMatch) {
      continue;
    }

    const jobsIndent = jobsMatch[1].length;
    let j = i + 1;
    while (j < lines.length) {
      const currentLine = lines[j];
      const currentTrimmed = currentLine.trim();
      const currentIndent = getLineIndent(currentLine);

      if (currentTrimmed !== '' && currentIndent <= jobsIndent) {
        break;
      }

      const jobStartMatch = currentLine.match(/^(\s*)([A-Za-z0-9_.-]+):\s*(?:#.*)?$/);
      if (jobStartMatch && currentIndent === jobsIndent + 2) {
        const jobIndent = jobStartMatch[1].length;
        const jobName = jobStartMatch[2];
        const jobLines = [{ text: currentLine, index: j }];
        j += 1;

        while (j < lines.length) {
          const nextLine = lines[j];
          const nextTrimmed = nextLine.trim();
          const nextIndent = getLineIndent(nextLine);
          const nextJobStartMatch = nextLine.match(/^(\s*)([A-Za-z0-9_.-]+):\s*(?:#.*)?$/);

          if (nextTrimmed !== '' && nextIndent <= jobsIndent) {
            break;
          }
          if (nextJobStartMatch && nextIndent === jobIndent) {
            break;
          }

          jobLines.push({ text: nextLine, index: j });
          j += 1;
        }

        jobs.push(parseWorkflowJobChunk(jobLines, jobIndent, jobName));
        continue;
      }

      j += 1;
    }

    i = j - 1;
  }

  return jobs;
}

function isConditionExplicitlyDisabled(ifCondition) {
  if (!ifCondition) {
    return false;
  }

  const normalized = stripWrappingQuotes(ifCondition).trim().toLowerCase();
  if (normalized === 'false') {
    return true;
  }
  return /^\$\{\{\s*false\s*\}\}$/.test(normalized);
}

function splitShellSegments(runLine) {
  return runLine
    .split(/&&|\|\||;|\|/)
    .map((segment) => segment.trim())
    .filter(Boolean);
}

function tokenizeShellSegment(segment) {
  const tokens = segment.match(/"[^"]*"|'[^']*'|\S+/g) ?? [];
  return tokens.map((token) => stripWrappingQuotes(token));
}

function parsePnpmInvocation(segment) {
  const tokens = tokenizeShellSegment(segment);
  if (tokens.length === 0) {
    return null;
  }

  let index = 0;
  while (index < tokens.length && /^[A-Za-z_][A-Za-z0-9_]*=.*/.test(tokens[index])) {
    index += 1;
  }
  if (index >= tokens.length || tokens[index] !== 'pnpm') {
    return null;
  }

  const args = tokens.slice(index + 1);
  const hasFilterOption = args.some((token) => (
    token === '--filter' || token === '-F' || token.startsWith('--filter=')
  ));

  function skipOptions(argsList, startIndex) {
    let cursor = startIndex;
    while (cursor < argsList.length) {
      const token = argsList[cursor];
      if (token === '--') {
        cursor += 1;
        break;
      }
      if (
        token === '--filter'
        || token === '-F'
        || token === '--dir'
        || token === '-C'
      ) {
        cursor += 2;
        continue;
      }
      if (token.startsWith('--filter=') || token.startsWith('--dir=')) {
        cursor += 1;
        continue;
      }
      if (token.startsWith('-')) {
        cursor += 1;
        continue;
      }
      break;
    }
    return cursor;
  }

  const commandIndex = skipOptions(args, 0);
  const command = args[commandIndex] ?? '';
  if (!command) {
    return null;
  }

  const remaining = args.slice(commandIndex + 1);
  let script = '';

  if (command === 'run') {
    const scriptIndex = skipOptions(remaining, 0);
    script = remaining[scriptIndex] ?? '';
  } else if (command.includes(':') || pnpmDirectScriptCommands.has(command)) {
    script = command;
  }

  return {
    rawSegment: segment,
    command,
    script,
    hasFilterOption,
  };
}

function getExecutableWorkflowRunSteps(workflowContent) {
  const jobs = parseWorkflowJobs(workflowContent);
  const executableSteps = [];

  for (const job of jobs) {
    if (isConditionExplicitlyDisabled(job.ifCondition)) {
      continue;
    }

    for (const step of job.steps) {
      if (step.runLines.length === 0) {
        continue;
      }
      if (isConditionExplicitlyDisabled(step.ifCondition)) {
        continue;
      }
      executableSteps.push(step);
    }
  }

  return executableSteps;
}

function collectPnpmInvocationsFromSteps(steps) {
  const invocations = [];
  for (const step of steps) {
    for (const runLine of step.runLines) {
      const segments = splitShellSegments(runLine);
      for (const segment of segments) {
        const invocation = parsePnpmInvocation(segment);
        if (invocation) {
          invocations.push({
            ...invocation,
            stepLine: step.line,
          });
        }
      }
    }
  }
  return invocations;
}

function invocationMatchesRequiredCommand(invocation, requirement) {
  if (requirement.type === 'install') {
    return invocation.command === 'install';
  }
  if (requirement.type === 'script') {
    return invocation.script === requirement.script;
  }
  return false;
}

function verifyWorkflowParserRegressionContracts() {
  const echoOnlyWorkflow = `
name: sample
on: push
jobs:
  demo:
    runs-on: ubuntu-latest
    steps:
      - run: echo "pnpm run accept:web"
`;
  const echoOnlyInvocations = collectPnpmInvocationsFromSteps(
    getExecutableWorkflowRunSteps(echoOnlyWorkflow),
  );
  const echoShouldNotMatch = echoOnlyInvocations.some((invocation) => (
    parserRegressionRequiredCommands.some((requirement) => (
      invocationMatchesRequiredCommand(invocation, requirement)
    ))
  ));
  if (echoShouldNotMatch) {
    fail('workflow 回归契约失败：echo 中出现的命令文本不应被识别为真实门禁命令');
  }

  const disabledRunWorkflow = `
name: sample
on: push
jobs:
  demo:
    runs-on: ubuntu-latest
    steps:
      - if: \${{ false }}
        run: pnpm run accept:web
`;
  const disabledInvocations = collectPnpmInvocationsFromSteps(
    getExecutableWorkflowRunSteps(disabledRunWorkflow),
  );
  if (disabledInvocations.length > 0) {
    fail('workflow 回归契约失败：显式禁用（if: false 或 ${{ false }}）的 step 不应计入可执行命令');
  }

  const multilineRunWorkflow = `
name: sample
on: push
jobs:
  demo:
    runs-on: ubuntu-latest
    steps:
      - run: |
          pnpm install
          pnpm run repo:verify && pnpm accept:web
`;
  const multilineInvocations = collectPnpmInvocationsFromSteps(
    getExecutableWorkflowRunSteps(multilineRunWorkflow),
  );
  const missingRequiredInMultiline = parserRegressionRequiredCommands.filter((requirement) => (
    !multilineInvocations.some((invocation) => (
      invocationMatchesRequiredCommand(invocation, requirement)
    ))
  ));
  if (missingRequiredInMultiline.length > 0) {
    fail('workflow 回归契约失败：多行 run 命令未被正确解析');
  }

  const nestedKeyWorkflow = `
name: sample
on: push
jobs:
  demo:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/setup-node@v4
        with:
          run: pnpm run accept:web
        env:
          if: false
`;
  const nestedKeyInvocations = collectPnpmInvocationsFromSteps(
    getExecutableWorkflowRunSteps(nestedKeyWorkflow),
  );
  if (nestedKeyInvocations.length > 0) {
    fail('workflow 回归契约失败：step 嵌套对象中的 run/if 不能被识别为 step 顶层字段');
  }

  const jobDisabledWorkflow = `
name: sample
on: push
jobs:
  demo:
    if: false
    runs-on: ubuntu-latest
    steps:
      - run: pnpm run accept:web
`;
  const jobDisabledInvocations = collectPnpmInvocationsFromSteps(
    getExecutableWorkflowRunSteps(jobDisabledWorkflow),
  );
  if (jobDisabledInvocations.length > 0) {
    fail('workflow 回归契约失败：job 显式禁用（if: false 或 ${{ false }}）时，step 命令不能计入');
  }
}

function verifyWorkflowConvergence() {
  const workflowFiles = listWorkflowFiles();
  const existingFrontendGateWorkflows = frontendGateWorkflowFiles.filter((name) => (
    workflowFiles.includes(name)
  ));

  if (
    existingFrontendGateWorkflows.includes('ci.yml')
    && existingFrontendGateWorkflows.includes('web-acceptance.yml')
  ) {
    fail('workflow 收敛失败：.github/workflows 不能同时存在 ci.yml 与 web-acceptance.yml');
  }

  if (
    existingFrontendGateWorkflows.length !== 1
    || existingFrontendGateWorkflows[0] !== 'ci.yml'
  ) {
    fail('workflow 收敛失败：前端门禁 workflow 只能保留 ci.yml');
  }

  const ciWorkflowContent = readTextFile(`${workflowDirRelativePath}/ci.yml`);
  const executableSteps = getExecutableWorkflowRunSteps(ciWorkflowContent);
  const ciInvocations = collectPnpmInvocationsFromSteps(executableSteps);

  const missingRequiredCommands = requiredCiWorkflowCommands.filter((requirement) => (
    !ciInvocations.some((invocation) => invocationMatchesRequiredCommand(invocation, requirement))
  ));
  if (missingRequiredCommands.length > 0) {
    const missingLabels = missingRequiredCommands.map((item) => item.label).join('、');
    fail(`workflow 收敛失败：ci.yml 缺少必需根级命令: ${missingLabels}`);
  }

  const forbiddenInvocation = ciInvocations.find((invocation) => (
    invocation.hasFilterOption || forbiddenCiInlineScripts.has(invocation.script)
  ));
  if (forbiddenInvocation) {
    fail(
      `workflow 收敛失败：ci.yml 不允许内联前端门禁命令（命中 step 行 ${forbiddenInvocation.stepLine}: ${forbiddenInvocation.rawSegment.trim()}）`,
    );
  }
}

function getMakeTargetCommands(makefileContent, targetName) {
  const lines = makefileContent.split(/\r?\n/);
  const targetLine = `${targetName}:`;
  const startIndex = lines.findIndex((line) => line.trim() === targetLine);
  if (startIndex === -1) {
    return null;
  }

  const commands = [];
  for (let i = startIndex + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^[A-Za-z0-9_.-]+:/.test(line) && !/^\s/.test(line)) {
      break;
    }
    if (/^\t/.test(line)) {
      commands.push(line.trim());
    }
  }
  return commands;
}

function normalizeMakeCommand(line) {
  return line.replace(/^@/, '').trim();
}

function containsForbiddenRootPnpmTest(line) {
  const command = normalizeMakeCommand(line);
  return /\bpnpm\s+test(?=$|[\s;|&])/.test(command);
}

function verifyAcceptWrapperStructure(relativePath, requiredCommand) {
  const content = readTextFile(relativePath);
  const lines = content.split(/\r?\n/).map((line) => line.trim());

  const hasRootDirLine = lines.some(
    (line) => line.startsWith('ROOT_DIR=') && !line.startsWith('#'),
  );
  if (!hasRootDirLine) {
    fail(`${relativePath} 缺少独立 ROOT_DIR= 赋值行`);
  }

  const hasCdRootDirLine = lines.some((line) => line === 'cd "$ROOT_DIR"');
  if (!hasCdRootDirLine) {
    fail(`${relativePath} 缺少独立 cd "$ROOT_DIR" 行`);
  }

  const hasRequiredCommand = lines.some((line) => line.includes(requiredCommand));
  if (!hasRequiredCommand) {
    fail(`${relativePath} 必须调用 ${requiredCommand}`);
  }

  if (!content.includes('兼容包装层')) {
    fail(`${relativePath} 缺少兼容包装层中文注释`);
  }
}

const bypassLine = 'pnpm --filter erp-web test:unit && pnpm test';
if (!containsForbiddenRootPnpmTest(bypassLine)) {
  fail('web-test 负向契约回归失败：未拦截混合命令（pnpm --filter erp-web test:unit && pnpm test）');
}

const noSpaceSeparatorBypassLines = [
  'pnpm test;echo ok',
  'pnpm test||echo fallback',
];
if (!noSpaceSeparatorBypassLines.every((line) => containsForbiddenRootPnpmTest(line))) {
  fail('web-test 负向契约回归失败：未拦截 pnpm test 的无空格分隔场景（; 或 ||）');
}

verifyWorkflowParserRegressionContracts();
verifyWorkflowConvergence();
verifyDeletedDocsPathsAreNotReferenced();
verifyMigrationVersionPrefixes();
verifyReleaseGateP0Stages();
verifyFrontendRouteRegistryReachability();

const missingFiles = requiredFiles.filter(
  (file) => !fs.existsSync(path.join(repoRoot, file)),
);
if (missingFiles.length > 0) {
  fail(`缺少根级基线文件: ${missingFiles.join(', ')}`);
}

const rootPackageJsonPath = path.join(repoRoot, 'package.json');
if (!fs.existsSync(rootPackageJsonPath)) {
  fail('缺少根级 package.json 文件');
}

const rootPackageJson = JSON.parse(fs.readFileSync(rootPackageJsonPath, 'utf8'));
const scripts = rootPackageJson.scripts ?? {};
const missingScripts = requiredScripts.filter((scriptName) => !scripts[scriptName]);
if (missingScripts.length > 0) {
  fail(`根级 package.json 缺少统一命令 scripts: ${missingScripts.join(', ')}`);
}

const entrypointNote = rootPackageJson['x-entrypoint-note'];
if (
  typeof entrypointNote !== 'string'
  || !entrypointNote.includes('package.json')
  || !entrypointNote.includes('官方入口')
  || !entrypointNote.includes('兼容包装层')
  || !entrypointNote.includes('Makefile')
  || !entrypointNote.includes('scripts/accept_*.sh')
  || entrypointNote.includes('scripts/*.sh')
) {
  fail('根级 package.json 的入口说明不符合约束：需明确为 Makefile 与 scripts/accept_*.sh 兼容包装层');
}

verifyAcceptWrapperStructure('scripts/accept_web.sh', 'pnpm accept:web');
verifyAcceptWrapperStructure('scripts/accept_backend.sh', 'pnpm accept:backend');
verifyAcceptWrapperStructure('scripts/accept_all.sh', 'pnpm accept:all');

const makefileContent = readTextFile('Makefile');
if (!makefileContent.includes('兼容包装层')) {
  fail('Makefile 缺少兼容包装层中文注释');
}

const acceptWebCommands = getMakeTargetCommands(makefileContent, 'accept-web');
if (!acceptWebCommands || !acceptWebCommands.some((line) => line.includes('pnpm accept:web'))) {
  fail('Makefile 的 accept-web 必须调用根级 pnpm accept:web');
}
if (acceptWebCommands.some((line) => /pnpm --filter|cargo /.test(line))) {
  fail('Makefile 的 accept-web 不能内联业务命令');
}

const acceptBackendCommands = getMakeTargetCommands(makefileContent, 'accept-backend');
if (
  !acceptBackendCommands
  || !acceptBackendCommands.some((line) => line.includes('pnpm accept:backend'))
) {
  fail('Makefile 的 accept-backend 必须调用根级 pnpm accept:backend');
}
if (acceptBackendCommands.some((line) => /pnpm --filter|cargo /.test(line))) {
  fail('Makefile 的 accept-backend 不能内联业务命令');
}

const acceptAllCommands = getMakeTargetCommands(makefileContent, 'accept-all');
if (!acceptAllCommands || !acceptAllCommands.some((line) => line.includes('pnpm accept:all'))) {
  fail('Makefile 的 accept-all 必须调用根级 pnpm accept:all');
}
if (acceptAllCommands.some((line) => /pnpm --filter|cargo /.test(line))) {
  fail('Makefile 的 accept-all 不能内联业务命令');
}

const webTestCommands = getMakeTargetCommands(makefileContent, 'web-test');
if (!webTestCommands) {
  fail('Makefile 缺少 web-test 目标');
}
if (!webTestCommands.some((line) => line.includes('pnpm --filter erp-web test:unit'))) {
  fail('Makefile 的 web-test 必须包含 pnpm --filter erp-web test:unit');
}
if (webTestCommands.some((line) => containsForbiddenRootPnpmTest(line))) {
  fail('Makefile 的 web-test 不允许出现根级 pnpm test（含混合命令场景）');
}
