# 工程验收门禁（后端）

本门禁用于保证后端代码在提交前满足基础质量要求。官方入口统一为仓库根目录 `package.json` scripts。

## 官方入口

```bash
pnpm run repo:verify
pnpm run accept:web
pnpm run accept:backend
pnpm run accept:all
```

依赖：
- Rust stable
- Docker（可选，仅在执行数据库联调校验时需要）
- sqlx-cli（可选，仅在执行数据库联调校验时需要）

## CI 使用

建议在 CI 中添加一步：

```bash
pnpm run accept:backend
```

## 兼容层（历史入口，不作为主推荐）

```bash
bash scripts/accept_backend.sh
ACCEPTANCE_WITH_DB=1 bash scripts/accept_backend.sh
```

> 兼容层命令仅用于历史流程迁移，主流程请使用 `pnpm run accept:backend`。

## 失败定位

- `pnpm run accept:backend` 失败：先查看根命令输出的失败阶段，再针对对应子步骤排查。
- `cargo fmt` 失败：本地执行后提交格式化结果。
- `cargo clippy` 失败：查看输出警告并修复后重跑。
- `cargo test --all` 失败：可用 `cargo test -p <crate> <test_name> -- --nocapture` 定位失败用例。
- Docker/迁移失败：确认 `infra/docker-compose.yml` 可启动、`.env`/`DATABASE_URL` 正确，必要时执行 `docker compose -f infra/docker-compose.yml logs -f` 查看日志，并单独运行 `sqlx migrate run`。
