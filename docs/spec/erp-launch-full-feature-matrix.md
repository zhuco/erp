# 计划上线完整功能实现表（工业级电商 ERP）

> 目的：作为项目上线范围、能力完整度和阶段排期的统一母表。
>
> 状态说明：
> - `已具备`：主链路已经有代码、接口与页面支撑，具备上线基础。
> - `部分具备`：已有路由 / 页面 / API / 测试中的部分证据，但业务闭环或生产细节不足。
> - `缺失`：缺少稳定实现，或只有概念文档没有形成可运行能力。
>
> 级别说明：
> - `P0`：首个可收费上线版本必须具备。
> - `P1`：上线后 1 到 2 个版本必须补齐。
> - `P2`：工业化增强或平台化阶段补齐。

## 1. 平台与租户基础

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 平台基础 | 多租户隔离 | P0 | 部分具备 | JWT / RBAC / `TenantGuard` / `api_rbac_tenant_isolation.rs` | 需补齐跨租户审计、租户级配置版本化和租户级限流 |
| 平台基础 | 多商户运营模型 | P0 | 缺失 | 现有模型以 tenant + shop 为主 | 需新增商户层、商户合同、商户结算与商户级工作台 |
| 平台基础 | 多组织 / 多公司 | P1 | 部分具备 | 文档有边界说明，系统页有权限和流程配置 | 缺少组织树、法人主体、事业部与跨组织单据路由 |
| 平台基础 | 数据权限 | P0 | 部分具备 | `DataScopesPage`、RBAC、权限矩阵文档 | 需形成按组织、仓库、店铺、客户、供应商的细粒度权限 |
| 平台基础 | 用户 / 角色 / 权限目录 | P0 | 已具备 | `UsersPage`、`RolesPage`、`PermissionsPage`、系统 API | 需补审批流和权限变更留痕闭环 |
| 平台基础 | 自定义字段 | P1 | 部分具备 | `SystemCustomFieldsPage`、集成测试 | 需验证在订单、商品、采购、售后等核心单据中的实际落地 |
| 平台基础 | 单据编号规则 | P1 | 部分具备 | `SystemWorkflowsPage`、流程配置文档 | 需补多组织、多单据类型、版本化和回溯能力 |
| 平台基础 | 审批与流程引擎 | P1 | 部分具备 | `system_governance_workflow_integration.rs`、系统工作流页面 | 需从“治理链路存在”提升为业务审批中心 |
| 平台基础 | 审计日志 / 操作日志 | P0 | 部分具备 | `AuditLogsPage`、`OperationLogsPage`、`security_and_audit.md` | 需统一关键业务动作审计口径 |
| 平台基础 | 系统配置中心 | P0 | 部分具备 | `SystemSettingsPage` | 需形成环境、租户、组织三级配置模型 |

## 2. 渠道与店铺接入

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 渠道接入 | 平台适配器框架 | P0 | 已具备 | `adapter-taobao` / `jd` / `pdd` / `douyin` | 需补平台差异化验签、字段映射和版本升级机制 |
| 渠道接入 | OAuth 授权与店铺接入 | P0 | 部分具备 | `platform_oauth_webhook_integration.rs`、`SystemIntegrationsPage` | 需补授权到期预警、自动续期、失败回滚和接入审批 |
| 渠道接入 | Webhook 接收与去重 | P0 | 已具备 | `erp-channel`、Inbox/Outbox、幂等文档 | 需补生产级告警和回放控制台 |
| 渠道接入 | Polling 补偿与对账 | P0 | 部分具备 | 架构文档、同步任务、mapping flow 集成测试 | 需形成平台差异化补偿策略与数据补漏报告 |
| 渠道接入 | 店铺同步任务中心 | P0 | 部分具备 | `SyncJobsPage`、`SyncJobRunsPage`、系统同步页面 | 需按对象区分订单 / 商品 / 售后 / 发货的 SLA 和失败归因 |
| 渠道接入 | 店铺健康总览 | P0 | 部分具备 | `SystemIntegrationsPage` 店铺总览 | 需补租户级 / 商户级大盘和自动化告警 |
| 渠道接入 | 商品与 SKU 映射 | P0 | 部分具备 | `ChannelMappingsPage`、商品匹配页、mapping flow | 需补批量规则、变更审批、映射冲突治理 |
| 渠道接入 | 渠道订单下载 / 查询 | P0 | 部分具备 | `ShopOrderQueryPage`、`sales/shops` | 需补真正的下载策略和差异追踪 |
| 渠道接入 | 渠道售后同步 | P1 | 部分具备 | `after-sale-cases`、售后相关 API | 需补平台售后状态、举证、退款回写细节 |
| 渠道接入 | 接口重试 / 死信 / 回放 | P0 | 部分具备 | `FailureQueuePage`、`SYS.DLQ` 权限、release gate | 需补人工回放与补偿编排工作台 |

