# 后续开发方向

> 更新日期：2026-05-06  
> 当前版本：`0.1.7`  
> 本文回答“接下来往哪里开发、先做什么、做到什么程度才算完成”。

## 当前阶段判断

项目已经具备较宽的工程骨架：Rust 多服务、React 管理端、OpenAPI、数据库迁移、平台适配器、订单与 WMS 主线、系统治理页面和部分测试门禁都已经存在。当前主要问题不是“没有功能入口”，而是“入口多、闭环深度不一致、状态口径和验收口径需要收敛”。

后续开发应从“铺页面和写任务卡”转向“围绕真实业务闭环做深、做稳、做可验收”。

## 北极星目标

用最短路径交付首批客户可试点的电商 ERP：订单从平台进入系统后，能经过审单、库存占用、仓储出库、物流发货、售后逆向、财务对账和经营报表，且全过程可追溯、可权限控制、可失败补偿。

## 阶段路线

| 阶段 | 目标 | 重点交付 | 完成标准 |
| --- | --- | --- | --- |
| 阶段 0：文档与口径收敛 | 文档入口稳定，功能范围统一 | 精简文档、统一产品范围、统一路线图、修复旧引用 | 新增需求能明确落到 `docs/PRODUCT_SCOPE.md` 或 `docs/ROADMAP.md` |
| 阶段 1：P0 试点闭环 | 打通首个可收费试点版本 | 店铺接入、订单审核、库存占用、出库发货、打印、售后最小闭环、基础对账 | 一条真实或仿真平台订单可端到端跑通，并通过门禁 |
| 阶段 2：稳定经营版本 | 从“可跑通”提升为“可日常经营” | 多仓、多店铺、异常中心、采购入库、补货建议、售后财务联动、经营报表 | 核心岗位能在工作台内完成日常批量作业和异常处理 |
| 阶段 3：工业化平台版本 | 支撑规模化客户与长期运维 | 多商户、多组织、审批中心、质量追溯、开放集成、SLA、自动化补偿 | 可支撑多租户持续上线、审计、告警、容量和安全要求 |

## 近期开发优先级

### 1. 订单主链路做深

- 统一销售订单、平台订单、审单、复核、异常、拆合单、分仓和履约状态机。
- 明确每个订单动作的权限、前置条件、后置影响和审计日志。
- 把订单工作台作为核心样板页，沉淀可复用的工作台布局、筛选、批量操作和明细联动模式。

### 2. 库存与仓储闭环做实

- 以库存台账为事实来源，明确可用、预占、锁定、在途、冻结、差异等状态。
- 补齐入库、上架、波次、拣货、复核、打包、出库、盘点、调拨的动作边界。
- 强化条码、批次、序列号、效期和库位策略，避免 WMS 只停留在查询页面。

### 3. 平台接入生产化

- 完成平台验签、授权续期、字段映射、订单补偿、失败重试和回放控制台。
- 按订单、商品、售后、发货分别定义同步任务 SLA 和失败归因。
- 优先让一个平台成为可验收样板，再横向复制到其他平台。

### 4. 售后、财务和报表联动

- 售后单必须能关联订单、商品、库存、物流、退款和责任归因。
- 基础财务先聚焦平台账单、订单金额、退款、运费、成本和结算差异。
- 报表优先回答经营问题：卖了多少、发了多少、退了多少、赚了多少、卡在哪里。

### 5. 工程门禁常态化

- 所有核心功能必须同步 OpenAPI、迁移、权限、审计、测试和 E2E 证据。
- 生成报告和临时验收记录不再长期堆在 `docs/`，避免文档再次膨胀。
- 发布前必须执行 `docs/OPERATIONS.md` 中的门禁清单。

## 模块推进顺序

1. 订单工作台与状态机：先统一动作和状态，再扩展页面细节。
2. 库存台账与出库履约：先保证库存影响正确，再做复杂仓内效率。
3. 平台同步与失败补偿：先保证数据进得来、补得上、查得清。
4. 商品主数据治理：先解决 SKU 映射、条码、单位、套装和渠道差异。
5. 售后最小闭环：先支持退款或退货，再扩展换货、补发、举证和逆向质检。
6. 财务对账与报表：先做经营对账，再逐步靠近管理会计。
7. 多组织、多商户、审批中心：在 P0 主链路稳定后再系统性展开。

