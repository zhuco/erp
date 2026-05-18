/* eslint-env node */

import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');
const pagesRoot = path.join(projectRoot, 'src', 'pages');
const baselineFile = path.join(__dirname, 'page-template-baseline.json');

const args = new Set(process.argv.slice(2));
const shouldOutputJson = args.has('--json');

const REQUIRED_TEMPLATE_ADOPTION_RATE = 0.95;
const UI_UNIFORMITY_TARGET_SCORE = 95;
const NEXT_BATCH_LIMIT = 16;

const SCORE_WEIGHTS = {
  templateAdoption: 20,
  operationConsistency: 25,
  tableConsistency: 20,
  visualTextConsistency: 15,
  a11yKeyboardConsistency: 10,
  engineeringGovernance: 10,
};

const recommendedTemplateTokens = [
  'WorkbenchPageTemplate',
  'SettingsPageTemplate',
  'UiModule03MainTableSection',
  'PlaceholderPage',
];

const componentOnlyPathSegments = [
  '/components/',
  '/shared/',
  '/review-native/',
  '/exceptions-native/',
  '/goods/product-form/',
  '/goods/kit-form/',
];

const p0PathPatterns = [
  '/orders/',
  '/sales/',
  '/products/',
  '/goods/products-query/',
  '/shopProducts/',
  '/inventory/',
  '/wms/',
  '/warehouses/',
  '/procurement/',
  '/afterSales/',
  '/finance/',
];

const p2PathPatterns = [
  '/login',
  '/auth/',
  '/insights/',
  '/stats/',
  '/dashboard',
  '/goods/product-form/',
  '/goods/kit-form/',
];

const p2FileNamePattern = /(?:Detail|Create|Edit|Settings|Config|Dashboard|Insight|Stats|Report)Page\.tsx$/;

const toPosixPath = (value) => value.split(path.sep).join('/');
const toFixedNumber = (value, digits = 4) => Number(value.toFixed(digits));
const formatRate = (rate) => `${(rate * 100).toFixed(2)}%`;
const formatScore = (score) => score.toFixed(1);
const safeRate = (count, total, emptyValue = 0) => (total > 0 ? count / total : emptyValue);
const countWhere = (items, predicate) => items.filter(predicate).length;

const isApplicableTemplatePage = (relativeFile) => {
  if (!relativeFile.startsWith('src/pages/') || !relativeFile.endsWith('.tsx')) {
    return false;
  }
  if (relativeFile.endsWith('.test.tsx') || relativeFile.endsWith('.test.ts')) {
    return false;
  }
  if (componentOnlyPathSegments.some((segment) => relativeFile.includes(segment))) {
    return false;
  }
  if (/\/(components|shared)\//.test(relativeFile)) {
    return false;
  }
  return /(?:Page|Workbench)\.tsx$/.test(relativeFile);
};

const collectPageFiles = async (dir) => {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const results = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...(await collectPageFiles(fullPath)));
      continue;
    }

    if (entry.isFile() && entry.name.endsWith('Page.tsx')) {
      results.push(fullPath);
    }
  }

  return results.sort((left, right) => toPosixPath(left).localeCompare(toPosixPath(right)));
};

const readBaseline = async () => {
  try {
    const raw = await fs.readFile(baselineFile, 'utf8');
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((item) => typeof item === 'string').sort() : [];
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') {
      return [];
    }
    throw error;
  }
};

const resolveWrappedFile = async (basePath) => {
  const candidates = [
    basePath,
    `${basePath}.tsx`,
    `${basePath}.ts`,
    path.join(basePath, 'index.tsx'),
    path.join(basePath, 'index.ts'),
  ];

  for (const candidate of candidates) {
    try {
      await fs.access(candidate);
      return candidate;
    } catch {
      continue;
    }
  }

  return null;
};