## 3. 商品与主数据

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 商品主数据 | 商品 / SPU / SKU 主档 | P0 | 部分具备 | `ProductsPage`、`products.ts`、`product_sku_schema.md` | 需补 SPU-SKU 层级治理、停售 / 启售规则和归档 |
| 商品主数据 | 条码管理 | P0 | 部分具备 | `ProductBarcodesPage`、条码 API | 需补重复校验、批量导入、扫码规则 |
| 商品主数据 | 单位与换算 | P1 | 部分具备 | `ProductUnitsPage`、单位转换 API | 需补采购单位、仓储单位、销售单位的贯通 |
| 商品主数据 | 包装规格 | P0 | 部分具备 | `ProductPackagingSpecsPage`、包装 API | 需补包材成本、物流体积重联动 |
| 商品主数据 | 组合装 / BOM | P0 | 部分具备 | `KitsPage`、`kit_bom.md` | 需补自动拆解、库存占用和成本分摊 |
| 商品主数据 | 渠道可售与上架 | P0 | 部分具备 | `ProductChannelAvailabilityPage` | 需补按店铺、仓库、活动、黑白名单的可售控制 |
| 商品主数据 | 生命周期与变更控制 | P1 | 部分具备 | `ProductLifecyclePage`、`ProductChangeControlPage` | 需补审核流、版本记录、变更生效时间 |
| 商品主数据 | 渠道货品下载 | P1 | 部分具备 | `ShopProductDownloadPage`、shop products API | 需补增量同步、差异比对和冲突处理 |
| 商品主数据 | 主数据标签体系 | P1 | 部分具备 | 订单标签 / SKU 标签 / 目录页面 | 需补跨域统一标签模型 |
| 商品主数据 | 主数据治理与去重 | P1 | 缺失 | 文档中有目标边界 | 需补数据质量规则、重复检测、主数据责任人机制 |
| 商品主数据 | 供应商货品档案 | P1 | 缺失 | 采购与供应商已有页面但未形成货品主档关联 | 需补供应商 SKU、起订量、交期、价格协议 |
| 商品主数据 | 质检标准 / 质量属性 | P2 | 缺失 | 仅有采购质检与售后质检页面线索 | 需补商品级质检模板、检验项和合格判定 |

## 4. 销售与 OMS

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| OMS | 订单接入与去重 | P0 | 已具备 | `erp-channel`、`erp-worker`、`oms_orders.rs`、架构文档 | 需补跨平台重复单与补单策略 |
| OMS | 订单标准化 | P0 | 部分具备 | 订单字段映射文档、orders API、OrdersPage | 需补地址、税号、发票字段的完整标准化 |
| OMS | 审单工作台 | P0 | 部分具备 | `OrderReviewPage`、审单规则页、Workbench 模板 | 需补真正的审单流配置、规则优先级和命中解释 |
| OMS | 异常中心与 SLA | P0 | 部分具备 | `OrderExceptionsPage`、`OrderExceptionSlaPage`、集成测试 | 需补责任归因、升级路径、异常成本统计 |
| OMS | 拆单 / 合单 | P0 | 部分具备 | `OrderSplitMergePage` 路由存在 | 当前实现深度不足，需补策略与审计闭环 |
| OMS | 无效货品快速修复 | P0 | 已具备 | 重匹配 / 手动匹配接口与审单页 | 需补批量规则、映射冲突提示和责任追踪 |
| OMS | 库存预占与放行 | P0 | 部分具备 | 库存 schema、ATP / FEFO 测试、订单状态机文档 | 需补真实订单占用策略和超卖防护 |
| OMS | 发货协同 / 包裹回写 | P0 | 部分具备 | `ShipmentsPage`、OMS 包裹页、物流上传页 | 需补多包裹、部分发货和平台回传一致性 |
| OMS | 销售结算 | P1 | 部分具备 | `OrderSettlementPage` | 需补账期、扣点、差异处理与核销闭环 |
| OMS | 销售对账 | P1 | 部分具备 | `OrderReconciliationsPage` | 需补平台账单导入、对账规则、差异任务 |
| OMS | 销售发票 | P1 | 部分具备 | 发票中心页面、财务页线索 | 需补订单到开票的端到端联动 |
| OMS | 手工建单 / 导入订单 | P1 | 部分具备 | `SalesOrderCreatePage`、`SalesOrderImportPage` | 需补模板校验、重复单识别、审批 |
| OMS | B2B / 批发订单场景 | P2 | 缺失 | 当前模型偏平台电商订单 | 需补合同价、信用额度、批量发货和账期订单 |

