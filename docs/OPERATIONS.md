# 运行、门禁与运维规范

> 更新日期：2026-10-02
> 本文合并原本地运行、工程规范、发布门禁、上线清单、安全、可观测性、灾备、失败处理和性能基线文档。

## 免费制造与贸易的当前验收入口

当前新 UI 为 Rust native/WASM。所有 Rust 构建、检查和测试均使用 `scripts/Invoke-ErpBuild.ps1`，固定工具链及缓存、锁和空间准入以根 AGENTS.md 为准；下方历史启动示例不授予绕过统一入口的权限。文档变更只检查编码、路径、引用与一致性，不启动编译。

部分回归目标已登记为 `free_erp_acceptance`。现 QA 身份为本机 `127.0.0.1:15438/erp_shared_contract_test`，角色 `erp_grid_test`；它与真实企业及另一个 UI 验收库分离。测试只追加独立 UUID 企业，禁止清空或覆盖旧夹具。先确认现有服务身份，不自动新建、重启或初始化 PostgreSQL。

```powershell
$env:ERP_GRID_TEST_DATABASE_URL = 'postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
& .\scripts\Invoke-ErpBuild.ps1 -Action test -Scope api -IntegrationTest free_erp_acceptance
```

需要定向复现时增加 `-IntegrationFilter mrp_`、`manufacturing_write`、`manufacturing_fractional_return` 或 `manufacturing_concurrent`，仍使用同一入口。缺数据库配置或连接不可用必须明确失败或记录 NOT_RUN/UNAVAILABLE；测试函数提前返回后显示的“passed”不能作为数据库验收。

