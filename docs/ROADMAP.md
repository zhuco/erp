# 后续开发计划

> 更新日期：2026-05-10
> 当前版本：`0.1.9`
> 本文是唯一长期维护的后续开发计划入口，负责回答“当前项目处于什么阶段、之后先做什么、做到什么程度才允许进入试点或发布”。

## 一、任务文档分析结论

旧路线图已经完成“从分散任务卡到统一口径”的第一轮收敛，但仍保留了较长的 `Task 0` 到 `Task 12` 任务堆叠、旧执行进展表和部分过时状态。当前项目已经不是“从零补入口”的阶段，后续开发文档必须从任务清单改为可持续执行的工程计划：

- 以 `docs/PRODUCT_SCOPE.md` 定义产品范围，本文只维护开发顺序、阶段目标、验收标准和近期执行队列。
- 以真实代码、OpenAPI、迁移、测试和门禁脚本作为现状依据，不再用临时报告或人工口头判断描述完成度。
- 以 P0 业务闭环为主线，不再继续横向铺菜单、堆页面或新增一次性任务文档。
- 以工业级架构约束治理大文件、路由、handler、service、repo、domain 边界，避免再次形成难维护的巨型文件。
- 以自动化 Release Gate 判断是否具备试点条件，验收结论必须能复现、能定位失败原因、能持续运行。

## 二、当前项目评估

### 1. 已具备的基础能力

| 维度 | 当前评估 | 主要依据 |
| --- | --- | --- |
| 文档治理 | 已收敛为 6 个长期文档入口 | `docs/README.md`、`docs/PRODUCT_SCOPE.md`、`docs/ROADMAP.md`、`docs/ARCHITECTURE.md`、`docs/FRONTEND_GUIDE.md`、`docs/OPERATIONS.md` |
| 产品范围 | P0/P1/P2 能力口径已明确 | 平台接入、订单、商品、库存、WMS、物流、售后、财务、报表、系统治理已列入统一范围 |
| 后端骨架 | Rust 多服务、事件链路和 OpenAPI 基础完整 | `erp-channel`、`erp-dispatcher`、`erp-worker`、`erp-api`、`adapter-*`、`openapi/erp_api_v1.yaml` |
| 数据演进 | 已有迁移体系和较宽业务模型 | `migrations/` 当前包含 100+ 个迁移文件 |
| 前端入口 | P0 菜单、路由注册表和成熟度标记已经建立 | `apps/erp-web/src/app/routeRegistry.ts`、`apps/erp-web/src/app/layout/menu.tsx` |
| 自动化门禁 | 仓库基线、OpenAPI gate、P0 route/menu 测试和 release gate 脚本已存在 | `pnpm run repo:verify`、`pnpm run accept:p0`、`pnpm run release:gate` |
| API 模块化 | `erp-api/src/lib.rs` 已降至约 9,000 行，非入口 API 模块已有 5,000 行上限守护 | `crates/erp-api/tests/app_modularization.rs` |
| Worker 模块化 | Worker 主入口已拆出多个业务模块并有守护测试 | `crates/erp-worker/tests/worker_modularization.rs` |
| 测试资产 | 后端集成测试、前端单元测试、E2E 均已有较多基础 | `crates/erp-api/tests/`、`apps/erp-web/src/**/*.test.ts*`、`apps/erp-web/e2e/` |

### 2. 主要缺口

