# 项目规范化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一仓库根级工程基线、命令入口、CI 门禁与规范文档，使本地、脚本与 GitHub Actions 使用同一套规则。

**Architecture:** 以仓库根目录为唯一工程规范入口，先建立自动校验脚本，再补齐根级配置与统一命令，再收敛 CI 和文档引用。所有脚本、README 与 workflow 最终只引用一套根级命令，不再各自维护门禁细节。

**Tech Stack:** pnpm workspace、Rust workspace、GitHub Actions、Node.js 脚本、Makefile、Markdown 文档

---

## 前置说明

- 当前 `F:\erp` 不是 Git 工作树；本计划中的 `git commit` 步骤默认在真实 Git 工作树或后续 worktree 中执行。
- 本计划只覆盖“工程规范化”主线，不包含 `ErpDataGrid` 重构任务。

## 文件结构

### 根级基线文件

- Create: `.editorconfig`
- Create: `.gitignore`
- Create: `rustfmt.toml`
- Create: `clippy.toml`
- Create: `scripts/verify_repo_baseline.mjs`

责任说明：

- `.editorconfig`：统一缩进、换行与编码基线
- `.gitignore`：统一忽略生成产物、临时文件和本地调试目录
- `rustfmt.toml` / `clippy.toml`：固化 Rust 侧格式化与静态检查策略
- `scripts/verify_repo_baseline.mjs`：自动校验仓库基线、脚本入口和 workflow 收敛状态

### 命令入口与脚本

- Modify: `package.json`
- Modify: `Makefile`
- Modify: `scripts/accept_backend.sh`
- Modify: `scripts/accept_web.sh`
- Create: `scripts/accept_all.sh`

责任说明：

- `package.json`：作为跨平台根级命令入口
- `Makefile`：对齐根级命令，不重复定义门禁逻辑
- `scripts/accept_*.sh`：退化为薄包装，只负责调用根级命令

### CI 与文档

- Modify: `.github/workflows/ci.yml`
- Delete: `.github/workflows/web-acceptance.yml`
- Modify: `README.md`
- Modify: `docs/ops/ACCEPTANCE.md`
- Modify: `docs/ops/progress_check.md`
- Modify: `docs/list.md`
- Create: `docs/ops/ENGINEERING_STANDARDS.md`

责任说明：

- `ci.yml`：成为唯一前端门禁 workflow
- `ENGINEERING_STANDARDS.md`：成为单一工程规范入口文档

### 验证文件

- Test: `scripts/verify_repo_baseline.mjs`

---

### Task 1：建立仓库基线校验与根级配置

**Files:**
- Create: `F:/erp/scripts/verify_repo_baseline.mjs`
- Create: `F:/erp/.editorconfig`
- Create: `F:/erp/.gitignore`
- Create: `F:/erp/rustfmt.toml`
- Create: `F:/erp/clippy.toml`
- Modify: `F:/erp/package.json`
- Test: `F:/erp/scripts/verify_repo_baseline.mjs`

- [ ] **Step 1: 先写失败的基线校验脚本**

```js
import fs from 'node:fs';

const requiredFiles = [
  '.editorconfig',
  '.gitignore',
  'rustfmt.toml',
  'clippy.toml',
];

const missing = requiredFiles.filter((file) => !fs.existsSync(file));
if (missing.length > 0) {
  console.error('缺少根级基线文件:', missing.join(', '));
  process.exit(1);
}
```

- [ ] **Step 2: 运行脚本确认当前仓库失败**

Run: `node scripts/verify_repo_baseline.mjs`

Expected: 退出码非 0，并输出缺少 `.editorconfig`、`.gitignore`、`rustfmt.toml`、`clippy.toml`

- [ ] **Step 3: 补齐根级配置文件与根级校验命令**

```json
{
  "scripts": {
    "repo:verify": "node scripts/verify_repo_baseline.mjs"
  }
}
```

`.editorconfig` 最小基线：

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 2
trim_trailing_whitespace = true

[*.rs]
indent_size = 4

