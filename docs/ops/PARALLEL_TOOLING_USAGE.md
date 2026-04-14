# 多 AI 并行工具使用说明

本说明对应以下已落地工具：

1. `scripts/ops/new_parallel_task.mjs`
   生成可解析的任务卡 Markdown。
2. `scripts/ops/validate_task_boundary.mjs`
   校验当前任务改动是否越出白名单/命中禁区。
3. `scripts/ops/generate_parallel_scorecard.mjs`
   根据 JSON 输入生成每日评分卡 Markdown。

## 1. 生成任务卡

示例：

```powershell
node scripts/ops/new_parallel_task.mjs `
  --name "售后 SLA 收敛" `
  --goal "修复售后超时率统计并补齐页面展示" `
  --background "用于售后工业级收敛与客服协同" `
  --owner "integrator-01" `
  --branch "codex/aftersales-sla-20260414" `
  --worktree ".worktrees/aftersales-sla-20260414" `
  --allowed "crates/erp-api/tests/after_sales_sla_escalation.rs" `
  --allowed "crates/erp-repo/src/after_sales_sla_rules.rs" `
  --allowed "apps/erp-web/src/pages/afterSales/AfterSalesSlaPage.tsx" `
  --disallowed "crates/erp-repo/src/audit_logs.rs" `
  --dependency "共享文件锁已登记" `
  --dod "通过 after_sales_sla_escalation 测试" `
  --dod "无越界修改" `
  --test "cargo test -p erp-api --test after_sales_sla_escalation -- --nocapture" `
  --output "docs/ops/tasks/2026-04-14/aftersales-sla.md"
```

生成结果：

1. 产出一份含 front matter 的任务卡。
2. 可直接交给不同 AI 会话执行。
3. 后续可被边界校验脚本自动识别。

## 2. 校验任务边界

显式传文件列表：

```powershell
node scripts/ops/validate_task_boundary.mjs `
  --task "docs/ops/tasks/2026-04-14/aftersales-sla.md" `
  --file "crates/erp-api/tests/after_sales_sla_escalation.rs" `
  --file "apps/erp-web/src/pages/afterSales/AfterSalesSlaPage.tsx"
```

从 git 自动读取当前变更：

```powershell
node scripts/ops/validate_task_boundary.mjs `
  --task "docs/ops/tasks/2026-04-14/aftersales-sla.md" `
  --mode working
```

支持模式：

1. `working`
   当前未暂存改动 + 未跟踪文件。
2. `staged`
   当前已暂存改动。
3. `all`
   `working` + `staged` 合并去重。

通过标准：

1. 输出 `PASS`
2. 所有文件都在 `allowed_files` 内
3. 无文件命中 `disallowed_files`

## 3. 生成评分卡

示例输入文件见：
[PARALLEL_SCORECARD_SAMPLE.json](/F:/erp/docs/ops/examples/PARALLEL_SCORECARD_SAMPLE.json)

生成命令：

```powershell
node scripts/ops/generate_parallel_scorecard.mjs `
  --input "docs/ops/examples/PARALLEL_SCORECARD_SAMPLE.json" `
  --output "docs/ops/scorecards/2026-04-14.md"
```

若不传 `--output`，则直接输出到终端。

评分规则已固化：

1. 六维加权：
   `parallel_isolation / code_and_tests / merge_governance / observability_and_audit / delivery_efficiency / stability`
2. 支持总分封顶：
   `main_direct_edits / success_without_persistence / db_skip_or_pool_timeout`
3. 支持单维封顶/封底：
   `rollback_plan_missing / multiple_ai_same_branch / logic_deleted_to_fix_compile`

## 4. 推荐执行顺序

建议每天固定按以下顺序执行：

1. 先生成或更新任务卡。
2. AI 子任务开始实现。
3. 合并前先跑边界校验。
4. 收工前生成评分卡。
5. 发布前再结合 `RELEASE_GATE_CHECKLIST_V2.md` 和 `GO_NO_GO_CHECKLIST_V2.md` 做总门禁。

## 5. 与售后工业级收敛的关系

这套工具最适合当前仓库里的以下场景：

1. 售后状态机与幂等
2. 退款财务对账与审计
3. SLA 与升级
4. 平台售后联动与补偿
5. 退货入库、换货、补发和客服工作台

对这些高风险链路，建议始终执行：

1. 任务卡白名单
2. 边界校验
3. 测试证据
4. 每日评分卡
