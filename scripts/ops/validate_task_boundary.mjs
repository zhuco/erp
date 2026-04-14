import fs from 'node:fs';
import path from 'node:path';
import { parseTaskCard, readGitChangedFiles, validateTaskFiles } from './task_card.mjs';

const LIST_FLAGS = {
  '--file': 'files',
};

const SINGLE_FLAGS = {
  '--task': 'task',
  '--mode': 'mode',
};

const USAGE = `Usage: node validate_task_boundary.mjs --task PATH [--file PATH ...] [--mode working|staged|all]
Options:
  --task PATH   任务卡 Markdown 文件
  --file PATH   显式校验的变更文件，可重复
  --mode MODE   未传 --file 时，从 git 自动读取变更文件，默认 working
`;

function printUsage(exitCode = 0) {
  process.stdout.write(`${USAGE}\n`);
  process.exit(exitCode);
}

function fail(message, details = []) {
  process.stdout.write(`FAIL: ${message}\n`);
  for (const detail of details) {
    process.stdout.write(`- ${detail}\n`);
  }
  process.exit(1);
}

const args = process.argv.slice(2);
if (!args.length || args.includes('--help')) {
  printUsage(args.includes('--help') ? 0 : 1);
}

const parsed = {
  files: [],
  mode: 'working',
};

for (let i = 0; i < args.length; i += 1) {
  const flag = args[i];
  if (LIST_FLAGS[flag]) {
    const next = args[i + 1];
    if (!next || next.startsWith('--')) {
      fail(`参数 ${flag} 缺少值`);
    }
    parsed[LIST_FLAGS[flag]].push(next);
    i += 1;
    continue;
  }
  if (SINGLE_FLAGS[flag]) {
    const next = args[i + 1];
    if (!next || next.startsWith('--')) {
      fail(`参数 ${flag} 缺少值`);
    }
    parsed[SINGLE_FLAGS[flag]] = next;
    i += 1;
    continue;
  }
  fail(`未知参数 ${flag}`);
}

if (!parsed.task) {
  fail('必须传入 --task');
}

const taskPath = path.resolve(parsed.task);
if (!fs.existsSync(taskPath)) {
  fail(`任务卡不存在: ${taskPath}`);
}

const cardText = fs.readFileSync(taskPath, 'utf8');
const taskCard = parseTaskCard(cardText);
const repoRoot = process.cwd();
const files = parsed.files.length
  ? parsed.files
  : readGitChangedFiles({ repoRoot, mode: parsed.mode });

if (!files.length) {
  process.stdout.write('PASS: 未发现需要校验的变更文件\n');
  process.exit(0);
}

const result = validateTaskFiles(taskCard, files);
if (!result.ok) {
  const details = [];
  if (result.blockedFiles.length) {
    details.push(`命中禁区: ${result.blockedFiles.join(', ')}`);
  }
  if (result.outOfScopeFiles.length) {
    details.push(`越出白名单: ${result.outOfScopeFiles.join(', ')}`);
  }
  fail('任务边界校验未通过', details);
}

process.stdout.write(`PASS: 共校验 ${result.checkedFiles.length} 个文件，全部在允许范围内\n`);
