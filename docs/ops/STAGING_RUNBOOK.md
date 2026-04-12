# Staging 部署 Runbook（Docker Compose）

## 1) 启动 / 停止

### 准备环境文件
```bash
cp .env.staging.example .env.staging
```
填写 `.env.staging` 的实际值（不要提交密钥）。

建议显式配置导出文件目录：
- `EXPORT_BASE_DIR=/data/erp/exports`（挂载持久卷或对象存储目录）

### 启动
```bash
docker compose --env-file .env.staging -f docker-compose.staging.yml up -d --build
```

### 停止
```bash
docker compose --env-file .env.staging -f docker-compose.staging.yml down
```

### 健康检查
```bash
curl http://localhost:${HTTP_PORT_API:-8080}/healthz
curl http://localhost:${HTTP_PORT_CHANNEL:-8081}/healthz
```

## 2) 数据库迁移

### 安装 sqlx-cli（一次性）
```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

### 迁移执行
```bash
set -a; source .env.staging; set +a
sqlx migrate run
```

### 回滚
本仓库迁移为前向迁移，回滚需通过备份恢复（详见 `docs/ops/DATA_RETENTION.md`）。

## 3) Feature Flags 开关（READ_ONLY 等）

> 详见 `docs/ops/FEATURE_FLAGS.md`，以下为常用操作示例。

### 一键只读（READ_ONLY）
```sql
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('00000000-0000-0000-0000-000000000000'::UUID, 'GLOBAL', 'READ_ONLY', true)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = true, updated_at = now();
```

### 禁用发货/打印
```sql
-- 发货禁用
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('<TENANT_ID>'::UUID, 'TENANT', 'SHIPPING_WRITE', false)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = false, updated_at = now();

-- 打印禁用
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('<TENANT_ID>'::UUID, 'TENANT', 'PRINTING', false)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = false, updated_at = now();
```

## 4) 日志 / Metrics

### 日志查看
```bash
docker compose --env-file .env.staging -f docker-compose.staging.yml logs -f erp-api
docker compose --env-file .env.staging -f docker-compose.staging.yml logs -f erp-worker
docker compose --env-file .env.staging -f docker-compose.staging.yml logs -f erp-channel
docker compose --env-file .env.staging -f docker-compose.staging.yml logs -f erp-web
```

### Metrics
```bash
curl http://localhost:${HTTP_PORT_API:-8080}/metrics
curl http://localhost:${WORKER_METRICS_PORT:-9090}/metrics
```

## 5) 回退策略（关键操作）

### 5.1 一键切回 READ_ONLY
```sql
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('00000000-0000-0000-0000-000000000000'::UUID, 'GLOBAL', 'READ_ONLY', true)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = true, updated_at = now();
```

### 5.2 停止 worker
```bash
docker compose --env-file .env.staging -f docker-compose.staging.yml stop erp-worker
```

### 5.3 禁用打印 / 发货
```sql
-- 打印
UPDATE feature_flags SET enabled = false, updated_at = now()
WHERE key = 'PRINTING';

-- 发货
UPDATE feature_flags SET enabled = false, updated_at = now()
WHERE key = 'SHIPPING_WRITE';
```

## 6) 预发布回归/Go-No-Go

### 6.1 门禁脚本
```bash
pnpm run repo:verify
pnpm run accept:web
pnpm run accept:backend
pnpm run accept:all
```

兼容层（历史入口，不作为主推荐）：
```bash
bash scripts/progress_check.sh
```

### 6.2 清单核验
- 逐项填写 `docs/ops/GO_NO_GO_CHECKLIST.md`。
- 记录回归证据（日志/截图/SQL 输出）。