const hasTemplateReference = async (source, file, visited = new Set()) => {
  if (recommendedTemplateTokens.some((token) => source.includes(token))) {
    return true;
  }

  const normalizedFile = toPosixPath(file);
  if (visited.has(normalizedFile)) {
    return false;
  }
  visited.add(normalizedFile);

  const wrapperImportPattern = /import\s+\{\s*([^}]+)\s*\}\s+from\s+['"]((?:\.\.?\/)[^'"]+)['"];?/g;
  const wrapperMatches = Array.from(source.matchAll(wrapperImportPattern));

  for (const match of wrapperMatches) {
    const importedNames = match[1]
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean)
      .map((item) => item.replace(/^type\s+/, ''))
      .map((item) => item.split(/\s+as\s+/i).pop()?.trim() ?? '')
      .filter(Boolean);

    const referencedName = importedNames.find((name) => new RegExp(`<${name}\\b`).test(source));
    if (!referencedName) {
      continue;
    }

    const wrappedFile = await resolveWrappedFile(path.resolve(path.dirname(file), match[2]));
    if (!wrappedFile) {
      continue;
    }

    try {
      const wrappedSource = await fs.readFile(wrappedFile, 'utf8');
      if (await hasTemplateReference(wrappedSource, wrappedFile, visited)) {
        return true;
      }
    } catch {
      continue;
    }
  }

  return false;
};

const classifyPagePriority = (relativeFile, source, isApplicable) => {
  if (!isApplicable || p2PathPatterns.some((pattern) => relativeFile.includes(pattern)) || p2FileNamePattern.test(relativeFile)) {
    return 'P2';
  }

  if (p0PathPatterns.some((pattern) => relativeFile.includes(pattern))) {
    return 'P0';
  }

  if (source.includes('WorkbenchPageTemplate') || source.includes('UiModule03MainTableSection') || source.includes('<Table')) {
    return 'P1';
  }

  return 'P2';
};

const classifyPageType = (relativeFile, source) => {
  if (/(?:Create|Edit)Page\.tsx$/.test(relativeFile)) {
    return '编辑页';
  }
  if (/DetailPage\.tsx$/.test(relativeFile)) {
    return '详情页';
  }
  if (/(?:Settings|Config)Page\.tsx$/.test(relativeFile)) {
    return '设置页';
  }
  if (/(?:Dashboard|Insight|Stats|Report)Page\.tsx$/.test(relativeFile) || relativeFile.includes('/insights/')) {
    return '看板/报表页';
  }
  if (source.includes('WorkbenchPageTemplate') || source.includes('UiModule03MainTableSection') || /<Table\b/.test(source)) {
    return '列表/工作台页';
  }
  return '其他页';
};

const hasAnyToken = (source, tokens) => tokens.some((token) => source.includes(token));

const buildPageAnalysis = async (file, baselineSet) => {
  const source = await fs.readFile(file, 'utf8');
  const relativeFile = toPosixPath(path.relative(projectRoot, file));
  const isApplicable = isApplicableTemplatePage(relativeFile);
  const isBaselineException = baselineSet.has(relativeFile);
  const workbenchPageTemplate = source.includes('WorkbenchPageTemplate');
  const uiModule03MainTableSection = source.includes('UiModule03MainTableSection');
  const settingsPageTemplate = source.includes('SettingsPageTemplate');
  const placeholderPage = source.includes('PlaceholderPage');
  const hasRecommendedTemplate = hasAnyToken(source, recommendedTemplateTokens);
  const hasGovernanceTemplateReference = await hasTemplateReference(source, file);
  const bareTable = /<Table\b/.test(source);
  const bareCard = /<Card\b/.test(source);
  const industrialReviewCompact = source.includes('gridPreset="industrial-review-compact"');
  const customSelectionCell = source.includes('selectionCellMode="custom"');
  const allInvertSelectionHeader = source.includes('selectionHeaderMode="allInvert"');
  const workbenchToolbar = source.includes('WorkbenchToolbar') || /\btoolbar(?:Left|Center|Right)?\s*=/.test(source);
  const dataTestId = source.includes('data-testid') || /\bdataTestId\s*=/.test(source);
  const columnSettings = /ColumnSettings|useWorkbenchColumnSettings|columnSettings/.test(source) || source.includes('列设置');
  const leftPanel = /\bleftPanel\s*=/.test(source);
  const rightPanel = /\brightPanel\s*=/.test(source);
  const bottomPanel = /\bbottomPanel\s*=/.test(source);
  const hasTableSurface = workbenchPageTemplate || uiModule03MainTableSection || bareTable || industrialReviewCompact;
  const priority = classifyPagePriority(relativeFile, source, isApplicable);
  const pageType = classifyPageType(relativeFile, source);

  return {
    file: relativeFile,
    priority,
    pageType,
    isApplicable,
    isBaselineException,
    workbenchPageTemplate,
    uiModule03MainTableSection,
    settingsPageTemplate,
    placeholderPage,
    hasRecommendedTemplate,
    hasGovernanceTemplateReference,
    bareTable,
    bareCard,
    industrialReviewCompact,
    customSelectionCell,
    allInvertSelectionHeader,
    workbenchToolbar,
    dataTestId,
    columnSettings,
    leftPanel,
    rightPanel,
    bottomPanel,
    hasTableSurface,
  };
};