## 验收口径

- 功能完成：有页面、API、数据库迁移、权限、审计、错误处理和测试证据。
- 链路完成：至少一条端到端业务流程可运行，不只是一组孤立页面。
- 可上线：通过构建、后端测试、前端测试、OpenAPI 校验、E2E、Release Gate 和安全检查。
- 可维护：文档入口更新，旧结论不再散落在临时任务卡中。

## 风险与治理

- 菜单过宽风险：新页面必须服务明确闭环，否则推迟。
- 文档膨胀风险：不再新增一次性计划文档，结论回写到权威文档。
- 状态割裂风险：订单、库存、仓储、物流、售后、财务必须共享状态字典和审计规则。
- 平台差异风险：每个平台先沉淀适配器契约，再做页面适配。
- 验收缺口风险：任何功能若无法自动化验证，只能进入试验状态，不能标记为完成。

## 代码现状分析（2026-05-01）

> 本节基于当前工作树代码扫描结果，不以历史文档为依据。

### 规模与覆盖

- 后端 Rust：`crates/` 下约 `278` 个 `.rs` 文件，核心服务包含 `erp-api`、`erp-channel`、`erp-dispatcher`、`erp-worker`、`erp-repo`、`adapter-*`。
- 前端 Web：`apps/erp-web/src` 下约 `691` 个 `.ts/.tsx` 文件，`apps/erp-web/src/pages` 下业务页面覆盖订单、WMS、库存、商品、售后、财务、采购、客户、营销、系统等域。
- API 契约：`openapi/erp_api_v1.yaml` 当前约 `508` 条路径，权限块约 `701` 处，接口面已经很宽。
- 后端路由：从 `crates/erp-api/src/router.rs`、`crates/erp-api/src/orders.rs`、`crates/erp-api/src/planning.rs`、`crates/erp-api/src/sales_orders.rs` 提取到约 `576` 条 Axum route，其中 WMS、OMS、订单、财务、售后、物流、平台店铺是主要接口域。
- 测试资产：`crates/erp-api/tests` 有 `46` 个 Rust 集成测试文件，`apps/erp-web/e2e` 有 `76` 个 Playwright 规格文件，说明项目已有验收基础，但测试分布不均。

### 主要结构风险

- `crates/erp-api/src/lib.rs` 已降至约 `9,082` 行，低于 `10,000` 行目标；本轮继续迁出 WMS 操作、商品、SKU、映射、物流、财务、售后、仓储基础、店铺同步、CRM、采购、库存/打印/报表等 handler，后续新增能力不得回填到入口大文件。
- `crates/erp-api/src/router.rs` 约 `1,823` 行，集中挂载约 `464` 条路由；需要按业务域拆成可测试的路由模块。
- `crates/erp-api/src/orders.rs` 约 `839` 行，订单结算、异常、拆合、回下载、标签、审核配置、履约、详情、审核流和共享 helper 已迁入子模块；后续重点转向 `lib.rs` 与 `router.rs`。
- `crates/erp-worker/src/main.rs` 约 `604` 行，事件处理器、运行时 helper 与内联测试已拆出；后续继续收敛启动编排和测试辅助模块。
- `apps/erp-web/src/app/router.tsx` 当前只显式挂载约 `19` 个路由项，`apps/erp-web/src/app/layout/menu.tsx` 只有 `10` 个菜单叶子，但页面目录和 API 模块远多于当前可达入口；下一阶段必须先解决“页面存在但用户路径不可达”的问题。
- `apps/erp-web/src/pages/Placeholders.tsx` 仍有 `8` 个占位导出，旧占位页不能继续作为 P0 能力依据。
- 迁移目录存在多个同编号前缀，例如 `0060`、`0073`、`0074`、`0079`、`0095`；如果迁移工具要求版本唯一，需要在开发前做迁移编号门禁。

### 当前优势

- 订单、WMS、售后、平台接入、权限、系统治理均已有接口、页面和测试痕迹，不是从零开发。
- OpenAPI、权限标记、E2E、后端集成测试和发布脚本已经存在，适合直接转向“闭环验收驱动”的开发方式。
- 订单工作台、共享主表入口、工作台布局和路由诊断已有可复用模式，应作为后续页面工业化模板。

## 详细开发计划（P0 试点闭环）