| 缺口 | 风险 | 后续处理原则 |
| --- | --- | --- |
| 真实 P0 端到端数据不足 | 页面和接口可达，但无法证明“平台订单到发货、售后、对账、报表”稳定跑通 | 先补可复现数据准备、执行、断言、回收，再扩业务范围 |
| Release Gate 仍偏静态覆盖 | 可能只证明路径存在，不能证明业务状态、库存影响和财务影响正确 | 发布门禁必须逐步包含真实 API 调用、数据库断言和前端 E2E |
| `router.rs` 仍集中挂载大量路由 | 路由变更冲突高，业务域边界不清 | 按业务域拆分 routes 子模块，router 只做聚合 |
| 部分 handler 仍偏大 | 业务编排、参数处理、SQL 和响应可能混在一起 | 继续推进 handler/service/repo/domain 分层 |
| 平台接入生产化不足 | 验签、授权续期、字段映射、重试回放、告警不完整会影响真实平台试点 | 先做一个平台样板，再复制到其他平台 |
| 库存与 WMS 现场深度不足 | 库存余额、台账、波次、拣货、复核、面单、发货状态可能不同步 | 以库存台账和出库单状态为事实源补端到端断言 |
| 售后、财务、报表联动仍需做深 | 退款、退货、成本、运费、利润、结算差异难以追溯 | 先交付最小售后路径和经营级对账，再扩完整财务 |
| 数据权限与审计仍需贯穿 | 多租户、多角色、多岗位使用时容易出现越权和不可追溯 | P0 动作必须同步权限、审计、错误码和 trace id |

### 3. 当前成熟度判断

当前版本适合作为“工业级架构与 P0 闭环的开发基线”，但还不能把所有 P0 能力视为可收费试点完成。下一阶段的核心不是继续增加菜单数量，而是把已有入口串成可复现、可回归、可观测、可失败恢复的业务闭环。

## 三、目标版本定义

| 版本 | 目标 | 必须完成 | 不做范围 |
| --- | --- | --- | --- |
| `v0.1.8`：P0 可验收闭环 | 用仿真或真实数据跑通一条主链路，并把失败点暴露在门禁报告中 | Release Gate 数据准备、订单审核、库存占用、WMS 出库、物流发货、售后最小路径、基础对账、报表断言 | 不追求多平台全量能力和复杂财务总账 |
| `v0.1.9`：单平台试点版 | 选定一个平台形成生产化样板 | 授权、验签、拉单/Webhook、字段映射、失败补偿、回放、告警、店铺级配置 | 不同时推进所有平台深度适配 |
| `v0.2.0`：可收费试点版 | 支撑首批客户日常经营和岗位协作 | 多店铺、多仓、异常中心、采购入库、售后财务联动、关键报表、运维手册 | 不建设通用低代码平台 |
| `v0.3.0`：工业化稳定版 | 支撑规模化租户、长期运维和持续交付 | 多组织、多商户、审批中心、质量追溯、开放集成、SLA、容量治理、安全审计 | 不做与电商 ERP 主线无关的泛办公功能 |

## 四、P0 闭环验收标准

P0 试点闭环必须覆盖以下链路。每个节点都要有页面入口、API 契约、权限、审计、错误处理、测试或 E2E 证据。

| 链路节点 | 验收标准 | 必须断言 |
| --- | --- | --- |
| 1. 基础数据准备 | 租户、用户、角色、店铺、仓库、库位、商品、SKU、渠道映射和库存初始量可自动准备 | 数据带租户边界，可重复执行且可清理 |
| 2. 平台订单入站 | Webhook 或拉单任务写入原始 payload，经 Inbox 去重、Outbox 分发、Worker 投影订单 | 幂等键生效，重复事件不产生重复订单 |
| 3. 商品与渠道映射 | 平台商品能映射到内部 SKU，异常映射能进入待处理队列 | 未映射商品不得进入可发货状态 |
| 4. 订单审核 | 运营可完成审核、复核、异常标记、重开限制和状态追踪 | 已发货、已签收、已完成、已取消订单不得非法回滚 |
| 5. 库存占用 | 审核通过后产生库存预占或锁定，库存台账记录来源和动作 | 余额、可用量、预占量、台账一致 |
| 6. WMS 出库 | 波次、拣货、复核、打包、出库按状态机推进 | 未复核或库存不足不得发货 |
| 7. 物流与打印 | 面单、打印任务、发货确认、轨迹或回写任务可追踪 | 重试不重复发货，失败进入可处理队列 |
| 8. 售后最小路径 | 至少支持退款或退货路径，并关联订单、库存、物流和财务影响 | 退款/退货状态、库存回补、财务差异可查 |
| 9. 财务对账 | 订单金额、退款、运费、成本、平台账单和利润快照可对齐 | 差异能定位到订单、店铺、平台、责任原因 |
| 10. 报表与运维 | 销售、履约、库存、售后、利润、异常和系统观测可查询 | 报表数据来源明确，trace id 可追溯到链路动作 |