const buildGroupMetrics = (group) => {
  const applicableGroup = group.filter((page) => page.isApplicable && !page.isBaselineException);
  const totalPages = group.length;
  const applicablePages = applicableGroup.length;
  const governanceTemplatePages = countWhere(applicableGroup, (page) => page.hasGovernanceTemplateReference);
  const workbenchPageTemplatePages = countWhere(group, (page) => page.workbenchPageTemplate);
  const uiModule03MainTableSectionPages = countWhere(group, (page) => page.uiModule03MainTableSection);
  const bareTablePages = countWhere(group, (page) => page.bareTable);
  const bareCardPages = countWhere(group, (page) => page.bareCard);
  const industrialReviewCompactPages = countWhere(group, (page) => page.industrialReviewCompact);
  const customSelectionCellPages = countWhere(group, (page) => page.customSelectionCell);
  const allInvertSelectionHeaderPages = countWhere(group, (page) => page.allInvertSelectionHeader);
  const workbenchToolbarPages = countWhere(group, (page) => page.workbenchToolbar);
  const dataTestIdPages = countWhere(group, (page) => page.dataTestId);
  const columnSettingsPages = countWhere(group, (page) => page.columnSettings);
  const baselineExceptions = countWhere(group, (page) => page.isBaselineException);

  return {
    totalPages,
    applicablePages,
    governanceTemplatePages,
    workbenchPageTemplatePages,
    uiModule03MainTableSectionPages,
    bareTablePages,
    bareCardPages,
    industrialReviewCompactPages,
    customSelectionCellPages,
    allInvertSelectionHeaderPages,
    workbenchToolbarPages,
    dataTestIdPages,
    columnSettingsPages,
    baselineExceptions,
    rates: {
      governanceTemplateRate: toFixedNumber(safeRate(governanceTemplatePages, applicablePages, 1)),
      workbenchPageTemplateRate: toFixedNumber(safeRate(workbenchPageTemplatePages, totalPages, 1)),
      uiModule03MainTableSectionRate: toFixedNumber(safeRate(uiModule03MainTableSectionPages, totalPages)),
      bareTableRate: toFixedNumber(safeRate(bareTablePages, totalPages)),
      bareCardRate: toFixedNumber(safeRate(bareCardPages, totalPages)),
      industrialReviewCompactRate: toFixedNumber(safeRate(industrialReviewCompactPages, totalPages)),
      workbenchToolbarRate: toFixedNumber(safeRate(workbenchToolbarPages, totalPages)),
      dataTestIdRate: toFixedNumber(safeRate(dataTestIdPages, totalPages)),
      columnSettingsRate: toFixedNumber(safeRate(columnSettingsPages, totalPages)),
    },
  };
};

const groupByPriority = (pages) => Object.fromEntries(
  ['P0', 'P1', 'P2'].map((priority) => [priority, buildGroupMetrics(pages.filter((page) => page.priority === priority))]),
);

const clampScore = (score, max) => Math.max(0, Math.min(max, score));