## 5. 仓储与库存

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| WMS | 仓库、库区、货位基础资料 | P0 | 部分具备 | `WarehousesPage`、`WarehouseAreasPage`、`WarehouseLocationsPage` | 需补容量、温区、货位属性与权限控制 |
| WMS | 入库单与收货 | P0 | 部分具备 | `InboundListPage`、`InboundReceivePage`、入库集成测试 | 需补采购、退货、调拨多来源统一 |
| WMS | 质检与上架 | P0 | 部分具备 | `InboundInspectPage`、`InboundPutawayPage` | 需补质检规则模板和不良品闭环 |
| WMS | 出库单与波次 | P0 | 部分具备 | `WmsOutboundsPage`、`WmsWavesPage`、WMS 流程测试 | 需补波次策略版本化和任务看板 |
| WMS | 拣货 / 复核 / 打包 / 发运 | P0 | 部分具备 | `WmsPickTasksPage`、`WmsPackPage`、`WmsShipPage` | 需补条码扫码、包装材消耗、异常回滚 |
| WMS | 出库确认 | P0 | 部分具备 | `WmsOutboundConfirmPage` | 需补承运商揽收与状态回写一致性 |
| WMS | 盘点管理 | P0 | 部分具备 | `StocktakesPage`、`StocktakeTasksPage` | 需补冻结策略、差异审批、追责分析 |
| WMS | 调拨 / 移库 / 报损报溢 | P1 | 部分具备 | `StockTransfersPage`、`StockMovesPage`、`StockAdjustmentsPage` | 需补在途库存、成本转移与审批 |
| WMS | 库存冻结与锁定 | P0 | 部分具备 | `InventoryLocksPage`、`InventoryFreezesPage` | 需补来源归因和自动释放策略 |
| WMS | ATP / FEFO / 批次 / 序列号 | P0 | 部分具备 | `inventory_atp_fefo_integration.rs`、序列化相关测试 | 需补页面执行闭环和业务规则验证 |
| WMS | 劳务、库位优化、补货、月台 | P1 | 部分具备 | `WmsLaborPage`、`WmsSlottingPage`、`WmsReplenishmentPage`、`WmsDockYardPage` | 需补任务调度、绩效与现场执行 |
| WMS | PDA / 扫码执行 | P1 | 缺失 | 当前前端以桌面工作台为主 | 需补移动端或手持端作业界面 |
| WMS | 自动化设备对接 | P2 | 缺失 | 无现成模块 | 需补输送线、自动分拣、电子秤、打印机等设备协同 |

## 6. 物流与履约

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 物流 | 承运商目录 | P0 | 部分具备 | `CouriersPage`、`carriers.ts` | 需补承运商 SLA、区域与服务能力模型 |
| 物流 | 物流产品与运价 | P0 | 部分具备 | `LogisticsProductsPage`、`LogisticsPricesPage` | 需补按店铺 / 仓 / 重量 / 区域自动选路 |
| 物流 | 物流接口配置 | P0 | 部分具备 | `LogisticsInterfacesPage` | 需补接口连通性测试和生产切换流程 |
| 物流 | 物流轨迹查询 | P0 | 部分具备 | `LogisticsTrackingPage` | 需补签收、拒收、妥投异常标准化回写 |
| 物流 | 物流异常工单 | P1 | 部分具备 | `LogisticsExceptionsPage` | 需补自动建单、SLA 与责任归因 |
| 物流 | 物流结算 | P1 | 部分具备 | `LogisticsSettlementsPage` | 需补账单导入、差异核对和承运商对账 |
| 物流 | 配送安装 | P2 | 部分具备 | `LogisticsInstallationsPage` | 需补服务预约、签字回单和服务评价 |
| 物流 | 仓配优化 | P1 | 部分具备 | `WarehouseOptimizationPage` | 需补真实策略引擎和成本模拟 |
| 物流 | 面单与打印联动 | P0 | 部分具备 | 打印设备 / 打印映射 / 打印模板页面 | 需补发货流程自动触发与异常补打 |

