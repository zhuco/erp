# 子任务失效容灾说明

## 目标
- 减少并行子任务在瞬时网络故障（如 `502 Bad Gateway`）时整轮中断。
- 标准化“自动重试 + 失败降级”的执行方式。

## 新增能力
- 新增脚本：`scripts/ops/run_subtask_with_retry.mjs`
- 新增库：`scripts/ops/lib/subtask_retry.mjs`
- 新增测试：`scripts/ops/subtask_retry.test.mjs`
- 任务卡 front matter 增加：
  - `retry_policy`
  - `fallback_strategy`

## 快速使用
1. 运行带自动重试的子任务命令：
```bash
pnpm run ops:subtask:retry -- --name r13-task3 --max-retries 3 --delay-seconds 10 -- node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-15/erp95-r13-03.md --mode working
```
2. 判定逻辑：
- 若命中瞬时故障特征（`502/503/504`、`upstream request failed`、`ECONNRESET`、`ETIMEDOUT` 等），自动重试。
- 若是业务错误或参数错误，不重试，立即失败。

3. 非瞬时失败处理（Windows Rust 链接）：
- `LNK1140` / `LNK1318` 属于构建链路容量或符号文件问题，不属于瞬时网络故障，重试通常无效。
- 优先收敛 `test_commands` 到最小目标（例如把 `cargo test -p erp-repo mapping_rules` 收敛为 `cargo test -p erp-repo --lib mapping_rules -- --nocapture`）。
- 如仍失败，再使用更轻量的验证替代（如 `cargo check -p <crate> --lib` + 精准单测），避免整轮子任务被单条重型链接命令阻塞。

4. 直接按任务卡执行（推荐）：
```bash
pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r13-03.md --mode working
```
该命令会自动读取任务卡中的 `retry_policy` 与 `fallback_strategy`，先做边界校验，再执行 `test_commands`。

## 推荐执行策略
1. 子任务默认重试策略：
- `retry_policy: transient-auto-retry(max=2,delay=8s)`
2. 子任务默认降级策略：
- `fallback_strategy: switch-to-local-execution`
3. 达到最大重试后：
- 立即切换本地执行同一任务，避免整轮阻塞。

## 验证
```bash
pnpm run test:ops
```