const buildScore = (rates) => {
  const templateAdoption = clampScore(
    Math.min(rates.templateAdoptionRate / REQUIRED_TEMPLATE_ADOPTION_RATE, 1) * SCORE_WEIGHTS.templateAdoption,
    SCORE_WEIGHTS.templateAdoption,
  );
  const operationConsistency = clampScore(
    (rates.workbenchToolbarApplicableRate * 0.35 + rates.directWorkbenchApplicableRate * 0.4 + rates.dataTestIdApplicableRate * 0.25) *
      SCORE_WEIGHTS.operationConsistency,
    SCORE_WEIGHTS.operationConsistency,
  );
  const tableConsistency = clampScore(
    (rates.industrialCompactApplicableRate * 0.35 +
      rates.customSelectionApplicableRate * 0.2 +
      rates.allInvertSelectionApplicableRate * 0.2 +
      (1 - rates.bareTableApplicableRate) * 0.25) *
      SCORE_WEIGHTS.tableConsistency,
    SCORE_WEIGHTS.tableConsistency,
  );
  const visualTextConsistency = clampScore(
    ((1 - rates.bareCardApplicableRate) * 0.35 +
      rates.dataTestIdApplicableRate * 0.25 +
      rates.noBaselineExceptionRate * 0.25 +
      rates.templateAdoptionRate * 0.15) *
      SCORE_WEIGHTS.visualTextConsistency,
    SCORE_WEIGHTS.visualTextConsistency,
  );
  const a11yKeyboardConsistency = clampScore(
    (rates.dataTestIdApplicableRate * 0.4 + rates.directWorkbenchApplicableRate * 0.3 + rates.customSelectionApplicableRate * 0.3) *
      SCORE_WEIGHTS.a11yKeyboardConsistency,
    SCORE_WEIGHTS.a11yKeyboardConsistency,
  );
  const engineeringGovernance = clampScore(
    (rates.templateAdoptionRate * 0.5 + rates.noBaselineExceptionRate * 0.2 + 0.3) * SCORE_WEIGHTS.engineeringGovernance,
    SCORE_WEIGHTS.engineeringGovernance,
  );
  const score =
    templateAdoption +
    operationConsistency +
    tableConsistency +
    visualTextConsistency +
    a11yKeyboardConsistency +
    engineeringGovernance;

  return {
    score: toFixedNumber(score, 1),
    target: UI_UNIFORMITY_TARGET_SCORE,
    gapToTarget: toFixedNumber(Math.max(0, UI_UNIFORMITY_TARGET_SCORE - score), 1),
    status: score >= UI_UNIFORMITY_TARGET_SCORE ? '达标' : '未达标',
    note:
      '深化评分：模板接入 20 分、操作流程 25 分、表格体验 20 分、视觉文案 15 分、可访问性与键盘 10 分、工程治理 10 分；当前分数用于基线和趋势，不直接替代人工验收。',
    components: {
      templateAdoption: toFixedNumber(templateAdoption, 1),
      operationConsistency: toFixedNumber(operationConsistency, 1),
      tableConsistency: toFixedNumber(tableConsistency, 1),
      visualTextConsistency: toFixedNumber(visualTextConsistency, 1),
      a11yKeyboardConsistency: toFixedNumber(a11yKeyboardConsistency, 1),
      engineeringGovernance: toFixedNumber(engineeringGovernance, 1),
    },
    weights: SCORE_WEIGHTS,
  };
};

const buildPageRisks = (page) => {
  const risks = [];
  if (page.isBaselineException) {
    risks.push('baseline例外');
  }
  if (page.isApplicable && !page.workbenchPageTemplate && page.priority !== 'P2') {
    risks.push('未直接采用WorkbenchPageTemplate');
  }
  if (page.uiModule03MainTableSection) {
    risks.push('仍使用UiModule03主表');
  }
  if (page.bareTable) {
    risks.push('存在裸Table');
  }
  if (page.bareCard) {
    risks.push('存在裸Card');
  }
  if (page.hasTableSurface && !page.industrialReviewCompact && page.priority !== 'P2') {
    risks.push('缺少industrial-review-compact');
  }
  if (page.hasTableSurface && !page.customSelectionCell && page.priority !== 'P2') {
    risks.push('缺少统一选择单元格');
  }
  if (page.hasTableSurface && !page.allInvertSelectionHeader && page.priority !== 'P2') {
    risks.push('缺少全选/反选表头契约');
  }
  if (page.hasTableSurface && !page.workbenchToolbar && page.priority !== 'P2') {
    risks.push('缺少统一工具栏标识');
  }
  if (!page.dataTestId && page.priority !== 'P2') {
    risks.push('缺少稳定data-testid');
  }
  return risks;
};

const getRiskWeight = (page) => {
  const priorityWeight = page.priority === 'P0' ? 30 : page.priority === 'P1' ? 20 : 10;
  const risks = buildPageRisks(page);
  const riskWeight = risks.reduce((sum, risk) => {
    if (risk === 'baseline例外') return sum + 10;
    if (risk === '未直接采用WorkbenchPageTemplate') return sum + 9;
    if (risk === '仍使用UiModule03主表') return sum + 7;
    if (risk === '存在裸Table') return sum + 6;
    if (risk === '存在裸Card') return sum + 4;
    if (risk === '缺少industrial-review-compact') return sum + 5;
    if (risk === '缺少统一工具栏标识') return sum + 4;
    if (risk === '缺少稳定data-testid') return sum + 3;
    return sum + 2;
  }, 0);
  return priorityWeight + riskWeight;
};

