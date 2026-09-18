# 衡策云（店铺 ERP / 平台订单为准）

> 当前版本：`0.1.9`（2026-05-10）

> 当前方向（2026-09-09）：当前无用户、业余自用；新 UI 全部采用 Rust 原生桌面与 Rust/WASM Web，共享 egui 界面和自研网格。旧 React 前端冻结。先自用实用，再邀请 3—5 家商家试用，有稳定收入后建设 SaaS。全部业务表格统一迁入自研数据网格，禁止 AG Grid 社区版和企业版；目前规范已制定，代码迁移尚未完成。开发先读 [AGENTS.md](AGENTS.md)、[组件库规范](docs/UI_COMPONENT_LIBRARY.md) 和 [路线图](docs/ROADMAP.md)。

衡策云是面向多平台电商经营的 ERP 基础工程骨架。当前版本聚焦“平台订单进入系统后，经过审单、库存、仓储、物流、售后、财务和报表形成可追溯闭环”，提供可编译的 Rust 多服务工程、React 管理端、OpenAPI 契约与工业化门禁。

## 现存代码能力（不代表本次运行验收）
- Webhook 入站 → Inbox 去重 → Outbox 事件
- Outbox → NATS 投递（至少一次）
- Worker 订阅 → 订单投影 upsert
- 多租户 RBAC/JWT 骨架
- Web 管理端（React/Vite/Ant Design Pro）与 OpenAPI 生成
- 系统管理：权限目录/角色/用户/数据权限、审计日志、配置中心、失败队列与 trace/metrics
- 后续产品范围与开发计划：以 `docs/PRODUCT_SCOPE.md` 和唯一计划入口 `docs/ROADMAP.md` 为准

## 服务与通路
- `erp-channel`：接收 `/webhooks/{platform}/{shop_id}`，落库原始 payload，去重写入 `inbox_events`，产生 `outbox_events(INBOX_EVENT_RECEIVED)`
- `erp-dispatcher`：扫描 `outbox_events`，发布到 `erp.outbox.{tenant}.{event_type}`
- `erp-worker`：订阅 `erp.outbox.>`，处理 `INBOX_EVENT_RECEIVED`，解析并 upsert `orders`、`order_items`
- `erp-api`：管理接口与鉴权（OpenAPI: `openapi/erp_api_v1.yaml`）
- `adapter-*`：平台适配器骨架（淘宝/京东/拼多多/抖音）

> 真实平台签名、鉴权、字段映射、链路细节与后续开发方向，详见 `docs/README.md`（文档中心）。

## 目录结构
- `crates/`: Rust workspace（通用库、适配器、服务）
- `apps/erp-ui`：首批 Rust/egui 网格实验台与 native/web 入口，参见 [运行说明](apps/erp-ui/README.md)
- `apps/erp-web`：冻结的旧 Web UI（Vite + React + Ant Design Pro）
- `migrations/`: 数据库迁移
- `openapi/`: OpenAPI 规范
- `infra/` / `docker-compose*.yml`: 基础设施与部署样例
- `docs/`: 合并后的产品范围、路线图、架构、前端规范与运维门禁

## 依赖
- Docker（Postgres/Redis/NATS）
- Rust 1.98.1（根工具链固定）；Edition 2024、rustfmt style edition 2024。配置已更新，代码兼容及构建待验收。
- `sqlx-cli`
- `pnpm`（Web UI）

## 后端历史启动示例

以下命令仅解释服务，实际构建先遵守 AGENTS.md 的统一入口/固定缓存规则。新 Rust UI 的启动与双端发布入口待实现。
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
1. 按 `scripts/seed.local.sql` 创建 tenant + shop（或使用 `docker-compose.local.yml` 自动注入种子数据）
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
- 兼容层说明见 `docs/OPERATIONS.md`

## 文档导航
- `docs/README.md`: 文档中心、阅读顺序与维护规则
- `docs/PRODUCT_SCOPE.md`: 产品范围、要实现的功能与优先级
- `docs/ROADMAP.md`: 唯一后续开发计划入口，包含当前评估、目标版本、近期执行队列与验收门禁
- `docs/ARCHITECTURE.md`: 架构、服务职责、数据域、状态和接口约定
- `docs/FRONTEND_GUIDE.md`: 前端页面、工作台、表格、测试和打印规范
- `docs/OPERATIONS.md`: 本地运行、验收命令、发布门禁、安全、运维要求与版本更新记录
- `openapi/`: API 规范（配合 `apps/erp-web` 生成客户端）