## 五、未来四个开发阶段

### 阶段 1：Release Gate 与真实数据闭环

**目标版本：** `v0.1.8`
**核心目标：** 把“是否能试点”从人工判断变成自动化门禁结论。

| 任务 | 开发内容 | 验收方式 |
| --- | --- | --- |
| 1.1 P0 场景数据夹具 | 建立可重复的数据准备、执行和回收机制，覆盖租户、店铺、仓、SKU、库存、订单、售后、财务样例 | 后端 gate 能从空库或测试库准备并清理 P0 数据 |
| 1.2 后端主链路 gate | 在 `scripts/release_gate_backend.mjs` 串联订单、库存、WMS、物流、售后、财务测试目标，禁止关键测试 skip | `pnpm run test:backend:release-gate` 给出明确通过或失败节点 |
| 1.3 前端主链路 E2E | 以真实 API 或稳定 mock 启动订单到发货、售后、对账的关键页面流程 | `pnpm run test:e2e:release-gate` 输出截图、视频和失败路径 |
| 1.4 P0 状态看板 | 让 route registry、菜单、权限、OpenAPI、E2E、后端测试可汇总为 P0 状态 | `pnpm run accept:p0` 覆盖入口可达与基础契约 |

**Done When：** 一条 P0 主链路可由命令自动准备数据、执行、断言、清理，并能在失败时指出具体环节。

### 阶段 2：订单、库存与 WMS 主链路做深

**目标版本：** `v0.1.8` 到 `v0.1.9`
**核心目标：** 让订单、库存、仓储和发货成为可日常作业的核心能力。

| 任务 | 开发内容 | 验收方式 |
| --- | --- | --- |
| 2.1 订单动作矩阵 | 统一审核、复核、取消、重开、拆单、合单、分仓、异常处理的前置条件、权限、错误码和审计 | 领域测试覆盖允许/拒绝动作矩阵 |
| 2.2 库存台账守护 | 明确可用、预占、锁定、冻结、在途、差异、回补的台账来源 | 集成测试断言余额和台账一致 |
| 2.3 ATP/FEFO 策略 | 补齐可承诺库存、批次、效期、库位策略和库存不足处理 | 库存策略测试覆盖正常、缺货、临期、批次冲突 |
| 2.4 WMS 现场执行 | 做深波次、拣货、复核、打包、称重、面单、出库、发货回写 | E2E 覆盖仓库岗位主流程 |
| 2.5 异常闭环 | 出库异常、物流缺失、库存差异、超时未处理进入异常队列 | 异常可定位责任人、处理动作和恢复结果 |

**Done When：** 仓库作业员不依赖数据库或开发者介入，即可按页面完成 P0 出库发货；库存和订单状态自动同步。

### 阶段 3：平台、商品、售后、财务与报表补齐

**目标版本：** `v0.1.9` 到 `v0.2.0`
**核心目标：** 从“订单发得出去”升级为“经营结果对得上”。

| 任务 | 开发内容 | 验收方式 |
| --- | --- | --- |
| 3.1 单平台生产化样板 | 选定一个平台做授权、验签、拉单/Webhook、发货回写、失败补偿、告警 | 平台样板具备沙箱或仿真验收记录 |
| 3.2 商品映射治理 | SKU、条码、套装、渠道商品、冲突映射、批量变更、映射版本 | 未映射/冲突映射能阻断发货并进入治理队列 |
| 3.3 售后最小闭环 | 退款或退货路径优先，逐步扩展换货、补发、逆向质检 | 售后影响库存、财务、物流和订单状态 |
| 3.4 经营级财务 | 平台账单、订单金额、退款、运费、成本、利润快照、结算差异 | 财务能定位差异来源和处理结果 |
| 3.5 关键报表 | 销售、发货、库存、售后、利润、异常、导出和角色化入口 | 报表指标有明确数据来源和过滤口径 |

