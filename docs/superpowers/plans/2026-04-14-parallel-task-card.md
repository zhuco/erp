# Parallel Task Ops Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a lightweight Node.js workflow for generating Markdown task cards with explicit guards and verifying that every change stays inside the approved files for the 多 AI 并行开发手册 effort.

**Architecture:** A standalone `task_card` helper builds/parses the YAML-style front matter and is shared by both the generator and the validator. The scaffolding CLI consumes that helper to materialize a card template, while the validator CLI reads the card, derives allow/deny sets, and inspects either passed-in paths or the active git diff/staged tree.

**Tech Stack:** Node.js ESM, built-in `node:test`, native `child_process`/`fs` utilities.

---

### Task 1: Task card metadata helpers
**Files:**
- Create: `scripts/ops/task_card.mjs`
- Modify: `scripts/ops/task_ops.test.mjs`

- [ ] **Step 1: Write the failing test**

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';

test('task card front matter round trips', () => {
  const card = createTaskCard({
    name: '并行任务示例',
    goal: '校验卡片',
    owner: 'test-owner',
    branch: 'codex/demo',
    worktree: 'worktree/parallel',
    allowedFiles: ['scripts/**', 'apps/backend/**'],
    disallowedFiles: ['apps/frontend/secret.md'],
    testCommands: ['node --test scripts/ops/task_ops.test.mjs'],
    delivery: 'Markdown + assets',
  });
  const parsed = parseTaskCard(card);
  assert.equal(parsed.name, '并行任务示例');
  assert(parsed.allowedFiles.includes('scripts/**'));
});
```

- [ ] **Step 2: Run test to verify it fails**
  Run: `node --test scripts/ops/task_ops.test.mjs --filter "task card front matter"`
  Expected: `ReferenceError: createTaskCard is not defined` because the helper module is missing.

- [ ] **Step 3: Write the minimal implementation**

```js
export function createTaskCard(metadata) {
  const front = buildFrontMatter(metadata);
  return `${front}\n## 业务背景\n...`;
}

export function parseTaskCard(text) {
  const raw = extractFrontMatter(text);
  return parseKeyValueBlock(raw);
}
```

- [ ] **Step 4: Run test to verify it passes**
  Run: `node --test scripts/ops/task_ops.test.mjs --filter "task card front matter"`
  Expected: PASS.

- [ ] **Step 5: Commit the changes**
  Run: `git add scripts/ops/task_card.mjs scripts/ops/task_ops.test.mjs && git commit -m "feat: add task card metadata helper"`

### Task 2: Task card scaffolding CLI
**Files:**
- Create: `scripts/ops/new_parallel_task.mjs`
- Modify: `scripts/ops/task_ops.test.mjs`

- [ ] **Step 1: Write the failing test**

```js
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';

const tmpDir = fs.mkdtempSync(new URL('file:///tmp/parallel-card-'));

test('new_parallel_task CLI produces file with required fields', () => {
  const dest = `${tmpDir}/card.md`;
  const result = spawnSync(process.execPath, ['scripts/ops/new_parallel_task.mjs', '--name', '示例', '--output', dest]);
  assert.equal(result.status, 0);
  const contents = fs.readFileSync(dest, 'utf8');
  assert(contents.includes('name: 示例'));
});
```

- [ ] **Step 2: Run test to see it fail**
  Run: `node --test scripts/ops/task_ops.test.mjs --filter "new_parallel_task CLI"`
  Expected: fails because CLI file missing.

- [ ] **Step 3: Implement CLI**
  Parse `process.argv`, require `--name`, support `--goal`, `--owner`, `--branch`, `--worktree`, repeatable `--allowed`, `--disallowed`, `--output`, reuse `createTaskCard`, and write the Markdown file.

- [ ] **Step 4: Run test for PASS**
  Run: `node --test scripts/ops/task_ops.test.mjs --filter "new_parallel_task CLI"`
  Expected: PASS and file contains YAML front matter.

- [ ] **Step 5: Commit the generator**
  Run: `git add scripts/ops/new_parallel_task.mjs scripts/ops/task_ops.test.mjs && git commit -m "feat: add task card scaffolding CLI"`

### Task 3: Task boundary validator CLI
**Files:**
- Create: `scripts/ops/validate_task_boundary.mjs`
- Modify: `scripts/ops/task_ops.test.mjs`

- [ ] **Step 1: Write the failing test**

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';

const card = createTaskCard({
  name: '边界核验',
  allowedFiles: ['scripts/**'],
  disallowedFiles: ['scripts/ops/secret.md'],
});

test('validate_task_boundary detects disallowed files', () => {
  const { pass, badFiles } = validateFilesAgainstCard(card, ['scripts/ops/validate_task_boundary.mjs', 'README.md']);
  assert(!pass);
  assert(badFiles.includes('README.md'));
});
```

- [ ] **Step 2: Run targeted test**
  Run: `node --test scripts/ops/task_ops.test.mjs --filter "validate_task_boundary"`
  Expected: fails because validator not implemented.

- [ ] **Step 3: Implement validator CLI**
  Export `validateFilesAgainstCard` for tests, implement `matchesPattern`, detect git diff (`git diff --name-only` or `git diff --cached --name-only` depending on `--mode`), and print `PASS`/`FAIL` along with offending paths.

- [ ] **Step 4: Run test to verify passing behavior**
  Run: `node --test scripts/ops/task_ops.test.mjs --filter "validate_task_boundary"`
  Expected: PASS and CLI exit code 0/1 as appropriate.

- [ ] **Step 5: Commit validator work**
  Run: `git add scripts/ops/validate_task_boundary.mjs scripts/ops/task_ops.test.mjs && git commit -m "feat: add task boundary validator"`

---

After the plan is saved, dispatch a plan-document-reviewer subagent (per the writing-plans instructions) with a short request to double-check the task breakdown and assumptions before execution. Once the reviewer approves, proceed with implementation using the chosen execution skill.