当前 `free_erp_acceptance` 包含算法、部分合成JWT路由及真实QA数据库回归；正式密码/角色/会话已接库存夹具和登录小连接池、取消及凭据竞争专项，具体证据见[账号接续](FREE_ERP_DEVELOPMENT_SPEC.md#登录连接池-密码计算和准入竞态)。其余业务正常账号、全部制造事务、正式双端、Release和恢复演练仍分别验收。源文件在测试前后变化时，只保留原产物证据，当前源码重新验收。历史索引见 `qa/acceptance/free-erp-audit.json`，阶段结果与剩余项见路线图；普通QA低成本hash不得用于登录性能资格。

领域库局部检查使用 `-Scope domain`；格式检查可用 `-Action fmt -FormatFiles @('crates/erp-domain/src/manufacturing.rs')`，写入另加 `-WriteFormat`。文件必须属于当前 Scope 的包且是叶模块。绝对路径同样经过包边界及重解析点检查；不借局部修复重排并发开发中的整个仓库。

BOM 结构的保存、展开和分页读取位于 `crates/erp-repo/src/shared_contract/bom_structure.rs`，保持 `SharedContractRepo` 公共方法。领域算法使用有界拓扑计算，合并共用子树需求；每条可达边保存一个汇总结果，深度为从根到父项的最长深度再加一。该结果不是全部路径的副本，正式树视图仍按结构和稳定身份分页查询。结构与展开结果均按每批 500 行写入原事务，回执和结果共同提交。

显式 Release 性能测量入口如下。它只允许保护库，新增一个独立 QA 企业、一个 1 万物料/5 万边/32 层快照以及 32 次展开（2 次预热、30 次计量），独立 SQL 核对数量、行数、深度和重试。测试默认忽略；缺配置或身份不符必须失败。实际测量从快照查询开始到结果和回执提交结束，原始样本写入 `qa/performance/free-erp-bom.json`。HTTP、正式双端、公网服务、其他宿主负荷和峰值内存另留证，不能用该测试总耗时或忽略结果宣布 ERP-P10 全部通过。

```powershell
$env:ERP_GRID_TEST_DATABASE_URL = 'postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
& .\scripts\Invoke-ErpBuild.ps1 -Action test -Scope api -IntegrationTest free_erp_bom_performance -Release -RunPerformance
```

制造写接口需要 `MFG.PRODUCTION.WRITE`，旧风格别名为 `manufacturing.write`；权限目录从 `PermissionCode::all` 生成。角色需明确授予能力，不依靠菜单隐藏或把只读用户视为生产操作员。授权变更与真实会话中的生效还须单独验收。

ERP 性能统一遵循 [免费 ERP 清单](FREE_ERP_CHECKLIST.md) 第二十、二十三节。固定 Release 产物、服务器/客户端配置、QA 数据摘要、请求速率和原始样本后再测量；未测项目保持 NOT_RUN。测试目标总耗时、编译成功、模拟远端和保存一行元数据均不是对应业务或性能通过证据。

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
- 依赖与字体许可证入口是 `pnpm run repo:verify:licenses`。它扫描 `Cargo.lock`、`package.json` 和 `apps/erp-ui/assets/fonts`，报告写到 `output/license-report.tsv`。未豁免的 AG Grid、缺失许可证或过期豁免都会失败。豁免必须写明名称、范围和到期日。共享门禁按包名和许可证自行判定 OK、BANNED、MISSING，调用方不能把禁用依赖改写成通过。本入口不代替漏洞扫描、Release SHA 绑定或 native/wasm 构建日志。
- 可观测性覆盖健康检查、指标、日志、失败队列和关键业务告警。

发布和回退按先扩展、再切换、再收缩记录。每一步保留产物 SHA、迁移清单和验证记录。兼容窗口写明当前版本、上一版本和结束日；窗口内客户端最低版本是上一版本，窗口关闭后旧端新提交被挡住并保留草稿。灰度开关只在窗口打开时为开，窗口关闭必须关掉。回退只换回上一产物 SHA，复制已提交金额和数量，不回退新业务事实。隔离库真实升级、停机时间和首屏仍须单独测量，不能用这份记录代替。

## 双端启动

- Windows 桌面产物是 `erp-grid-lab.exe`，启动参数 `--erp`，版本 `0.1.0`，服务器范围 `erp-api-v1`。
- WASM Web 产物是 `erp_ui.wasm`，页面入口 `index.html?erp`，同一版本和服务器范围。模块示例 `--ui-modules` 不是业务入口。
- 离线只保留草稿。离线保存和库存查询被拒绝，不写库存。
- 真实两端登录、查询、编辑、保存、冲突、断网、重启和文件打印仍须单独验收。本说明不代替打包产物。

## 公网来源

- 开发进程未设置 `ERP_PUBLIC_WEB=1` 时，CORS 仍允许任意来源。这不是公网规则。
- 公网进程必须设置 `ERP_PUBLIC_WEB=1`，并在 `ERP_PUBLIC_ORIGINS` 写入逗号分隔的精确 HTTPS 来源。通配、HTTP 和未登记来源不放行。
- Cookie 会话要求 HttpOnly、Secure、SameSite Lax 或 Strict，以及 CSRF。Bearer 不存 CSRF。改密后旧代次会话失效。短期下载链接仍校验租户、账号、对象和有效期。
- HTTPS 终止、反向代理和桌面安全存储不由这条记录代替。

## 可观测性

- `/healthz` 同时报告进程存活和数据库可用。进程能应答但数据库不通时返回 `process ok; db not ready`，不能把两者写成同一个状态。请求号只进日志和追踪，不作为指标标签。
- Worker 的 `/healthz` 使用同一口径。`/metrics` 的 `erp_process_alive` 与 `erp_database_reachable` 分开。告警 `ErpProcessDown` 和 `ErpDatabaseUnreachable` 各持续 1 分钟，责任人按进程日志或数据库恢复处理。
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

## 巡检和容量

- 每月检查备份是否可恢复，每季度做一次完整恢复演练。重大存储或数据库变更后追加一次演练，上次日期早于当天即到期。变更当天不算追加演练。
- 每次演练必须写明责任人和停止动作。没有责任人的自动脚本不得启动。
- 容量巡检只记录观察值和下一步。它不代替 24 小时运行、容量升降、Worker 切换或数据库切换。
- 已结束任务可以归档。未知命令、恢复工件和业务数据保持原行，不由清理脚本删除。

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

- 公网来源必须是精确的 HTTPS 名称。现有开发 CORS 允许任意来源，不能直接作为公网规则。反向代理、HTTPS 终止和桌面平台安全存储尚未随部署交付。
- Cookie 会话必须是 HttpOnly、Secure，SameSite 为 Strict 或 Lax，并校验 CSRF。Bearer 会话不使用 Cookie。改密推进凭证代次后，各端旧会话失效。
- 短期下载链接只对同一租户和同一操作者有效，过期后拒绝。客户端材料不得包含数据库地址或对象存储主密钥。诊断日志去掉含 password、token、secret 的片段。
- 所有业务 API 必须经过鉴权和租户隔离。
- 管理员、财务、仓库、运营等角色必须按最小权限授权。
- 密钥、Token、平台授权信息和设备令牌不得出现在前端日志、浏览器存储或普通报表中。
- 数据导出、批量修改、作废、回放、退款、财务结算等高风险动作必须审计。

## 版本更新记录

### v0.1.9（2026-05-10）

- 清理仓库产物：删除根目录 `output/`、`apps/erp-web/output/` 下的 Playwright 截图、对比报告、调试日志与 mock 启动截图，以及根目录残留的 `.codex-*`、`.tmp-*` 临时日志/截图产物。
- 忽略规则收紧：补充 `logs/`、`output/`、`apps/erp-web/output/`、`.codex-*`、`.tmp-*` 等忽略项，避免本地联调和视觉比对产物再次进入版本库。
- 版本推进：Web 前端版本号从 `0.1.8` 递增到 `0.1.9`，并同步更新仓库首页、文档中心、开发计划与运维文档中的当前版本标记。
- 提交范围：本次提交仅包含仓库清理、忽略规则和版本文档更新，不混入现有业务功能改动。

### v0.1.8（2026-05-09）

- 版本推进：Web 前端版本号从 `0.1.7` 递增到 `0.1.8`，并同步更新仓库首页、文档中心与开发计划中的当前版本标记。
- 文档收口：延续“长期维护文档为主”的发布方式，本次更新集中沉淀到 `README.md`、`docs/README.md`、`docs/ROADMAP.md` 与本文件，不再新增一次性发布快照文档。
- 发布口径：将当前阶段明确为 `v0.1.8`，继续以 P0 可验收闭环为目标版本，围绕 Release Gate、订单、库存、WMS、物流、售后、财务和报表链路推进验收。
- 提交范围：本次发版提交仅包含版本号与更新文档调整，不混入其他业务代码改动。

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
