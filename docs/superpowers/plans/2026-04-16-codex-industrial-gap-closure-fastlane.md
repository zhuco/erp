# 工业级评估后的 Codex 多子任务开发补全文档（2026-04-16）

> 适用场景：在当前仓库基础上快速补齐“可交付性短板”，并用 `gpt-5.3-codex` 并行加速实施。
> 
> 子任务数量上限：**5**（本方案严格控制为 5）。

## 1. 实施前基线（证据）

- `pnpm run repo:verify`：通过（OpenAPI gate 通过）
- `pnpm run accept:web`：通过（但存在大量 Vite chunk 循环依赖告警）
- `pnpm run test:backend:release-gate`：失败（本地数据库 `localhost:5432` 未启动，非代码逻辑回归）
- `pnpm --filter erp-web test:unit -- src/build/manualChunks.test.ts`：失败（10 项中 8 项失败）

关键客观规模指标（扫描得到）：

- `web_pages_tsx=240`
- `web_api_modules_ts=171`
- `erp_api_tests_rs=46`
- `openapi_v1_paths=483`
- `router_placeholder_import_refs=1`（`sales/shop-inventory-sync` 仍依赖占位页导出）

## 2. 子任务拆分（<=5）

### 子任务 1：前端分包策略修复（高优先级）

**目标**

修复 `manualChunks` 与测试预期不一致问题，建立可持续分包规则，降低 chunk 循环依赖风险。

**文件边界**

- `apps/erp-web/src/build/manualChunks.ts`
- `apps/erp-web/src/build/manualChunks.test.ts`（仅必要时）

**完成标准**

- `pnpm --filter erp-web test:unit -- src/build/manualChunks.test.ts` 通过
- `pnpm --filter erp-web build` 通过

---

### 子任务 2：销售库存同步路由去占位（高优先级）

**目标**

将 `sales/shop-inventory-sync` 从占位页切换到可执行页面（复用现有库存同步任务页），缩小“可见但不可用”页面比例。

**文件边界**

- `apps/erp-web/src/app/router.tsx`
- `apps/erp-web/src/pages/sales/SalesEcommercePlaceholderPages.tsx`（仅最小清理）

**完成标准**

- 路由不再依赖占位导出
- `pnpm --filter erp-web build` 通过

---

### 子任务 3：工业级进度评估脚本化（高优先级）

**目标**

新增统一评估脚本，生成 JSON 报告，避免后续评估口径漂移。

**文件边界**

- `scripts/assess_industrial_progress.mjs`（新增）
- `package.json`（新增脚本入口）
- `scripts/ops/industrial_progress_assessment.test.mjs`（可选）

**完成标准**

- 支持 `--out <path>`，默认输出 `docs/reports/_generated/industrial-progress-latest.json`
- 至少输出：规模指标、门禁状态、placeholder 路由引用计数

---

### 子任务 4：生成 Codex 可执行补全计划与任务卡（中优先级）

**目标**

把当前补齐方案落地为可复制执行的任务卡文档，便于后续继续并行推进。

**文件边界**

- `docs/superpowers/plans/2026-04-16-codex-industrial-gap-closure-fastlane.md`

**完成标准**

- 文档含 5 个且仅 5 个子任务
- 每个子任务有：目标、文件边界、验收命令、回滚要点

---

### 子任务 5：实施后复评与差异报告（中优先级）

**目标**

基于新证据给出“实施前 vs 实施后”的进度变化，形成管理层可读结论。

**文件边界**

- `docs/reports/2026-04-16-industrial-progress-reassessment.md`

**完成标准**

- 包含前后对比表（至少含 8 个关键指标）
- 明确仍未闭环项与下一轮建议

## 3. Codex 并行执行建议（gpt-5.3-codex）

- Worker-A（`gpt-5.3-codex`）：子任务 1
- Worker-B（`gpt-5.3-codex`）：子任务 2
- Worker-C（`gpt-5.3-codex`）：子任务 3
- 主线程：子任务 4 + 子任务 5，并负责集成验证

执行约束：

- 每个 worker 仅修改其“文件边界”内文件
- 不回退他人改动
- 先跑本子任务验证，再汇总全局验证

## 4. 统一验证清单（集成后）

1. `pnpm --filter erp-web test:unit -- src/build/manualChunks.test.ts`
2. `pnpm run accept:web`
3. `pnpm run repo:verify`
4. `pnpm run assess:industrial-progress`（新增）

## 5. 风险与止损

- 若后端 gate 仍失败且报数据库不可达，归类为“环境阻塞”，不误判为代码缺陷。
- 若 `accept:web` 通过但仍存在高风险构建告警，必须在复评报告中单列“未阻断风险”。
- 若并行改动触发文件边界冲突，优先保留先通过验证的一侧，再最小化合并。
