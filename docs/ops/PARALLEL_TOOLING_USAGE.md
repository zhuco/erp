# 多 AI 并行工具使用说明（V2）

本文档覆盖以下已落地工具：

1. `scripts/ops/new_parallel_task.mjs`
   生成可解析的任务卡 Markdown。
2. `scripts/ops/validate_task_boundary.mjs`
   校验任务边界（白名单/禁区）并校验共享锁。
3. `scripts/ops/register_shared_lock.mjs`
   管理共享锁（申请/释放/查看）。
4. `scripts/ops/generate_parallel_scorecard.mjs`
   根据 JSON 输入生成每日评分卡。

## 1. 生成任务卡

```powershell
node scripts/ops/new_parallel_task.mjs `
  --name "OMS 例外审核规则收敛" `
  --goal "统一例外审核动作与规则" `
  --background "降低订单审核链路回归" `
  --owner "W2-owner" `
  --window "W2" `
  --domain "oms-review-exception" `
  --slot "T1" `
  --branch "codex/oms-review-rules-20260414" `
  --worktree ".worktrees/oms-review-rules-20260414" `
  --allowed "crates/erp-api/src/orders/review_control.rs" `
  --allowed "apps/erp-web/src/pages/orders/review-native/**" `
  --disallowed "crates/erp-api/src/orders/intake.rs" `
  --shared-lock-target "crates/erp-api/src/orders.rs" `
  --dependency "共享锁已登记" `
  --dod "审核规则回归测试通过" `
  --test "cargo test -p erp-api --test oms_orders -- --nocapture" `
  --submission-format "task | commit | changed_files | test_results | residual_risks" `
  --output "docs/ops/tasks/2026-04-14/oms-review-rules.md"
```

## 2. 共享锁登记

共享锁策略：`docs/ops/config/PARALLEL_SHARED_LOCKS_V2.json`  
共享锁状态：`docs/ops/state/PARALLEL_SHARED_LOCK_REGISTRY.json`

查看锁状态：

```powershell
node scripts/ops/register_shared_lock.mjs --mode status
```

申请锁：

```powershell
node scripts/ops/register_shared_lock.mjs `
  --mode acquire `
  --task "docs/ops/tasks/2026-04-14/oms-review-rules.md" `
  --owner "W2-owner" `
  --pattern "crates/erp-api/src/orders.rs"
```

释放锁：

```powershell
node scripts/ops/register_shared_lock.mjs `
  --mode release `
  --task "docs/ops/tasks/2026-04-14/oms-review-rules.md" `
  --owner "W2-owner"
```

## 3. 校验任务边界

显式文件校验：

```powershell
node scripts/ops/validate_task_boundary.mjs `
  --task "docs/ops/tasks/2026-04-14/oms-review-rules.md" `
  --file "crates/erp-api/src/orders/review_control.rs"
```

从 Git 自动读取变更：

```powershell
node scripts/ops/validate_task_boundary.mjs `
  --task "docs/ops/tasks/2026-04-14/oms-review-rules.md" `
  --mode all
```

自定义策略/锁文件（调试或演练）：

```powershell
node scripts/ops/validate_task_boundary.mjs `
  --task "docs/ops/tasks/2026-04-14/oms-review-rules.md" `
  --mode all `
  --policy-file "docs/ops/config/PARALLEL_SHARED_LOCKS_V2.json" `
  --lock-registry "docs/ops/state/PARALLEL_SHARED_LOCK_REGISTRY.json"
```

通过标准：

1. 输出 `PASS`。
2. 所有改动文件在 `allowed_files` 内。
3. 无改动命中 `disallowed_files`。
4. 若命中共享锁策略文件，当前任务必须已登记有效锁。

## 4. 生成评分卡

```powershell
node scripts/ops/generate_parallel_scorecard.mjs `
  --input "docs/ops/examples/PARALLEL_SCORECARD_SAMPLE.json" `
  --output "docs/ops/scorecards/2026-04-14.md"
```

## 5. 推荐执行顺序

1. 先生成或更新任务卡。
2. 若涉及共享锁文件，先申请锁。
3. 子任务实现并运行最小测试。
4. 提交前执行边界校验。
5. 收工前生成评分卡并记录风险。

## 6. 与 10 窗口矩阵联动

窗口职责、路径白名单与提示词见：

- `docs/ops/PARALLEL_WINDOW_MATRIX_V2.md`
- `docs/ops/config/PARALLEL_WINDOW_MATRIX_V2.json`