> **执行方式：** 每个任务先补失败测试或验收用例，再实现最小变更，通过后再进入下一任务。每个任务都必须同步 OpenAPI、权限、审计、错误处理和文档口径。  
> **推荐节奏：** 1 个任务对应 1 个短分支或 PR；任务完成后至少运行本任务列出的命令。  
> **总体目标：** 让一条平台订单可从店铺接入进入系统，经订单审核、库存占用、WMS 出库、面单打印、发货回写、售后处理、财务对账和经营报表完成端到端闭环。

### Task 0：建立 P0 基线与迁移门禁

**目标：** 在继续开发前锁定当前可运行基线，避免迁移编号、OpenAPI 或文档路径问题继续扩散。

**Files:**
- Modify: `scripts/verify_repo_baseline.mjs`
- Modify: `scripts/openapi_bundle.mjs`
- Modify: `package.json`
- Inspect: `migrations/`
- Inspect: `openapi/erp_api_v1.yaml`
- Inspect: `docs/OPERATIONS.md`

**Steps:**
- [ ] 写迁移编号检查：扫描 `migrations/*.sql`，若工具链要求唯一版本号，则对重复前缀给出清晰失败信息。
- [ ] 写 OpenAPI 路由覆盖检查：抽取 `crates/erp-api/src/router.rs` 和子 router 中的 route 字符串，对比 `openapi/erp_api_v1.yaml` 的 P0 路径。
- [ ] 写文档旧路径检查：禁止新增已删除的历史文档目录引用，所有长期结论必须回写到当前 6 份核心文档。
- [ ] 将以上检查挂入 `pnpm run repo:verify`，失败时输出中文诊断。

**Tests:**
- Run: `pnpm run repo:verify`
- Run: `pnpm run check:utf8`
- Run: `git diff --check`

**Done When:**
- 重复迁移编号、OpenAPI 缺失、旧文档路径引用都能在开发期被自动拦截。

### Task 1：前端路由与菜单可达性收敛

**目标：** 让 P0 页面有统一路由注册、统一菜单入口、统一权限守卫，解决“页面存在但路由不可达”的问题。

**Files:**
- Modify: `apps/erp-web/src/app/router.tsx`
- Modify: `apps/erp-web/src/app/layout/menu.tsx`
- Modify: `apps/erp-web/src/app/layout/routeMeta.ts`
- Modify: `apps/erp-web/src/app/navigation/routeAliases.ts`
- Create: `apps/erp-web/src/app/routeRegistry.tsx`
- Test: `apps/erp-web/src/app/routeRegistry.test.tsx`
- Test: `apps/erp-web/e2e/workbench-menu-navigation.spec.ts`

**Steps:**
- [ ] 写失败测试：校验菜单叶子项必须能在 router 中找到对应路径。
- [ ] 建立 `routeRegistry.tsx`，把 path、title、permission、lazy loader、菜单分组和是否 P0 统一登记。
- [ ] 用 route registry 生成 `router.tsx` children，避免手工维护路由数组。
- [ ] 用同一 registry 生成 `menu.tsx` 菜单，避免菜单和路由分叉。
- [ ] 把订单、渠道、商品、库存、WMS、售后、财务、报表、系统治理 P0 页面接入路由。
- [ ] 保留 `routeAliases.ts` 只做历史路径跳转，不再承载新页面规划。

**Tests:**
- Run: `pnpm --dir apps/erp-web exec vitest --run src/app/routeRegistry.test.tsx src/app/layout/menu.test.ts`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts --project=chromium`

**Done When:**
- P0 菜单叶子全部可导航，权限不足走 `PermissionGuard`，未知路由走 `NotFoundPage`。

### Task 2：清理 P0 占位页并定义页面成熟度

**目标：** P0 范围内不允许继续用占位页冒充功能页，所有页面必须有真实 API、加载态、空态和错误态。

**Files:**
- Modify: `apps/erp-web/src/pages/Placeholders.tsx`
- Modify: `apps/erp-web/src/pages/orders/*`
- Modify: `apps/erp-web/src/pages/wms/*`
- Modify: `apps/erp-web/src/pages/inventory/*`
- Modify: `apps/erp-web/src/pages/products/*`
- Modify: `apps/erp-web/src/pages/afterSales/*`
- Modify: `apps/erp-web/src/pages/finance/*`
- Test: `apps/erp-web/src/pages/*/*.test.tsx`

**Steps:**
- [ ] 写扫描测试：P0 route 不得引用 `PlaceholderPage`。
- [ ] 为每个 P0 页面补齐 API 查询、空状态、错误提示、权限提示和 `data-testid`。
- [ ] 将非 P0 占位页从菜单隐藏，保留为后续 P1/P2 路由草稿时必须标注“未上线”。
- [ ] 建立页面成熟度枚举：`draft`、`wired`、`p0-ready`、`deprecated`。

**Tests:**
- Run: `pnpm --dir apps/erp-web exec vitest --run src/app/routeRegistry.test.tsx`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/release-gate-main-chain.spec.ts --project=chromium`