const toNextPageItem = (page) => ({
  file: page.file,
  priority: page.priority,
  pageType: page.pageType,
  riskScore: getRiskWeight(page),
  risks: buildPageRisks(page),
});

const buildNextBatch = (pages, priority, limit = NEXT_BATCH_LIMIT) => pages
  .filter((page) => page.priority === priority)
  .map(toNextPageItem)
  .filter((page) => page.risks.length > 0)
  .sort((left, right) => right.riskScore - left.riskScore || left.file.localeCompare(right.file))
  .slice(0, limit);

const buildShortfalls = (report) => [
  {
    id: 'direct-workbench-template',
    title: '完整工作台直接采用不足',
    current: `${report.summary.workbenchPageTemplatePages}/${report.summary.totalPages}`,
    rate: toFixedNumber(report.rates.directWorkbenchRate),
    target: 'P0/P1列表页优先提升到95%以上',
    impact: '用户在不同模块之间仍会遇到不同工具栏、筛选区和面板入口。',
    nextAction: '优先迁移P0中仍使用UiModule03MainTableSection或裸Table/Card的页面。',
  },
  {
    id: 'industrial-grid-contract',
    title: '工业紧凑表格契约覆盖不足',
    current: `${report.summary.industrialReviewCompactPages}/${report.summary.totalPages}`,
    rate: toFixedNumber(report.rates.industrialCompactRate),
    target: 'P0主表页优先统一到industrial-review-compact',
    impact: '行高、选择列、表头、横向滚动和列设置体验容易漂移。',
    nextAction: '把目录/主表类页面逐批接入gridPreset、selectionCellMode和selectionHeaderMode。',
  },
  {
    id: 'bare-table-card',
    title: '页面级裸Table/Card仍偏多',
    current: `Table ${report.summary.bareTablePages}，Card ${report.summary.bareCardPages}`,
    rate: toFixedNumber(Math.max(report.rates.bareTableRate, report.rates.bareCardRate)),
    target: '主列表和主卡片结构收敛到统一模板插槽',
    impact: '搜索、按钮、密度、空态、加载态和错误态容易各自实现。',
    nextAction: '对下一批P0页面做模板套壳，并保留特殊详情/设置页豁免。',
  },
  {
    id: 'baseline-exceptions',
    title: 'baseline例外仍需治理',
    current: `${report.summary.baselineExceptions}`,
    rate: toFixedNumber(report.rates.baselineExceptionRate),
    target: '禁止新增，逐步清零或登记长期豁免原因',
    impact: '迁移期例外会削弱95%统一性目标的可持续性。',
    nextAction: '为每个例外页面补原因、归属模块和清理批次。',
  },
];

const buildSamples = (pages) => ({
  workbenchPageTemplate: pages.filter((page) => page.workbenchPageTemplate).map((page) => page.file).slice(0, 20),
  uiModule03MainTableSection: pages.filter((page) => page.uiModule03MainTableSection).map((page) => page.file).slice(0, 20),
  bareTable: pages.filter((page) => page.bareTable).map((page) => page.file).slice(0, 20),
  bareCard: pages.filter((page) => page.bareCard).map((page) => page.file).slice(0, 20),
  industrialReviewCompact: pages.filter((page) => page.industrialReviewCompact).map((page) => page.file).slice(0, 20),
  baselineExceptions: pages.filter((page) => page.isBaselineException).map((page) => page.file).slice(0, 20),
});

const printPageList = (title, pages) => {
  console.log(`[ui-uniformity] ${title}:`);
  if (pages.length === 0) {
    console.log('  - 暂无');
    return;
  }
  for (const page of pages) {
    console.log(`  - ${page.file}｜${page.pageType}｜${page.risks.join('、')}`);
  }
};

