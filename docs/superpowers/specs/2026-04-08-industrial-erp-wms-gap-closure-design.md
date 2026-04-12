# 工业级电商 ERP/WMS 缺口收口设计

## 1. 结论

当前仓库已经具备 SaaS 底座、OMS/WMS 主链、库存、采购、售后、财务、营销、打印、报表和平台同步基础，不再是“缺模块”阶段，而是“缺工业级深度”阶段。

建议路线：

1. 不换主栈，继续沿用 `Rust + Postgres + NATS + React + OpenAPI`。
2. 先补“占位/演示域”和“生产化治理域”，再补 WMS 现场深度与计划补货。
3. 技术选型按“最好用、体验最好、适合当前仓库演进”确定为：
   - 核心事务：继续放在当前 Rust 域模型中
   - 流程编排与审批：`Camunda 8`
   - 专题统计与驾驶舱：`ClickHouse`
   - 企业级 SSO/IdP：`Keycloak`

## 2. 当前状态

已具备：

- 多租户、RBAC、数据权限、审计日志、特性开关、DLQ、基础可观测。
- 订单下载/新建/审核/复核/提交，WMS 入库/出库/波次/拣货/打包/盘点/调拨。
- 库存台账、批次、序列号、商品/SKU/BOM、采购、售后、财务、营销、打印、报表。
- 平台订单下载、商品解析/组合装匹配、库存/售后同步设置与触发接口。

明确仍是占位或演示态：