**Done When:**
- P0 页面无占位组件，页面可达性、接口加载和异常展示可自动验证。

### Task 3：订单状态机与动作守卫统一

**目标：** 统一平台订单、销售订单、审单、复核、拆合单、分仓、出库和发货状态，所有动作都有前置条件、权限、审计和幂等规则。

**Files:**
- Modify: `crates/erp-domain/src/lib.rs`
- Modify: `crates/erp-types/src/lib.rs`
- Modify: `crates/erp-api/src/orders.rs`
- Modify: `crates/erp-api/src/orders/*`
- Modify: `crates/erp-repo/src/orders/store.rs`
- Modify: `crates/erp-worker/src/main.rs`
- Modify: `openapi/erp_api_v1.yaml`
- Test: `crates/erp-api/tests/oms_orders.rs`
- Test: `crates/erp-api/tests/order_chain_permissions.rs`
- Test: `crates/erp-api/tests/order_exception_sla_integration.rs`

**Steps:**
- [ ] 写失败测试：非法状态动作返回稳定错误码，不产生库存、发货或财务副作用。
- [ ] 把订单状态、动作和错误码移到领域类型，前后端只消费统一枚举。
- [ ] 为审核通过、审核驳回、异常挂起、释放、拆单、合单、分仓、提交出库、取消建立动作守卫。
- [ ] 为每个动作写入 `order_state_logs` 和操作审计。
- [ ] 将 OpenAPI 补齐动作请求体、响应体、错误码和权限标记。

**Tests:**
- Run: `cargo test -p erp-api --test oms_orders -- --nocapture`
- Run: `cargo test -p erp-api --test order_chain_permissions -- --nocapture`
- Run: `cargo test -p erp-api --test order_exception_sla_integration -- --nocapture`

**Done When:**
- 任意订单动作都能说明“谁在什么状态下做了什么、影响了什么、失败如何处理”。

### Task 4：订单工作台 P0 完整闭环

