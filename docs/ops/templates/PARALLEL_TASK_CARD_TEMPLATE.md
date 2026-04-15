# 多 AI 并行子任务卡模板（V2）

```yaml
---
name: 任务名称
goal: 一句话目标
background: 业务背景说明
owner: 负责人（如 W2-owner）
window: W2
domain: oms-review-exception
slot: T1
branch: codex/<domain>-<scope>-<yyyymmdd>
worktree: .worktrees/<domain>-<scope>-<yyyymmdd>
allowed_files:
  - crates/erp-api/src/orders/review_control.rs
  - apps/erp-web/src/pages/orders/review-native/**
disallowed_files:
  - crates/erp-api/src/orders/intake.rs
shared_lock_targets:
  - crates/erp-api/src/orders.rs
dependencies:
  - 共享锁已登记
dod:
  - 核心场景回归通过
  - 无越界修改
test_commands:
  - cargo test -p erp-api --test oms_orders -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: commit hash + 测试结果 + 残余风险
---
```

## 目标

- 写清业务结果，不写实现动作。

## 业务背景

- 说明为什么做这个子任务、风险是什么、影响哪条主链路。

## DoD

- 列出可验收项（行为、数据、审计、回滚点）。

## 依赖前提

- 上游任务、共享锁、环境依赖。

## 测试命令

- 只写当前任务最小必跑集。

## 提交格式

- `task`
- `commit`
- `changed_files`
- `test_results`
- `residual_risks`

## 说明

1. 可用脚本生成模板：`node scripts/ops/new_parallel_task.mjs ...`
2. 共享锁文件改动前，先执行：`node scripts/ops/register_shared_lock.mjs --mode acquire ...`
3. 提交前必须执行：`node scripts/ops/validate_task_boundary.mjs --task <task-card> --mode all`