- `apps/erp-web/src/pages/planning/*`
- `apps/erp-web/src/pages/stats/*`
- `apps/erp-web/src/pages/system/SystemWorkflowsPage.tsx`
- `apps/erp-web/src/pages/system/SystemCustomFieldsPage.tsx`
- `apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- `apps/erp-web/src/pages/system/SystemAuditReportsPage.tsx`

仍缺生产级联调闭环：

- `docs/reports/archive/platform_integration_report.md` 已明确真实平台凭证、严格验签、字段逐平台核对和真实订单 E2E 仍未完成。

## 3. 市场能力线与目标

对齐官方产品能力线后，工业级目标不是“再补几个 CRUD 页面”，而是补足以下能力：

- WMS 对齐 Manhattan / Oracle / SAP EWM 的能力线：统一履约、labor、slotting、补货、cross-dock、dock-yard、自动化协同、RF/PDA。
- Planning 对齐 SAP IBP 能力线：预测、服务水平、安全库存、EOQ、多级库存优化、采购/调拨建议、促销备货。
- 流程治理对齐 Camunda：BPMN、DMN、Human Task、Forms、跨系统编排。
- 统计驾驶舱对齐 ClickHouse：实时分析、经营与履约专题指标。
- 身份治理对齐 Keycloak：SSO、Identity Brokering、LDAP/AD。

## 4. 按分类的缺口与补齐方案

### 4.1 渠道接入与外部同步

缺口：
- 真实 OAuth/Webhook/验签/字段映射联调闭环
- 库存与售后回传补偿、回放、SLA

方案：
- 保留 `crates/adapter-*`
- 把集成中心做成真实后端能力，不再依赖页面演示

优先级：`P0`

### 4.2 OMS 与订单协同

缺口：
- 拆单、合单、补发、重发、结算、对账、异常 SLA
- 风控解释、自动派单、升级、回退、超时催办

方案：
- 订单工作台拆成审单台、异常池、结算/对账台
- 强化状态轨迹与动作权限

优先级：`P0`

### 4.3 WMS 现场执行

缺口：
- RF/PDA、任务穿插、labor、slotting、补货、dock-yard、设备协同
- cartonization、站点化包装台、主管调度台

方案：
- 保留 WMS 领域模型
- 增加“现场执行层”，不再继续堆后台列表页

优先级：`P0-P1`

### 4.4 库存与主数据治理

缺口：
- ATP、FEFO、多包装层级、货主库存、库存对账中心

方案：
- 在 `inventory/products/skus` 域中增加承诺库存、效期策略、包装规格与对账中心

优先级：`P1`

### 4.5 计划补货与采购调拨

缺口：
- 销售预测、补货规则、采购计划、调拨建议、促销备货、执行看板全部仍是占位页

方案：
- 新建 `planning` 独立域
- 形成“预测 -> 库存目标 -> 采购/调拨建议 -> 执行 -> 反馈”闭环

优先级：`P0-P1`

### 4.6 售后、物流与利润经营

缺口：
- 换货/补发主流程、逆向处置矩阵、物流索赔/拒收/拦截、订单级利润口径

方案：
- 打通售后、物流、财务三域
- 先落订单级利润，再做 SKU/渠道/仓库切片

优先级：`P1`

### 4.7 系统扩展与流程治理

缺口：
- 工作流、自定义字段、集成中心、审计报表仍是 `localStorage` 演示

方案：
- 交易状态机继续留在 Rust
- 审批、人机协同、跨系统编排交给 `Camunda 8`
- 元数据引擎承接自定义字段

优先级：`P0`

### 4.8 统计、驾驶舱与运维治理

缺口：
- `stats` 仍是 placeholder
- 缺统一指标层、业务 SLA 告警、企业 SSO

方案：
- 交易查询仍走 Postgres
- 专题分析接入 `ClickHouse`
- 身份治理接入 `Keycloak`

优先级：`P0`

## 5. 分阶段顺序

### 阶段 A：基础收口

- 集成中心后端化
- 工作流/自定义字段/审计报表后端化
- 平台接入生产化
- Keycloak 接入

### 阶段 B：OMS/WMS 工业化

- 订单异常 SLA、拆合单、结算/对账
- PDA/RF、labor、slotting、补货、dock-yard、交接

### 阶段 C：计划补货与利润经营

- 预测、补货、采购/调拨建议、促销备货
- 订单级利润、成本归因

### 阶段 D：数据产品与智能运营

- ClickHouse 指标仓
- `stats` 专题页
- 经营、履约、库存、采购、售后驾驶舱

## 6. 仓库内优先改造范围

- 平台接入：`crates/erp-api/src/lib.rs`、`crates/erp-channel/src/main.rs`、`crates/erp-worker/src/main.rs`、`crates/adapter-*`
- OMS/WMS：`crates/erp-api/src/orders.rs`、`crates/erp-repo/src/orders/*`、`crates/erp-repo/src/wms/*`、`apps/erp-web/src/pages/orders/*`、`apps/erp-web/src/pages/wms/*`
- Planning：`apps/erp-web/src/pages/planning/*`、新增 `crates/erp-api/src/planning.rs`、新增 `crates/erp-repo/src/planning/*`
- 系统扩展：四个 `System*Page.tsx` + 新增 `crates/erp-api/src/system_extensions.rs`
- 统计：`apps/erp-web/src/pages/stats/*`、`apps/erp-web/src/api/stats.ts`、新增 `crates/erp-analytics/*`

## 7. 参考依据

仓库内：

- `docs/list.md`
- `docs/reports/archive/feature_gap_matrix.md`
- `docs/reports/archive/platform_integration_report.md`
- `docs/spec/order_workflow_api_updates_2026-04-07.md`
- `docs/spec/wms_workflow_api_updates_2026-04-07.md`
- `docs/spec/wms_inbound_inventory_workflow_api_updates_2026-04-08.md`
- `docs/spec/platform_sync_and_mapping_api_updates_2026-04-08.md`

外部官方：

- Manhattan WMS: https://www.manh.com/products/warehouse-management
- Oracle WMS: https://www.oracle.com/scm/logistics/warehouse-management/
- SAP EWM: https://www.sap.com/products/scm/extended-warehouse-management.html
- SAP IBP Demand Planning: https://www.sap.com/westbalkans/products/scm/integrated-business-planning/features/demand-planning.html
- Camunda Platform: https://camunda.com/platform/
- ClickHouse: https://clickhouse.com/
- Keycloak: https://www.keycloak.org/
