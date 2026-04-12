# 工程规范统一入口

## 目标
本项目的工程规范以仓库根目录 `package.json` scripts 为唯一官方命令入口。所有研发、验收与发布前检查文档应优先引用根级命令，避免在主入口文档中分散引用底层子命令。

## 唯一命令入口
- 官方入口：仓库根目录 `pnpm run <script>`
- 兼容层入口：`Makefile`、`scripts/accept_*.sh`、`scripts/progress_check.sh`（仅用于兼容历史流程，不作为主推荐入口）

## 统一验收命令
以下命令是工程规范中的标准入口：

```bash
pnpm run repo:verify
pnpm run accept:web
pnpm run accept:backend
pnpm run accept:all
```

## 说明
- 允许在文档中补充底层展开命令用于排障与原理说明。
- 主流程、检查清单、发布门禁等内容必须优先给出上述根级命令。

## 已知非规范类问题（截至 2026-03-24）
- 规范化主线（命令入口、workflow、文档入口）已收敛，当前遗留项主要是业务代码或格式问题，不再是入口/脚本漂移问题。
- `pnpm run repo:verify`：通过（exit code 0）。
- `pnpm run accept:web`：通过（exit code 0）；`pnpm lint`、`pnpm typecheck`、`pnpm build` 均通过。Vite chunk 体积告警存在，但不构成失败项。
- `pnpm run accept:backend`：失败（exit code 1），失败阶段为 `cargo fmt --all --check`。当前主要失败文件包括：
  - `crates/erp-api/src/lib.rs`
  - `crates/erp-api/tests/crm_and_stats_api.rs`
- 上述后端失败归属为代码格式问题（Rust fmt 未对齐），不属于规范入口、脚本路由或命令编排问题。