**Done When：** 运营、仓库、客服、财务和管理者能围绕同一笔订单看到一致的业务事实和经营结果。

### 阶段 4：工业级架构治理与运营能力

**目标版本：** `v0.2.0` 到 `v0.3.0`
**核心目标：** 支撑多租户规模化上线、长期维护和持续交付。

| 任务 | 开发内容 | 验收方式 |
| --- | --- | --- |
| 4.1 路由分域 | 将 `router.rs` 按订单、库存、WMS、平台、商品、售后、财务、系统治理拆分 | 路由行为不变，模块化测试通过 |
| 4.2 分层落地 | handler 只做鉴权/参数/响应，service 做业务编排，repo 做持久化，domain 做状态机 | 新增功能不再把 SQL 和业务编排写进 handler |
| 4.3 模块规模治理 | 非入口 API 模块保持 5,000 行以内，继续把大 handler 拆成 bounded context 子目录 | `cargo test -p erp-api --test app_modularization -- --nocapture` |
| 4.4 运维与可观测 | 关键链路补 trace、metrics、失败队列、告警、回放、SLA | 发布门禁覆盖观测入口和失败处理 |
| 4.5 安全与租户隔离 | 高风险动作审计、数据导出审计、密钥脱敏、租户隔离回归测试 | 权限/租户隔离测试长期纳入 gate |
| 4.6 性能与容量 | 订单列表、工作台、波次、报表、导出、同步任务建立性能预算 | 压测或性能 E2E 有可复现阈值 |

**Done When：** 新增业务域可按固定工程模板落地，路由、权限、OpenAPI、测试、观测和文档都有统一入口。

## 六、近期执行队列

近期只维护可进入开发的任务，执行时优先从上到下推进；并行开发时最多拆成 5 条不互相覆盖写集的子任务。

| ID | 任务 | 主要文件 / 目录 | 验收命令 | Done When |
| --- | --- | --- | --- | --- |
| `D1` | 建立 P0 Release Gate 数据夹具 | `crates/erp-api/tests/support.rs`、`crates/erp-api/tests/*release*`、`scripts/release_gate_backend.mjs` | `pnpm run test:backend:release-gate` | 测试能自动准备并清理租户、店铺、SKU、库存、订单、售后和财务样例 |
| `D2` | 打通前端主链路 E2E | `apps/erp-web/e2e/release-gate-main-chain.spec.ts`、`apps/erp-web/e2e/release-gate-cross-chain.spec.ts` | `pnpm run test:e2e:release-gate` | E2E 覆盖登录、订单、库存、WMS、发货、售后、财务、报表关键路径 |
| `D3` | 拆分后端路由聚合 | `crates/erp-api/src/router.rs`、`crates/erp-api/src/routes/` | `cargo test -p erp-api --test app_modularization -- --nocapture` | `router.rs` 只保留组合入口，业务路由按域拆分且 OpenAPI 行为不变 |
| `D4` | 订单动作矩阵与状态守卫 | `crates/erp-domain`、`crates/erp-api/src/orders/`、`crates/erp-api/tests/oms_orders.rs` | `cargo test -p erp-api --test oms_orders -- --nocapture` | 审核、复核、取消、重开、拆合单、分仓和异常动作有统一允许/拒绝断言 |
| `D5` | 库存占用与 WMS 发货硬约束 | `crates/erp-api/src/wms_operation_handlers/`、`crates/erp-api/src/inventory_print_report_handlers/`、`crates/erp-api/tests/wave_ship_integration.rs` | `cargo test -p erp-api --test wave_ship_integration -- --nocapture` | 未复核、库存不足、物流缺失、重复发货都会被阻断并记录原因 |
| `D6` | 单平台接入生产化样板 | `crates/adapter-*`、`crates/erp-channel`、`crates/erp-api/tests/platform_*` | `cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture` | 一个平台完成授权、验签、拉单/Webhook、字段映射、失败补偿和回放样板 |
| `D7` | 商品与渠道映射治理 | `crates/erp-api/src/product_handlers.rs`、`crates/erp-api/src/sku_handlers.rs`、`crates/erp-api/src/mapping_handlers.rs` | `cargo test -p erp-api --test skus -- --nocapture` | 未映射、冲突映射、套装/条码异常能进入治理流程并阻断错误履约 |
| `D8` | 售后最小闭环 | `crates/erp-api/src/after_sales_handlers.rs`、`crates/erp-api/tests/after_sales*.rs` | `cargo test -p erp-api --test after_sales -- --nocapture` | 退款或退货路径能联动订单、库存、物流、财务和审计 |
| `D9` | 基础财务对账与利润追踪 | `crates/erp-api/src/finance_handlers.rs`、`crates/erp-api/src/finance_management.rs`、`crates/erp-api/tests/finance_reconcile.rs` | `cargo test -p erp-api --test finance_reconcile -- --nocapture` | 平台账单、订单金额、退款、运费、成本和利润差异可定位 |
| `D10` | P0 状态看板与文档门禁 | `apps/erp-web/src/app/routeRegistry.ts`、`docs/ROADMAP.md`、`docs/OPERATIONS.md` | `pnpm run accept:p0 && pnpm run check:utf8` | P0 入口、成熟度、验收命令和文档状态保持同步 |