## 7. 采购与计划

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 采购 | 采购申请 | P0 | 部分具备 | `ProcurementRequestsPage` | 需补审批、预算和来源需求关联 |
| 采购 | 采购计划 | P1 | 部分具备 | `ProcurementPlansPage` | 需补来源可追溯与供应约束 |
| 采购 | 寻源 / 选品 | P1 | 部分具备 | `ProcurementSourcingPage` | 需补询价、比价、供应商打分 |
| 采购 | 采购订单 | P0 | 部分具备 | `PurchaseOrdersPage`、采购订单详情 | 需补变更、交期、异常和审批流 |
| 采购 | 到货 / 收货 | P0 | 部分具备 | `PurchaseReceiptsPage` | 需补与 WMS 入库的强关联 |
| 采购 | 采购退货 | P1 | 部分具备 | `PurchaseReturnsPage` | 需补供应商索赔和财务回写 |
| 采购 | 供应商对账 | P1 | 部分具备 | `ProcurementReconciliationsPage` | 需补对账单导入、差异闭环、核销 |
| 采购 | 供应商绩效 | P1 | 部分具备 | `SupplierPerformancePage` | 需补交期、质量、价格、服务四维指标 |
| 计划 | 销售预测 | P1 | 部分具备 | `SalesForecastPage` | 需补预测模型、准确率回溯、版本比较 |
| 计划 | 安全库存与补货规则 | P0 | 部分具备 | `ReplenishmentRulesPage` | 需补分仓分级规则与异常库存处理 |
| 计划 | 采购补货计划 | P0 | 部分具备 | `PurchasePlansPage`、补货集成测试 | 需补审批、执行、关闭反馈 |
| 计划 | 调拨建议 | P1 | 部分具备 | `TransferSuggestionsPage` | 需补跨仓成本、在途和优先级 |
| 计划 | 促销备货 | P1 | 部分具备 | `PromoStockingPage` | 需补营销联动和供应保障 |
| 计划 | 执行反馈闭环 | P1 | 部分具备 | `PlanExecutionPage` | 需补计划偏差、反馈回写和责任分析 |

## 8. 客户与售后

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| CRM | 客户档案 | P1 | 部分具备 | `CustomersPage`、客户详情页 | 需补客户 360、历史交易、沟通记录 |
| CRM | 客户分类 / 标签 / 分群 | P1 | 部分具备 | `CustomerCategoriesPage`、标签、分群页面 | 需补自动分群和跨模块引用 |
| CRM | 会员等级 / 积分 / 储值 | P1 | 部分具备 | `MemberLevelsPage`、`CustomerLoyaltyPage` | 需补会员规则和订单联动 |
| CRM | 生命周期运营 | P2 | 部分具备 | `CustomerLifecyclePage` | 需补自动触达、事件分层与运营动作 |
| CRM | 客服工单 | P1 | 部分具备 | `CustomerServiceTicketsPage` | 需补 SLA、知识库、工单归因 |
| 售后 | 售后单主流程 | P0 | 部分具备 | `AfterSalesListPage`、售后详情 / 收货页 | 需补类型细分、状态标准化和财务回写 |
| 售后 | 退款拦截 | P0 | 部分具备 | `AfterSalesInterceptsPage` | 需补规则引擎、审批和平台联动 |
| 售后 | 换货补发 | P0 | 部分具备 | `AfterSalesExchangesPage` | 需补和 OMS / WMS 的下游单据联动 |
| 售后 | 退货质检 / 退货入库 | P0 | 部分具备 | `AfterSalesInspectionsPage`、`ReturnInboundPage` | 需补逆向仓储与质量归因 |
| 售后 | 平台售后聚合 | P1 | 部分具备 | `AfterSaleCaseListPage`、`aftersales_case_schema.md` | 需补平台差异字段与举证材料管理 |
| 售后 | 退款单据 | P1 | 部分具备 | `RefundDocsPage` | 需补财务退款、原路退回和失败重试 |
| 售后 | 售后 SLA / 设置 | P1 | 部分具备 | `AfterSalesSlaPage`、`AfterSalesSettingsPage` | 需补升级、预警、责任归因和统计 |