**目标：** 订单查询、订单审核、订单复核三个页面复用统一主表入口，并完成批量动作、明细联动、保存视图和状态恢复。

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/OrderQueryPage.tsx`
- Modify: `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx`
- Modify: `apps/erp-web/src/api/orders.ts`
- Test: `apps/erp-web/src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`
- Test: `apps/erp-web/e2e/order-workbench-pages.spec.ts`

**Steps:**
- [ ] 写失败测试：三个工作台必须都通过 `OrderWorkbenchMainGridEntry` 渲染主表。
- [ ] 补齐列表筛选、保存视图、选中行、焦点行、右侧详情、底部子表状态恢复。
- [ ] 批量审核、批量挂起、批量释放、批量提交出库必须展示成功和失败明细。
- [ ] 前端动作按钮根据后端动作可用性而不是硬编码状态判断。
- [ ] E2E 覆盖订单进入、审核、异常、复核、提交出库的最短链路。

**Tests:**
- Run: `pnpm --dir apps/erp-web exec vitest --run src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/order-workbench-pages.spec.ts e2e/order-processing-flow.spec.ts --project=chromium`

**Done When:**
- 运营可在订单工作台内完成 P0 审单与异常处理，不需要跳出多个孤立页面。

### Task 5：库存台账与 ATP/FEFO 守护

**目标：** 确保订单审核、分仓、出库、售后、采购入库都通过库存台账产生可追溯影响。

**Files:**
- Modify: `crates/erp-repo/src/inventory.rs`
- Modify: `crates/erp-repo/src/wms/store.rs`
- Modify: `crates/erp-api/src/router.rs`
- Modify: `apps/erp-web/src/api/inventory.ts`
- Modify: `apps/erp-web/src/pages/inventory/*`
- Modify: `openapi/erp_api_v1.yaml`
- Test: `crates/erp-api/tests/inventory_atp_fefo_integration.rs`
- Test: `crates/erp-api/tests/wms_inventory_workflow_permissions.rs`

**Steps:**
- [ ] 写失败测试：库存余额不能绕过台账直接修改。
- [ ] 明确可用、预占、锁定、冻结、在途、残次、差异状态的计算规则。
- [ ] 订单审核占用库存，取消和驳回释放库存，出库扣减库存。
- [ ] 批次、效期、序列号场景优先走 FEFO 和批次策略。
- [ ] 前端库存页展示余额、台账、预占来源和异常差异。

**Tests:**
- Run: `cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture`
- Run: `cargo test -p erp-api --test wms_inventory_workflow_permissions -- --nocapture`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/inventory.spec.ts e2e/inventory-governance-packaging.spec.ts --project=chromium`

**Done When:**
- 任何库存变化都能回溯到业务单据、动作、操作者和台账记录。

### Task 6：WMS 出库与波次执行闭环

**目标：** 订单提交出库后，WMS 能完成波次、拣货、复核、打包、称重、面单、发货和异常处理。

**Files:**
- Modify: `crates/erp-repo/src/wms/store.rs`
- Modify: `crates/erp-api/src/router.rs`
- Modify: `crates/erp-worker/src/main.rs`
- Modify: `apps/erp-web/src/api/wmsExecution.ts`
- Modify: `apps/erp-web/src/pages/wms/*`
- Modify: `apps/erp-web/src/pages/shipments/*`
- Test: `crates/erp-api/tests/wave_ship_integration.rs`
- Test: `crates/erp-api/tests/wms_outbound_serials.rs`
- Test: `apps/erp-web/e2e/wms-wave-main-chain.spec.ts`

**Steps:**
- [ ] 写失败测试：未复核、未称重或物流缺失时不能发货。
- [ ] 统一出库单动作：打印、审核、拣货、复核、打包、称重、发货、取消。
- [ ] 处理波次拆分、部分缺货、复核差异、序列号扫描和包裹追加。
- [ ] 发货成功后写库存扣减、物流状态、订单状态和操作审计。
- [ ] 前端 WMS 作业页支持扫码输入、键盘流、异常提示和作业结果反馈。

**Tests:**
- Run: `cargo test -p erp-api --test wave_ship_integration -- --nocapture`
- Run: `cargo test -p erp-api --test wms_outbound_serials -- --nocapture`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-flow.spec.ts --project=chromium`

**Done When:**
- 仓库作业员能按 WMS 页面完成 P0 出库，订单状态、库存、物流状态同步更新。

### Task 7：平台接入样板与补偿机制

**目标：** 选定一个平台作为样板，完成授权、验签、订单同步、字段映射、补偿、失败回放和状态回写。

**建议样板：** 优先选择拼多多，因为代码中已有 PDD-only 运行开关；若业务优先抖音，则以抖音适配器替换样板平台，但任务结构不变。

**Files:**
- Modify: `crates/adapter-pdd/src/lib.rs`
- Modify: `crates/adapter-douyin/src/lib.rs`
- Modify: `crates/erp-channel/src/main.rs`
- Modify: `crates/erp-worker/src/main.rs`
- Modify: `crates/erp-repo/src/platform_sync.rs`
- Modify: `apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- Modify: `apps/erp-web/src/pages/channels/*`
- Modify: `openapi/erp_api_v1.yaml`
- Test: `crates/erp-api/tests/platform_oauth_webhook_integration.rs`
- Test: `crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs`

**Steps:**
- [ ] 写失败测试：签名错误、店铺不匹配、重复事件、授权过期都返回稳定错误。
- [ ] 完成样板平台验签和 token 刷新，失败原因写入同步任务。
- [ ] 同步任务按订单、商品、售后、发货拆分对象类型和 SLA。
- [ ] 补偿任务支持按店铺、时间窗、对象类型、失败原因回放。
- [ ] 前端集成页展示授权状态、同步进度、失败原因、重试按钮和回放结果。

**Tests:**
- Run: `cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture`
- Run: `cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/system-integrations.spec.ts e2e/channels-shops-list-visibility.spec.ts --project=chromium`

**Done When:**
- 至少一个平台可完成订单进入、同步补偿、失败修复和发货状态回写的样板闭环。

### Task 8：商品、SKU 与渠道映射治理

**目标：** 让平台订单商品能稳定映射到内部 SKU，并支撑条码、单位、套装、批次和渠道可售规则。

**Files:**
- Modify: `crates/erp-repo/src/products.rs`
- Modify: `crates/erp-api/src/router.rs`
- Modify: `apps/erp-web/src/api/products.ts`
- Modify: `apps/erp-web/src/api/skus.ts`
- Modify: `apps/erp-web/src/api/shopProducts.ts`
- Modify: `apps/erp-web/src/pages/products/*`
- Modify: `apps/erp-web/src/pages/skus/*`
- Modify: `apps/erp-web/src/pages/shopProducts/*`
- Test: `apps/erp-web/e2e/products.spec.ts`
- Test: `apps/erp-web/e2e/shop-product-match-governance.spec.ts`

**Steps:**
- [ ] 写失败测试：未映射、重复映射、禁售 SKU、单位不匹配时订单进入异常池。
- [ ] 建立 SKU 映射状态、映射版本、冲突原因和审批记录。
- [ ] 商品主数据支持条码、单位换算、包装规格、套装、批次策略和渠道可售。
- [ ] 前端商品下载与映射页支持批量确认、冲突修复、变更审计。
- [ ] 订单行保存平台原始商品信息和内部 SKU 映射结果。

**Tests:**
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/products.spec.ts e2e/skus.spec.ts e2e/shop-product-match-governance.spec.ts --project=chromium`
- Run: `cargo test -p erp-api --test skus -- --nocapture`

**Done When:**
- 平台订单行进入系统后能自动或人工映射到可履约 SKU，冲突可追溯、可修复。

### Task 9：售后最小闭环与库存/财务联动

**目标：** 至少打通退款或退货退款路径，并能关联订单、物流、库存、退款和责任归因。

**Files:**
- Modify: `crates/erp-repo/src/after_sales.rs`
- Modify: `crates/erp-api/src/router.rs`
- Modify: `crates/erp-worker/src/main.rs`
- Modify: `apps/erp-web/src/api/afterSales.ts`
- Modify: `apps/erp-web/src/pages/afterSales/*`
- Modify: `apps/erp-web/src/pages/finance/*`
- Test: `crates/erp-api/tests/after_sales_state_machine_guard.rs`
- Test: `crates/erp-api/tests/after_sales_finance_reconcile.rs`
- Test: `apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`

**Steps:**
- [ ] 写失败测试：售后状态非法流转不能影响库存或财务。
- [ ] 售后单支持申请、审核、收货、质检、退款、关闭和异常升级。
- [ ] 退货入库生成库存台账，退款生成财务差异或结算项。
- [ ] 前端售后页展示订单关联、商品明细、物流、退款、质检和操作日志。
- [ ] 平台售后同步与 ERP 售后状态保持映射版本。

**Tests:**
- Run: `cargo test -p erp-api --test after_sales_state_machine_guard -- --nocapture`
- Run: `cargo test -p erp-api --test after_sales_finance_reconcile -- --nocapture`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/after-sales-return-resend-main-chain.spec.ts e2e/after-sales-workflow.spec.ts --project=chromium`

**Done When:**
- 客服可处理一条 P0 售后路径，库存与财务影响可查询、可审计。

### Task 10：基础财务对账与经营报表

**目标：** 让订单金额、退款、运费、成本、结算差异进入基础财务闭环，并形成管理者可读报表。

**Files:**
- Modify: `crates/erp-api/src/finance_management.rs`
- Modify: `crates/erp-repo/src/finance.rs`
- Modify: `crates/erp-api/src/router.rs`
- Modify: `apps/erp-web/src/api/finance.ts`
- Modify: `apps/erp-web/src/api/reports.ts`
- Modify: `apps/erp-web/src/pages/finance/*`
- Modify: `apps/erp-web/src/pages/reports/*`
- Test: `crates/erp-api/tests/finance_reconcile.rs`
- Test: `crates/erp-api/tests/reports_exports.rs`
- Test: `apps/erp-web/e2e/finance-operations.spec.ts`

**Steps:**
- [ ] 写失败测试：结算差异、重复回款、退款未入账必须可识别。
- [ ] 平台账单导入后能与订单、退款、运费、成本匹配。
- [ ] 对账任务产出成功项、差异项、待人工处理项。
- [ ] 报表展示销售额、发货量、退款额、毛利、异常数量、库存风险。
- [ ] 支持导出权限、导出审计和导出失败重试。

**Tests:**
- Run: `cargo test -p erp-api --test finance_reconcile -- --nocapture`
- Run: `cargo test -p erp-api --test reports_exports -- --nocapture`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/finance-operations.spec.ts e2e/release-gate-cross-chain.spec.ts --project=chromium`

**Done When:**
- 管理者能看到 P0 经营结果，财务能定位订单、退款和账单差异。

### Task 11：后端大文件模块化

**目标：** 降低 `erp-api` 和 `erp-worker` 维护风险，不改变外部接口行为。

**Files:**
- Modify: `crates/erp-api/src/lib.rs`
- Modify: `crates/erp-api/src/router.rs`
- Modify: `crates/erp-api/src/orders.rs`
- Modify: `crates/erp-worker/src/main.rs`
- Create: `crates/erp-api/src/routes/*.rs`
- Create: `crates/erp-api/src/handlers/*.rs`
- Create: `crates/erp-worker/src/handlers/*.rs`
- Test: `crates/erp-api/tests/app_modularization.rs`

**Steps:**
- [ ] 写模块化保护测试：核心 public API、OpenAPI 路径和权限标记在拆分前后不变。
- [ ] 先拆无业务逻辑风险的 route builder：按 `orders`、`wms`、`finance`、`after_sales`、`platform`、`system` 分文件。
- [ ] 再拆 handler：每个文件只保留一个业务域的请求/响应、校验和调用 repo。
- [ ] 最后拆 worker 事件处理器：Inbox、订单、库存、发货、售后、财务分别成模块。
- [ ] 禁止在拆分过程中改变 SQL、状态机、错误码和返回结构。

**Tests:**
- Run: `cargo test -p erp-api --test app_modularization -- --nocapture`
- Run: `cargo test --all`
- Run: `pnpm run repo:verify`

**Done When:**
- `crates/erp-api/src/lib.rs` 已降至约 `9,082` 行，低于 `10,000` 行目标；本轮继续迁出 WMS 操作、商品、SKU、映射、物流、财务、售后、仓储基础、店铺同步、CRM、采购、库存/打印/报表等 handler，后续新增能力不得回填到入口大文件。

### Task 12：P0 发布门禁与端到端验收

**目标：** 把 P0 试点闭环固定为自动化门禁，避免“页面可打开但业务不可上线”。

**Files:**
- Modify: `package.json`
- Modify: `scripts/verify_repo_baseline.mjs`
- Modify: `apps/erp-web/e2e/release-gate-main-chain.spec.ts`
- Modify: `apps/erp-web/e2e/release-gate-cross-chain.spec.ts`
- Modify: `docs/OPERATIONS.md`

**Steps:**
- [ ] 定义 P0 release gate 场景：登录、店铺授权、订单进入、商品映射、审核、库存占用、出库、打印、发货、售后、对账、报表。
- [ ] 对每个场景声明测试数据、前置状态、执行动作、期望状态和清理方式。
- [ ] 将 P0 E2E 加入 `pnpm run accept:all` 或新增 `pnpm run accept:p0`。
- [ ] Release Gate 输出失败摘要：失败页面、失败动作、接口错误、trace id、截图和视频路径。
- [ ] 更新 `docs/OPERATIONS.md` 的发布门禁说明。

**Tests:**
- Run: `pnpm run accept:backend`
- Run: `pnpm run accept:web`
- Run: `pnpm run repo:verify`
- Run: `pnpm --dir apps/erp-web exec playwright test e2e/release-gate-main-chain.spec.ts e2e/release-gate-cross-chain.spec.ts --project=chromium`

**Done When:**
- P0 是否可试点不再依赖人工口头判断，而由 release gate 给出明确通过或失败原因。

## 计划执行顺序

1. 先做 `Task 0` 到 `Task 2`：解决门禁、路由、占位页，保证后续功能有稳定入口。
2. 再做 `Task 3` 到 `Task 6`：订单、库存、WMS 是 P0 试点版核心闭环，必须串行推进并保持测试红绿闭环。
3. 然后做 `Task 7` 到 `Task 10`：平台接入、商品映射、售后、财务和报表补齐经营闭环。
4. `Task 11` 模块化贯穿执行：每完成一个业务域，就顺手迁移该域 route/handler，避免最后一次性大拆分。
   - 后端工业级约束：除入口 `lib.rs` 外，API 单模块不得超过 `5,000` 行；超过阈值必须按 bounded context 建子目录聚合模块，并由 `app_modularization` 测试守护。
5. 最后做 `Task 12`：把已打通的链路固化成发布门禁。

## 优先交付切片

如果只能先交付一个最小试点版本，按以下切片执行：

1. `Task 0`：门禁和迁移检查。
2. `Task 1`：P0 菜单与路由可达。
3. `Task 3`：订单状态机与动作守卫。
4. `Task 5`：库存占用和台账。
5. `Task 6`：WMS 出库发货。
6. `Task 7`：一个平台样板同步。
7. `Task 12`：主链路 release gate。

该切片完成后，项目即可进入“单平台、单租户、多店铺、订单到发货”的试点验收；售后、财务和报表随后作为第二切片补齐。

## 执行进展（2026-05-01）

本轮已先落地低风险、可自动化验证的 P0 基础任务，避免在状态机、库存和 WMS 大文件上做未充分验证的大改。

| 任务 | 当前状态 | 已落地内容 | 后续动作 |
| --- | --- | --- | --- |
| Task 0：P0 基线与门禁 | 已部分完成 | `repo:verify` 增加历史文档路径、迁移重复基线、Release Gate 阶段、前端 route registry 可达性检查 | 后续再补 OpenAPI route 差异的更细粒度报告 |
| Task 1：前端路由与菜单 | 已部分完成 | 新增 `routeRegistry`，把 P0 试点入口接入菜单与 router，Release Gate 访问路径可达 | 后续把 registry 反向驱动 router/menu，减少手写重复 |
| Task 2：P0 占位页治理 | 已部分完成 | P0 route registry 增加成熟度，测试禁止 P0 使用 `PlaceholderPage` | 后续将更多页面纳入成熟度扫描 |
| Task 3：订单状态机 | 已部分完成 | 新增订单审核重开领域守卫，阻止已发货、已签收、已完成、已取消订单执行 rollback/requeue；补充领域层红绿测试与接口级回归测试 | 下一轮继续补审核、提交、取消、拆合单的统一动作矩阵与错误码 |
| Task 5/6：库存与 WMS | 未开始业务实现 | 本轮只让库存、WMS、发货入口进入 P0 router/menu | 下一轮从未复核/物流缺失不得发货测试开始 |
| Task 7/10/12：平台、财务与 Gate | 已部分完成 | 新增 `accept:p0`，Release Gate 静态覆盖登录、订单、库存、WMS、发货、售后、财务、报表 | 后续补真实端到端数据准备与回收 |
| Task 11：后端大文件模块化 | 已部分完成 | 新增模块化守护测试；`erp-api/src/lib.rs` 已从约 `50,744` 行继续降至约 `9,082` 行；新增迁出 `wms_operation_handlers`、`product_handlers`、`sku_handlers`、`mapping_handlers`、`logistics_handlers`、`finance_handlers`、`after_sales_handlers`、`warehouse_master_handlers`、`shop_sync_handlers`、`crm_handlers`、`procurement_handlers`、`inventory_print_report_handlers`；本轮进一步按 bounded context 细分 `inventory_print_report_handlers/{inventory_core,stock_count,stock_movement,wms_inbound,printing,reports_exports}` 与 `wms_operation_handlers/{outbound_core,wave,workflow,fulfillment,views}`，并新增非 root API 模块 `5,000` 行上限守护 | 下一轮拆 `router.rs` 路由挂载，并继续细分 `logistics_handlers`、`after_sales_handlers`、`shop_sync_handlers`、`warehouse_master_handlers`，逐步推进 handler/router/service/repo 分层 |

本轮新增推荐命令：

```bash
pnpm run accept:p0
```

该命令当前执行：仓库基线检查、OpenAPI gate、前端 typecheck、路由/菜单注册表单元测试。