## 七、工程约束

### 1. 文档约束

- `docs/ROADMAP.md` 是唯一后续开发计划入口；不要再新增“阶段任务卡”“并行窗口矩阵”“一次性验收报告”到 `docs/`。
- 产品范围变化更新 `docs/PRODUCT_SCOPE.md`，架构约束更新 `docs/ARCHITECTURE.md`，前端规范更新 `docs/FRONTEND_GUIDE.md`，门禁与发布更新 `docs/OPERATIONS.md`。
- 文档必须使用 UTF-8 无 BOM；提交前运行 `pnpm run check:utf8`。

### 2. 后端约束

- `router` 只挂载路由和组合业务域，不写业务编排。
- `handler` 只做鉴权、租户解析、参数校验、调用 service、响应映射和错误码转换。
- `service` 负责业务事务、领域动作编排、跨仓储协调和幂等处理。
- `repo` 负责持久化查询和数据映射，不承载 HTTP、权限或页面语义。
- `domain` 负责状态机、业务守卫、枚举和纯规则，优先用单元测试覆盖。
- 除入口兼容文件外，API 单模块不得超过 5,000 行；新增大功能必须按 bounded context 建目录。
- 新增业务 API 必须同步 OpenAPI、权限、审计、错误码、测试和前端调用。

### 3. 前端约束

- P0 菜单叶子必须登记到 `appRouteRegistry`，且不能使用占位页冒充完成。
- 页面必须具备加载态、空态、错误态、权限态、关键操作反馈和稳定 `data-testid`。
- 复杂页面优先沉淀为工作台模板，不要在页面内重复实现筛选、批量动作、明细抽屉和导出逻辑。
- 前端按钮权限必须和后端 API 权限一致，不能只靠隐藏按钮保护高风险动作。

### 4. 验收约束

- 代码格式：`cargo fmt --all --check`。
- 后端主链路：按变更范围运行对应 `cargo test -p erp-api --test ... -- --nocapture`。
- Worker 模块化：`cargo test -p erp-worker --test worker_modularization -- --nocapture`。
- 前端入口：`pnpm run accept:p0`。
- Web 交付：`pnpm run accept:web`。
- 仓库基线：`pnpm run repo:verify`。
- 发布前：`pnpm run release:gate` 或 `pnpm run release:gate:full`。
- 文档和编码：`pnpm run check:utf8` 与 `git diff --check`。

## 八、状态看板