## 9. 财务与管理会计

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 财务 | 应收管理 | P0 | 部分具备 | `ArManagementPage` | 需补收款、核销、账龄和订单关联 |
| 财务 | 应付管理 | P0 | 部分具备 | `ApManagementPage`、`ApDocsPage` | 需补付款计划、审批和供应商对账联动 |
| 财务 | 对账中心 | P0 | 部分具备 | `ReconcilePage`、销售 / 采购对账页 | 需补多来源账单导入和差异闭环 |
| 财务 | 开票中心 | P1 | 部分具备 | `InvoicesPage` | 需补税率、红冲、发票回写订单 |
| 财务 | 费用分摊 | P1 | 部分具备 | `ExpenseAllocationPage` | 需补规则版本化和多维归集 |
| 财务 | 成本核算 | P1 | 部分具备 | `CostingPage`、库存成本页 | 需补核算方法、期末调账、异常差异分析 |
| 财务 | 利润报表 | P0 | 部分具备 | `ProfitPage`、利润集成测试 | 需补利润口径管理和复盘分析 |
| 财务 | 代销结算 | P2 | 部分具备 | `ConsignmentPage` | 需补协议、扣点、库存归属与核销 |
| 财务 | 税务管理 | P2 | 部分具备 | `TaxManagementPage` | 需补税码、税率生效、发票税务联动 |
| 财务 | 总账 / 期间结账 | P2 | 缺失 | 当前范围仍偏管理会计 | 需决定是否内建或与外部财务系统集成 |
| 财务 | 跨组织 / 跨商户结算 | P2 | 缺失 | 缺少商户层与多公司模型 | 需补主体间往来、内部结算与利润拆分 |

## 10. 营销与价格

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 营销 | 渠道价表 | P1 | 部分具备 | `MarketingPriceListsPage` | 需补按渠道 / 店铺 / 客户等级的价格策略 |
| 营销 | 促销活动 | P1 | 部分具备 | `MarketingPromotionsPage` | 需补促销规则引擎和订单实时计算 |
| 营销 | 赠品策略 | P1 | 部分具备 | `MarketingGiftPoliciesPage` | 需补赠品库存与成本分摊 |
| 营销 | 优惠券 | P2 | 部分具备 | `MarketingCouponsPage` | 需补发行、核销、退款回退 |
| 营销 | 促销费用 | P1 | 部分具备 | `MarketingCostsPage` | 需补与财务费用归集联动 |
| 营销 | 活动复盘 | P1 | 部分具备 | `MarketingPostmortemsPage` | 需补 ROI、转化率和库存影响复盘 |
| 营销 | 会员营销自动化 | P2 | 缺失 | CRM 生命周期页面存在但未连通 | 需补人群包、自动触发和多渠道触达 |

## 11. 报表与经营驾驶舱

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 报表 | 销售统计 | P0 | 部分具备 | `SalesStatsPage`、日报、趋势、销售周期页 | 需补按组织、店铺、渠道钻取 |
| 报表 | 发货统计 | P0 | 部分具备 | `ShippingStatsPage` | 需补承运商维度和履约时效 |
| 报表 | 畅销 / 滞销分析 | P1 | 部分具备 | `BestSellerAnalysisPage`、`SlowMovingAnalysisPage` | 需补补货联动和库存策略建议 |
| 报表 | 库存预警与洞察 | P0 | 部分具备 | `InventoryAlertsPage`、inventory insights API | 需补告警订阅、消息通知和责任归因 |
| 报表 | 利润与经营分析 | P0 | 部分具备 | `ProfitPage`、`ReportsPage` | 需补管理驾驶舱和可配置指标 |
| 报表 | 导出任务中心 | P0 | 部分具备 | `ExportJobsPage`、导出 API | 需补大批量导出、权限和审计 |
| 报表 | 高层驾驶舱 | P1 | 部分具备 | 仪表盘和 reports 页面存在 | 需补 KPI、异常总览、下钻链路 |
| 报表 | 自定义报表 / BI | P2 | 缺失 | 当前项目明确不做自助 BI 平台 | 可选择通过外部 BI 集成替代 |

