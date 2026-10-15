# 易格制造与贸易 ERP

> 当前版本：`0.1.9`（2026-05-10）

> 当前方向（2026-10-01）：面向生产制造和贸易公司，官方提供基础免费在线服务，后续增加收费功能。免费版一个使用者账号，推荐同账号多端在线，单会话为待最终选择的备选。Rust 原生桌面与 Rust/WASM Web 共用 egui 和自研 DataGrid；旧 React 前端冻结，AG Grid 全面禁用。功能与验收见 [免费 ERP 清单](docs/FREE_ERP_CHECKLIST.md)，实施顺序仍在 [路线图](docs/ROADMAP.md)。开发先读 [AGENTS.md](AGENTS.md) 和 [组件库规范](docs/UI_COMPONENT_LIBRARY.md)。

项目已有电商订单、商品、库存等 Rust 后端、工作台及组件基础，将复用于贸易进销存和制造 BOM、MRP、生产执行及成本管理。新产品范围已制定；免费制造与贸易产品、公网托管及相关验收尚未完成，历史电商代码不等于制造能力已实现。

详细交接见[免费ERP实施规范](docs/FREE_ERP_DEVELOPMENT_SPEC.md)：第十八章包含100项功能的位置、验收与性能矩阵，第十九章记录原事实读取，第二十章记录原成本批次及命令身份，第二十一章记录收货写入当前授权、真实多端重放、操作者原证据和提交前有效期。各章实际通过范围与待实现范围分别留证。

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
