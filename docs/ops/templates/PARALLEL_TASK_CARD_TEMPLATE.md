# 多 AI 并行子任务卡模板（脚本/人工双兼容）

```yaml
---
name: 任务名称
goal: 一句话目标
background: 业务背景说明
owner: 主负责人或主控会话
branch: codex/<task-name>
worktree: .worktrees/<task-name>
allowed_files:
  - scripts/ops/**
disallowed_files:
  - apps/erp-web/**
dependencies:
  - 共享文件锁已登记
dod:
  - 通过指定测试
  - 无越界修改
test_commands:
  - node --test scripts/ops/task_ops.test.mjs
delivery: commit hash + 测试结果 + 风险说明
---
```

## 目标
补充业务成功标准、用户结果和边界。

## 业务背景
说明该任务与售后、退换货、补发、平台联动、逆向入库或客服工作台的关系。

## DoD
- 写明必须达到的完成定义
- 包括幂等、审计、落库、SLA 或回滚要求

## 依赖前提
- 写明依赖的上游任务、共享文件锁、平台沙箱、数据库准备

## 测试命令
- 写明单测、集成测试、E2E 或回放命令

## 交付格式
- `commit hash`
- 变更摘要
- 测试结果
- 残余风险

说明：
- 若使用脚本生成，可执行 `node scripts/ops/new_parallel_task.mjs ...`
- 若手工填写，请保持 front matter 字段完整，便于 `validate_task_boundary.mjs` 自动解析
