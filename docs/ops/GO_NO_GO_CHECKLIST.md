# Go/No-Go Checklist (Pre-Production)

> 勾选清单用于上线前确认。每项包含检查方法/命令、通过标准、证据位置。

## A) 构建与门禁（后端/前端）
- [ ] **仓库基线校验**：检查方法/命令：`pnpm run repo:verify`；通过标准：退出码 0；证据位置：终端/CI 日志。
- [ ] **后端验收门禁**：检查方法/命令：`pnpm run accept:backend`；通过标准：退出码 0；证据位置：终端/CI 日志。
- [ ] **前端验收门禁**：检查方法/命令：`pnpm run accept:web`；通过标准：退出码 0 且产物可生成；证据位置：终端/CI 日志与 `apps/erp-web/dist/`。
- [ ] **全量验收**：检查方法/命令：`pnpm run accept:all`；通过标准：退出码 0；证据位置：终端/CI 日志。

## B) 数据库与迁移（migrations/回滚策略/备份）
- [ ] **迁移应用状态**：检查方法/命令：`sqlx migrate info`（需设置 `DATABASE_URL`）；通过标准：无 `Pending` 迁移；证据位置：命令输出或发布记录截图。
- [ ] **数据库备份**：检查方法/命令：`pg_dump $env:DATABASE_URL > backups/erp-<date>.sql` 或运行 `scripts/backup_postgres.sh`；通过标准：备份文件生成且可恢复；证据位置：`backups/` 下备份文件路径。
- [ ] **回滚策略确认**：检查方法/命令：复核 `docs/ops/FAILURE_HANDLING.md` 与 `docs/ops/DATA_RETENTION.md`；通过标准：回滚步骤/数据保留策略已更新；证据位置：对应文档路径或发布说明链接。
- [ ] **导出文件清理**：检查方法/命令：`EXPORT_JOBS_RETENTION_DAYS=1 cargo run -p erp-worker --bin purge`；通过标准：过期导出文件被删除且 `export_jobs` 状态一致；证据位置：`EXPORT_BASE_DIR` 目录对比或日志截图。

## C) 可靠性（乱序、幂等、worker失败重试/DLQ、feature flags）
- [ ] **乱序/去重链路**：检查方法/命令：`cargo test -p erp-channel webhook_dedup_does_not_duplicate_events`；通过标准：测试通过且无重复入库；证据位置：测试日志关键字 `ok`。
- [ ] **订单乱序保护**：检查方法/命令：SQL 验证 `orders` upsert 旧 `platform_updated_at` 不覆盖新值（示例见 `crates/erp-repo/src/orders/store.rs`）；通过标准：旧时间戳更新被忽略；证据位置：SQL 结果截图或验证记录。
- [ ] **幂等表存在**：检查方法/命令：`psql $env:DATABASE_URL -c "select count(*) from idempotency_keys;"`；通过标准：表存在且查询成功；证据位置：SQL 输出或截图。
- [ ] **DLQ 可观测**：检查方法/命令：`psql $env:DATABASE_URL -c "select count(*) from dead_letters;"`；通过标准：计数为 0 或有明确处理记录；证据位置：SQL 输出或处理记录。
- [ ] **Worker 重试路径**：检查方法/命令：`cargo test -p erp-worker shipment_skipped_when_feature_disabled`；通过标准：测试通过，日志包含 `feature disabled` 分支；证据位置：测试日志关键字 `feature disabled`。

## D) 安全（tenant隔离、RBAC、敏感信息、最小权限）
- [ ] **JWT Secret 非默认**：检查方法/命令：核对 `.env` 或部署环境变量 `JWT_SECRET`；通过标准：不为 `CHANGE_ME_SUPER_SECRET`；证据位置：部署配置截图/密钥管理记录。
- [ ] **Tenant 隔离**：检查方法/命令：代码审查 `crates/erp-api/src/lib.rs` 路由均包含 `/v1/tenants/{tenant_id}`；通过标准：所有租户资源均受 tenant_id 限定；证据位置：代码审查记录或文件路径。
- [ ] **RBAC 生效**：检查方法/命令：审查 `apps/erp-web/src/app/router.tsx` + `apps/erp-web/src/features/rbac.tsx`；通过标准：核心页面使用权限守卫；证据位置：文件路径或截图。

## E) 观测（日志、trace、metrics、告警建议）
- [ ] **日志/Trace 可用**：检查方法/命令：启动服务并观察日志；通过标准：日志包含 `tracing initialized`；证据位置：服务日志关键字 `tracing initialized`。
- [ ] **API Metrics**：检查方法/命令：`curl http://localhost:8080/metrics`；通过标准：返回 `erp_` 前缀指标；证据位置：响应体或截图。
- [ ] **Worker Metrics**：检查方法/命令：`curl http://localhost:9090/metrics`（或 `WORKER_METRICS_PORT`）；通过标准：返回 `erp_` 指标；证据位置：响应体或截图。

## F) 运行与回退（只读、开关策略）
- [ ] **只读模式**：检查方法/命令：设置 `feature_flags` 表中 `READ_ONLY` 为 `true`，执行写接口应返回 `feature_disabled`；通过标准：写操作被阻断；证据位置：API 响应或日志关键字 `feature_disabled`。
- [ ] **开关策略**：检查方法/命令：复核 `docs/ops/FEATURE_FLAGS.md`；通过标准：明确开关表、默认值与启停流程；证据位置：文档路径或发布说明链接。

## G) 业务验收（订单→出库→发货→打印→售后最小闭环）
- [ ] **订单入库/投影**：检查方法/命令：使用 `docs/dev/seed.sql` 注入订单数据或调用订单创建接口（如已提供）；通过标准：`orders` 表有记录且状态可查询；证据位置：SQL 输出或截图。
- [ ] **出库/发货链路**：检查方法/命令：Web UI 完成出库/打包/发货确认；通过标准：订单详情出现发货状态且 `shipments` 表新增记录；证据位置：UI 截图或 SQL 输出。
- [ ] **打印/售后闭环**：检查方法/命令：订单详情创建打印任务并进入售后列表核验；通过标准：`print_jobs` 表有记录，售后列表可检索；证据位置：UI 截图或 SQL 输出。