const printTextReport = (report) => {
  console.log('[ui-uniformity] ERP UI 统一性基线评分');
  console.log(
    `[ui-uniformity] 页面总数: ${report.summary.totalPages}, 适用页面数: ${report.summary.applicablePages}, baseline例外: ${report.summary.baselineExceptions}`,
  );
  console.log(
    `[ui-uniformity] 治理口径模板采用: ${report.summary.governanceTemplatePages}/${report.summary.applicablePages} (${formatRate(report.rates.templateAdoptionRate)}，目标 ${formatRate(REQUIRED_TEMPLATE_ADOPTION_RATE)})`,
  );
  console.log(
    `[ui-uniformity] WorkbenchPageTemplate直接采用: ${report.summary.workbenchPageTemplatePages} (${formatRate(report.rates.directWorkbenchRate)}，适用页面 ${formatRate(report.rates.directWorkbenchApplicableRate)})`,
  );
  console.log(
    `[ui-uniformity] UiModule03MainTableSection页面: ${report.summary.uiModule03MainTableSectionPages} (${formatRate(report.rates.oldMainTableRate)})`,
  );
  console.log(
    `[ui-uniformity] 裸Table/Card页面: Table ${report.summary.bareTablePages} (${formatRate(report.rates.bareTableRate)}), Card ${report.summary.bareCardPages} (${formatRate(report.rates.bareCardRate)})`,
  );
  console.log(
    `[ui-uniformity] 工业紧凑表格: ${report.summary.industrialReviewCompactPages} (${formatRate(report.rates.industrialCompactRate)}), 统一选择: ${report.summary.customSelectionCellPages}, 全选/反选表头: ${report.summary.allInvertSelectionHeaderPages}`,
  );
  console.log(
    `[ui-uniformity] 工具栏/data-testid/列设置: WorkbenchToolbar ${report.summary.workbenchToolbarPages}, data-testid ${report.summary.dataTestIdPages}, 列设置 ${report.summary.columnSettingsPages}`,
  );
  console.log(
    `[ui-uniformity] 总评分: ${formatScore(report.score.score)} / 100，目标 ${report.score.target}，差距 ${formatScore(report.score.gapToTarget)}，状态 ${report.score.status}`,
  );
  console.log(
    `[ui-uniformity] 评分拆分: 模板 ${formatScore(report.score.components.templateAdoption)}, 操作 ${formatScore(report.score.components.operationConsistency)}, 表格 ${formatScore(report.score.components.tableConsistency)}, 视觉文案 ${formatScore(report.score.components.visualTextConsistency)}, a11y键盘 ${formatScore(report.score.components.a11yKeyboardConsistency)}, 治理 ${formatScore(report.score.components.engineeringGovernance)}`,
  );
  console.log(`[ui-uniformity] 评分说明: ${report.score.note}`);
  console.log('[ui-uniformity] P0/P1/P2摘要:');
  for (const priority of ['P0', 'P1', 'P2']) {
    const item = report.prioritySummary[priority];
    console.log(
      `  - ${priority}: 总页 ${item.totalPages}, 适用 ${item.applicablePages}, 模板治理 ${item.governanceTemplatePages}, Workbench ${item.workbenchPageTemplatePages}, 旧主表 ${item.uiModule03MainTableSectionPages}, 裸Table ${item.bareTablePages}, 裸Card ${item.bareCardPages}, 工业紧凑 ${item.industrialReviewCompactPages}, 例外 ${item.baselineExceptions}`,
    );
  }
  console.log('[ui-uniformity] 主要短板:');
  for (const shortfall of report.shortfalls) {
    console.log(`  - ${shortfall.title}: 当前 ${shortfall.current}；行动 ${shortfall.nextAction}`);
  }
  printPageList('下一批P0页面清单', report.nextBatch.P0);
  printPageList('下一批P1页面清单', report.nextBatch.P1.slice(0, 10));
  console.log('[ui-uniformity] 输出JSON可使用: node apps/erp-web/scripts/assess-ui-uniformity.mjs --json');
};

const pageFiles = await collectPageFiles(pagesRoot);
const baseline = await readBaseline();
const baselineSet = new Set(baseline);
const pages = [];

for (const file of pageFiles) {
  try {
    pages.push(await buildPageAnalysis(file, baselineSet));
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') {
      continue;
    }
    throw error;
  }
}

