# 任务完成情况检查（Progress Check）

用于固化任务完成后的门禁与覆盖检查，生成可审计报告。官方命令入口统一为仓库根目录 `package.json` scripts。

## 官方入口
```bash
pnpm run repo:verify
pnpm run accept:web
pnpm run accept:backend
pnpm run accept:all
```

## 兼容层（历史入口，不作为主推荐）
```bash
bash scripts/progress_check.sh
make progress-check
```

> 兼容层仅用于历史流程迁移；主流程请以上述根级命令为准。`PROGRESS_SMOKE=1` 仅适用于兼容脚本，不属于根级主入口契约。

## 检查项
### 后端门禁
- 主入口：`pnpm run accept:backend`
- 底层展开：`cargo fmt --all --check`、`cargo clippy`、`cargo test --all`、`cargo build --all`

### 前端门禁
- 主入口：`pnpm run accept:web`
- 底层展开：`pnpm run lint`、`pnpm run typecheck`、`pnpm run build`

### 迁移检查
默认使用 `rg` 扫描 `migrations/` 中关键表是否存在（例如 `tenants`、`shops`、`orders`、`order_items`、`outbox_events`、`inventory_balances`）。如需 DB 级校验，可用 `psql` 的 `to_regclass` 手工核验：
```sql
select to_regclass('public.orders'), to_regclass('public.order_items');
```
如需验证空库迁移与索引，可执行：
```bash
docker compose -f infra/docker-compose.yml down -v
docker compose -f infra/docker-compose.yml up -d
DATABASE_URL=postgres://erp:erp@localhost:5432/erp sqlx migrate run
```
```sql
select tablename, indexname
from pg_indexes
where indexdef like '%tenant_id%' and indexdef like '%created_at%';
```

### OpenAPI 覆盖检查
检查 `openapi/erp_api_v1.yaml` 是否包含关键路径（示例：`/healthz`、`/auth/login`、`/v1/tenants/{tenant_id}/orders`、`/v1/tenants/{tenant_id}/inventory`）。

### 关键 rg 扫描
使用 `rg` 校验多租户字段与权限标记存在，例如：
- `migrations/` 与 `crates/` 中存在 `tenant_id`
- `openapi/erp_api_v1.yaml` 中存在 `x-permissions`

### 最小闭环 smoke test
建议在本地跑通一次最小闭环验证（手动执行）：
1. `docker compose -f infra/docker-compose.yml up -d`
2. `sqlx migrate run` 并按 `docs/dev/seed.sql` 注入租户/店铺
3. 启动 `erp-api`、`erp-channel`、`erp-dispatcher`、`erp-worker`
4. 导入/创建订单数据（可使用 `docs/dev/seed.sql` 或调用订单创建接口，如已提供）
5. 查询 `orders` 或观察 worker 日志确认投影完成

## 兼容脚本可选环境变量
- `PROGRESS_SMOKE=1`：仅对 `scripts/progress_check.sh` 生效，用于启用脚本内置 smoke 测试。
