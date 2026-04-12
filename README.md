# 衡策云 (SaaS 多店铺 / 日百万单 / 平台订单为准)

衡策云是面向多平台电商订单的 ERP 基础工程骨架。当前版本聚焦在“订单闭环通路”，提供可编译的 Rust 多服务工程、最小可运行链路与文档模板，方便按任务卡补齐平台鉴权、字段映射与业务功能。

## 当前能力
- Webhook 入站 → Inbox 去重 → Outbox 事件
- Outbox → NATS 投递（至少一次）
- Worker 订阅 → 订单投影 upsert
- 多租户 RBAC/JWT 骨架
- Web 管理端（React/Vite/Ant Design Pro）与 OpenAPI 生成
- 系统管理：权限目录/角色/用户/数据权限、审计日志、配置中心、失败队列与 trace/metrics

## 服务与通路
- `erp-channel`：接收 `/webhooks/{platform}/{shop_id}`，落库原始 payload，去重写入 `inbox_events`，产生 `outbox_events(INBOX_EVENT_RECEIVED)`
- `erp-dispatcher`：扫描 `outbox_events`，发布到 `erp.outbox.{tenant}.{event_type}`
- `erp-worker`：订阅 `erp.outbox.>`，处理 `INBOX_EVENT_RECEIVED`，解析并 upsert `orders`、`order_items`
- `erp-api`：管理接口与鉴权（OpenAPI: `openapi/erp_api_v1.yaml`）
- `adapter-*`：平台适配器骨架（淘宝/京东/拼多多/抖音）

> 真实平台签名/鉴权/字段映射与链路细节，详见 `docs/spec` 与 `docs/list.md`（统一入口）。

## 目录结构
- `crates/`: Rust workspace（通用库、适配器、服务）
- `apps/erp-web`: Web UI（Vite + React + Ant Design Pro）
- `migrations/`: 数据库迁移
- `openapi/`: OpenAPI 规范
- `infra/` / `docker-compose*.yml`: 基础设施与部署样例
- `docs/`: 规范、运维、验收与任务卡

## 依赖
- Docker（Postgres/Redis/NATS）
- Rust stable >= 1.78
- `sqlx-cli`
- `pnpm`（Web UI）

## 快速开始（后端闭环）
```bash
cp env/.env.example .env
docker compose -f infra/docker-compose.yml up -d

cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run

# 建议分别开终端
cargo run -p erp-api        # http://localhost:8080
cargo run -p erp-channel    # http://localhost:8081
cargo run -p erp-dispatcher
cargo run -p erp-worker
```

## 最小闭环验证
1. 按 `docs/dev/seed.sql` 创建 tenant + shop（或使用 `docker-compose.local.yml` 自动注入种子数据）
2. 调用 webhook：
   ```bash
   curl -XPOST "http://localhost:8081/webhooks/taobao/<SHOP_UUID>" \
     -H "Content-Type: application/json" \
     -d '{"platform_order_id":"TB123","created_at":"2026-01-06T10:00:00Z","total_cent":1000}'
   ```
3. 观察 dispatcher/worker 日志或查询 `orders` 表

## Web UI
```bash
pnpm install
pnpm run dev
```

## 开发门禁（统一入口）
仓库官方命令入口为根级 `package.json` scripts：

```bash
pnpm run repo:verify
pnpm run accept:web
pnpm run accept:backend
pnpm run accept:all
```

兼容层（历史入口，不作为主推荐）：
- `Makefile`（例如 `make accept-web`）
- `scripts/accept_*.sh`
- `scripts/progress_check.sh`
- 兼容层说明见 `docs/ops/ENGINEERING_STANDARDS.md`

## 文档导航
- `docs/list.md`: 统一入口（文档分层、冲突裁决、主题归并）
- `docs/spec/`: 架构、数据模型、平台接入与流程规范
- `docs/web/`: 前端开发、测试验收与打印桥接
- `docs/ops/`: 运维、发布、门禁与运行手册
- `docs/dev/前后端使用方式.md`: 前端、后端与联调的本地使用说明（Windows / PowerShell）
- `docs/ops/ENGINEERING_STANDARDS.md`: 工程规范与命令统一入口
- `docs/reports/README.md`: 报告目录入口（含当前保留文档与历史归档指引）
- `docs/superpowers/`: 设计与实施计划留痕
- `openapi/`: API 规范（配合 `apps/erp-web` 生成客户端）