const eligibleApplicablePages = pages.filter((page) => page.isApplicable && !page.isBaselineException);
const applicablePageCount = eligibleApplicablePages.length;
const totalPages = pages.length;
const governanceTemplatePages = countWhere(eligibleApplicablePages, (page) => page.hasGovernanceTemplateReference);
const directRecommendedTemplatePages = countWhere(eligibleApplicablePages, (page) => page.hasRecommendedTemplate);
const workbenchPageTemplatePages = countWhere(pages, (page) => page.workbenchPageTemplate);
const uiModule03MainTableSectionPages = countWhere(pages, (page) => page.uiModule03MainTableSection);
const bareTablePages = countWhere(pages, (page) => page.bareTable);
const bareCardPages = countWhere(pages, (page) => page.bareCard);
const industrialReviewCompactPages = countWhere(pages, (page) => page.industrialReviewCompact);
const customSelectionCellPages = countWhere(pages, (page) => page.customSelectionCell);
const allInvertSelectionHeaderPages = countWhere(pages, (page) => page.allInvertSelectionHeader);
const workbenchToolbarPages = countWhere(pages, (page) => page.workbenchToolbar);
const dataTestIdPages = countWhere(pages, (page) => page.dataTestId);
const columnSettingsPages = countWhere(pages, (page) => page.columnSettings);
const leftPanelPages = countWhere(pages, (page) => page.leftPanel);
const rightPanelPages = countWhere(pages, (page) => page.rightPanel);
const bottomPanelPages = countWhere(pages, (page) => page.bottomPanel);

const rates = {
  templateAdoptionRate: safeRate(governanceTemplatePages, applicablePageCount, 1),
  directRecommendedTemplateRate: safeRate(directRecommendedTemplatePages, applicablePageCount, 1),
  directWorkbenchRate: safeRate(workbenchPageTemplatePages, totalPages, 1),
  directWorkbenchApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.workbenchPageTemplate), applicablePageCount, 1),
  oldMainTableRate: safeRate(uiModule03MainTableSectionPages, totalPages),
  oldMainTableApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.uiModule03MainTableSection), applicablePageCount),
  bareTableRate: safeRate(bareTablePages, totalPages),
  bareTableApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.bareTable), applicablePageCount),
  bareCardRate: safeRate(bareCardPages, totalPages),
  bareCardApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.bareCard), applicablePageCount),
  industrialCompactRate: safeRate(industrialReviewCompactPages, totalPages),
  industrialCompactApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.industrialReviewCompact), applicablePageCount),
  customSelectionApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.customSelectionCell), applicablePageCount),
  allInvertSelectionApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.allInvertSelectionHeader), applicablePageCount),
  workbenchToolbarApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.workbenchToolbar), applicablePageCount),
  dataTestIdApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.dataTestId), applicablePageCount),
  columnSettingsApplicableRate: safeRate(countWhere(eligibleApplicablePages, (page) => page.columnSettings), applicablePageCount),
  baselineExceptionRate: safeRate(baseline.length, totalPages),
  noBaselineExceptionRate: 1 - safeRate(baseline.length, totalPages),
};

const report = {
  generatedAt: new Date().toISOString(),
  projectRoot: toPosixPath(projectRoot),
  target: {
    templateAdoptionRate: REQUIRED_TEMPLATE_ADOPTION_RATE,
    uiUniformityScore: UI_UNIFORMITY_TARGET_SCORE,
  },
  summary: {
    totalPages,
    applicablePages: applicablePageCount,
    baselineExceptions: baseline.length,
    governanceTemplatePages,
    directRecommendedTemplatePages,
    workbenchPageTemplatePages,
    uiModule03MainTableSectionPages,
    bareTablePages,
    bareCardPages,
    industrialReviewCompactPages,
    customSelectionCellPages,
    allInvertSelectionHeaderPages,
    workbenchToolbarPages,
    dataTestIdPages,
    columnSettingsPages,
    leftPanelPages,
    rightPanelPages,
    bottomPanelPages,
  },
  rates: Object.fromEntries(Object.entries(rates).map(([key, value]) => [key, toFixedNumber(value)])),
  prioritySummary: groupByPriority(pages),
  score: null,
  shortfalls: [],
  nextBatch: {
    P0: buildNextBatch(pages, 'P0'),
    P1: buildNextBatch(pages, 'P1'),
    P2: buildNextBatch(pages, 'P2'),
    baselineExceptions: pages
      .filter((page) => page.isBaselineException)
      .map(toNextPageItem)
      .sort((left, right) => right.riskScore - left.riskScore || left.file.localeCompare(right.file))
      .slice(0, NEXT_BATCH_LIMIT),
  },
  samples: buildSamples(pages),
};

report.score = buildScore(rates);
report.shortfalls = buildShortfalls(report);

if (shouldOutputJson) {
  console.log(JSON.stringify(report, null, 2));
} else {
  printTextReport(report);
}