[Makefile]
indent_style = tab
```

- [ ] **Step 4: 再次运行基线校验，确认只剩后续未完成项**

Run: `pnpm run repo:verify`

Expected: 仍可能失败，但不再报缺少根级文件；失败原因应转移到“命令入口”或“workflow 漂移”

- [ ] **Step 5: 提交**

```bash
git add .editorconfig .gitignore rustfmt.toml clippy.toml package.json scripts/verify_repo_baseline.mjs
git commit -m "chore: add repository baseline configs"
```

### Task 2：统一根级命令入口与门禁脚本

**Files:**
- Modify: `F:/erp/package.json`
- Modify: `F:/erp/Makefile`
- Modify: `F:/erp/scripts/accept_backend.sh`
- Modify: `F:/erp/scripts/accept_web.sh`
- Create: `F:/erp/scripts/accept_all.sh`
- Test: `F:/erp/scripts/verify_repo_baseline.mjs`

- [ ] **Step 1: 扩展失败校验，让它要求存在统一根级命令**

```js
const requiredScripts = [
  'lint',
  'typecheck',
  'test',
  'build',
  'accept:web',
  'accept:backend',
  'accept:all',
];
```

- [ ] **Step 2: 运行校验确认当前缺少统一命令**

Run: `pnpm run repo:verify`

Expected: FAIL，并指出根级 `package.json` 缺少统一 scripts

- [ ] **Step 3: 在 `package.json`、`Makefile` 与 shell 脚本中收敛到一套入口**

`package.json` 目标结构：

```json
{
  "scripts": {
    "lint": "pnpm --filter erp-web lint",
    "typecheck": "pnpm --filter erp-web typecheck",
    "build": "pnpm --filter erp-web build",
    "test": "cargo test --all && pnpm --filter erp-web test:unit",
    "accept:web": "pnpm lint && pnpm typecheck && pnpm build",
    "accept:backend": "cargo fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all",
    "accept:all": "pnpm accept:backend && pnpm accept:web"
  }
}
```

`scripts/accept_web.sh` 目标结构：

```bash
#!/usr/bin/env bash
set -euo pipefail
pnpm accept:web
```

- [ ] **Step 4: 运行一致性校验**

Run: `pnpm run repo:verify`

Expected: “统一命令入口”相关检查通过

- [ ] **Step 5: 提交**

```bash
git add package.json Makefile scripts/accept_backend.sh scripts/accept_web.sh scripts/accept_all.sh scripts/verify_repo_baseline.mjs
git commit -m "chore: unify repository command entrypoints"
```

### Task 3：收敛 GitHub Actions 工作流

**Files:**
- Modify: `F:/erp/.github/workflows/ci.yml`
- Delete: `F:/erp/.github/workflows/web-acceptance.yml`
- Modify: `F:/erp/scripts/verify_repo_baseline.mjs`
- Test: `F:/erp/scripts/verify_repo_baseline.mjs`

- [ ] **Step 1: 给基线校验增加 workflow 收敛规则**

```js
const workflowFiles = fs.readdirSync('.github/workflows');
const webWorkflows = workflowFiles.filter((name) => name.includes('web') || name === 'ci.yml');
if (webWorkflows.includes('ci.yml') && webWorkflows.includes('web-acceptance.yml')) {
  console.error('前端门禁 workflow 重复存在: ci.yml 与 web-acceptance.yml');
  process.exit(1);
}
```

- [ ] **Step 2: 运行校验，确认当前 workflow 漂移**

Run: `pnpm run repo:verify`

Expected: FAIL，并提示存在重复或漂移的前端 workflow

- [ ] **Step 3: 收敛为单一 workflow**

`ci.yml` 目标行为：

```yaml
- name: Install dependencies
  run: pnpm install

- name: Verify repo baseline
  run: pnpm run repo:verify

- name: Accept web
  run: pnpm run accept:web
```

要求：

- 只保留一个前端门禁 workflow
- workflow 中调用根级命令，而不是重复内联一套规则

- [ ] **Step 4: 重新运行基线校验**

Run: `pnpm run repo:verify`

Expected: workflow 收敛检查通过

- [ ] **Step 5: 提交**

```bash
git add .github/workflows/ci.yml .github/workflows/web-acceptance.yml scripts/verify_repo_baseline.mjs
git commit -m "ci: consolidate web acceptance workflow"
```

### Task 4：收敛文档入口

**Files:**
- Create: `F:/erp/docs/ops/ENGINEERING_STANDARDS.md`
- Modify: `F:/erp/README.md`
- Modify: `F:/erp/docs/ops/ACCEPTANCE.md`
- Modify: `F:/erp/docs/ops/progress_check.md`
- Modify: `F:/erp/docs/list.md`
- Test: `F:/erp/README.md`

- [ ] **Step 1: 先写文档结构骨架**

```md
# 工程规范

## 唯一命令入口
- `pnpm run repo:verify`
- `pnpm run accept:web`
- `pnpm run accept:backend`
- `pnpm run accept:all`
```

- [ ] **Step 2: 用 `rg` 找出旧文档中的漂移入口**

Run: `rg -n "accept_web|accept_backend|pnpm --filter erp-web lint|cargo clippy --all-targets" README.md docs`

Expected: 输出多处旧入口引用，作为待替换清单

- [ ] **Step 3: 更新 README 与 ops 文档，只保留统一入口**

要求：

- README 的“开发门禁”部分优先展示根级命令
- `docs/ops/ACCEPTANCE.md` 和 `docs/ops/progress_check.md` 引用同一套命令
- `docs/list.md` 补充 `ENGINEERING_STANDARDS.md` 为单一规范入口

- [ ] **Step 4: 再次用 `rg` 校验文档引用已收敛**

Run: `rg -n "accept_web|accept_backend" README.md docs`

Expected: 旧入口仅在“兼容脚本说明”里出现，不再作为主入口

- [ ] **Step 5: 提交**

```bash
git add README.md docs/ops/ENGINEERING_STANDARDS.md docs/ops/ACCEPTANCE.md docs/ops/progress_check.md docs/list.md
git commit -m "docs: consolidate engineering standards entrypoint"
```

### Task 5：做最终验证并记录剩余问题

**Files:**
- Modify: `F:/erp/docs/ops/ENGINEERING_STANDARDS.md`
- Test: `F:/erp/scripts/verify_repo_baseline.mjs`

- [ ] **Step 1: 运行仓库基线校验**

Run: `pnpm run repo:verify`

Expected: PASS

- [ ] **Step 2: 运行前端门禁**

Run: `pnpm run accept:web`

Expected: PASS；若失败，失败项属于业务代码问题，不应再是入口漂移问题

- [ ] **Step 3: 运行后端门禁**

Run: `pnpm run accept:backend`

Expected: PASS；若失败，失败项属于代码问题，不应再是脚本或入口问题

- [ ] **Step 4: 在工程规范文档中记录剩余已知问题**

```md
## 已知非规范类问题

- 如果 `pnpm run accept:web` 失败，应按 ESLint / TypeScript / Build 输出修复业务代码
- 如果 `pnpm run accept:backend` 失败，应按 rustfmt / clippy / cargo test 输出修复后端代码
```

- [ ] **Step 5: 提交**

```bash
git add docs/ops/ENGINEERING_STANDARDS.md
git commit -m "chore: finalize repository standardization baseline"
```