## 12. 组织、质量与协同

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 组织协同 | 组织架构 / 岗位 | P1 | 缺失 | 当前仅有用户 / 角色模型 | 需补组织树、岗位和职责归属 |
| 组织协同 | 审批中心 | P1 | 部分具备 | 工作流 / 治理链路存在 | 需补统一审批待办与审批记录中心 |
| 组织协同 | 任务中心 / 待办 | P1 | 缺失 | 仪表盘仅有轻量待办展示 | 需补跨模块任务汇总与 SLA |
| 组织协同 | 消息通知 / 机器人 | P1 | 缺失 | 文档有目标，代码中未形成中心 | 需补企业微信 / 邮件 / 站内消息 |
| 质量管理 | 采购质检体系 | P1 | 部分具备 | 采购质检页面存在 | 需补检验标准、抽检、质检结论模板 |
| 质量管理 | 退货质检体系 | P1 | 部分具备 | 售后质检页面存在 | 需补质检结果影响库存、赔付和供应商追责 |
| 质量管理 | 批次追溯与召回 | P2 | 部分具备 | 批次 / 序列号与 FEFO 线索存在 | 需补召回任务和影响分析 |
| 人力与行政 | 员工 / 考勤 / 薪酬 | P2 | 缺失 | 现阶段未建模 | 建议作为扩展域，不纳入首发上线 |
| 办公协同 | 知识库 / SOP / 培训 | P2 | 缺失 | 仅有 Markdown 文档 | 需补产品内知识入口与流程嵌入 |

## 13. 非功能与工程治理

| 功能域 | 能力项 | 级别 | 当前状态 | 当前依据 | 关键差距 / 验收点 |
| --- | --- | --- | --- | --- | --- |
| 工程治理 | OpenAPI 契约治理 | P0 | 已具备 | `openapi/erp_api_v1.yaml`、repo verify | 需持续保证前后端契约一致 |
| 工程治理 | 后端集成测试 | P0 | 已具备 | `crates/erp-api/tests` 多条集成测试 | 需提升领域覆盖和回归分层 |
| 工程治理 | 前端 E2E 门禁 | P0 | 部分具备 | release gate E2E、Playwright 用例 | 需覆盖更多跨域业务链路 |
| 工程治理 | 前端信息架构治理 | P0 | 缺失 | 当前菜单结构仍不稳定 | 需按产品蓝图完成菜单重构 |
| 工程治理 | 研发演示入口隔离 | P0 | 缺失 | `AG Grid Enterprise` 暴露在系统菜单 | 需区分生产菜单与研发入口 |
| 运维治理 | 可观测性 | P0 | 部分具备 | `ObservabilityPage`、相关文档 | 需补业务指标、告警等级和租户视图 |
| 运维治理 | 失败队列 / 补偿 | P0 | 部分具备 | `FailureQueuePage`、DLQ 权限 | 需补人工回放与补偿编排 |
| 运维治理 | 性能基线 | P1 | 部分具备 | `PERF_BASELINE.md`、Grid 优化文档 | 需补高并发压测与容量规划 |
| 运维治理 | 安全与审计 | P0 | 部分具备 | `SECURITY_GATE.md`、审计文档 | 需补凭据轮换、敏感字段脱敏和合规报表 |
| 运维治理 | 数据保留 / 备份 / 灾备 | P0 | 部分具备 | `DATA_RETENTION.md`、`DR_RUNBOOK.md` | 需补定期演练与恢复验收记录 |
| 运维治理 | SSO 与统一身份 | P1 | 部分具备 | `stats_clickhouse_sso_smoke.rs`、`SSO_RUNBOOK.md` | 需扩展到正式用户体系与权限同步 |
| 运维治理 | 发布门禁 | P0 | 已具备 | `RELEASE_GATE.md`、`release:gate:full` | 需持续把新能力纳入门禁 |

## 14. 用法建议

- 评估当前项目完成度时，以本表逐项打分，不再只按“页面数量”估算。
- 规划首个收费上线版本时，仅允许选择 `P0` 且状态达到“已具备”或“部分具备但有明确补齐计划”的能力。
- 规划后续版本时，优先补 `P1` 中直接影响经营闭环、财务闭环、协同效率和菜单信息架构的能力。