| 业务域 | 当前状态 | 下一步 | 主要验收 |
| --- | --- | --- | --- |
| 文档治理 | 6 个核心入口已形成 | 以后只更新长期文档，不新增临时任务文档 | `pnpm run check:utf8` |
| 平台接入 | 有适配器、Inbox/Outbox、同步与映射基础 | 做一个平台生产化样板 | `platform_oauth_webhook_integration` |
| 商品主数据 | 商品、SKU、映射、条码等基础较宽 | 做映射冲突治理和批量变更闭环 | `skus`、映射相关测试 |
| 订单 OMS | 订单工作台、审核、复核和部分守卫已落地 | 完成动作矩阵、拆合单、分仓和异常一致性 | `oms_orders`、`order_*` 测试 |
| 库存/WMS | WMS 与库存接口覆盖广，模块化已有进展 | 做深现场扫描、复核、库存台账和发货硬约束 | `wave_ship_integration`、`wms_*` 测试 |
| 物流/打印 | 物流、面单、打印任务入口已有 | 加强失败重试、轨迹、回写和重复发货保护 | `shipment_confirm`、`print_jobs_audit` |
| 售后 | 售后接口和测试已有多条路径 | 收敛退款/退货 P0 路径并联动库存财务 | `after_sales*` 测试 |
| 财务/报表 | 财务管理、利润和报表入口已有 | 对齐账单、退款、运费、成本和利润差异 | `finance_reconcile`、`reports_exports` |
| 前端 P0 | route registry、菜单一致性和占位治理已建立 | 把 P0 E2E 从入口覆盖升级为业务闭环 | `pnpm run accept:p0`、`test:e2e:release-gate` |
| 后端架构 | API/Worker 大文件已明显拆分 | 继续拆 `router.rs` 和大 handler，推进 service/repo 分层 | `app_modularization`、`worker_modularization` |
| 运维发布 | repo/openapi/release gate 基础存在 | 引入真实数据 gate、失败报告和观测断言 | `release:gate:full` |

## 九、推荐并行开发策略

后续一次迭代最多拆成 5 个并行子任务，必须保证写集不冲突，并由主线统一集成和验证。

| 并行线 | 适合任务 | 不应同时修改 |
| --- | --- | --- |
| 线 A：Release Gate | `D1`、`D2`、`D10` | 与业务 handler 大改混在同一文件 |
| 线 B：订单主链路 | `D4` | 库存/WMS 细节实现 |
| 线 C：库存/WMS | `D5` | 订单领域状态定义 |
| 线 D：平台/商品 | `D6`、`D7` | 售后和财务核心文件 |
| 线 E：售后/财务 | `D8`、`D9` | 平台适配器和路由大拆分 |

合并顺序建议：先合 Release Gate 数据与门禁，再合订单动作矩阵，然后合库存/WMS，最后合平台、商品、售后和财务。任何阶段如果门禁不稳定，应先修门禁和数据夹具，不继续扩大功能面。

## 十、风险与治理

| 风险 | 表现 | 治理方式 |
| --- | --- | --- |
| 文档再次膨胀 | 每轮开发新增临时总结、任务卡和快照 | 只更新 6 个核心文档，临时结论放 Issue、PR、提交信息或临时目录 |
| 大文件回潮 | 新功能继续写进 `lib.rs`、`router.rs` 或单个 handler | 模块化测试守护行数，新增功能必须先选定 bounded context |
| Gate 失真 | 命令只检查静态文件，不验证真实业务状态 | 每个 P0 节点补数据库断言、API 断言和 E2E 断言 |
| 业务状态分叉 | 前端展示、OpenAPI、领域枚举、数据库状态不一致 | 状态字典收敛到 domain/OpenAPI，并用测试覆盖转换 |
| 平台差异外泄 | 页面或业务服务直接处理平台原始字段 | 差异只留在 adapter 和映射层，内部使用统一模型 |
| 越权与审计缺失 | 高风险动作只靠前端隐藏或无审计日志 | 后端权限强校验，审计记录动作、操作者、原因和关联对象 |
| 试点不可运维 | 同步失败、发货失败、对账差异无法定位 | 失败队列、trace id、metrics、告警和人工回放必须进入 P0 验收 |
