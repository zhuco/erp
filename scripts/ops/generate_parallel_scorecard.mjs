#!/usr/bin/env node
import fs from 'node:fs/promises';
import path from 'node:path';
import { calculateScorecard, renderMarkdown } from './lib/scorecard.mjs';

const args = process.argv.slice(2);

const usage = () => {
  console.log('用法：node scripts/ops/generate_parallel_scorecard.mjs --input <json> [--output <md>]');
  process.exit(1);
};

const getValue = (flag) => {
  const index = args.indexOf(flag);
  if (index === -1 || index === args.length - 1) {
    return null;
  }
  return args[index + 1];
};

const inputPath = getValue('--input');
if (!inputPath) {
  console.error('必须提供 --input 参数');
  usage();
}

const outputPath = getValue('--output');

const loadInput = async () => {
  if (inputPath === '-') {
    const chunks = [];
    for await (const chunk of process.stdin) {
      chunks.push(chunk);
    }
    return JSON.parse(Buffer.concat(chunks).toString('utf8'));
  }
  const resolved = path.isAbsolute(inputPath) ? inputPath : path.resolve(process.cwd(), inputPath);
  const content = await fs.readFile(resolved, 'utf8');
  return JSON.parse(content);
};

const writeOutput = async (markdown) => {
  if (outputPath) {
    const resolved = path.isAbsolute(outputPath) ? outputPath : path.resolve(process.cwd(), outputPath);
    await fs.mkdir(path.dirname(resolved), { recursive: true });
    await fs.writeFile(resolved, markdown, 'utf8');
    console.log(`已写入评分卡：${resolved}`);
  } else {
    console.log(markdown);
  }
};

const main = async () => {
  try {
    const payload = await loadInput();
    const scorecard = calculateScorecard(payload);
    const markdown = renderMarkdown(scorecard);
    await writeOutput(markdown);
  } catch (error) {
    console.error('生成评分卡失败：', error.message);
    process.exit(1);
  }
};

main();
