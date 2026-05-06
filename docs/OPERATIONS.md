# 运行、门禁与运维规范

> 更新日期：2026-05-06  
> 本文合并原本地运行、工程规范、发布门禁、上线清单、安全、可观测性、灾备、失败处理和性能基线文档。

## 本地依赖

- Docker：Postgres、Redis、NATS 等基础设施。
- Rust stable：遵循 `rust-toolchain.toml`。
- `sqlx-cli`：执行数据库迁移。
- Node.js 与 `pnpm`：运行 Web UI 与前端测试。

## 后端本地启动

```bash
cp env/.env.example .env
docker compose -f infra/docker-compose.yml up -d
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run
cargo run -p erp-api
cargo run -p erp-channel
cargo run -p erp-dispatcher
cargo run -p erp-worker
```

本地演示数据可参考 `scripts/seed.local.sql`。

## 前端本地启动

```bash
pnpm install
pnpm run dev
```

如只运行 Web 应用，可进入 `apps/erp-web` 后执行对应脚本；仓库级命令仍以根目录 `package.json` 为准。

## 仓库验收命令

优先使用根级脚本作为统一入口：

```bash
pnpm run repo:verify
pnpm run accept:p0
pnpm run accept:web
pnpm run accept:backend
pnpm run accept:all
```

兼容历史入口：`Makefile`、`scripts/accept_*.sh`、`scripts/progress_check.sh`。如果历史入口与根级脚本冲突，以根级脚本为准。

## 发布门禁

发布前至少满足以下条件：

- 数据库迁移可从空库顺序执行，不存在重复编号或不可重放迁移。
- OpenAPI 与后端路由、前端 API 调用保持一致。
- 后端格式化、Clippy、单元测试和集成测试通过。
- 前端类型检查、构建、关键 Vitest 和 Playwright 用例通过。
- 核心链路具备审计日志、权限检查、错误处理、幂等或补偿方案。
- 安全检查覆盖鉴权、租户隔离、权限、敏感信息、日志脱敏和数据导出。
- 可观测性覆盖健康检查、指标、日志、失败队列和关键业务告警。

## 可观测性

- `/healthz` 用于深度健康检查。
- `/metrics` 输出 Prometheus 指标。
- 失败队列必须能按租户、平台、对象、时间、错误类型和重试状态查询。
- P0 链路至少监控 Webhook 入站、Outbox 堆积、Worker 消费、订单投影、同步任务失败和发货回写失败。

## 告警分级

| 级别 | 场景 | 响应目标 |
| --- | --- | --- |
| P0 | 订单入站、审单、出库、发货全局不可用 | 30 分钟内响应，4 小时内恢复或给出绕行方案 |
| P1 | 单平台、单租户或核心异步链路大面积失败 | 2 小时内响应，1 个工作日内恢复 |
| P2 | 局部功能失败且有人工绕行 | 1 个工作日内响应，3 个工作日内修复 |
| P3 | 低频体验、文案、报表或非阻塞问题 | 2 个工作日内响应，排入迭代 |

## 数据保留与灾备

- 原始平台 payload、审计日志、状态变更和关键财务记录必须按租户维度保留。
- 删除、脱敏和导出必须记录操作者、时间、原因和范围。
- 备份恢复演练必须覆盖数据库、对象存储、消息堆积和配置恢复。
- 灾备文档中的恢复步骤必须能由非原开发者执行。

## 失败处理

- 失败记录必须包含租户、业务对象、动作、错误码、错误详情、重试次数和下一次处理时间。
- 可重试错误使用指数退避或固定策略；不可重试错误进入人工处理。
- 人工回放必须有权限控制、原因记录和结果反馈。
- 重复回放必须依赖幂等键，不能产生重复订单、重复扣库存或重复发货。

## 安全要求

- 所有业务 API 必须经过鉴权和租户隔离。
- 管理员、财务、仓库、运营等角色必须按最小权限授权。
- 密钥、Token、平台授权信息和设备令牌不得出现在前端日志、浏览器存储或普通报表中。
- 数据导出、批量修改、作废、回放、退款、财务结算等高风险动作必须审计。

## 版本更新记录

### v0.1.7（2026-05-06）

- 文档治理：将分散历史文档收敛到 `docs/README.md`、`docs/PRODUCT_SCOPE.md`、`docs/ROADMAP.md`、`docs/ARCHITECTURE.md`、`docs/FRONTEND_GUIDE.md` 与 `docs/OPERATIONS.md`。
- P0 门禁：补充 route registry、菜单/路由一致性、`accept:p0` 与主链路 release gate 静态覆盖。
- 后端模块化：`erp-api/src/lib.rs` 降至约 `9,082` 行，拆出业务 handler/helper，并对非入口 API 模块建立 `5,000` 行上限守护。
- 工业级拆分：将库存/打印/报表与 WMS 操作按 bounded context 细分到子目录聚合模块，继续保留外部路由行为不变。
- 订单守卫：补充订单审核重开领域守卫，阻止已发货、已签收、已完成、已取消订单执行回滚/重新排队。
- Worker 模块化：将订单审核、发货、库存、同步、售后、打印、波次、Inbox、DLQ、财务利润和运行时 helper 从 `main.rs` 拆出。

## 文档治理

- 临时任务卡、并行窗口矩阵、阶段性评分卡和一次性验收快照不再进入 `docs/`。
- 需要长期保留的结论必须合并到 `docs/PRODUCT_SCOPE.md`、`docs/ROADMAP.md`、`docs/ARCHITECTURE.md`、`docs/FRONTEND_GUIDE.md` 或本文。
- 需要追溯旧计划和旧报告时，通过 Git 历史查看已删除文档。
